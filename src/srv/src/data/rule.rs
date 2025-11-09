//! How availability is determined

use crate::data::{Preference, TimeInterval};
use chrono::{DateTime, Days, Months, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

/// Once every `n` units. Fields are added together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Error while parsing a [`Rule`] from a string.
#[derive(Debug, Error)]
pub enum ParseRuleError {
    #[error("invalid token")]
    Invalid,

    /// Failed to parse a [`DateTime`].
    #[error(transparent)]
    ParseDateTime(#[from] chrono::format::ParseError),
}

impl std::str::FromStr for Rule {
    type Err = ParseRuleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((start, end)) = s
            .strip_prefix("from ")
            .and_then(|s| s.split_once(" until "))
        {
            let start = start.parse()?;
            let end = end.parse()?;
            Ok(Rule {
                include: SmallVec::from_buf([TimeInterval { start, end }]),
                rep: None,
                pref: Preference(0.0),
            })
        } else {
            todo!()
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
