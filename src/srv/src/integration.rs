//! Integration functions for communicating with the Python frontend

use crate::data::*;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroUsize,
    sync::{
        LazyLock,
        atomic::{AtomicBool, AtomicU64, Ordering::Relaxed},
    },
};
use xml_rpc::{Fault, Server};

type Result<T> = std::result::Result<T, Fault>;

pub(crate) static EXIT_REQUESTED: AtomicBool = const { AtomicBool::new(false) };
pub(crate) static SLOTS: Mutex<Vec<Slot>> = Mutex::new(Vec::new());
pub(crate) static TASKS: Mutex<LazyLock<TaskMap>> = Mutex::new(LazyLock::new(TaskMap::default));
pub(crate) static USERS: Mutex<LazyLock<UserMap>> = Mutex::new(LazyLock::new(UserMap::default));
pub(crate) static NEXT_USER_ID: AtomicU64 = AtomicU64::new(0);
pub(crate) static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Python requirements for constructing a [`Slot`]
#[derive(Debug, Serialize, Deserialize)]
pub struct PySlot {
    /// Beginning of the slot
    pub start: DateTime<Utc>,

    /// Conclusion of the slot
    pub end: DateTime<Utc>,

    /// The minimum number of [`User`]s that must be assigned to the slot
    pub min_staff: Option<usize>,

    /// Optional name for the slot
    pub name: Option<String>,
}

impl From<PySlot> for Slot {
    #[inline]
    fn from(slot: PySlot) -> Self {
        let PySlot {
            start,
            end,
            min_staff,
            name,
        } = slot;
        Slot {
            interval: TimeInterval(start..end),
            min_staff: min_staff.and_then(NonZeroUsize::new),
            name,
        }
    }
}

/// Python requirements for constructing a [`Task`]
#[derive(Debug, Serialize, Deserialize)]
pub struct PyTask {
    /// The title of the task
    pub title: String,

    /// The task description
    pub desc: Option<String>,

    /// When the task should be completed by
    /// ([`None`] if no deadline)
    pub deadline: Option<DateTime<Utc>>,

    /// Tasks that must be completed before this one can start
    pub awaiting: Option<Vec<TaskId>>,
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

/// Python requirements for constructing a [`User`]
#[derive(Debug, Serialize, Deserialize)]
pub struct PyUser {
    /// The name of the user
    pub name: String,
}

impl From<(UserId, PyUser)> for User {
    #[inline]
    fn from((id, user): (UserId, PyUser)) -> Self {
        let PyUser { name, .. } = user;
        User {
            id,
            name,
            availability: Vec::new(),
            user_prefs: UserMap::default(),
            skills: SkillMap::default(),
        }
    }
}

/// Close the server after completing all ongoing tasks.
///
/// # Syntax
/// ```py
/// def quit(p: {}) -> None;
/// ```
///
/// # Examples
/// ```py
/// # request server close
/// proxy.quit({})
/// ```
pub fn quit(_: ()) -> Result<()> {
    EXIT_REQUESTED.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// Insert one or more slots into the slot list.
///
/// Argument must be an array, even if only adding one.
///
/// # Syntax
/// ```py
/// def add_slots(p: {'to_add': list[{
///   'start': datetime,
///   'end': datetime,          # must be >= `start`
///   'min_staff': int | None,  # cannot be negative; None is equivalent to 0
///   'name': str | None,
/// }]}) -> None;
/// ```
///
/// # Examples
/// ```py
/// # add a single slot requiring at least 3 staff on duty
/// proxy.add_slots({'to_add': [{
///   'start': datetime.strptime("21/11/06 16:30", "%d/%m/%y %H:%M"),
///   'end': datetime.strptime("21/11/06 18:30", "%d/%m/%y %H:%M"),
///   'min_staff': 3,
/// }]})
/// ```
pub fn add_slots(to_add: Vec<PySlot>) -> Result<()> {
    println!("srv: recieved users: {to_add:?}");
    SLOTS.lock().extend(to_add.into_iter().map(Slot::from));
    Ok(())
}

/// Insert one or more tasks into the user table. Returns the generated IDs of the newly created tasks in the order they were provided.
///
/// Argument must be an array, even if only adding one.
///
/// # Syntax
/// ```py
/// def add_tasks(p: {'to_add': list[{
///   'title': str,
///   'desc': str | None,
///   'deadline': datetime | None,
///   'awaiting': list[TaskId] | None,
/// }]}) -> list[TaskId];
/// ```
///
/// # Examples
/// ```py
/// # add a single task titled "wash dishes"
/// proxy.add_tasks({'to_add': [{'title': "wash dishes"}]})
///
/// # add a task titled "train intern" with a description
/// proxy.add_tasks({'to_add': [{
///   'title': "train intern",
///   'desc': "the new intern, joel, needs to be trained on how to work the register.",
/// }]})
///
/// # add a task titled "write budget" that must be completed by November 21, 2006 at 4:30pm
/// proxy.add_tasks({'to_add': [{
///   'title': "write budget",
///   'deadline': datetime.strptime("21/11/06 16:30", "%d/%m/%y %H:%M"),
/// }]})
///
/// # add two tasks titled "buy shelves" and "buy products",
/// # then add a task titled "stock shelves" dependent on both
/// ids = proxy.add_tasks({'to_add': [{'title': "buy shelves"}, {'title': "buy products"}]})
/// proxy.add_tasks({'to_add': [{'title': "stock shelves", 'awaiting': ids}]})
/// ```
///
/// **See also:** [`datetime`](https://docs.python.org/3/library/datetime.html)
pub fn add_tasks(to_add: Vec<PyTask>) -> Result<Vec<TaskId>> {
    println!("srv: recieved tasks: {to_add:?}");
    let additional = to_add.len().try_into().unwrap();
    let start = NEXT_TASK_ID.fetch_add(additional, Relaxed);
    let ids = (start..start + additional).map(TaskId);
    TASKS.lock().extend(
        ids.clone()
            .zip(to_add)
            .map(Task::from)
            .map(|task| (task.id, task)),
    );
    Ok(ids.collect())
}

/// Insert one or more users into the user table. Returns the generated IDs of the newly created users in the order they were provided.
///
/// Argument must be an array, even if only adding one.
///
/// # Syntax
/// ```py
/// def add_users(p: {'to_add': list[{'name': str}]}) -> list[UserId];
/// ```
///
/// # Examples
/// ```py
/// # add a single user named "bob"
/// proxy.add_users({'to_add': [{'name': "bob"}]})
///
/// # add a user named "tom" and a user named "sally"
/// proxy.add_users({'to_add': [{'name': "tom"}, {'name': "sally"}]})
/// ```
pub fn add_users(to_add: Vec<PyUser>) -> Result<Vec<UserId>> {
    println!("srv: recieved users: {to_add:?}");
    let additional = to_add.len().try_into().unwrap();
    let start = NEXT_USER_ID.fetch_add(additional, Relaxed);
    let ids = (start..start + additional).map(UserId);
    USERS.lock().extend(
        ids.clone()
            .zip(to_add)
            .map(User::from)
            .map(|user| (user.id, user)),
    );
    Ok(ids.collect())
}

pub(crate) fn register(server: &mut Server) {
    server.register_simple("quit", quit);
    server.register_simple("add_slots", add_slots);
    server.register_simple("add_tasks", add_tasks);
    server.register_simple("add_users", add_users);
}
