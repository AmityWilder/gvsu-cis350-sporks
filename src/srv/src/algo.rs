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

use crate::{
    data::{Slot, Task, TaskId, User, UserId},
    math::Graph,
};
use miette::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

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
    /// A task was encountered that is not in the provided `tasks` dictionary.
    #[error("task {_0} does not exist")]
    NonExistentTask(TaskId),
}

impl Schedule {
    /// Generate a schedule based on the provided requirements.
    ///
    /// See [module-level documentation](crate::algo) for more details.
    pub fn generate(
        _slots: &[Slot],
        tasks: &HashMap<TaskId, Task>,
        _users: &HashMap<UserId, User>,
    ) -> Result<Self, SchedulingError> {
        use SchedulingError::*;

        let dep_graph = Graph::from_forward(
            tasks
                .iter()
                .map(|(&a, Task { awaiting: bs, .. })| (a, bs.iter().copied())),
        )
        .ok_or_else(|| todo!())?;

        // use BFS to sort the graph
        // tasks must create a DAG (no cycles)
        let dep_order = dep_graph
            .bfs(
                dep_graph
                    .verts()
                    .copied()
                    .filter(|v| !dep_graph.receivers().contains(v)),
            )
            .collect::<Vec<_>>();

        // debug
        println!("task order:");
        for (n, id) in dep_order.into_iter().enumerate() {
            let title = tasks.get(&id).ok_or(NonExistentTask(id))?.title.as_str();
            println!("{n:>4}. {title} ({id})");
        }

        todo!()
    }
}

#[cfg(test)]
mod scheduler_tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test0() {
        let slots = vec![];
        let tasks = [
            (
                TaskId(5436),
                Task {
                    title: "foo".to_string(),
                    desc: String::new(),
                    skills: HashMap::new(),
                    deadline: None,
                    awaiting: HashSet::from_iter([]),
                },
            ),
            (
                TaskId(2537),
                Task {
                    title: "bar".to_string(),
                    desc: String::new(),
                    skills: HashMap::new(),
                    deadline: None,
                    awaiting: HashSet::from_iter([TaskId(5436)]),
                },
            ),
            (
                TaskId(3423),
                Task {
                    title: "baz".to_string(),
                    desc: String::new(),
                    skills: HashMap::new(),
                    deadline: None,
                    awaiting: HashSet::from_iter([TaskId(5436)]),
                },
            ),
        ]
        .into_iter()
        .collect();
        let users = [].into_iter().collect();
        dbg!(Schedule::generate(&slots, &tasks, &users)).unwrap();
    }
}
