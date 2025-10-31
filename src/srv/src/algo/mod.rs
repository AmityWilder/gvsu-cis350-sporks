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
//! [^deps]: [`Task`] `a` is &lt;a dependent of/dependant on&gt; [`Task`] `b` if `a`'s [`deps`](Task::deps)-field contains `b`.
//! [^pref-mag]: A [`Preference`] is of higher magnitude when it is further from zero; i.e. [`f32::abs`]
//!
//! TODO: consider [PERT](https://en.wikipedia.org/wiki/Program_evaluation_and_review_technique)

use std::collections::BTreeMap;

use crate::data::{Preference, Slot, Task, TaskId, TaskMap, TimeInterval, User, UserId};
use daggy::{Dag, Walker, WouldCycle};
use miette::Result;
use petgraph::visit::Topo;
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
///
/// # Panics
/// This function may panic if `dict` is ill-formed--that is, if
/// a task is dependent on a task that does not exist.
///
/// # Errors
/// This function may return an error if the dependencies contain
/// cycles.
pub fn dep_graph(dict: &TaskMap) -> Result<DepGraph<'_>, WouldCycle<Vec<()>>> {
    use std::iter::repeat_n;

    // tasks must create a DAG (no cycles)
    let mut g = Dag::with_capacity(dict.len(), dict.values().map(|task| task.deps.len()).sum());

    // all nodes must be inserted before any edges because creating an edge
    // involving a node that has not been inserted yet causes an error, even
    // if that edge would be valid if both nodes were in the graph.
    let key_indices = FxHashMap::from_iter(dict.values().map(|task| (task.id, g.add_node(task))));

    // NOTE: parallel edges are not a concern because dependencies are stored
    // by Task in a set and are therefore unique.
    g.add_edges(
        dict.values()
            .flat_map(|Task { id, deps, .. }| repeat_n(id, deps.len()).zip(deps))
            .map(|(child, parent)| (key_indices[parent], key_indices[child], ())),
    )?;

    Ok(g)
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
        slots: &[Slot],
        tasks: &TaskMap,
        users: &FxHashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        let _deps = dep_graph(tasks)?;
        // let ord = dep_order(&deps);
        for slot in slots {
            let mut candidates = users
                .values()
                .filter_map(|u| {
                    let mut it = u
                        .availability
                        .iter()
                        .map(|(t, p)| (*p, t))
                        .filter(|(p, t)| {
                            *p > Preference::NEG_INFINITY && slot.interval.is_overlapping(t)
                        })
                        .peekable();

                    it.peek().is_some().then(|| (u, it.collect()))
                })
                .collect::<Vec<(&User, BTreeMap<Preference, &TimeInterval>)>>();

            candidates.sort_by_cached_key(|(_, prefs)| {
                std::cmp::Reverse(
                    *prefs
                        .first_key_value() // maximum preference
                        .expect("candidates are filtered by overlap with this slot")
                        .0,
                )
            });

            // TODO
        }

        todo!()
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;
    use chrono::prelude::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    fn dbg_ord(dep_graph: &DepGraph<'_>) {
        println!("task order:");
        for (n, task) in dep_order(dep_graph).enumerate() {
            println!(
                "{n:>4}. {} ({}){}\n        deps: {{{}}}",
                &task.title,
                task.id,
                match &task.deadline {
                    Some(x) => format!("\n        deadline: {}", x.format("%b %d, %Y - %H:%M")),
                    None => String::new(),
                },
                task.deps
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

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
                deps: FxHashSet::from_iter([$(TaskId($dep)),*]),
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
        dbg_ord(&dag);
        assert_eq!(
            dep_order(&dag)
                .map(|task| task.title.as_str())
                .collect::<Vec<_>>(),
            vec!["foo", "baz", "bar"]
        );
    }
}
