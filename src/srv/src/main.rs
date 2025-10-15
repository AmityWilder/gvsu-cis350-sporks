//! # gvsu-cis350-sporks
//!
//! A management scheduling application (generator end; executed by backend)

#![deny(
    clippy::undocumented_unsafe_blocks,
    clippy::missing_safety_doc,
    reason = "multi-person projects should document dangers"
)]
#![warn(missing_docs)]
#![cfg_attr(
    not(any(test, debug_assertions)),
    deny(
        clippy::missing_panics_doc,
        clippy::panic,
        clippy::unimplemented,
        clippy::unwrap_used,
        // clippy::expect_used,
        // clippy::unreachable,
        reason = "prefer errors over panicking"
    )
)]
#![cfg_attr(
    not(any(test, debug_assertions)),
    forbid(clippy::todo, reason = "production code should not use `todo`")
)]

use anyhow::{anyhow, bail};
use chrono::prelude::*;
use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use math::Graph;
use miette::{
    Context, Diagnostic, LabeledSpan, NamedSource, Result, Severity, SourceOffset, SourceSpan,
    miette,
};
use serde::{Deserialize, Serialize, de::Visitor};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::BufReader,
    ops::Range,
    path::PathBuf,
};
use thiserror::Error;

pub mod math;

/// Code uniquely identifying a user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(u32);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "u{:x}", self.0)
    }
}

/// Code uniquely identifying a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(u64);

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{:x}", self.0)
    }
}

/// Code uniquely identifying a skill - used to determine which users *can* be scheduled on a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(u32);

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s{:x}", self.0)
    }
}

/// Metadata regarding a skill
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skill {
    /// Display name of the skill
    pub name: String,
    /// Description of the skill
    pub desc: String,
}

/// Preference/opposition score.
///
/// Range: `-inf, -1.0..=1.0, inf`
///
/// Infinities should be reserved for cases where failure to meet the requirement
/// would cause legal or other undesireable problems and should be *hard*-rejected.
///
/// # Values
///
/// ## `0.0`
/// No preference.
///
/// Equivalent to not being listed at all; which should be preferred for storage reasons.
///
/// ## `1.0`
/// Maximize scheduling. Only do otherwise if no other option.
///
/// ## `-1.0`
/// Minimize scheduling. Only do otherwise if no other option.
///
/// ## [`+inf`](`f32::INFINITY`)
/// **Always** schedule.
///
/// ### Towards time
/// If unable to be scheduled at this time, **schedule production should result in an error requiring manager input.**
///
/// **ex:** leader at critical event
///
/// ### Towards users
/// If unable to be scheduled *together*, **do not schedule *this* user.**
///
/// **ex:** handler
///
/// ## [`-inf`](`f32::NEG_INFINITY`)
/// **Never** schedule.
///
/// ### Towards time
/// If unable to be scheduled any other time, **do not schedule *this* user.**
///
/// **ex:** sick, mourning, vacation; physically *unable* to be present.
///
/// ### Towards users
/// If unable to be scheduled *separately*, **do not schedule *that* user.**
///
/// **ex:** restraining order, history of harassment
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Preference(f32);

impl std::fmt::Display for Preference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_infinite() {
            write!(f, "{}inf", b"+-"[self.0.is_sign_negative() as usize])
        } else if self.0.is_nan() {
            f.write_str("NaN")
        } else {
            write!(f, "{}%", self.0 * 100.0)
        }
    }
}

impl std::ops::Deref for Preference {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Preference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Preference {
    /// Mandatory
    pub const INFINITY: Self = Self(f32::INFINITY);
    /// Forbidden
    pub const NEG_INFINITY: Self = Self(f32::NEG_INFINITY);
    /// Maximum (100%) refusal
    pub const MIN: Self = Self(-1.0);
    /// Maximum (100%) preference
    pub const MAX: Self = Self(1.0);

