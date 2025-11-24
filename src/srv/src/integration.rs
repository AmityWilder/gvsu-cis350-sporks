//! Integration functions for communicating with the Python frontend
//!
//! The main reason for the `Py...` types is so that structures without IDs can be passed.
//! Additionally, many backend types have non-[`None`] "None-like" values (such as empty strings).

use crate::data::*;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::{LazyLock, atomic::AtomicBool},
};
use xml_rpc::{Fault, Server};

type Result<T> = std::result::Result<T, Fault>;

pub(crate) static EXIT_REQUESTED: AtomicBool = const { AtomicBool::new(false) };
pub(crate) static SLOTS: RwLock<LazyLock<SlotMap>> = RwLock::new(LazyLock::new(SlotMap::default));
pub(crate) static TASKS: RwLock<LazyLock<TaskMap>> = RwLock::new(LazyLock::new(TaskMap::default));
pub(crate) static USERS: RwLock<LazyLock<UserMap>> = RwLock::new(LazyLock::new(UserMap::default));

mod re_serde {
    use regex::Regex;

    struct RegexVisitor;

    impl<'de> serde::de::Visitor<'de> for RegexVisitor {
        type Value = Regex;

        #[inline]
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("Regex pattern")
        }

        #[inline]
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Regex::new(v).map_err(E::custom)
        }
    }

    #[inline]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(RegexVisitor)
    }

    #[inline]
    pub(crate) fn serialize<S>(re: &Regex, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(re.as_str())
    }
}

/// A string filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    /// Strings that match the pattern `^literal_text.*$`.
    StartsWith(String),

    /// Strings that match the pattern `^.*literal_text$`.
    EndsWith(String),

    /// Strings that match the pattern `^.*literal_text.*$`.
    Contains(String),

    /// Strings that match the pattern `^literal_text$`.
    Exactly(String),

    /// Strings that match the regex pattern.
    Regex(#[serde(with = "re_serde")] Regex),
}

impl Pattern {
    /// Construct a [`Pattern`] that matches any string starting with literal `s`.
    #[inline]
    pub const fn starts_with(s: String) -> Result<Self> {
        Ok(Self::StartsWith(s))
    }

    /// Construct a [`Pattern`] that matches any string ending with literal `s`.
    #[inline]
    pub const fn ends_with(s: String) -> Result<Self> {
        Ok(Self::EndsWith(s))
    }

    /// Construct a [`Pattern`] that matches any string containging literal `s`.
    #[inline]
    pub const fn contains(s: String) -> Result<Self> {
        Ok(Self::Contains(s))
    }

    /// Construct a [`Pattern`] that matches any string exactly equal to literal `s`.
    #[inline]
    pub const fn exactly(s: String) -> Result<Self> {
        Ok(Self::Exactly(s))
    }

    /// Construct a [`Pattern`] that uses [`regex`] to match strings.
    ///
    /// # Errors
    ///
    /// Produces a [422 Unprocessable Content](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/422)
    /// error if the argument is not valid [`regex`].
    #[inline]
    pub fn regex(s: String) -> Result<Self> {
        Regex::new(&s)
            .map(Pattern::Regex)
            .map_err(|e| Fault::new(422, format!("invalid regex: {e}")))
    }

    /// Test if `haystack` matches the [`Pattern`].
    pub fn is_match(&self, haystack: &str) -> bool {
        match self {
            Pattern::StartsWith(s) => haystack.starts_with(s),
            Pattern::EndsWith(s) => haystack.ends_with(s),
            Pattern::Contains(s) => haystack.contains(s),
            Pattern::Exactly(s) => haystack == s,
            Pattern::Regex(re) => re.is_match(haystack),
        }
    }
}

/// Once every `n` units. Fields are added together.
/// [`None`] and `0` are equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyFreq {
    /// Repeat every `n` seconds.
    pub seconds: Option<u8>,

    /// Repeat every `n` minutes.
    pub minutes: Option<u8>,

    /// Repeat every `n` hours.
    pub hours: Option<u8>,

    /// Repeat every `n` days.
    pub days: Option<u8>,

    /// Repeat every `n` weeks.
    pub weeks: Option<u8>,

    /// Repeat every `n` months.
    pub months: Option<u8>,

    /// Repeat every `n` years.
    pub years: Option<u16>,
}

impl From<PyFreq> for Frequency {
    #[inline]
    fn from(value: PyFreq) -> Self {
        Self {
            seconds: value.seconds.unwrap_or(0),
            minutes: value.minutes.unwrap_or(0),
            hours: value.hours.unwrap_or(0),
            days: value.days.unwrap_or(0),
            weeks: value.weeks.unwrap_or(0),
            months: value.months.unwrap_or(0),
            years: value.years.unwrap_or(0),
        }
    }
}

impl From<Frequency> for PyFreq {
    #[inline]
    fn from(value: Frequency) -> Self {
        Self {
            seconds: (value.seconds != 0).then_some(value.seconds),
            minutes: (value.minutes != 0).then_some(value.minutes),
            hours: (value.hours != 0).then_some(value.hours),
            days: (value.days != 0).then_some(value.days),
            weeks: (value.weeks != 0).then_some(value.weeks),
            months: (value.months != 0).then_some(value.months),
            years: (value.years != 0).then_some(value.years),
        }
    }
}

