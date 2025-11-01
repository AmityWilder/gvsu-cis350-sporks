//! See [`Slot`]

use chrono::prelude::*;
use miette::Result;
use serde::{Deserialize, Serialize, de::Visitor};
use std::{num::NonZeroUsize, ops::Range};

/// A timerange, mainly intended for timeslots.
///
/// # [Ordering](`Ord`)
///
/// [`TimeInterval`] is ordered by start, then end.
/// In other words, if [`TimeInterval`] `a` starts before [`TimeInterval`] `b`,
/// then `a` will be ordered ahead of `b` no matter when either ends.
/// However, if both start at the same time and date, then the one that ends first
/// will be ordered ahead of the one that ends later.
///
/// The main purpose of implementing [`Ord`] for [`TimeInterval`] is so that
/// it can be used as a key in a [`BTreeMap`](`std::collections::BTreeMap`)
/// or [`BTreeSet`](`std::collections::BTreeSet`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct TimeInterval(pub Range<DateTime<Utc>>);

/// Custom [`Deserialize`] implementation needed for reading [`TimeInterval`] as map keys.
///
/// ```
/// # use {std::collections::BTreeMap, crate::TimeInterval, serde_json::{self, json}};
/// let events = serde_json::from_value::<BTreeMap<TimeInterval, Vec<String>>>(json!({
///     "2025-09-23T19:44:54+00:00..2025-09-23T19:45:54+00:00": [
///         "foo",
///         "bar"
///     ]
/// }));
///
/// assert_eq!(
///     events.unwrap(),
///     BTreeMap::from_iter([(
///         TimeInterval(
///             "2025-09-23T19:44:54+00:00".parse().unwrap()
///                 .."2025-09-23T19:45:54+00:00".parse().unwrap()
///         ),
///         vec!["foo".to_string(), "bar".to_string()]
///     )])
/// );
/// ```
impl<'de> Deserialize<'de> for TimeInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TimeIntervalVisitor;
        use serde::de::Error;

        impl<'de> Visitor<'de> for TimeIntervalVisitor {
            type Value = TimeInterval;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct TimeInterval")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let start = seq
                    .next_element::<DateTime<Utc>>()?
                    .ok_or_else(|| Error::invalid_length(0, &self))?;
                let end = seq
                    .next_element::<DateTime<Utc>>()?
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                Ok(TimeInterval(start..end))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(field_identifier, rename_all = "lowercase")]
                enum Field {
                    Start,
                    End,
                }

                let mut start = None;
                let mut end = None;
                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        Field::Start => {
                            if start.is_some() {
                                return Err(Error::duplicate_field("start"));
                            }
                            start = Some(value);
                        }
                        Field::End => {
                            if end.is_some() {
                                return Err(Error::duplicate_field("end"));
                            }
                            end = Some(value);
                        }
                    }
                }
                let start = start.ok_or_else(|| Error::missing_field("start"))?;
                let end = end.ok_or_else(|| Error::missing_field("end"))?;
                Ok(TimeInterval(start..end))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let (start, end) = v
                    .split_once("..")
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                let start = start.parse::<DateTime<Utc>>().map_err(Error::custom)?;
                let end = end.parse::<DateTime<Utc>>().map_err(Error::custom)?;
                Ok(TimeInterval(start..end))
            }
        }

        deserializer
            .deserialize_map(TimeIntervalVisitor)
            .and_then(|interval| {
                if interval.start <= interval.end {
                    Ok(interval)
                } else {
                    Err(Error::invalid_value(
                        serde::de::Unexpected::Other("time-reversed interval"),
                        &TimeIntervalVisitor,
                    ))
                }
            })
    }
}

impl std::ops::Deref for TimeInterval {
    type Target = Range<DateTime<Utc>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for TimeInterval {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialOrd for TimeInterval {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeInterval {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.0.start.cmp(&other.0.start) {
            std::cmp::Ordering::Equal => self.0.end.cmp(&other.0.end),
            ord => ord,
        }
    }
}

impl TimeInterval {
    /// Returns whether `self` and `other` occupy some shared range of time.
    /// i.e. their intersection is non-null.
    pub fn is_overlapping(&self, other: &Self) -> bool {
        self.0.end < other.0.start || other.0.end < self.0.start
    }

    /// The overlapping subset of two [`TimeInterval`]s.
    /// Returns [`None`] if there is no overlap.
    pub fn intersection(&self, _other: &Self) -> Option<Self> {
        todo!()
    }

    /// The superset of two overlapping/adjacent [`TimeInterval`]s.
    /// Returns [`None`] if they do not overlap, meaning there would be no change.
    pub fn union(&self, _other: &Self) -> Option<Self> {
        todo!()
    }

    /// A [`TimeInterval`] with `other` excluded.
    /// Returns [`None`] if `other` completely absorbs `self`.
    pub fn difference(&self, _other: &Self) -> Option<Self> {
        todo!()
    }
}

/// A segment of time that can be allocated for work, such as a "shift".
///
/// Slots are ordered by their [`interval`](`Slot::interval`)
/// (See [`TimeInterval` ordering](TimeInterval#ordering)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// The time period the slot refers to.
    pub interval: TimeInterval,

    /// [`None`]: Slot exists as an opportunity to
    /// work on tasks, not as a shift that must be
    /// covered.
    ///
    /// [`Some`]: an error may be emitted if there
    /// is not enough staff to cover the shift,
    /// even if all tasks are completed.
    pub min_staff: Option<NonZeroUsize>,

    /// Name for the slot, if it has one.
    pub name: Option<String>,
}

impl std::ops::Deref for Slot {
    type Target = TimeInterval;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.interval
    }
}

impl std::ops::DerefMut for Slot {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.interval
    }
}
