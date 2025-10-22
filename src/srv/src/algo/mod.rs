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

use crate::data::{Slot, Task, TaskId, User, UserId};
use daggy::{Dag, Walker};
use miette::Result;
use petgraph::{prelude::NodeIndex, visit::Topo};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Create a dependency graph of tasks along with a topological sorting of them
pub fn dep_order(
    tasks: &FxHashMap<TaskId, Task>,
) -> Result<(Dag<&Task, ()>, Vec<NodeIndex>), SchedulingError> {
    // tasks must create a DAG (no cycles)
    let mut dep_graph = Dag::with_capacity(
        tasks.len(),
        tasks.values().map(|task| task.awaiting.len()).sum(),
    );
    let key_indices = tasks
        .keys()
        .enumerate()
        .map(|(i, k)| (k, i as u32))
        .collect::<FxHashMap<_, _>>();

    for task in tasks.values() {
        dep_graph.add_node(task);
    }

    for (a, task) in tasks.values().enumerate() {
        for b in &task.awaiting {
            dep_graph.add_edge(
                key_indices
                    .get(&b)
                    .copied()
                    .expect("all awaiting should be in graph")
                    .into(),
                (a as u32).into(),
                (),
            )?;
        }
    }

    let dep_order = Topo::new(&dep_graph).iter(&dep_graph).collect::<Vec<_>>();

    // debug
    println!("task order:");
    for (n, task) in dep_order.iter().map(|&i| &dep_graph[i]).enumerate() {
        println!("{n:>4}. {} ({})", &task.title, task.id);
        for dependency in &task.awaiting {
            println!("      * {dependency}");
        }
    }

    Ok((dep_graph, dep_order))
}

/// A collection of time slots along with the tasks and users assigned to them.
#[derive(Debug, Serialize, Deserialize)]
pub struct Schedule {
    /// Timeslots and their assignments.
    pub slots: Vec<(Slot, FxHashSet<TaskId>, FxHashSet<UserId>)>,
}

/// Error generated while attempting to create a schedule.
///
/// Requires prompting manager to resolve.
#[derive(Debug, Error)]
pub enum SchedulingError {
    /// A task was encountered that is not in the provided `tasks` dictionary.
    #[error("task {_0} does not exist")]
    NonExistentTask(TaskId),

    /// Failed to construct a DAG due to existence of a cycle.
    #[error(transparent)]
    WouldCycle(#[from] daggy::WouldCycle<()>),
}

impl Schedule {
    /// Generate a schedule based on the provided requirements.
    ///
    /// See [module-level documentation](crate::algo) for more details.
    pub fn generate(
        _slots: &[Slot],
        tasks: &FxHashMap<TaskId, Task>,
        _users: &FxHashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        let (_dag, _ord) = dep_order(tasks)?;

        todo!()
    }
}

#[cfg(test)]
mod scheduler_tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn test0() {
        let tasks = [
            Task {
                id: TaskId(5436),
                title: "foo".to_string(),
                desc: String::new(),
                skills: HashMap::new(),
                deadline: None,
                awaiting: HashSet::from_iter([]),
            },
            Task {
                id: TaskId(2537),
                title: "bar".to_string(),
                desc: String::new(),
                skills: HashMap::new(),
                deadline: None,
                awaiting: HashSet::from_iter([TaskId(3423)]),
            },
            Task {
                id: TaskId(3423),
                title: "baz".to_string(),
                desc: String::new(),
                skills: HashMap::new(),
                deadline: None,
                awaiting: HashSet::from_iter([TaskId(5436)]),
            },
        ]
        .into_iter()
        .map(|task| (task.id, task))
        .collect();

        let (dag, ord) = dep_order(&tasks).unwrap();
        assert_eq!(
            ord.iter()
                .map(|&i| dag[i].title.as_str())
                .collect::<Vec<_>>(),
            vec!["foo", "baz", "bar"]
        );
    }
}