    /// Clamp to `-inf, 0.0..=1.0, +inf`
    pub const fn saturate(self) -> Self {
        if self.0.is_infinite() {
            self
        } else {
            Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
        }
    }
}

/// Level of skill
///
/// 0.0 = no skill.
/// 1.0 = skill of one user with baseline skill.
/// Can be multiplied by number of users.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Proficiency(f32);

impl std::fmt::Display for Proficiency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_infinite() {
            write!(f, "{}inf", b"+-"[self.0.is_sign_negative() as usize])
        } else if self.0.is_nan() {
            f.write_str("NaN")
        } else {
            write!(f, "{}%", self.0 * 100.0)
        }
    }
}

impl std::ops::Deref for Proficiency {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Proficiency {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Proficiency {
    /// No proficiency
    pub const ZERO: Self = Self(0.0);
    /// Baseline proficiency
    pub const ONE: Self = Self(1.0);
    /// Alias for [`Self::ZERO`]
    pub const MIN: Self = Self::ZERO;
    /// Alias for [`f32::MAX`]
    pub const MAX: Self = Self(f32::MAX);

    /// Clamp between [`Self::MIN`] and [`Self::MAX`]
    pub const fn saturate(self) -> Self {
        Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
    }
}

/// A timerange, mainly intended for timeslots.
///
/// # [Ordering](`Ord`)
///
/// [`TimeInterval`] is ordered by start, then end.
/// In other words, if [`TimeInterval`] `a` starts before [`TimeInterval`] `b`,
/// then `a` will be ordered ahead of `b` no matter when either ends.
/// However, if both start at the same time and date, then the one that ends first
/// will be ordered ahead of the one that ends later.
///
/// The main purpose of implementing [`Ord`] for [`TimeInterval`] is so that
/// it can be used as a key in a [`BTreeMap`](`std::collections::BTreeMap`)
/// or [`BTreeSet`](`std::collections::BTreeSet`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct TimeInterval(pub Range<DateTime<Utc>>);

/// Custom [`Deserialize`] implementation needed for reading [`TimeInterval`] as map keys.
///
/// ```
/// # use {std::collections::BTreeMap, crate::TimeInterval, serde_json::{self, json}};
/// let events = serde_json::from_value::<BTreeMap<TimeInterval, Vec<String>>>(json!({
///     "2025-09-23T19:44:54+00:00..2025-09-23T19:45:54+00:00": [
///         "foo",
///         "bar"
///     ]
/// }));
///
/// assert_eq!(
///     events.unwrap(),
///     BTreeMap::from_iter([(
///         TimeInterval(
///             "2025-09-23T19:44:54+00:00".parse().unwrap()
///                 .."2025-09-23T19:45:54+00:00".parse().unwrap()
///         ),
///         vec!["foo".to_string(), "bar".to_string()]
///     )])
/// );
/// ```
impl<'de> Deserialize<'de> for TimeInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TimeIntervalVisitor;
        use serde::de::Error;

        impl<'de> Visitor<'de> for TimeIntervalVisitor {
            type Value = TimeInterval;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct TimeInterval")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let start = seq
                    .next_element::<DateTime<Utc>>()?
                    .ok_or_else(|| Error::invalid_length(0, &self))?;
                let end = seq
                    .next_element::<DateTime<Utc>>()?
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                Ok(TimeInterval(start..end))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(field_identifier, rename_all = "lowercase")]
                enum Field {
                    Start,
                    End,
                }