/// How to repeat a [`Rule`]'s intervals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyRep {
    /// The frequency of the repetition.
    pub every: PyFreq,

    /// When the repetition begins.
    pub start: DateTime<Utc>,

    /// When the repetition should end. [`None`] if permanent.
    pub until: Option<DateTime<Utc>>,
}

impl From<PyRep> for Repetition {
    #[inline]
    fn from(value: PyRep) -> Self {
        let PyRep {
            every,
            start,
            until,
        } = value;
        Self {
            every: every.into(),
            start,
            until,
        }
    }
}

impl From<Repetition> for PyRep {
    #[inline]
    fn from(value: Repetition) -> Self {
        let Repetition {
            every,
            start,
            until,
        } = value;
        Self {
            every: every.into(),
            start,
            until,
        }
    }
}

/// Python requirements for constructing a [`Rule`]
#[derive(Debug, Serialize, Deserialize)]
pub struct PyRule {
    /// The specific intervals this rule involves, before repeating.
    pub include: SmallVec<[TimeInterval; 1]>,

    /// How often `include` repeats.
    /// [`None`] if one-off.
    pub repeat: Option<PyRep>,

    /// How much the [`User`] prefers the times described by this rule
    ///
    /// See [`Preference`]
    pub preference: f32,
}

impl From<(RuleId, PyRule)> for Rule {
    #[inline]
    fn from((id, value): (RuleId, PyRule)) -> Self {
        let PyRule {
            include,
            repeat,
            preference,
        } = value;
        Self {
            id,
            include,
            rep: repeat.map(From::from),
            pref: Preference(preference),
        }
    }
}

impl From<Rule> for (RuleId, PyRule) {
    #[inline]
    fn from(value: Rule) -> Self {
        let Rule {
            id,
            include,
            rep,
            pref: Preference(preference),
        } = value;
        (
            id,
            PyRule {
                include,
                repeat: rep.map(From::from),
                preference,
            },
        )
    }
}

impl From<&Rule> for (RuleId, PyRule) {
    #[inline]
    fn from(value: &Rule) -> Self {
        let Rule {
            id,
            include,
            rep,
            pref: Preference(preference),
        } = value;
        (
            *id,
            PyRule {
                include: include.clone(),
                repeat: rep.as_ref().cloned().map(From::from),
                preference: *preference,
            },
        )
    }
}

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

impl From<(SlotId, PySlot)> for Slot {
    #[inline]
    fn from((id, slot): (SlotId, PySlot)) -> Self {
        let PySlot {
            start,
            end,
            min_staff,
            name,
        } = slot;
        Self {
            id,
            interval: TimeInterval { start, end },
            min_staff: min_staff.and_then(NonZeroUsize::new),
            name: name.unwrap_or_default(),
        }
    }
}

impl From<Slot> for (SlotId, PySlot) {
    #[inline]
    fn from(slot: Slot) -> Self {
        let Slot {
            id,
            interval: TimeInterval { start, end },
            min_staff,
            name,
        } = slot;
        (
            id,
            PySlot {
                start,
                end,
                min_staff: min_staff.map(NonZeroUsize::get),
                name: (!name.is_empty()).then_some(name),
            },
        )
    }
}

