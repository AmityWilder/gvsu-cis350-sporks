//! See [`Slot`]

use chrono::prelude::*;
use miette::Result;
use serde::{Deserialize, Serialize, de::Visitor};
use std::num::NonZeroUsize;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct TimeInterval {
    /// Beginning of the interval
    pub start: DateTime<Utc>,

    /// Conclusion of the interval
    pub end: DateTime<Utc>,
}

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
                Ok(TimeInterval { start, end })
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
                Ok(TimeInterval { start, end })
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
                Ok(TimeInterval { start, end })
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

impl std::ops::RangeBounds<DateTime<Utc>> for TimeInterval {
    fn start_bound(&self) -> std::ops::Bound<&DateTime<Utc>> {
        std::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&DateTime<Utc>> {
        std::ops::Bound::Excluded(&self.end)
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
        match self.start.cmp(&other.start) {
            std::cmp::Ordering::Equal => self.end.cmp(&other.end),
            ord => ord,
        }
    }
}

impl TimeInterval {
    /// Returns whether `self` and `other` occupy some shared range of time.
    /// i.e. their intersection is non-null.
    pub(crate) fn _is_overlapping(&self, other: &Self) -> bool {
        debug_assert!(self.start <= self.end && other.start <= other.end);
        !(self.end < other.start || other.end < self.start)
    }

    /// Returns whether `self` completely encloses `other`.
    pub(crate) fn contains(&self, other: &Self) -> bool {
        debug_assert!(self.start <= self.end && other.start <= other.end);
        self.start <= other.start && other.end <= self.end
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

    /// Name for the slot. Empty if unnamed.
    pub name: String,
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

#[cfg(test)]
mod tests {
    use crate::time_interval;

    #[test]
    fn test_interval_contains_self() {
        assert!(
            time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/5/2025 - 4/8/2025 }),
            "an interval should contain itself"
        );
    }

    #[test]
    fn test_interval_contains_later_start() {
        assert!(
            time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/6/2025 - 4/8/2025 }),
            "an interval starting later but sharing an end should count as contained"
        );
    }

    #[test]
    fn test_interval_contains_earlier_end() {
        assert!(
            time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/5/2025 - 4/7/2025 }),
            "an interval sharing a start but ending earlier should count as contained"
        );
    }

    #[test]
    fn test_interval_contains_later_start_and_earlier_end() {
        assert!(
            time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/6/2025 - 4/7/2025 }),
            "an interval starting later and ending earlier should count as contained"
        );
    }

    #[test]
    fn test_interval_not_contains_earlier_start() {
        assert!(
            !time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/4/2025 - 4/6/2025 }),
            "an interval starting earlier should not count as contained, even if ending earlier"
        );
        assert!(
            !time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/4/2025 - 4/8/2025 }),
            "an interval starting earlier should not count as contained, even if sharing an end"
        );
        assert!(
            !time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/4/2025 - 4/7/2025 }),
            "an interval starting earlier should not count as contained, even if sharing a duration"
        );
    }

    #[test]
    fn test_interval_not_contains_later_end() {
        assert!(
            !time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/4/2025 - 4/6/2025 }),
            "an interval starting earlier should not count as contained, even if ending earlier"
        );
        assert!(
            !time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/4/2025 - 4/8/2025 }),
            "an interval starting earlier should not count as contained, even if sharing an end"
        );
        assert!(
            !time_interval! { 4/5/2025 - 4/8/2025 }
                .contains(&time_interval! { 4/4/2025 - 4/7/2025 }),
            "an interval starting earlier should not count as contained, even if sharing a duration"
        );
    }
}
