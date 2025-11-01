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
pub use crate::{datetime, slots, tasks, time_interval, users};

/// Create a [`DateTime`](chrono::prelude::DateTime) literal.
#[macro_export]
macro_rules! datetime {
    ($mo:literal/$d:literal/$yr:literal @ $hr:literal:$m:literal) => {
        <chrono::prelude::Utc as chrono::TimeZone>::from_utc_datetime(
            &chrono::prelude::Utc,
            &chrono::prelude::NaiveDateTime::new(
                chrono::prelude::NaiveDate::from_ymd_opt($yr, $mo, $d)
                    .unwrap_or_else(|| panic!("`{}/{}/{}` is not a valid date", $mo, $d, $yr)),
                chrono::prelude::NaiveTime::from_hms_opt($hr, $m, 0)
                    .unwrap_or_else(|| panic!("`{}:{}` is not a valid time", $hr, $m)),
            ),
        )
    };

    ($mo:literal/$d:literal/$yr:literal) => {
        <chrono::prelude::Utc as chrono::TimeZone>::from_utc_datetime(
            &chrono::prelude::Utc,
            &chrono::prelude::NaiveDateTime::new(
                chrono::prelude::NaiveDate::from_ymd_opt($yr, $mo, $d)
                    .unwrap_or_else(|| panic!("`{}/{}/{}` is not a valid date", $mo, $d, $yr)),
                Default::default(),
            ),
        )
    };
}

/// Create a [`TimeInterval`] literal.
#[macro_export]
macro_rules! time_interval {
    (
        $mo0:literal/$d0:literal/$yr0:literal @ $hr0:literal:$m0:literal -
        $mo1:literal/$d1:literal/$yr1:literal @ $hr1:literal:$m1:literal
    ) => {
        $crate::data::slot::TimeInterval(
            $crate::datetime!($mo0/$d0/$yr0 @ $hr0:$m0)..
            $crate::datetime!($mo1/$d1/$yr1 @ $hr1:$m1)
        )
    };

    (
        $mo0:literal/$d0:literal/$yr0:literal -
        $mo1:literal/$d1:literal/$yr1:literal
    ) => {
        $crate::data::slot::TimeInterval(
            $crate::datetime!($mo0/$d0/$yr0)..
            $crate::datetime!($mo1/$d1/$yr1)
        )
    };
}

/// Create a [`Vec`] of [`Slot`]s for testing.
///
/// Expects `m/d/y - m/d/y` format. Time can be appended to date with `@ h:m`.
#[macro_export]
macro_rules! slots {
    ($(
        $mo0:literal/$d0:literal/$yr0:literal$( @ $hr0:literal:$m0:literal)? -
        $mo1:literal/$d1:literal/$yr1:literal$( @ $hr1:literal:$m1:literal)?
        $([$min_staff:literal])?
        $(| $name:literal)?
    ),+ $(,)?) => {
        vec![$(
            $crate::data::slot::Slot {
                interval: $crate::time_interval!($mo0/$d0/$yr0$( @ $hr0:$m0)? - $mo1/$d1/$yr1$( @ $hr1:$m1)?),
                min_staff: None$(.or(std::num::NonZeroUsize::new($min_staff)))?,
                name: None$(.or(Some($name.to_string())))?
            }
        ),*]
    };

    () => {
        Vec::<$crate::data::slot::Slot>::new()
    };
}

/// Create a [`TaskMap`] for testing.
#[macro_export]
macro_rules! tasks {
    ($(
        $id:literal: $title:literal
        $([$mo:literal/$d:literal/$yr:literal$( @ $hr:literal:$m:literal)?])?
        { $($dep:literal),* $(,)? }
    ),+ $(,)?) => {
        [$(
            $crate::data::task::Task {
                id: $crate::data::task::TaskId($id),
                title: $title.to_string(),
                desc: String::new(),
                skills: Default::default(/* TODO */),
                deadline: None$(.or(Some(
                    datetime!($mo/$d/$yr$( @ $hr:$m)?)
                )))?,
                deps: $crate::data::task::TaskSet::from_iter([$($crate::data::task::TaskId($dep)),*]),
            }
        ),*]
            .into_iter()
            .map(|task| (task.id, task))
            .collect::<$crate::data::task::TaskMap>()
    };

    () => {
        $crate::data::task::TaskMap::default()
    };
}

/// Create a [`UserMap`] for testing.
#[macro_export]
macro_rules! users {
    ($(
        $id:literal: $name:literal
        {$(
            $mo0:literal/$d0:literal/$yr0:literal$( @ $hr0:literal:$m0:literal)? -
            $mo1:literal/$d1:literal/$yr1:literal$( @ $hr1:literal:$m1:literal)?
            | $pref:expr
        ),* $(,)?}
    ),+ $(,)?) => {
        [$(
            $crate::data::user::User {
                id: $crate::data::user::UserId($id),
                name: $name.to_string(),
                availability: vec![$(
                    (
                        $crate::time_interval!($mo0/$d0/$yr0$( @ $hr0:$m0)? - $mo1/$d1/$yr1$( @ $hr1:$m1)?),
                        $crate::data::user::Preference($pref),
                    )
                ),*],
                user_prefs: Default::default(/* TODO */),
                skills: Default::default(/* TODO */),
            }
        ),*]
            .into_iter()
            .map(|user| (user.id, user))
            .collect::<$crate::data::user::UserMap>()
    };

    () => {
        $crate::data::user::UserMap::default()
    };
}
