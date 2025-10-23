//! Generate a schedule based on the provided requirements.
//!
//! # Prioritization
//!
//!
//! In descending order of importance:
//!
//! 1. Minimize legal issues[^legal]
//! 1. Maximize task completion
//! 1. Minimize deadlines missed
//! 1. Maximize tasks completed ahead of deadline
//!    - Descending order of quantity of dependents[^deps]
//! 1. Maximize user scheduling preferences fulfilled
//!    - Descending order of preference magnitude[^pref-mag]
//! 1. Minimize quantity of users scheduled simultaneously
//!
//! [^legal]: [`Preference`] of &pm;inf ([`Preference::INFINITY`]/[`Preference::NEG_INFINITY`]).
//! [^deps]: [`Task`] `a` is &lt;a dependent of/dependant on&gt; [`Task`] `b` if `a`'s [`awaiting`](Task::awaiting)-field contains `b`.
//! [^pref-mag]: A [`Preference`] is of higher magnitude when it is further from zero; i.e. [`f32::abs`]
//!
//! TODO: consider using [Dinic's Algorithm](https://en.wikipedia.org/wiki/Dinic%27s_algorithm)
//! TODO: consider [topological sorting](https://en.wikipedia.org/wiki/Topological_sorting)
//! TODO: consider [PERT](https://en.wikipedia.org/wiki/Program_evaluation_and_review_technique)

use crate::data::{Slot, Task, TaskId, TaskMap, User, UserId};
use daggy::{Dag, NodeIndex, Walker, WouldCycle};
use miette::Result;
use petgraph::visit::{Topo, VisitMap, Visitable, WalkerIter};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error generated while attempting to create a schedule.
///
/// Requires prompting manager to resolve.
#[derive(Debug, Error)]
pub enum SchedulingError {
    /// A task was encountered that is not in the provided `tasks` dictionary.
    #[error("task {_0} does not exist")]
    NonExistentTask(TaskId),

    /// Failed to construct a DAG due to existence of a cycle.
    #[error("task dependencies cannot be cyclic")]
    WouldCycle(#[from] WouldCycle<Vec<()>>),
}

type DepGraph<'a> = Dag<&'a Task, ()>;

/// Create a [dependency graph](DepGraph) for the task map.
pub fn dep_graph(tasks: &TaskMap) -> Result<DepGraph<'_>, SchedulingError> {
    // tasks must create a DAG (no cycles)
    let mut dep_graph = Dag::with_capacity(
        tasks.len(),
        tasks.values().map(|task| task.awaiting.len()).sum(),
    );
    let key_indices = tasks
        .keys()
        .enumerate()
        .map(|(i, k)| (k, i as u32))
        .collect::<FxHashMap<&TaskId, u32>>();

    for task in tasks.values() {
        dep_graph.add_node(task);
    }

    dep_graph.add_edges(tasks.values().enumerate().flat_map(|(a, task)| {
        task.awaiting
            .iter()
            .map(|b| {
                key_indices
                    .get(&b)
                    .copied()
                    .expect("all awaiting should be in graph")
                    .into()
            })
            .zip(std::iter::repeat((a as u32).into()))
            .map(|(a, b)| (a, b, ()))
    }))?;

    #[cfg(debug_assertions)]
    {
        // debug
        println!("task order:");
        for (n, task) in dep_order(&dep_graph).enumerate() {
            println!(
                "{n:>4}. {} ({}){}\n        deps: {{{}}}",
                &task.title,
                task.id,
                match &task.deadline {
                    Some(x) => format!("\n        deadline: {}", x.format("%b %d, %Y - %H:%M")),
                    None => String::new(),
                },
                task.awaiting
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    Ok(dep_graph)
}

/// Creates a topological sorting iterator over a [`DepGraph`].
pub fn dep_order<'a>(graph: &DepGraph<'a>) -> impl Iterator<Item = &'a Task> + Clone {
    Topo::new(graph).iter(graph).map(|i| graph[i])
}

/// A collection of time slots along with the tasks and users assigned to them.
#[derive(Debug, Serialize, Deserialize)]
pub struct Schedule {
    /// Timeslots and their assignments.
    pub slots: Vec<(Slot, FxHashSet<TaskId>, FxHashSet<UserId>)>,
}

impl Schedule {
    /// Generate a schedule based on the provided requirements.
    ///
    /// See [module-level documentation](crate::algo) for more details.
    pub fn generate(
        _slots: &[Slot],
        tasks: &TaskMap,
        _users: &FxHashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        let _dag = dep_graph(tasks)?;

        todo!()
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;
    use chrono::prelude::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    macro_rules! test_project {
        ($(
            $id:literal: $title:literal
            $([$mo:literal/$d:literal/$yr:literal$( @ $hr:literal:$m:literal)?])?
            { $($dep:literal),* $(,)? }
        ),* $(,)?) => {
            [$(Task {
                id: TaskId($id),
                title: $title.to_string(),
                desc: String::new(),
                skills: FxHashMap::default(),
                deadline: None$(.or(Some(
                    Utc.from_utc_datetime(
                        &NaiveDateTime::new(
                            NaiveDate::from_ymd_opt($yr, $mo, $d)
                                .unwrap_or_else(|| panic!(
                                    "`{}/{}/{}` is not a valid date",
                                    $mo,
                                    $d,
                                    $yr,
                                )),
                            None$(.or(Some(NaiveTime::from_hms_opt($hr, $m, 0)
                                .unwrap_or_else(|| panic!(
                                    "`{}:{}` is not a valid time",
                                    $hr,
                                    $m,
                                )))))?
                                .unwrap_or(NaiveTime::default()),
                        ),
                    ))
                ))?,
                awaiting: FxHashSet::from_iter([$(TaskId($dep)),*]),
            }),*]
                .into_iter()
                .map(|task| (task.id, task))
                .collect()
        };
    }

    #[test]
    fn test0() {
        let tasks = test_project! {
            5436: "foo" [4/12/2025 @ 5:30] {},
            2537: "bar" [4/12/2025] { 3423 },
            3423: "baz" { 5436 },
        };

        let dag = dep_graph(&tasks).unwrap();
        assert_eq!(
            dep_order(&dag)
                .map(|task| task.title.as_str())
                .collect::<Vec<_>>(),
            vec!["foo", "baz", "bar"]
        );
    }
}
