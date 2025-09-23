//! # gvsu-cis350-sporks
//!
//! A management scheduling application (generator end; executed by backend)
//!
//! This program is the scheduling server run by the backend.

#![forbid(
    clippy::undocumented_unsafe_blocks,
    clippy::missing_safety_doc,
    clippy::missing_panics_doc,
    reason = "multi-person projects should document dangers"
)]
#![deny(
    clippy::panic,
    clippy::unimplemented,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unreachable,
    reason = "prefer errors over panicking"
)]
#![warn(missing_docs)]
#![cfg_attr(
    not(debug_assertions),
    forbid(clippy::todo, reason = "production code should not use `todo`")
)]

use chrono::prelude::*;
use colored::Colorize;
use lexopt::prelude::*;
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, Visitor},
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::{BufReader, stdin},
    ops::Range,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Code uniquely identifying a user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(u32);

/// Code uniquely identifying a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(u64);

/// Code uniquely identifying a skill - used to determine which users *can* be scheduled on a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(u32);

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
    /// Other errors
    #[error("{0}")]
    Other(Cow<'static, str>),
}

impl Schedule {
    /// Generate a schedule based on the provided requirements.
    pub fn generate(
        _slots: &[Slot],
        _tasks: &HashMap<TaskId, Task>,
        _users: &HashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        Err(SchedulingError::Other(Cow::Borrowed("not yet implemented")))
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

#[derive(Debug)]
struct CmdLineData {
    pub users_path: PathBuf,
    pub slots_path: PathBuf,
    pub tasks_path: PathBuf,
    pub output_path: PathBuf,
}

/// Parse command line arguments for data.
fn get_data(mut parser: lexopt::Parser) -> Result<CmdLineData, ArgsError> {
    macro_rules! default_path {
        (user) => {
            "./users.json"
        };
        (slot) => {
            "./slots.json"
        };
        (task) => {
            "./tasks.json"
        };
        (output) => {
            "./schedule.json"
        };
    }

    let mut users_path = None;
    let mut slots_path = None;
    let mut tasks_path = None;
    let mut output_path = None;

    while let Some(arg) = parser.next()? {
        match arg {
            Short('u') | Long("users") => {
                if users_path.is_none() {
                    users_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DuplicateArg)?
                }
            }
            Short('s') | Long("slots") => {
                if slots_path.is_none() {
                    slots_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DuplicateArg)?
                }
            }
            Short('t') | Long("tasks") => {
                if tasks_path.is_none() {
                    tasks_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DuplicateArg)?
                }
            }
            Short('o') | Long("output") => {
                if output_path.is_none() {
                    output_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DuplicateArg)?
                }
            }

            Short('h') | Long("help") => {
                #[derive(Debug, Default)]
                struct Value<'a> {
                    /// should be uppercase
                    pub name: &'a str,
                    /// `[NAME]` instead of `<NAME>`
                    pub optional: bool,
                    /// `...`
                    pub variadic: bool,
                }

