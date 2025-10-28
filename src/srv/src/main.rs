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

use crate::data::{Slot, Task, TaskId, TaskMap, User, UserId, UserMap};
use chrono::{DateTime, Utc};
use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use miette::{IntoDiagnostic, LabeledSpan, NamedSource, Result, SourceOffset, miette};
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::BufReader,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed},
};
use xml_rpc::{Fault, Server};

pub mod algo;
pub mod data;

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
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./users.csv"))]
    users: PathBuf,

    /// Provide path to timeslot data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./slots.csv"))]
    slots: PathBuf,

    /// Provide path to task data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./tasks.csv"))]
    tasks: PathBuf,

    /// Provide path to output data file
    #[arg(short, long, value_name = "PATH", default_value_os_t = PathBuf::from("./schedule.csv"))]
    output: PathBuf,
}

/// A handle that indicates it the server has started, then
/// indicates that the server has closed when the application ends.
struct RunningHandle(());

impl Drop for RunningHandle {
    fn drop(&mut self) {
        println!("srv: closed")
    }
}

impl RunningHandle {
    pub fn init() -> Self {
        println!("srv: running");
        Self(())
    }
}

fn main() -> Result<()> {
    let Cli {
        users,
        slots,
        tasks,
        output: _,
    } = match Cli::try_parse() {
        Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
            return e.print().into_diagnostic();
        }
        cli => cli.into_diagnostic(),
    }?;

    fn try_load<T: Serialize + DeserializeOwned + Default>(
        path: &Path,
        name: &'static str,
    ) -> Result<T> {
        match File::open(path) {
            // successfully loaded
            Ok(file) => serde_json::from_reader(BufReader::new(file)).map_err(|e| {
                let source = std::fs::read_to_string(path).unwrap();
                miette!(
                    labels = vec![LabeledSpan::new_primary_with_span(
                        Some(e.to_string()),
                        SourceOffset::from_location(&source, e.line(), e.column())
                    )],
                    "could not parse file"
                )
                .with_source_code(
                    NamedSource::new(path.display().to_string(), source).with_language("JSON"),
                )
            }),

            // not found, generate one
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default = T::default();
                // File::create(path)
                //     .into_diagnostic()
                //     .and_then(|file| serde_json::to_writer(file, &default).into_diagnostic())?;
                // let source = match path.canonicalize() {
                //     Ok(absolute) => absolute.display().to_string(),
                //     Err(_) => path.display().to_string(),
                // };
                // let e = miette!(
                //     severity = Severity::Warning,
                //     labels = vec![LabeledSpan::new_primary_with_span(
                //         Some(format!("{e}")),
                //         0..source.len(),
                //     )],
                //     "could not load {name} data; generating a default"
                // )
                // .with_source_code(source);
                // println!("{e:?}");
                Ok(default)
            }

            // other error
            Err(e) => {
                let source = match path.canonicalize() {
                    Ok(absolute) => absolute.display().to_string(),
                    Err(_) => path.display().to_string(),
                };
                Err(miette!(
                    labels = vec![LabeledSpan::new_primary_with_span(
                        Some(e.to_string()),
                        0..source.len(),
                    )],
                    "could not load {name} data"
                )
                .with_source_code(source))
            }
        }
    }

    let mut users = try_load::<UserMap>(&users, "user")?;
    let _slots = try_load::<Vec<Slot>>(&slots, "time slot")?;
    let mut tasks = try_load::<TaskMap>(&tasks, "task")?;

    // let schedule =
    //     Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users)).into_diagnostic()?;

    // serde_json::to_writer(File::create(output).into_diagnostic()?, &dbg!(schedule))
    //     .into_diagnostic()?;

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let mut server = Server::new();

    static EXIT_REQUESTED: AtomicBool = const { AtomicBool::new(false) };
    static NEXT_USER_ID: AtomicU32 = const { AtomicU32::new(0) };
    static NEXT_TASK_ID: AtomicU64 = const { AtomicU64::new(0) };
    static TASKS_TO_ADD: Mutex<Vec<Task>> = const { Mutex::new(Vec::new()) };
    static USERS_TO_ADD: Mutex<Vec<User>> = const { Mutex::new(Vec::new()) };

    NEXT_USER_ID.store(users.keys().map(|k| k.0).max().unwrap_or(0), Relaxed);
    NEXT_TASK_ID.store(tasks.keys().map(|k| k.0).max().unwrap_or(0), Relaxed);

    // quit
    {
        server.register_simple("quit", |()| {
            EXIT_REQUESTED.store(true, Relaxed);
            Ok(())
        });
    }

    // add_users
    {
        /// Python requirements for constructing a [`User`]
        #[derive(Debug, Serialize, Deserialize)]
        pub struct PyUser {
            name: String,
        }

        impl From<(UserId, PyUser)> for User {
            #[inline]
            fn from((id, user): (UserId, PyUser)) -> Self {
                let PyUser { name, .. } = user;
                User {
                    id,
                    name,
                    availability: Vec::new(),
                    user_prefs: FxHashMap::default(),
                    skills: FxHashMap::default(),
                }
            }
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct AddUsersParams {
            to_add: Vec<PyUser>,
        }

        server.register_simple(
            "add_users",
            |AddUsersParams { to_add }: AddUsersParams| -> Result<Vec<UserId>, Fault> {
                println!("srv: recieved users: {to_add:?}");
                let additional = to_add.len().try_into().unwrap();
                let start = NEXT_USER_ID.fetch_add(additional, Relaxed);
                let ids = (start..start + additional).map(UserId);
                USERS_TO_ADD
                    .lock()
                    .extend(ids.clone().zip(to_add).map(User::from));
                Ok(ids.collect())
            },
        );
    }

    // add_tasks
    {
        /// Python requirements for constructing a [`Task`]
        #[derive(Debug, Serialize, Deserialize)]
        pub struct PyTask {
            title: String,
            desc: Option<String>,
            deadline: Option<DateTime<Utc>>,
            awaiting: Option<Vec<TaskId>>,
        }

        impl From<(TaskId, PyTask)> for Task {
            #[inline]
            fn from((id, task): (TaskId, PyTask)) -> Self {
                let PyTask {
                    title, deadline, ..
                } = task;
                Task {
                    id,
                    title,
                    desc: task.desc.unwrap_or_default(),
                    skills: FxHashMap::default(),
                    deadline,
                    deps: task.awaiting.map(FxHashSet::from_iter).unwrap_or_default(),
                }
            }
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct AddTasksParams {
            to_add: Vec<PyTask>,
        }

        server.register_simple(
            "add_tasks",
            |AddTasksParams { to_add }: AddTasksParams| -> Result<Vec<TaskId>, Fault> {
                println!("srv: recieved tasks: {to_add:?}");
                let additional = to_add.len().try_into().unwrap();
                let start = NEXT_TASK_ID.fetch_add(additional, Relaxed);
                let ids = (start..start + additional).map(TaskId);
                TASKS_TO_ADD
                    .lock()
                    .extend(ids.clone().zip(to_add).map(Task::from));
                Ok(ids.collect())
            },
        );
    }

    let bound_server = server.bind(&socket).unwrap();
    let _marker = RunningHandle::init();
    loop {
        bound_server.poll();

        {
            let mut tasks_to_add = TASKS_TO_ADD.lock();
            if !tasks_to_add.is_empty() {
                println!("srv: adding tasks: {tasks_to_add:?}");
                tasks.extend(
                    std::mem::take(&mut *tasks_to_add)
                        .into_iter()
                        .map(|task| (task.id, task)),
                );
            }
        }

        {
            let mut users_to_add = USERS_TO_ADD.lock();
            if !users_to_add.is_empty() {
                println!("srv: adding users: {users_to_add:?}");
                users.extend(
                    std::mem::take(&mut *users_to_add)
                        .into_iter()
                        .map(|user| (user.id, user)),
                );
            }
        }

        if EXIT_REQUESTED.load(Relaxed) {
            break;
        }
    }
    Ok(())
}
