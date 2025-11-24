//! How availability is determined

use crate::data::{Preference, TimeInterval};
use chrono::{DateTime, Days, Months, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Code uniquely identifying a [`Rule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(pub u64);

/// Once every `n` units. Fields are added together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Frequency {
    /// Repeat every `n` seconds
    pub seconds: u8,
    /// Repeat every `n` minutes
    pub minutes: u8,
    /// Repeat every `n` hours
    pub hours: u8,
    /// Repeat every `n` days
    pub days: u8,
    /// Repeat every `n` weeks
    pub weeks: u8,
    /// Repeat every `n` months
    pub months: u8,
    /// Repeat every `n` years
    pub years: u16,
}

impl Frequency {
    #[inline]
    fn checked_add_date(self, date: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let seconds = i64::from(self.seconds) + 60 * i64::from(self.minutes);
        let days = u64::from(self.days) + 7 * u64::from(self.weeks);
        let months = u32::from(self.months) + 12 * u32::from(self.years);
        date.checked_add_signed(TimeDelta::seconds(seconds))?
            .checked_add_days(Days::new(days))?
            .checked_add_months(Months::new(months))
    }
}

/// How to repeat a [`Rule`]'s intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repetition {
    /// The frequency of the repetition.
    pub every: Frequency,

    /// When the repetition begins.
    pub start: DateTime<Utc>,

    /// When the repetition should end. [`None`] if permanent.
    pub until: Option<DateTime<Utc>>,
}

struct RepetitionIter<'a> {
    rep: &'a Repetition,
    curr: Option<DateTime<Utc>>,
}

impl Iterator for RepetitionIter<'_> {
    type Item = DateTime<Utc>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.curr
            .filter(|date| self.rep.until.as_ref().is_none_or(|end| date <= end))
            .inspect(|date| {
                self.curr = self.rep.every.checked_add_date(*date);
            })
    }
}

impl Repetition {
    #[inline]
    fn iter(&self) -> RepetitionIter<'_> {
        RepetitionIter {
            rep: self,
            curr: Some(self.start),
        }
    }
}

/// A rule for determining availability.
///
/// Ex:
/// - "available every Monday 3pm-7pm"
/// - "never available on Fridays"
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rule {
    /// The specific intervals this rule involves, before repeating.
    pub include: SmallVec<[TimeInterval; 1]>,

    /// How often `include` repeats. [`None`] if one-off.
    pub rep: Option<Repetition>,

    /// How strongly to enforce this rule.
    pub pref: Preference,
}

impl FromIterator<TimeInterval> for Rule {
    #[inline]
    fn from_iter<T: IntoIterator<Item = TimeInterval>>(iter: T) -> Self {
        Self {
            include: SmallVec::from_iter(iter),
            rep: None,
            pref: Preference(0.0),
        }
    }
}

impl Rule {
    /// Whether the rule fully covers the interval with at least one
    /// `include` or the repetition of an `include`.
    pub fn contains(&self, interval: &TimeInterval) -> bool {
        match self.rep {
            Some(rep) => {
                // bounds test
                (interval.start >= rep.start && rep.until.is_none_or(|end| interval.end <= end))
                    && rep.iter().any(|date| {
                        // TODO: consider something akin to modulo
                        let offset = date.signed_duration_since(rep.start);
                        self.include
                            .iter()
                            .filter_map(|t| {
                                t.start
                                    .checked_add_signed(offset)
                                    .zip(t.end.checked_add_signed(offset))
                                    .map(|(start, end)| TimeInterval { start, end })
                            })
                            .any(|t| t.contains(interval))
                    })
            }
            None => self.include.iter().any(|t| t.contains(interval)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_interval;
    use smallvec::smallvec;

    #[test]
    fn test_one_include_no_rep() {
        let rule = Rule {
            include: smallvec![time_interval! { 4/5/2025 - 5/5/2025 }],
            rep: None,
            pref: Preference(0.0),
        };

        assert!(
            rule.contains(&time_interval! { 4/5/2025 - 5/5/2025 }),
            "identical should count as contained"
        );

        assert!(
            !rule.contains(&time_interval! { 4/5/2025 - 5/6/2025 }),
            "later end should not count as contained"
        );
        assert!(
            !rule.contains(&time_interval! { 4/4/2025 - 5/5/2025 }),
            "earlier start should not count as contained"
        );
        assert!(
            !rule.contains(&time_interval! { 4/4/2025 - 5/6/2025 }),
            "earlier start + later end should not count as contained"
        );

        assert!(
            rule.contains(&time_interval! { 4/6/2025 - 5/6/2025 }),
            "later start should count as contained"
        );
        assert!(
            rule.contains(&time_interval! { 4/5/2025 - 5/4/2025 }),
            "earlier end should count as contained"
        );
        assert!(
            rule.contains(&time_interval! { 4/6/2025 - 5/4/2025 }),
            "later start + earlier end should count as contained"
        );
    }

    #[test]
    fn test_multiple_include_no_rep() {
        let rule = Rule {
            include: smallvec![time_interval! { 4/5/2025 - 5/5/2025 }],
            rep: None,
            pref: Preference(0.0),
        };

        assert!(rule.contains(&time_interval! { 4/5/2025 - 5/5/2025 }));
    }

    #[test]
    fn test_one_include_some_rep() {
        let rule = Rule {
            include: smallvec![time_interval! { 4/5/2025 - 5/5/2025 }],
            rep: None,
            pref: Preference(0.0),
        };

        assert!(rule.contains(&time_interval! { 4/5/2025 - 5/5/2025 }));
    }

    #[test]
    fn test_multiple_include_some_rep() {
        let rule = Rule {
            include: smallvec![time_interval! { 4/5/2025 - 5/5/2025 }],
            rep: None,
            pref: Preference(0.0),
        };

        assert!(rule.contains(&time_interval! { 4/5/2025 - 5/5/2025 }));
    }
}