                let mut start = None;
                let mut end = None;
                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        Field::Start => {
                            if start.is_some() {
                                return Err(Error::duplicate_field("start"));
                            }
                            start = Some(value);
                        }
                        Field::End => {
                            if end.is_some() {
                                return Err(Error::duplicate_field("end"));
                            }
                            end = Some(value);
                        }
                    }
                }
                let start = start.ok_or_else(|| Error::missing_field("start"))?;
                let end = end.ok_or_else(|| Error::missing_field("end"))?;
                Ok(TimeInterval(start..end))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let (start, end) = v
                    .split_once("..")
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                let start = start.parse::<DateTime<Utc>>().map_err(Error::custom)?;
                let end = end.parse::<DateTime<Utc>>().map_err(Error::custom)?;
                Ok(TimeInterval(start..end))
            }
        }

        deserializer
            .deserialize_map(TimeIntervalVisitor)
            .and_then(|interval| {
                if interval.start <= interval.end {
                    Ok(interval)
                } else {
                    Err(Error::invalid_value(
                        serde::de::Unexpected::Other("time-reversed interval"),
                        &TimeIntervalVisitor,
                    ))
                }
            })
    }
}

impl std::ops::Deref for TimeInterval {
    type Target = Range<DateTime<Utc>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for TimeInterval {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialOrd for TimeInterval {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeInterval {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.0.start.cmp(&other.0.start) {
            std::cmp::Ordering::Equal => self.0.end.cmp(&other.0.end),
            ord => ord,
        }
    }
}

/// A person who can be scheduled to work on a task.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    /// Display name for representing the user on the manager-facing UI.
    /// Can be changed without changing the user's ID.
    pub name: String,

    /// Preferences regarding times the user can or can't be scheduled.
    ///
    /// Ex:
    /// - "available every Monday 3pm-7pm",
    /// - "never available on Fridays"
    pub availability: BTreeMap<TimeInterval, Preference>,

    /// Preference towards sharing slots with other users.
    ///
    /// Ex:
    /// - "doesn't like Brian"
    /// - "works better when Sally is there"
    pub user_prefs: HashMap<UserId, Preference>,

    /// A dictionary of the user's skills and their capability with each skill.
    ///
    /// Skills the user has 0 proficiency with should be excluded to save memory,
    /// as a missing skill is implied to be 0% proficiency.
    pub skills: HashMap<SkillId, Proficiency>,
}

/// Proficiency requirements for a skill on a [`Task`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProficiencyReq {
    /// The ideal proficiency.
    pub target: Proficiency,

    /// The lower bound of the target - try to stay above this.
    pub soft_min: Proficiency,
    /// The upper bound of the target - try to stay below this.
    pub soft_max: Proficiency,

    /// The lower bound - reject any solution below this.
    pub hard_min: Proficiency,
    /// The upper bound - reject any solution above this.
    pub hard_max: Proficiency,
}

impl ProficiencyReq {
    /// Construct a new [`ProficiencyReq`] from ideal and ranges.
    pub fn new<R1, R2>(target: Proficiency, soft_range: R1, hard_range: R2) -> Option<Self>
    where
        R1: std::ops::RangeBounds<Proficiency>,
        R2: std::ops::RangeBounds<Proficiency>,
    {
        trait BoundExt<T> {
            fn get(self) -> Option<T>;
        }

        impl<T: Copy> BoundExt<T> for std::ops::Bound<&T> {
            fn get(self) -> Option<T> {
                match self {
                    std::ops::Bound::Included(x) | std::ops::Bound::Excluded(x) => Some(*x),
                    std::ops::Bound::Unbounded => None,
                }
            }
        }

        fn range_to_vals(
            range: impl std::ops::RangeBounds<Proficiency>,
        ) -> (Proficiency, Proficiency) {
            (
                range.start_bound().get().unwrap_or(Proficiency::MIN),
                range.end_bound().get().unwrap_or(Proficiency::MAX),
            )
        }

        let (hard_min, hard_max) = range_to_vals(hard_range);
        let (soft_min, soft_max) = range_to_vals(soft_range);
        (hard_min <= soft_min && soft_max <= hard_max).then_some(Self {
            target,
            soft_min,
            soft_max,
            hard_min,
            hard_max,
        })
    }
}

