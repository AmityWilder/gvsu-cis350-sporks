#![deny(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc)]

use chrono::prelude::*;
use colored::Colorize;
use lexopt::prelude::*;
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, Visitor},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::BufReader,
    ops::Range,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct UserId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct TaskId(u64);

/// Preference/opposition score.
///
/// Range: `-INF, -1.0..=+1.0, INF`
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
/// ## [`+INF`](`f32::INFINITY`)
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
/// ## [`-INF`](`f32::NEG_INFINITY`)
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
type Preference = f32;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
struct TimeInterval(pub Range<DateTime<Utc>>);

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
struct User {
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
}

/// A product or service to be completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    pub title: String,
    pub desc: String,
    pub deadline: Option<DateTime<Utc>>,
    pub awaiting: HashSet<TaskId>,
}

/// A segment of time that can be allocated for work, such as a "shift".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Slot {
    pub interval: TimeInterval,
}

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.interval.partial_cmp(&other.interval)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Schedule {
    pub slots: Vec<(Slot, HashSet<TaskId>, HashSet<UserId>)>,
}

#[derive(Debug, Error)]
pub enum SchedulingError {}

impl Schedule {
    pub fn generate(
        _slots: &[Slot],
        _tasks: &HashMap<TaskId, Task>,
        _users: &HashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        todo!()
    }
}

#[derive(Debug, Error)]
enum ArgsError {
    #[error("argument error")]
    LexoptError(#[from] lexopt::Error),
    #[error("filesystem error")]
    IOError(#[from] std::io::Error),
    #[error("data should only be provided once")]
    DataReassigned,
}

#[derive(Debug)]
struct CmdLineData {
    pub users_path: PathBuf,
    pub slots_path: PathBuf,
    pub tasks_path: PathBuf,
}

/// Parse command line arguments for data.
fn get_data(mut parser: lexopt::Parser) -> Result<CmdLineData, ArgsError> {
    #![deny(
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::unreachable,
        reason = "only errors in this fn pretty please"
    )]

    const USERS_PATH_DEFAULT: &str = "./users.json";
    const SLOTS_PATH_DEFAULT: &str = "./slots.json";
    const TASKS_PATH_DEFAULT: &str = "./tasks.json";

    let mut users_path = None;
    let mut slots_path = None;
    let mut tasks_path = None;

    while let Some(arg) = parser.next()? {
        match arg {
            Short('u') | Long("users") => {
                if users_path.is_none() {
                    users_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DataReassigned)?
                }
            }
            Short('s') | Long("slots") => {
                if slots_path.is_none() {
                    slots_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DataReassigned)?
                }
            }
            Short('t') | Long("tasks") => {
                if tasks_path.is_none() {
                    tasks_path = Some(PathBuf::from(parser.value()?));
                } else {
                    Err(ArgsError::DataReassigned)?
                }
            }

            Long("help") => {
                println!(
                    "{0} {1} {2}\
                    \n\
                    \n{3}\
                    \n  {4}, {7} {11}  Provide path to user data file, otherwise default to {USERS_PATH_DEFAULT}\
                    \n  {5}, {8} {11}  Provide path to slot data file, otherwise default to {SLOTS_PATH_DEFAULT}\
                    \n  {6}, {9} {11}  Provide path to task data file, otherwise default to {TASKS_PATH_DEFAULT}\
                    \n      {10}          Display this message",
                    "Usage:".bold().bright_green(),
                    parser
                        .bin_name()
                        .unwrap_or("gvsu-cis350-sporks")
                        .bold()
                        .bright_cyan(),
                    "[OPTIONS]".cyan(),
                    "Options:".bold().bright_green(),
                    "-u".bold().bright_cyan(),
                    "-s".bold().bright_cyan(),
                    "-t".bold().bright_cyan(),
                    "--users".bold().bright_cyan(),
                    "--slots".bold().bright_cyan(),
                    "--tasks".bold().bright_cyan(),
                    "--help".bold().bright_cyan(),
                    "<PATH>".cyan(),
                );
                std::process::exit(0);
            }

            _ => Err(arg.unexpected())?,
        }
    }

    Ok(CmdLineData {
        users_path: users_path.unwrap_or_else(|| PathBuf::from(USERS_PATH_DEFAULT)),
        slots_path: slots_path.unwrap_or_else(|| PathBuf::from(SLOTS_PATH_DEFAULT)),
        tasks_path: tasks_path.unwrap_or_else(|| PathBuf::from(TASKS_PATH_DEFAULT)),
    })
}

/// Wrapper so that main can provide standardized error printing
fn inner_main() -> Result<(), Box<dyn std::error::Error>> {
    let CmdLineData {
        users_path,
        slots_path,
        tasks_path,
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

    let schedule = Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users))?;

    dbg!(schedule);

    Ok(())
}

fn main() {
    if let Err(e) = inner_main() {
        let mut err: Option<&dyn std::error::Error> = Some(e.as_ref());
        let mut i = 0;
        while let Some(e) = err {
            eprintln!("{:indent$}{e}", "", indent = i);
            i += 2;
            err = e.source();
        }
        std::process::exit(1);
    }
}
