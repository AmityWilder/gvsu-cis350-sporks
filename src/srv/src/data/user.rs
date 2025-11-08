//! See [`User`]

use crate::data::{
    skill::{Proficiency, SkillMap},
    slot::TimeInterval,
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

/// Preference/opposition score.
///
/// Range: `-inf, -1.0..=1.0, inf`
///
/// Infinities should be reserved for cases where failure to meet the requirement
/// would cause legal or other undesireable problems and should be *hard*-rejected.
///
/// # Values
///
/// ## `0.0`
/// No preference.
///
/// Equivalent to not being listed at all; which should be preferred for storage reasons.
///
/// ## `1.0`
/// Maximize scheduling. Only do otherwise if no other option.
///
/// ## `-1.0`
/// Minimize scheduling. Only do otherwise if no other option.
///
/// ## [`+inf`](`f32::INFINITY`)
/// **Always** schedule.
///
/// ### Towards time
/// If unable to be scheduled at this time, **schedule production should result in an error requiring manager input.**
///
/// **ex:** leader at critical event
///
/// ### Towards users
/// If unable to be scheduled *together*, **do not schedule *this* user.**
///
/// **ex:** handler
///
/// ## [`-inf`](`f32::NEG_INFINITY`)
/// **Never** schedule.
///
/// ### Towards time
/// If unable to be scheduled any other time, **do not schedule *this* user.**
///
/// **ex:** sick, mourning, vacation; physically *unable* to be present.
///
/// ### Towards users
/// If unable to be scheduled *separately*, **do not schedule *that* user.**
///
/// **ex:** restraining order, history of harassment
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Preference(pub f32);

impl PartialEq for Preference {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for Preference {}

impl PartialOrd for Preference {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Preference {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .expect("preference may be inf, but should never be NaN")
    }
}

impl std::fmt::Display for Preference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_infinite() {
            write!(f, "{}inf", b"+-"[self.0.is_sign_negative() as usize])
        } else if self.0.is_nan() {
            f.write_str("NaN")
        } else {
            write!(f, "{}%", self.0 * 100.0)
        }
    }
}

impl std::ops::Deref for Preference {
    type Target = f32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Preference {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Preference {
    /// Mandatory
    pub const INFINITY: Self = Self(f32::INFINITY);
    /// Forbidden
    pub const NEG_INFINITY: Self = Self(f32::NEG_INFINITY);
    /// Maximum (100%) refusal
    pub const MIN: Self = Self(-1.0);
    /// Maximum (100%) preference
    pub const MAX: Self = Self(1.0);

    /// Clamp to `-inf, 0.0..=1.0, +inf`
    pub const fn saturate(self) -> Self {
        if self.0.is_infinite() {
            self
        } else {
            Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
        }
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
    ///
    /// Ex:
    /// - "available every Monday 3pm-7pm",
    /// - "never available on Fridays"
    pub availability: Vec<(TimeInterval, Preference)>,

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