/// A product or service to be completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// The name of the task.
    pub title: String,

    /// The task description.
    pub desc: String,

    /// Skills required to perform the task.
    ///
    /// Optimize covering with users whose combined capability equals the float provided (maxed out at 1.0 per individual)
    /// Prefer to overshoot (except in great excess, like 200+%) rather than undershoot, but prioritizing closer matches.
    pub skills: HashMap<SkillId, ProficiencyReq>,

    /// [`None`]: Task has no "completion" state.
    pub deadline: Option<DateTime<Utc>>,

    /// Tasks that must be completed before this one can be scheduled
    /// (estimated by deadlines).
    pub awaiting: HashSet<TaskId>,
}

/// A segment of time that can be allocated for work, such as a "shift".
///
/// Slots are ordered by their [`interval`](`Slot::interval`)
/// (See [`TimeInterval` ordering](TimeInterval#ordering)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// The time period the slot refers to.
    pub interval: TimeInterval,
}

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.interval.partial_cmp(&other.interval)
    }
}

/// A collection of time slots along with the tasks and users assigned to them.
#[derive(Debug, Serialize, Deserialize)]
pub struct Schedule {
    /// Timeslots and their assignments.
    pub slots: Vec<(Slot, HashSet<TaskId>, HashSet<UserId>)>,
}

/// Error generated while attempting to create a schedule.
///
/// Requires prompting manager to resolve.
#[derive(Debug, Error)]
pub enum SchedulingError {
    /// A task was encountered that is not in the provided `tasks` dictionary.
    #[error("task {_0} does not exist")]
    NonExistentTask(TaskId),
}