impl From<&Slot> for (SlotId, PySlot) {
    #[inline]
    fn from(slot: &Slot) -> Self {
        slot.clone().into()
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
    pub awaiting: Option<TaskSet>,
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

impl From<Task> for (TaskId, PyTask) {
    #[inline]
    fn from(task: Task) -> Self {
        let Task {
            id,
            title,
            desc,
            skills: _,
            deadline,
            deps,
        } = task;
        (
            id,
            PyTask {
                title,
                desc: (!desc.is_empty()).then_some(desc),
                deadline,
                awaiting: (!deps.is_empty()).then(|| deps.clone()),
            },
        )
    }
}

impl From<&Task> for (TaskId, PyTask) {
    #[inline]
    fn from(task: &Task) -> Self {
        let Task {
            id,
            title,
            desc,
            skills: _,
            deadline,
            deps,
        } = task;
        (
            *id,
            PyTask {
                title: title.clone(),
                desc: (!desc.is_empty()).then(|| desc.clone()),
                deadline: *deadline,
                awaiting: (!deps.is_empty()).then(|| deps.iter().copied().collect()),
            },
        )
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
            availability: RuleMap::default(),
            user_prefs: UserMap::default(),
            skills: SkillMap::default(),
        }
    }
}

impl From<User> for (UserId, PyUser) {
    #[inline]
    fn from(user: User) -> Self {
        let User { id, name, .. } = user;
        (id, PyUser { name })
    }
}

impl From<&User> for (UserId, PyUser) {
    #[inline]
    fn from(user: &User) -> Self {
        let User { id, name, .. } = user;
        (*id, PyUser { name: name.clone() })
    }
}

/// Add one or more availability rules to one or more users.
///
/// Returns the generated IDs of the newly created rules in the order they were provided.
///
/// If a provided user does not exist, those rules will not be created and that user will be missing from the returned dictionary.
///
/// # Signature
/// ```py
/// def add_rules(to_add: dict[
///   UserId,
///   list[{
///     'start': datetime,
///     'end': datetime,  # must be >=`start`
///     'pref': float,    # must be between -1 and +1, or exactly +/-infinity
///   }]
/// ]) -> set[UserId];
/// ```
pub fn add_rules(to_add: UserMap<Vec<PyRule>>) -> Result<UserMap<Vec<RuleId>>> {
    let mut users = USERS.write();
    Ok(to_add
        .into_iter()
        .filter_map(|(user_id, rules)| {
            users.get_mut(&user_id).map(|user| {
                let ids = RuleId::take(rules.len().try_into().unwrap());
                user.availability.extend(
                    ids.clone()
                        .zip(rules)
                        .map(Rule::from)
                        .map(|rule| (rule.id, rule)),
                );
                (user_id, ids.collect())
            })
        })
        .collect())
}

/// Insert one or more slots into the slot list.
///
/// Returns the generated IDs of the newly created slots in the order they were provided.
///
/// Argument must be an array, even if only adding one.
///
/// # Signature
/// ```py
/// def add_slots(list[{
///   'start': datetime,
///   'end':   datetime,        # must be >=`start`
///   'min_staff': int | None,  # cannot be negative; None is equivalent to 0
///   'name': str | None,
/// }]) -> list[SlotId];
/// ```
///
/// # Examples
/// ```py
/// # add a single slot requiring at least 3 staff on duty
/// proxy.add_slots([{
///   'start': datetime.strptime("21/11/06 16:30", "%d/%m/%y %H:%M"),
///   'end':   datetime.strptime("21/11/06 18:30", "%d/%m/%y %H:%M"),
///   'min_staff': 3,
/// }])
/// ```
pub fn add_slots(to_add: Vec<PySlot>) -> Result<Vec<SlotId>> {
    let ids = SlotId::take(to_add.len().try_into().unwrap());
    SLOTS.write().extend(
        ids.clone()
            .zip(to_add)
            .map(Slot::from)
            .map(|slot| (slot.id, slot)),
    );
    Ok(ids.collect())
}

/// Insert one or more tasks into the user table.
///
/// Returns the generated IDs of the newly created tasks in the order they were provided.
///
/// Argument must be an array, even if only adding one.
///
/// # Signature
/// ```py
/// def add_tasks(to_add: list[{
///   'title': str,
///   'desc': str | None,
///   'deadline': datetime | None,
///   'awaiting': set[TaskId] | None,
/// }]) -> list[TaskId];
/// ```
///
/// # Examples
/// ```py
/// # add a single task titled "wash dishes"
/// proxy.add_tasks([{'title': "wash dishes"}])
///
/// # add a task titled "train intern" with a description
/// proxy.add_tasks([{
///   'title': "train intern",
///   'desc': "the new intern, joel, needs to be trained on how to work the register.",
/// }])
///
/// # add a task titled "write budget" that must be completed by November 21, 2006 at 4:30pm
/// proxy.add_tasks([{
///   'title': "write budget",
///   'deadline': datetime.strptime("21/11/06 16:30", "%d/%m/%y %H:%M"),
/// }])
///
/// # add two tasks titled "buy shelves" and "buy products",
/// # then add a task titled "stock shelves" dependent on both
/// ids = proxy.add_tasks([{'title': "buy shelves"}, {'title': "buy products"}])
/// proxy.add_tasks([{'title': "stock shelves", 'awaiting': ids}])
/// ```
///
/// **See also:** [`datetime`](https://docs.python.org/3/library/datetime.html)
pub fn add_tasks(to_add: Vec<PyTask>) -> Result<Vec<TaskId>> {
    let ids = TaskId::take(to_add.len().try_into().unwrap());
    TASKS.write().extend(
        ids.clone()
            .zip(to_add)
            .map(Task::from)
            .map(|task| (task.id, task)),
    );
    Ok(ids.collect())
}

/// Insert one or more users into the user table.
///
/// Returns the generated IDs of the newly created users in the order they were provided.
///
/// Argument must be an array, even if only adding one.
///
/// # Signature
/// ```py
/// def add_users(to_add: list[{'name': str}]) -> list[UserId];
/// ```
///
/// # Examples
/// ```py
/// # add a single user named "bob"
/// proxy.add_users([{'name': "bob"}])
///
/// # add a user named "tom" and a user named "sally"
/// proxy.add_users([{'name': "tom"}, {'name': "sally"}])
/// ```
pub fn add_users(to_add: Vec<PyUser>) -> Result<Vec<UserId>> {
    let ids = UserId::take(to_add.len().try_into().unwrap());
    USERS.write().extend(
        ids.clone()
            .zip(to_add)
            .map(User::from)
            .map(|user| (user.id, user)),
    );
    Ok(ids.collect())
}

/// A filter for selecting [`Rule`]s from the backend database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFilter {
    /// A whitelist of the exact [`Rule::id`]s that should be included.
    pub ids: Option<RuleSet>,

    /// The least preference the [`Rule`] can require.
    pub min_pref: Option<f32>,

    /// The greatest preference the [`Rule`] can require.
    pub max_pref: Option<f32>,
}

