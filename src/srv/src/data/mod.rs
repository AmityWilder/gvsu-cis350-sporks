//! Data that is used for generating schedules

pub mod pref;
pub mod rule;
pub mod skill;
pub mod slot;
pub mod task;
pub mod user;

pub use pref::*;
pub use rule::*;
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
        $crate::data::slot::TimeInterval {
            start: $crate::datetime!($mo0/$d0/$yr0 @ $hr0:$m0),
            end: $crate::datetime!($mo1/$d1/$yr1 @ $hr1:$m1)
        }
    };

    (
        $mo0:literal/$d0:literal/$yr0:literal -
        $mo1:literal/$d1:literal/$yr1:literal
    ) => {
        $crate::data::slot::TimeInterval {
            start: $crate::datetime!($mo0/$d0/$yr0),
            end: $crate::datetime!($mo1/$d1/$yr1)
        }
    };
}

/// Create a [`Vec`] of [`Slot`]s for testing.
///
/// Expects `m/d/y - m/d/y` format. Time can be appended to date with `@ h:m`.
#[macro_export]
macro_rules! slots {
    ($(
        $id:literal:
        $mo0:literal/$d0:literal/$yr0:literal$( @ $hr0:literal:$m0:literal)? -
        $mo1:literal/$d1:literal/$yr1:literal$( @ $hr1:literal:$m1:literal)?
        $([$min_staff:literal])?
        $(| $name:literal)?
    ),+ $(,)?) => {
        vec![$(
            $crate::data::slot::Slot {
                id: $crate::data::slot::SlotId($id),
                interval: $crate::time_interval!($mo0/$d0/$yr0$( @ $hr0:$m0)? - $mo1/$d1/$yr1$( @ $hr1:$m1)?),
                min_staff: None$(.or(std::num::NonZeroUsize::new($min_staff)))?,
                name: None$(.or(Some($name.to_string())))?.unwrap_or(String::new())
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
                    Rule {
                        include: smallvec::smallvec![$crate::time_interval!($mo0/$d0/$yr0$( @ $hr0:$m0)? - $mo1/$d1/$yr1$( @ $hr1:$m1)?)],
                        rep: None,
                        pref: $crate::data::pref::Preference($pref),
                    },
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

macro_rules! id_type {
    ($(#[$m:meta])* impl Id<$repr:ty> for $Type:ident as $prefix:literal) => {
        ::paste::paste! {
            #[doc = " Code uniquely identifying a [`" $Type "`]."]
            $(#[$m])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
            pub struct [<$Type Id>](pub(crate) $repr);

            #[allow(dead_code)]
            static [<NEXT_ $Type:snake:upper _ID>]: ::std::sync::atomic::[<Atomic $repr:camel>] = ::std::sync::atomic::[<Atomic $repr:camel>]::new(0);

            #[allow(dead_code)]
            impl [<$Type Id>] {
                pub(crate) fn store(value: $repr) {
                    [<NEXT_ $Type:snake:upper _ID>].store(value, ::std::sync::atomic::Ordering::Relaxed);
                }

                pub(crate) fn next() -> Option<Self> {
                    Self::take(1).next()
                }

                pub(crate) fn take(n: $repr) -> ::std::iter::Map<::std::ops::Range<$repr>, fn($repr) -> Self> {
                    let start = [<NEXT_ $Type:snake:upper _ID>].fetch_add(n, ::std::sync::atomic::Ordering::Relaxed);
                    (start..start + n).map(Self)
                }
            }

            impl std::fmt::Display for [<$Type Id>] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, concat!($prefix, ".{:x}"), self.0)
                }
            }

            #[doc = " A dictionary associating [`" [<$Type Id>] "`]s with `T`."]
            pub type [<$Type Map>]<T = $Type> = ::rustc_hash::FxHashMap<[<$Type Id>], T>;

            #[doc = " A set of [`" [<$Type Id>] "`]s."]
            pub type [<$Type Set>] = ::rustc_hash::FxHashSet<[<$Type Id>]>;
        }
    };
}

pub(crate) use id_type;