                impl std::fmt::Display for Value<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        let name = self.name;
                        let (open, close) = if self.optional {
                            ('[', ']')
                        } else {
                            ('<', '>')
                        };
                        let trail = if self.variadic { "..." } else { "" };
                        write!(f, "{open}{name}{close}{trail}")
                    }
                }

                impl<'a> Value<'a> {
                    pub const fn new(name: &'a str) -> Self {
                        Self {
                            name,
                            optional: false,
                            variadic: false,
                        }
                    }

                    /// Mark the value as optional (wrap with `[]` instead of `<>`).
                    pub const fn optional(mut self) -> Self {
                        self.optional = true;
                        self
                    }

                    /// Mark the value as variadic (append with `...`).
                    pub const fn variadic(mut self) -> Self {
                        self.variadic = true;
                        self
                    }

                    /// The length of the display string in bytes.
                    pub const fn len(&self) -> usize {
                        self.name.len()
                            + if self.optional { "[]" } else { "<>" }.len()
                            + if self.variadic { "..." } else { "" }.len()
                    }
                }

                #[derive(Debug, Default)]
                struct RunOption<'a> {
                    pub short: Option<char>,
                    pub long: Option<&'a str>,
                    pub vals: &'a [Value<'a>],
                    pub msg: &'a str,
                }

                impl<'a> RunOption<'a> {
                    pub const fn new(msg: &'a str) -> Self {
                        Self {
                            msg,
                            short: None,
                            long: None,
                            vals: &[],
                        }
                    }

                    pub const fn short(mut self, ch: char) -> Self {
                        self.short = Some(ch);
                        self
                    }

                    pub const fn long(mut self, s: &'a str) -> Self {
                        self.long = Some(s);
                        self
                    }

                    pub const fn values(mut self, vals: &'a [Value<'a>]) -> Self {
                        self.vals = vals;
                        self
                    }
                }

                static USAGES: [&[(bool, &str)]; 1] =
                    [&[(true, "gvsu-cis350-sporks"), (false, "[OPTIONS]")]];

                static OPTIONS: [RunOption; 5] = [
                    // --users
                    RunOption::new(concat!(
                        "Provide path to user data file, otherwise default to ",
                        default_path!(user)
                    ))
                    .short('u')
                    .long("users")
                    .values(&[Value::new("PATH")]),
                    // --slots
                    RunOption::new(concat!(
                        "Provide path to slot data file, otherwise default to ",
                        default_path!(slot)
                    ))
                    .short('s')
                    .long("slots")
                    .values(&[Value::new("PATH")]),
                    // --tasks
                    RunOption::new(concat!(
                        "Provide path to task data file, otherwise default to ",
                        default_path!(task)
                    ))
                    .short('t')
                    .long("tasks")
                    .values(&[Value::new("PATH")]),
                    // --output
                    RunOption::new(concat!(
                        "Provide path to output schedule to, otherwise default to ",
                        default_path!(output)
                    ))
                    .short('o')
                    .long("output")
                    .values(&[Value::new("PATH")]),
                    // --help
                    RunOption::new("Display this message")
                        .short('h')
                        .long("help"),
                ];

                print!("{}", "Usage:".bold().bright_green());
                for usage in USAGES {
                    for (bold, text) in usage {
                        print!(
                            " {}",
                            if *bold {
                                text.bright_cyan().bold()
                            } else {
                                text.cyan()
                            }
                        );
                    }
                    println!();
                    print!("{:indent$}", "", indent = "Usage:".len());
                }
                println!();

                println!("{}", "Options:".bold().bright_green());

                let longest_short = if OPTIONS.iter().any(|opt| opt.short.is_some()) {
                    "-*".len()
                } else {
                    0
                };

                let longest_long = OPTIONS
                    .iter()
                    .map(|opt| opt.long)
                    .filter_map(|x| x.map(|x| "--".len() + x.len()))
                    .max()
                    .unwrap_or(0);

                let longest_args = OPTIONS
                    .iter()
                    .map(|opt| opt.vals)
                    .filter(|x| !x.is_empty())
                    .map(|vals| vals.iter().map(|s| s.len() + " ".len()).sum::<usize>())
                    .max()
                    .unwrap_or(0);

                for option in &OPTIONS {
                    print!(
                        "  {:>short_width$}{} {:<long_width$} {:<args_width$} {}",
                        option.short.map_or_else(
                            || "".normal(),
                            |ch| format!("-{ch}").bold().bright_cyan()
                        ),
                        if option.short.is_some() { ',' } else { ' ' },
                        option
                            .long
                            .map_or_else(|| "".normal(), |s| format!("--{s}").bold().bright_cyan()),
                        option
                            .vals
                            .iter()
                            // I know this isn't the same as `join`.
                            // The trailing space is desired and this saves allocations.
                            .map(|v| format!("{v} "))
                            .collect::<String>()
                            .cyan(),
                        option.msg,
                        short_width = longest_short,
                        long_width = longest_long,
                        args_width = longest_args,
                    );
                    println!();
                }

                std::process::exit(0);
            }

            _ => Err(arg.unexpected())?,
        }
    }

    Ok(CmdLineData {
        users_path: users_path.unwrap_or_else(|| PathBuf::from(default_path!(user))),
        slots_path: slots_path.unwrap_or_else(|| PathBuf::from(default_path!(slot))),
        tasks_path: tasks_path.unwrap_or_else(|| PathBuf::from(default_path!(task))),
        output_path: output_path.unwrap_or_else(|| PathBuf::from(default_path!(output))),
    })
}

/// Wrapper so that main can provide standardized error printing
fn inner_main() -> Result<(), Box<dyn std::error::Error>> {
    let CmdLineData {
        users_path,
        slots_path,
        tasks_path,
        output_path,
    } = get_data(lexopt::Parser::from_env())?;

    fn load_from_path<T>(path: impl AsRef<Path>) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        serde_json::from_reader(BufReader::new(File::open(path)?)).map_err(Into::into)
    }

    let users = load_from_path::<HashMap<UserId, User>>(users_path)?;
    let slots = load_from_path::<Vec<Slot>>(slots_path)?;
    let tasks = load_from_path::<HashMap<TaskId, Task>>(tasks_path)?;

    // let schedule = Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users))?;
    // serde_json::to_writer(File::create(output_path)?, &dbg!(schedule))?;

    let mut buf = String::new();
    loop {
        buf.clear();
        stdin().read_line(&mut buf)?;
        let mut parser = lexopt::Parser::from_args(buf.split_whitespace());
        while let Some(arg) = parser.next()? {
            match arg {
                Long("exit") => return Ok(()),
                _ => Err(arg.unexpected())?,
            }
        }
    }
}

/// Recursively print the error and its sources
fn printerr(e: &dyn std::error::Error) {
    let mut err = Some(e);
    let mut i = 0;
    while let Some(e) = err {
        eprintln!("{:indent$}{e}", "", indent = i);
        i += 2;
        err = e.source();
    }
}

fn main() {
    if let Err(e) = inner_main() {
        printerr(e.as_ref());
        std::process::exit(1);
    }
}