/// Returns an dictionary of all current availability rules associated with each user, filtered by the parameters.
///
/// Users that do not exist will be missing from the returned dictionary.
///
/// Each filter parameter is combined as "and" (tasks must satisfy *all* conditions to be included).
/// Parameters that are [`None`] will be ignored.
///
/// # Signature
/// ```py
/// def get_rules(filter: dict[UserId, {
///     'ids': set[RuleId],
///     'min_pref': float | None,
///     'max_pref': float | None,  # must be >=`min_pref`
/// }]) -> list[(
///   {
///     'include': list[range[datetime]],
///     'repeat': {
///       'every': {
///         seconds: int | None,  # will always be >=1 if not None
///         minutes: int | None,  # will always be >=1 if not None
///         hours:   int | None,  # will always be >=1 if not None
///         days:    int | None,  # will always be >=1 if not None
///         weeks:   int | None,  # will always be >=1 if not None
///         months:  int | None,  # will always be >=1 if not None
///         years:   int | None,  # will always be >=1 if not None
///       },
///       'start': datetime,
///       'until': datetime | None,  # will always be >=`start` if not None
///     } | None,
///   },
///   f32,
/// )];
/// ```
pub fn get_rules(filter: UserMap<RuleFilter>) -> Result<UserMap<RuleMap<PyRule>>> {
    let users = USERS.read();
    filter
        .into_iter()
        .flat_map(|(user_id, filter)| {
            users.get(&user_id).map(|user| {
                let RuleFilter {
                    ids,
                    min_pref,
                    max_pref,
                } = filter;
                let ids = ids.as_ref();
                Ok((
                    user_id,
                    user.availability
                        .values()
                        .filter(|rule| {
                            min_pref.is_none_or(|x| rule.pref.0 >= x)
                                && max_pref.is_none_or(|x| rule.pref.0 <= x)
                                // note that None => "do not filter", which is distinct from {} => "never"
                                && ids.is_none_or(|x| x.contains(&rule.id))
                        })
                        .map(From::from)
                        .collect(),
                ))
            })
        })
        .collect()
}

/// A filter for selecting [`Slot`]s from the backend database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotFilter {
    /// A whitelist of the exact [`Slot::id`]s that should be included.
    pub ids: Option<SlotSet>,

    /// The ealiest datetime the [`Slot`] can start at.
    pub starting_after: Option<DateTime<Utc>>,

    /// The latest datetime the [`Slot`] can start at.
    pub starting_before: Option<DateTime<Utc>>,

    /// The ealiest datetime the [`Slot`] can end at.
    pub ending_after: Option<DateTime<Utc>>,

    /// The latest datetime the [`Slot`] can end at.
    pub ending_before: Option<DateTime<Utc>>,

    /// The least staff the [`Slot`] can require (0 is equivalent to [`None`]).
    pub min_staff_min: Option<usize>,

    /// The greatest staff the [`Slot`] can require (0 is equivalent to [`None`]).
    pub min_staff_max: Option<usize>,

    /// A [`Pattern`] the [`Slot::name`] must [match](Pattern::is_match).
    pub name_pat: Option<Pattern>,
}

/// Returns an array of all current slots.
///
/// Each filter parameter is combined as "and" (tasks must satisfy *all* conditions to be included).
/// Parameters that are [`None`] will be ignored.
///
/// Patterns should use `^$` (match start followed immediately by end) to match against empty names,
/// as an empty pattern will always match (the empty set is a subset of every set).
///
/// # Signature
/// ```py
/// def get_slots(filter: {
///   'ids': set[SlotId] | None,
///   'starting_before': datetime | None,  # inclusive
///   'starting_after':  datetime | None,  # inclusive
///   'ending_before':   datetime | None,  # inclusive
///   'ending_after':    datetime | None,  # inclusive
///   'min_staff_min': int | None,         # must be positive
///   'min_staff_max': int | None,         # must be positive and >=`min_staff_min`
///   'name_pat': Pattern | None,
/// }) -> list[{
///   'start': datetime,
///   'end':   datetime,        # will always be >=`start`
///   'min_staff': int | None,  # will always be >=1 if not None
///   'name': str | None,
/// }];
/// ```
pub fn get_slots(filter: SlotFilter) -> Result<SlotMap<PySlot>> {
    let SlotFilter {
        ids,
        starting_before,
        starting_after,
        ending_before,
        ending_after,
        min_staff_min,
        min_staff_max,
        name_pat,
    } = filter;
    let ids = ids.as_ref();
    let name_pat = name_pat.as_ref();
    Ok(SLOTS
        .read()
        .values()
        .filter(|slot| {
            starting_before.is_none_or(|x| slot.start <= x)
                && starting_after.is_none_or(|x| slot.start >= x)
                && ending_before.is_none_or(|x| slot.end <= x)
                && ending_after.is_none_or(|x| slot.end >= x)
                && min_staff_min.is_none_or(|x| slot.min_staff.map_or(0, NonZeroUsize::get) >= x)
                && min_staff_max.is_none_or(|x| slot.min_staff.map_or(0, NonZeroUsize::get) <= x)
                // note that None => "do not filter", which is distinct from {} => "never"
                && ids.is_none_or(|x| x.contains(&slot.id))
                // use "^$" to match against empty names
                && name_pat.is_none_or(|x| x.is_match(&slot.name))
        })
        .map(From::from)
        .collect())
}

/// A filter for selecting [`Task`]s from the backend database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilter {
    /// A whitelist of the exact [`Task::id`]s that should be included.
    pub ids: Option<TaskSet>,

    /// A [`Pattern`] the [`Task::title`] must [match](Pattern::is_match).
    pub title_pat: Option<Pattern>,

    /// A [`Pattern`] the [`Task::desc`] must [match](Pattern::is_match).
    pub desc_pat: Option<Pattern>,

    /// The ealiest datetime the [`Task::deadline`] can be.
    pub deadline_after: Option<DateTime<Utc>>,

    /// The latest datetime the [`Task::deadline`] can be.
    pub deadline_before: Option<DateTime<Utc>>,
}

