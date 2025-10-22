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

use algo::Schedule;
use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use miette::{LabeledSpan, Result, Severity, miette};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

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

    fn try_load<T: Serialize + DeserializeOwned + Default>(path: &Path, name: &str) -> Result<T> {
        match File::open(path) {
            // successfully loaded
            Ok(file) => {
                serde_json::from_reader(BufReader::new(file)).map_err(miette::Error::from_err)
            }

            // not found, generate one
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default = T::default();
                File::create(path)
                    .map_err(miette::Error::from_err)
                    .and_then(|file| {
                        serde_json::to_writer(file, &default).map_err(miette::Error::from_err)
                    })?;
                Ok(default)
            }

            // other error
            Err(e) => {
                let source = path.display().to_string();
                Err(miette!(
                    severity = Severity::Error,
                    labels = vec![LabeledSpan::at(0..source.len(), e.to_string())],
                    "could not load {name} data"
                )
                .with_source_code(source))
            }
        }
    }

    let users = try_load(&users, "user")?;
    let slots: Vec<_> = try_load(&slots, "time slot")?;
    let tasks = try_load(&tasks, "task")?;

    let schedule = Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users))
        .map_err(miette::Error::from_err)?;

    serde_json::to_writer(
        File::create(output).map_err(miette::Error::from_err)?,
        &dbg!(schedule),
    )
    .map_err(miette::Error::from_err)?;

    Ok(())
}
