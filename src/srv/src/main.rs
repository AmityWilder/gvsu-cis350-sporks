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
use miette::{
    Diagnostic, IntoDiagnostic, LabeledSpan, NamedSource, Result, SourceCode, SourceOffset,
    SourceSpan, SpanContents,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;

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

/// IO errors aside from [`NotFound`](std::io::ErrorKind::NotFound).
#[derive(Debug, Diagnostic, Error)]
#[error("could not load {name} data")]
pub struct LoadError {
    name: &'static str,

    #[source_code]
    source: String,

    #[label(primary, "{e}")]
    primary_span: SourceSpan,

    #[source]
    e: std::io::Error,
}

/// Error while trying to parse a file
#[derive(Debug, Diagnostic, Error)]
#[error("could not parse file")]
pub struct ParseError {
    #[source_code]
    source: NamedSource<String>,

    #[label(primary, "{e}")]
    primary_span: SourceOffset,

    #[source]
    e: serde_json::Error,
}

fn main() -> Result<()> {
    let Cli {
        users,
        slots,
        tasks,
        output,
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
                ParseError {
                    primary_span: SourceOffset::from_location(&source, e.line(), e.column()),
                    e,
                    source: NamedSource::new(path.display().to_string(), source)
                        .with_language("JSON"),
                }
                .into()
            }),

            // not found, generate one
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default = T::default();
                File::create(path)
                    .into_diagnostic()
                    .and_then(|file| serde_json::to_writer(file, &default).into_diagnostic())?;
                Ok(default)
            }

            // other error
            Err(e) => {
                let source = match path.canonicalize() {
                    Ok(absolute) => absolute.display().to_string(),
                    Err(_) => path.display().to_string(),
                };
                Err(LoadError {
                    e,
                    name,
                    primary_span: (0..source.len()).into(),
                    source,
                }
                .into())
            }
        }
    }

    let users = try_load(&users, "user")?;
    let slots: Vec<_> = try_load(&slots, "time slot")?;
    let tasks = try_load(&tasks, "task")?;

    let schedule =
        Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users)).into_diagnostic()?;

    serde_json::to_writer(File::create(output).into_diagnostic()?, &dbg!(schedule))
        .into_diagnostic()?;

    Ok(())
}