/// Returns a dictionary of all current tasks, filtered by the parameters.
///
/// Each filter parameter is combined as "and" (tasks must satisfy *all* conditions to be included).
/// Parameters that are [`None`] will be ignored.
///
/// # Signature
/// ```py
/// def get_tasks(filter: {
///   'ids': set[TaskId] | None,
///   'title_pat': Pattern | None,
///   'desc_pat':  Pattern | None,
///   'deadline_before': datetime | None,  # inclusive
///   'deadline_after':  datetime | None,  # inclusive
/// }) -> dict[
///   TaskId, {
///     'title': str,
///     'desc':  str | None,
///     'deadline': datetime | None,
///     'awaiting': set[TaskId] | None,
///   }
/// ];
/// ```
///
/// **See also:** [`Pattern`]
pub fn get_tasks(filter: TaskFilter) -> Result<TaskMap<PyTask>> {
    let TaskFilter {
        ids,
        title_pat,
        desc_pat,
        deadline_before,
        deadline_after,
    } = filter;
    let ids = ids.as_ref();
    let title_pat = title_pat.as_ref();
    let desc_pat = desc_pat.as_ref();
    Ok(TASKS
        .read()
        .values()
        .filter(|task| {
            // lack of deadline is equivalent to infinite deadline. there exists no inf<=datetime.
            deadline_before.is_none_or(|x| task.deadline.is_some_and(|d| d <= x))
                // lack of deadline is equivalent to infinite deadline. every no datetime<=inf.
                && deadline_after.is_none_or(|x| task.deadline.is_none_or(|d| d >= x))
                // note that None => "do not filter", which is distinct from {} => "never"
                && ids.is_none_or(|x| x.contains(&task.id))
                && title_pat.is_none_or(|x| x.is_match(&task.title))
                && desc_pat.is_none_or(|x| x.is_match(&task.desc))
        })
        .map(From::from)
        .collect())
}

/// A filter for selecting [`User`]s from the backend database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFilter {
    /// A whitelist of the exact [`User::id`]s that should be included.
    pub ids: Option<Vec<UserId>>,

    /// A [`Pattern`] the [`User::name`] must [match](Pattern::is_match).
    pub name_pat: Option<Pattern>,
}

/// Returns a dictionary of all current users, filtered by the parameters.
///
/// Each filter parameter is combined as "and" (users must satisfy *all* conditions to be included).
/// Parameters that are `None` will be ignored.
///
/// # Signature
/// ```py
/// def get_users(filter: {
///   'ids': list[UserId] | None,
///   'name_pat': Pattern | None,
/// }) -> dict[UserId, {'name': str}];
/// ```
///
/// **See also:** [`Pattern`]
pub fn get_users(filter: UserFilter) -> Result<UserMap<PyUser>> {
    let UserFilter { ids, name_pat } = filter;
    let ids = ids.as_ref();
    let name_pat = name_pat.as_ref();
    Ok(USERS
        .read()
        .values()
        .filter(|user| {
            ids.is_none_or(|x| x.contains(&user.id))
                && name_pat.is_none_or(|x| x.is_match(&user.name))
        })
        .map(From::from)
        .collect())
}

/// A change to a set ([`HashSet`](std::collections::HashSet) or [`BTreeSet`](std::collections::BTreeSet)).
#[derive(Debug, Clone, Deserialize)]
pub struct KeySetDelta<K: Eq + std::hash::Hash> {
    /// Key(s) to be removed from the set.
    #[serde(default = "Default::default")]
    pub delete: FxHashSet<K>,

    /// Key(s) to be added to the set.
    #[serde(default = "Default::default")]
    pub create: Vec<K>,
}