impl Schedule {
    /// Generate a schedule based on the provided requirements.
    ///
    /// # Prioritization
    ///
    // Markdown tip:
    // using `1.` over and over allows the order to be moved around without replacing the numbers on all the other items.
    // Markdown notices the repeated `1.`s and replaces them with the number that describes their actual order.
    ///
    /// In descending order of importance:
    ///
    /// 1. Minimize legal issues[^legal]
    /// 1. Maximize task completion
    /// 1. Minimize deadlines missed
    /// 1. Maximize tasks completed ahead of deadline
    ///    - Descending order of quantity of dependents[^deps]
    /// 1. Maximize user scheduling preferences fulfilled
    ///    - Descending order of preference magnitude[^pref-mag]
    /// 1. Minimize quantity of users scheduled simultaneously
    ///
    /// [^legal]: [`Preference`] of &pm;inf ([`Preference::INFINITY`]/[`Preference::NEG_INFINITY`]).
    /// [^deps]: [`Task`] `a` is &lt;a dependent of/dependant on&gt; [`Task`] `b` if `a`'s [`awaiting`](Task::awaiting)-field contains `b`.
    /// [^pref-mag]: A [`Preference`] is of higher magnitude when it is further from zero; i.e. [`f32::abs`]
    ///
    /// TODO: consider using [Dinic's Algorithm](https://en.wikipedia.org/wiki/Dinic%27s_algorithm)
    pub fn generate(
        _slots: &[Slot],
        tasks: &HashMap<TaskId, Task>,
        _users: &HashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        use SchedulingError::*;

        let dep_graph = Graph::from_forward(
            tasks
                .iter()
                .map(|(&a, Task { awaiting: bs, .. })| (a, bs.iter().copied())),
        )
        .ok_or_else(|| todo!())?;

        // use BFS to sort the graph
        // tasks must create a DAG (no cycles)
        let dep_order = dep_graph
            .bfs(
                dep_graph
                    .verts()
                    .copied()
                    .filter(|v| !dep_graph.receivers().contains(v)),
            )
            .collect::<Vec<_>>();

        // debug
        println!("task order:");
        for (n, id) in dep_order.into_iter().enumerate() {
            let title = tasks.get(&id).ok_or(NonExistentTask(id))?.title.as_str();
            println!("{n:>4}. {title} ({id})");
        }

        todo!()
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;

    #[test]
    fn test0() {
        let slots = vec![];
        let tasks = [
            (
                TaskId(5436),
                Task {
                    title: "foo".to_string(),
                    desc: String::new(),
                    skills: HashMap::new(),
                    deadline: None,
                    awaiting: HashSet::from_iter([]),
                },
            ),
            (
                TaskId(2537),
                Task {
                    title: "bar".to_string(),
                    desc: String::new(),
                    skills: HashMap::new(),
                    deadline: None,
                    awaiting: HashSet::from_iter([TaskId(5436)]),
                },
            ),
            (
                TaskId(3423),
                Task {
                    title: "baz".to_string(),
                    desc: String::new(),
                    skills: HashMap::new(),
                    deadline: None,
                    awaiting: HashSet::from_iter([TaskId(5436)]),
                },
            ),
        ]
        .into_iter()
        .collect();
        let users = [].into_iter().collect();
        dbg!(Schedule::generate(&slots, &tasks, &users)).unwrap();
    }
}

/// Error while trying to read command-line arguments.
///
/// Not currently recoverable.
#[derive(Debug, Error)]
pub enum ArgsError {
    /// Error reading arguments
    #[error("argument error")]
    LexoptError(#[from] lexopt::Error),

    /// Error involving filesystem
    #[error("filesystem error")]
    IOError(#[from] std::io::Error),

    /// Repetition of argument that should not be repeated
    #[error("data should only be provided once")]
    DuplicateArg,
}

const STYLE: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::BrightCyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

/// Sporks scheduling software
#[derive(Debug, Parser)]
#[command(version, propagate_version = true, about, long_about = None, styles = STYLE, color = clap::ColorChoice::Always)]
pub struct Cli {
    /// Provide path to user data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./users.json"))]
    users: PathBuf,

    /// Provide path to timeslot data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./slots.json"))]
    slots: PathBuf,

    /// Provide path to task data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./tasks.json"))]
    tasks: PathBuf,

    /// Provide path to output data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./schedule.json"))]
    output: PathBuf,
}

fn main() -> Result<()> {
    let Cli {
        users,
        slots,
        tasks,
        output,
    } = match Cli::try_parse() {
        Ok(x) => Ok(x),
        Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
            return e.print().map_err(miette::Error::from_err);
        }
        Err(e) => Err(miette::Error::from_err(e)),
    }?;

    let users: HashMap<UserId, User> =
        serde_json::from_reader(BufReader::new(File::open(&users).map_err(|e| {
            let source = users.display().to_string();
            miette!(
                severity = Severity::Error,
                labels = vec![LabeledSpan::at(0..source.len(), e.to_string())],
                help = "make sure the file exists and can be read",
                "could not load user data"
            )
            .with_source_code(source)
        })?))
        .map_err(miette::Error::from_err)?;

    let slots: Vec<Slot> =
        serde_json::from_reader(BufReader::new(File::open(&slots).map_err(|e| {
            let source = slots.display().to_string();
            miette!(
                severity = Severity::Error,
                labels = vec![LabeledSpan::at(0..source.len(), e.to_string())],
                help = "make sure the file exists and can be read",
                "could not load slot data"
            )
            .with_source_code(source)
        })?))
        .map_err(miette::Error::from_err)?;

    let tasks: HashMap<TaskId, Task> =
        serde_json::from_reader(BufReader::new(File::open(&tasks).map_err(|e| {
            let source = tasks.display().to_string();
            miette!(
                severity = Severity::Error,
                labels = vec![LabeledSpan::at(0..source.len(), e.to_string())],
                help = "make sure the file exists and can be read",
                "could not load task data"
            )
            .with_source_code(source)
        })?))
        .map_err(miette::Error::from_err)?;

    let schedule = Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users))
        .map_err(miette::Error::from_err)?;

    serde_json::to_writer(
        File::create(output).map_err(miette::Error::from_err)?,
        &dbg!(schedule),
    )
    .map_err(miette::Error::from_err)?;

    Ok(())
}
