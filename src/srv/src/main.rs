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

use crate::{
    data::*,
    integration::{EXIT_REQUESTED, NEXT_TASK_ID, NEXT_USER_ID, SLOTS, TASKS, USERS},
};
use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use miette::{IntoDiagnostic, LabeledSpan, NamedSource, Result, SourceOffset, miette};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::BufReader,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::atomic::Ordering::Relaxed,
};
use xml_rpc::Server;

pub mod algo;
pub mod data;
pub mod integration;

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
    #[arg(value_name = "SOCKET")]
    socket: SocketAddr,

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
        socket,
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

    let slots = try_load::<Vec<Slot>>(&slots, "slot")?;
    let tasks = try_load::<TaskMap>(&tasks, "task")?;
    let users = try_load::<UserMap>(&users, "user")?;

    NEXT_TASK_ID.store(tasks.keys().map(|k| k.0 + 1).max().unwrap_or(0), Relaxed);
    NEXT_USER_ID.store(users.keys().map(|k| k.0 + 1).max().unwrap_or(0), Relaxed);
    *SLOTS.write() = slots;
    **TASKS.write() = tasks;
    **USERS.write() = users;

    let mut server = Server::new();

    integration::register(&mut server);

    let bound_server = server.bind(&socket).unwrap();
    let _marker = RunningHandle::init();
    loop {
        bound_server.poll();
        if EXIT_REQUESTED.load(Relaxed) {
            break Ok(());
        }
    }
}
