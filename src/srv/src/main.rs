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

use xml_rpc::{Fault, Server};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use algo::Schedule;
use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use miette::{IntoDiagnostic, LabeledSpan, NamedSource, Result, Severity, SourceOffset, miette};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize)]
struct PingParams {
    code: i32
}

fn ping_callback(mut p: PingParams) -> Result<i32, Fault> {
    println!("srv: ping - code: {}", p.code);
    p.code = (p.code * 2369 - 3865) % 47635;
    println!("srv: ping - code: {}", p.code);
    Ok(p.code)
}

fn main() -> Result<()> {
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let mut server = Server::new();

    server.register_simple("ping", &ping_callback);
    let bound_server = server.bind(&socket).unwrap();
    println!("srv: running");
    bound_server.run();

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
                File::create(path)
                    .into_diagnostic()
                    .and_then(|file| serde_json::to_writer(file, &default).into_diagnostic())?;
                let source = match path.canonicalize() {
                    Ok(absolute) => absolute.display().to_string(),
                    Err(_) => path.display().to_string(),
                };
                let e = miette!(
                    severity = Severity::Warning,
                    labels = vec![LabeledSpan::new_primary_with_span(
                        Some(format!("{e}")),
                        0..source.len(),
                    )],
                    "could not load {name} data; generating a default"
                )
                .with_source_code(source);
                println!("{e:?}");
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

    let users = try_load(&users, "user")?;
    let slots: Vec<_> = try_load(&slots, "time slot")?;
    let tasks = try_load(&tasks, "task")?;

    let schedule =
        Schedule::generate(&dbg!(slots), &dbg!(tasks), &dbg!(users)).into_diagnostic()?;

    serde_json::to_writer(File::create(output).into_diagnostic()?, &dbg!(schedule))
        .into_diagnostic()?;

    Ok(())
}
