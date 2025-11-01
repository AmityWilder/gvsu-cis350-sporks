use crate::data::skill::{Proficiency, SkillId};
use chrono::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// Code uniquely identifying a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub u64);

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t.{:x}", self.0)
    }
}

/// Proficiency requirements for a skill on a [`Task`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProficiencyReq {
    /// The ideal proficiency.
    pub target: Proficiency,

    /// The lower bound of the target - try to stay above this.
    pub soft_min: Proficiency,
    /// The upper bound of the target - try to stay below this.
    pub soft_max: Proficiency,

    /// The lower bound - reject any solution below this.
    pub hard_min: Proficiency,
    /// The upper bound - reject any solution above this.
    pub hard_max: Proficiency,
}

impl ProficiencyReq {
    /// Construct a new [`ProficiencyReq`] from ideal and ranges.
    pub fn new<R1, R2>(target: Proficiency, soft_range: R1, hard_range: R2) -> Option<Self>
    where
        R1: std::ops::RangeBounds<Proficiency>,
        R2: std::ops::RangeBounds<Proficiency>,
    {
        trait BoundExt<T> {
            fn get(self) -> Option<T>;
        }

        impl<T: Copy> BoundExt<T> for std::ops::Bound<&T> {
            fn get(self) -> Option<T> {
                match self {
                    std::ops::Bound::Included(x) | std::ops::Bound::Excluded(x) => Some(*x),
                    std::ops::Bound::Unbounded => None,
                }
            }
        }

        fn range_to_vals(
            range: impl std::ops::RangeBounds<Proficiency>,
        ) -> (Proficiency, Proficiency) {
            (
                range.start_bound().get().unwrap_or(Proficiency::MIN),
                range.end_bound().get().unwrap_or(Proficiency::MAX),
            )
        }

        let (hard_min, hard_max) = range_to_vals(hard_range);
        let (soft_min, soft_max) = range_to_vals(soft_range);
        (hard_min <= soft_min && soft_max <= hard_max).then_some(Self {
            target,
            soft_min,
            soft_max,
            hard_min,
            hard_max,
        })
    }
}

/// A product or service to be completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Duplicate of the task's ID.
    pub id: TaskId,

    /// The name of the task.
    pub title: String,

    /// The task description.
    pub desc: String,

    /// Skills required to perform the task.
    ///
    /// Optimize covering with users whose combined capability equals the float provided (maxed out at 1.0 per individual)
    /// Prefer to overshoot (except in great excess, like 200+%) rather than undershoot, but prioritizing closer matches.
    pub skills: FxHashMap<SkillId, ProficiencyReq>,

    /// [`None`]: Task has no "completion" state.
    pub deadline: Option<DateTime<Utc>>,

    /// Dependencies - [`Task`]s that must be completed before this one can be scheduled (estimated by deadlines).
    pub deps: FxHashSet<TaskId>,
}

/// A dictionary associating task IDs with their tasks.
pub type TaskMap = FxHashMap<TaskId, Task>;
