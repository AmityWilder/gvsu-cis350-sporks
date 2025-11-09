//! See [`User`]

use crate::data::{
    pref::Preference,
    rule::Rule,
    skill::{Proficiency, SkillMap},
};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// Code uniquely identifying a user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub u64);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "u.{:x}", self.0)
    }
}

/// A person who can be scheduled to work on a task.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    /// Duplicate of the task's ID.
    pub id: UserId,

    /// Display name for representing the user on the manager-facing UI.
    /// Can be changed without changing the user's ID.
    pub name: String,

    /// Preferences regarding times the user can or can't be scheduled.
    pub availability: Vec<Rule>,

    /// Preference towards sharing slots with other users.
    ///
    /// Ex:
    /// - "doesn't like Brian"
    /// - "works better when Sally is there"
    pub user_prefs: UserMap<Preference>,

    /// A dictionary of the user's skills and their capability with each skill.
    ///
    /// Skills the user has 0 proficiency with should be excluded to save memory,
    /// as a missing skill is implied to be 0% proficiency.
    pub skills: SkillMap<Proficiency>,
}

/// A dictionary associating [`UserId`]s with `T`.
pub type UserMap<T = User> = FxHashMap<UserId, T>;

/// A set of [`UserId`]s.
pub type UserSet = FxHashSet<UserId>;
