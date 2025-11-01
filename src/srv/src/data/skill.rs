use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Code uniquely identifying a skill - used to determine which users *can* be scheduled on a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub u32);

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s.{:x}", self.0)
    }
}

/// Metadata regarding a skill
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skill {
    /// Display name of the skill
    pub name: String,
    /// Description of the skill
    pub desc: String,
}

/// A dictionary associating [`SkillId`]s with `T`.
pub type SkillMap<T = Skill> = FxHashMap<SkillId, T>;

/// Level of skill
///
/// 0.0 = no skill.
/// 1.0 = skill of one user with baseline skill.
/// Can be multiplied by number of users.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Proficiency(f32);

impl std::fmt::Display for Proficiency {
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

impl std::ops::Deref for Proficiency {
    type Target = f32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Proficiency {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Proficiency {
    /// No proficiency
    pub const ZERO: Self = Self(0.0);
    /// Baseline proficiency
    pub const ONE: Self = Self(1.0);
    /// Alias for [`Self::ZERO`]
    pub const MIN: Self = Self::ZERO;
    /// Alias for [`f32::MAX`]
    pub const MAX: Self = Self(f32::MAX);

    /// Clamp between [`Self::MIN`] and [`Self::MAX`]
    pub const fn saturate(self) -> Self {
        Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
    }
}