impl<K: Eq + std::hash::Hash> Default for KeySetDelta<K> {
    fn default() -> Self {
        Self {
            delete: Default::default(),
            create: Default::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash> KeySetDelta<K> {
    fn apply(&mut self, target: &mut FxHashSet<K>) {
        target.retain(|k| !self.delete.remove(k));
        target.extend(std::mem::take(&mut self.create));
    }
}

/// A change to a collection that cannot create new elements, only remove or modify.
#[derive(Debug, Clone, Deserialize)]
pub struct NoGrowSetDelta<K: Eq + std::hash::Hash, V> {
    /// Key(s) to be removed from the collection.
    #[serde(default = "Default::default")]
    pub delete: FxHashSet<K>,

    /// Key(s) in the collection to be replaced with new value(s).
    #[serde(default = "Default::default")]
    pub update: FxHashMap<K, V>,
}

impl<K: Eq + std::hash::Hash, V> Default for NoGrowSetDelta<K, V> {
    fn default() -> Self {
        Self {
            delete: Default::default(),
            update: Default::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash, V> NoGrowSetDelta<K, V> {
    #[allow(dead_code, reason = "future-proof")]
    fn apply(&mut self, target: &mut FxHashMap<K, V>) {
        target.retain(|k, v| {
            if self.delete.remove(k) {
                false
            } else {
                if let Some(replacement) = self.update.remove(k) {
                    *v = replacement;
                }
                true
            }
        });
    }
}

/// A change to a collection.
#[derive(Debug, Clone, Deserialize)]
pub struct SetDelta<K: Eq + std::hash::Hash, V, U = (K, V)> {
    /// Key(s) to be removed from the collection.
    #[serde(default = "Default::default")]
    pub delete: FxHashSet<K>,

    /// Value(s) to be added to the collection.
    #[serde(default = "Default::default")]
    pub create: Vec<U>,

    /// Key(s) in the collection to be replaced with new value(s).
    #[serde(default = "Default::default")]
    pub update: FxHashMap<K, V>,
}

impl<K: Eq + std::hash::Hash, V, U> Default for SetDelta<K, V, U> {
    fn default() -> Self {
        Self {
            delete: Default::default(),
            create: Default::default(),
            update: Default::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash, V> SetDelta<K, V, (K, V)> {
    fn apply(&mut self, target: &mut FxHashMap<K, V>) {
        target.retain(|k, v| {
            if self.delete.remove(k) {
                false
            } else {
                if let Some(replacement) = self.update.remove(k) {
                    *v = replacement;
                }
                true
            }
        });
        target.extend(std::mem::take(&mut self.create));
    }
}

/// A [`SetDelta`] for sets where the key is auto-generated and thus left unspecified for [`SetDelta::create`] mode.
/// Example: pushing to a [`Vec`].
pub type AutoIdSetDelta<K, V> = SetDelta<K, V, V>;

/// [`None`] to ignore and keep existing value.
/// [`Some`] to replace the value.
pub type Update<T> = Option<T>;

trait ApplyUpdate {
    type Target;

    fn apply(self, target: &mut Self::Target);
}

impl<T> ApplyUpdate for Update<T> {
    type Target = T;

    #[inline]
    fn apply(self, target: &mut T) {
        if let Some(new_value) = self {
            *target = new_value;
        }
    }
}

/// A mutation request for a [`Rule`].
#[derive(Debug, Clone, Deserialize)]
pub struct RuleDelta {
    /// See [`Rule::include`]
    #[serde(default)]
    pub include: AutoIdSetDelta<usize, TimeInterval>,

    /// See [`Rule::rep`]
    #[serde(default)]
    pub rep: Update<Option<Repetition>>,

    /// See [`Rule::pref`]
    #[serde(default)]
    pub pref: Update<Preference>,
}

/// A mutation request for a [`Slot`].
#[derive(Debug, Clone, Deserialize)]
pub struct SlotDelta {
    /// See [`Slot::interval`]
    #[serde(default)]
    pub interval: Update<TimeInterval>,

    /// See [`Slot::min_staff`]
    #[serde(default)]
    pub min_staff: Update<Option<NonZeroUsize>>,

    /// See [`Slot::name`]
    #[serde(default)]
    pub name: Update<String>,
}

/// Mutate [`Slot`]s.
///
/// Returns a collection of all failed changes.
/// If all requested changes were successful, the list will be empty.
pub fn mut_slots(delta: SlotMap<SlotDelta>) -> Result<SlotSet> {
    let mut slots = SLOTS.write();
    Ok(delta
        .into_iter()
        .filter_map(|(slot_id, delta)| {
            if let Some(slot) = slots.get_mut(&slot_id) {
                delta.interval.apply(&mut slot.interval);
                delta.min_staff.apply(&mut slot.min_staff);
                delta.name.apply(&mut slot.name);
                None
            } else {
                Some(slot_id)
            }
        })
        .collect())
}

/// A mutation request for a [`Task`].
#[derive(Debug, Clone, Deserialize)]
pub struct TaskDelta {
    /// See [`Task::title`]
    #[serde(default)]
    pub title: Update<String>,

    /// See [`Task::desc`]
    #[serde(default)]
    pub desc: Update<String>,

    /// See [`Task::skills`]
    #[serde(default)]
    pub skills: SetDelta<SkillId, ProficiencyReq>,

    /// See [`Task::deadline`]
    #[serde(default)]
    pub deadline: Update<Option<DateTime<Utc>>>,

    /// See [`Task::deps`]
    #[serde(default)]
    pub deps: KeySetDelta<TaskId>,
}

/// Mutate [`Task`]s.
///
/// Returns a collection of all failed changes.
/// If all requested changes were successful, the list will be empty.
pub fn mut_tasks(delta: TaskMap<TaskDelta>) -> Result<TaskSet> {
    let mut tasks = TASKS.write();
    Ok(delta
        .into_iter()
        .filter_map(|(task_id, mut delta)| {
            if let Some(task) = tasks.get_mut(&task_id) {
                delta.title.apply(&mut task.title);
                delta.desc.apply(&mut task.desc);
                delta.skills.apply(&mut task.skills);
                delta.deadline.apply(&mut task.deadline);
                delta.deps.apply(&mut task.deps);
                None
            } else {
                Some(task_id)
            }
        })
        .collect())
}

/// A mutation request for a [`User`].
#[derive(Debug, Clone, Deserialize)]
pub struct UserDelta {
    /// See [`User::name`]
    #[serde(default)]
    pub name: Update<String>,

    /// Cannot grow. Use [`add_rules`] to create new rules.
    ///
    /// **Reasoning:**
    /// Newly created [`Rule`]s would become inaccessible because
    /// successful items do not have their IDs returned by `mut_` methods.
    ///
    /// See [`User::availability`]
    #[serde(default)]
    pub availability: NoGrowSetDelta<RuleId, RuleDelta>,

    /// See [`User::user_prefs`]
    #[serde(default)]
    pub user_prefs: SetDelta<UserId, Preference>,

    /// See [`User::skills`]
    #[serde(default)]
    pub skills: SetDelta<SkillId, Proficiency>,
}

/// Mutate [`User`]s.
///
/// Returns a collection of all failed changes.
/// If all requested changes were successful, the list will be empty.
pub fn mut_users(delta: UserMap<UserDelta>) -> Result<UserMap<RuleSet>> {
    let mut users = USERS.write();
    Ok(delta
        .into_iter()
        .filter_map(|(user_id, mut delta)| {
            if let Some(user) = users.get_mut(&user_id) {
                delta.name.apply(&mut user.name);
                {
                    let NoGrowSetDelta { delete, update } = &mut delta.availability;
                    user.availability.retain(|k, rule| {
                        if delete.remove(k) {
                            false
                        } else {
                            if let Some(delta) = update.remove(k) {
                                {
                                    let SetDelta {
                                        mut delete,
                                        create,
                                        mut update,
                                    } = delta.include;
                                    let mut it = 0..;
                                    rule.include.retain(|v| {
                                        let i = it.next().unwrap();
                                        if delete.remove(&i) {
                                            false
                                        } else {
                                            if let Some(replacement) = update.remove(&i) {
                                                *v = replacement;
                                            }
                                            true
                                        }
                                    });
                                    rule.include.extend(create);
                                }
                                if let Some(new_value) = delta.rep {
                                    rule.rep = new_value;
                                }
                                if let Some(new_value) = delta.pref {
                                    rule.pref = new_value;
                                }
                            }
                            true
                        }
                    });
                }
                delta.user_prefs.apply(&mut user.user_prefs);
                delta.skills.apply(&mut user.skills);
            }
            todo!()
        })
        .collect())
}

/// Removes one or more rules from one or more users.
///
/// Returns a collection of all failed removals.
/// If all requested removals were successful, the list will be empty.
///
/// Argument must be an array, even if only removing one.
///
/// # Signature
/// ```py
/// def pop_rules(to_pop: dict[UserId, set[RuleId]]) -> dict[UserId, set[RuleId]];
/// ```
pub fn pop_rules(to_pop: UserMap<RuleSet>) -> Result<UserMap<RuleSet>> {
    let mut users = USERS.write();
    Ok(to_pop
        .into_iter()
        .map(|(user, mut rules)| {
            if let Some(user) = users.get_mut(&user) {
                user.availability.retain(|id, _| !rules.remove(id));
            }
            (user, rules)
        })
        .filter(|(_user, rules)| !rules.is_empty())
        .collect())
}

/// Removes slots by ID.
///
/// Returns a list of any IDs that failed to be removed (ex: slot with that ID did not exist).
/// If all requested removals were successful, the list will be empty.
///
/// Argument must be an array, even if only removing one.
///
/// # Signature
/// ```py
/// def pop_slots(to_pop: set[SlotId]) -> set[SlotId];
/// ```
pub fn pop_slots(mut to_pop: SlotSet) -> Result<SlotSet> {
    SLOTS.write().retain(|id, _| !to_pop.remove(id));
    Ok(to_pop)
}

/// Removes tasks by ID.
///
/// Returns a list of any IDs that failed to be removed (ex: task with that ID did not exist).
/// If all requested removals were successful, the list will be empty.
///
/// Argument must be an array, even if only removing one.
///
/// # Signature
/// ```py
/// def pop_tasks(to_pop: set[TaskId]) -> set[TaskId];
/// ```
pub fn pop_tasks(mut to_pop: TaskSet) -> Result<TaskSet> {
    TASKS.write().retain(|id, _| !to_pop.remove(id));
    Ok(to_pop)
}

/// Removes users by ID.
///
/// Returns a list of any IDs that failed to be removed (ex: user with that ID did not exist).
/// If all requested removals were successful, the list will be empty.
///
/// Argument must be an array, even if only adding one.
///
/// # Signature
/// ```py
/// def pop_users(to_pop: set[UserId]) -> set[UserId];
/// ```
pub fn pop_users(mut to_pop: UserSet) -> Result<UserSet> {
    USERS.write().retain(|id, _| !to_pop.remove(id));
    Ok(to_pop)
}

/// Save all current [`Slot`] data to a file stored at `path`.
pub fn save_slots(path: PathBuf) -> Result<()> {
    csv::WriterBuilder::default()
        .from_path(path)
        .and_then(|mut w| w.serialize(SLOTS.read().values().collect::<Vec<_>>()))
        .map_err(|e| Fault::new(500, e.to_string()))
}

/// Save all current [`Task`] data to a file stored at `path`.
pub fn save_tasks(path: PathBuf) -> Result<()> {
    csv::WriterBuilder::default()
        .from_path(path)
        .and_then(|mut w| w.serialize(TASKS.read().values().collect::<Vec<_>>()))
        .map_err(|e| Fault::new(500, e.to_string()))
}

/// Save all current [`User`] data to a file stored at `path`.
///
/// Also saves all [`Rule`]s.
pub fn save_users(path: PathBuf) -> Result<()> {
    csv::WriterBuilder::default()
        .from_path(path)
        .and_then(|mut w| w.serialize(USERS.read().values().collect::<Vec<_>>()))
        .map_err(|e| Fault::new(500, e.to_string()))
}

/// Load all current [`Slot`] data to a file stored at `path`.
///
/// **WARNING:** Current data will be overwitten without saving!
pub fn load_slots(path: PathBuf) -> Result<()> {
    let mut next_id = 0;
    **SLOTS.write() = csv::ReaderBuilder::default()
        .from_path(path)
        .and_then(|r| {
            r.into_deserialize::<Slot>()
                .map(|x| {
                    x.map(|slot| {
                        next_id = next_id.max(slot.id.0 + 1);
                        (slot.id, slot)
                    })
                })
                .collect()
        })
        .map_err(|e| Fault::new(500, e.to_string()))?;
    SlotId::store(next_id);
    Ok(())
}

/// Load all current [`Task`] data to a file stored at `path`.
///
/// **WARNING:** Current data will be overwitten without saving!
pub fn load_tasks(path: PathBuf) -> Result<()> {
    let mut next_id = 0;
    **TASKS.write() = csv::ReaderBuilder::default()
        .from_path(path)
        .and_then(|r| {
            r.into_deserialize::<Task>()
                .map(|x| {
                    x.map(|task| {
                        next_id = next_id.max(task.id.0 + 1);
                        (task.id, task)
                    })
                })
                .collect()
        })
        .map_err(|e| Fault::new(500, e.to_string()))?;
    TaskId::store(next_id);
    Ok(())
}

/// Load all current [`User`] data to a file stored at `path`.
///
/// Also loads all [`Rule`]s.
///
/// **WARNING:** Current data will be overwitten without saving!
pub fn load_users(path: PathBuf) -> Result<()> {
    let mut next_id = 0;
    let mut rule_id = 0;
    **USERS.write() = csv::ReaderBuilder::default()
        .from_path(path)
        .and_then(|r| {
            r.into_deserialize::<User>()
                .map(|x| {
                    x.map(|user| {
                        next_id = next_id.max(user.id.0 + 1);
                        if let Some(max) = user.availability.keys().map(|id| id.0).max() {
                            rule_id = max.max(rule_id);
                        }
                        (user.id, user)
                    })
                })
                .collect()
        })
        .map_err(|e| Fault::new(500, e.to_string()))?;
    UserId::store(next_id);
    RuleId::store(rule_id);
    Ok(())
}

/// Clear all current [`Slot`] data.
///
/// **WARNING:** Current data will not be saved!
pub fn wipe_slots((): ()) -> Result<()> {
    SLOTS.write().clear();
    SlotId::store(0);
    Ok(())
}

/// Clear all current [`Task`] data.
///
/// **WARNING:** Current data will not be saved!
pub fn wipe_tasks((): ()) -> Result<()> {
    TASKS.write().clear();
    TaskId::store(0);
    Ok(())
}

/// Clear all current [`User`] data.
///
/// Also clears all [`Rule`]s.
///
/// **WARNING:** Current data will not be saved!
pub fn wipe_users((): ()) -> Result<()> {
    USERS.write().clear();
    UserId::store(0);
    RuleId::store(0);
    Ok(())
}

/// Close the server after completing all ongoing tasks.
///
/// # Signature
/// ```py
/// def quit(_: {}) -> None;
/// ```
///
/// # Examples
/// ```py
/// # request server close
/// proxy.quit({})
/// ```
pub fn quit((): ()) -> Result<()> {
    EXIT_REQUESTED.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

pub(crate) fn register(server: &mut Server) {
    server.register_simple("pat_starts_with", Pattern::starts_with);
    server.register_simple("pat_ends_with", Pattern::ends_with);
    server.register_simple("pat_contains", Pattern::contains);
    server.register_simple("pat_exactly", Pattern::exactly);
    server.register_simple("pat_regex", Pattern::regex);

    server.register_simple("add_rules", add_rules);
    server.register_simple("add_slots", add_slots);
    server.register_simple("add_tasks", add_tasks);
    server.register_simple("add_users", add_users);

    server.register_simple("get_rules", get_rules);
    server.register_simple("get_slots", get_slots);
    server.register_simple("get_tasks", get_tasks);
    server.register_simple("get_users", get_users);

    // rules can be mutated through `availability` field of `mut_users`
    server.register_simple("mut_slots", mut_slots);
    server.register_simple("mut_tasks", mut_tasks);
    server.register_simple("mut_users", mut_users);

    server.register_simple("pop_rules", pop_rules);
    server.register_simple("pop_slots", pop_slots);
    server.register_simple("pop_tasks", pop_tasks);
    server.register_simple("pop_users", pop_users);

    server.register_simple("save_slots", save_slots);
    server.register_simple("save_tasks", save_tasks);
    server.register_simple("save_users", save_users);

    server.register_simple("load_slots", load_slots);
    server.register_simple("load_tasks", load_tasks);
    server.register_simple("load_users", load_users);

    server.register_simple("wipe_slots", wipe_slots);
    server.register_simple("wipe_tasks", wipe_tasks);
    server.register_simple("wipe_users", wipe_users);

    server.register_simple("quit", quit);
}
