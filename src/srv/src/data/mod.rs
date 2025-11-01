//! Data that is used for generating schedules

pub mod skill;
pub mod slot;
pub mod task;
pub mod user;

pub use skill::*;
pub use slot::*;
pub use task::*;
pub use user::*;

#[cfg(test)]
pub use crate::{datetime, slots, tasks, users};

/// Create a [`DateTime`](chrono::prelude::DateTime) literal.
#[macro_export]
macro_rules! datetime {
    ($mo:literal/$d:literal/$yr:literal$( @ $hr:literal:$m:literal)?) => {
        <chrono::prelude::Utc as chrono::TimeZone>::from_utc_datetime(
            &chrono::prelude::Utc,
            &chrono::prelude::NaiveDateTime::new(
                chrono::prelude::NaiveDate::from_ymd_opt($yr, $mo, $d)
                    .unwrap_or_else(|| panic!(
                        "`{}/{}/{}` is not a valid date",
                        $mo,
                        $d,
                        $yr,
                    )),
                None$(.or(Some(chrono::prelude::NaiveTime::from_hms_opt($hr, $m, 0)
                    .unwrap_or_else(|| panic!(
                        "`{}:{}` is not a valid time",
                        $hr,
                        $m,
                    )))))?
                    .unwrap_or_default(),
            ),
        )
    };
}

/// Create a [`Vec`] of [`Slot`s](slot::Slot) for testing.
///
/// Expects `m/d/y - m/d/y` format. Time can be appended to date with `@ h:m`.
#[macro_export]
macro_rules! slots {
    ($(
        $mo0:literal/$d0:literal/$yr0:literal$( @ $hr0:literal:$m0:literal)?
        -
        $mo1:literal/$d1:literal/$yr1:literal$( @ $hr1:literal:$m1:literal)?
        $(| $name:literal)?
    ),+ $(,)?) => {
        vec![$(
            $crate::data::slot::Slot {
                interval: $crate::data::slot::TimeInterval(
                    $crate::datetime!($mo0/$d0/$yr0$( @ $hr0:$m0)?)..
                    $crate::datetime!($mo1/$d1/$yr1$( @ $hr1:$m1)?)
                ),
                name: None$(.or(Some($name.to_string())))?
            }
        ),*]
    };

    () => {
        Vec::<$crate::data::slot::Slot>::new()
    };
}

/// Create a [`TaskMap`](task::TaskMap) for testing.
#[macro_export]
macro_rules! tasks {
    ($(
        $id:literal: $title:literal
        $([$mo:literal/$d:literal/$yr:literal$( @ $hr:literal:$m:literal)?])?
        { $($dep:literal),* $(,)? }
    ),* $(,)?) => {
        [$(
            $crate::data::task::Task {
                id: $crate::data::task::TaskId($id),
                title: $title.to_string(),
                desc: String::new(),
                skills: $crate::data::skill::SkillMap::default(),
                deadline: None$(.or(Some(
                    datetime!($mo/$d/$yr$( @ $hr:$m)?)
                )))?,
                deps: $crate::data::task::TaskSet::from_iter([$($crate::data::task::TaskId($dep)),*]),
            }
        ),*]
            .into_iter()
            .map(|task: $crate::data::task::Task| (task.id, task))
            .collect::<$crate::data::task::TaskMap>()
    };

    () => {
        $crate::data::task::TaskMap::default()
    };
}

/// Create a [`UserMap`](user::UserMap) for testing.
#[macro_export]
macro_rules! users {
    ($(
        $id:literal: $name:literal
    ),* $(,)?) => {
        [$(
            $crate::data::user::User {
                id: $id
                name: $name,
                availability: (),
                user_prefs: (),
                skills: (),
            }
        ),*]
            .into_iter()
            .map(|user: $crate::data::user::User| (user.id, user))
            .collect::<$crate::data::user::UserMap>()
    };

    () => {
        $crate::data::user::UserMap::default()
    };
}
