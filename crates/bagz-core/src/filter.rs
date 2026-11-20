//! Filters over the example catalog.
//!
//! Comparisons are case-insensitive and ignore surrounding whitespace.
//! A blank constraint is ignored. Area, mode, and schedule match the whole
//! field, so `Austin` does not match `East Austin`.

use crate::error::Error;
use crate::job::Job;

/// Constraints for [`filter_jobs`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobFilter {
    mode: Option<String>,
    area: Option<String>,
    schedule: Option<String>,
    skill: Option<String>,
    min_match: Option<u8>,
}

impl JobFilter {
    /// Build a filter.
    ///
    /// `min_match` of `None` does not constrain fit. Any other value must be
    /// an integer from 0 to 100.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MatchOutOfRange`] when `min_match` is outside 0..=100.
    pub fn try_new(
        mode: Option<String>,
        area: Option<String>,
        schedule: Option<String>,
        skill: Option<String>,
        min_match: Option<i64>,
    ) -> Result<Self, Error> {
        let min_match = match min_match {
            Some(value) => Some(checked_match(value)?),
            None => None,
        };
        Ok(Self {
            mode,
            area,
            schedule,
            skill,
            min_match,
        })
    }

    /// Whether `job` satisfies every set constraint.
    #[must_use]
    pub fn matches(&self, job: &Job) -> bool {
        field_matches(&self.mode, &job.mode)
            && field_matches(&self.area, &job.area)
            && field_matches(&self.schedule, &job.schedule)
            && skill_matches(&self.skill, &job.skills)
            && self
                .min_match
                .is_none_or(|min_match| job.match_pct >= min_match)
    }
}

/// Jobs that match `query`, in their original order.
#[must_use]
pub fn filter_jobs<'a>(jobs: &'a [Job], query: &JobFilter) -> Vec<&'a Job> {
    jobs.iter().filter(|job| query.matches(job)).collect()
}

fn checked_match(value: i64) -> Result<u8, Error> {
    u8::try_from(value)
        .ok()
        .filter(|value| *value <= 100)
        .ok_or(Error::MatchOutOfRange(value))
}

fn field_matches(want: &Option<String>, actual: &str) -> bool {
    match want {
        Some(want) if !want.trim().is_empty() => actual.trim().eq_ignore_ascii_case(want.trim()),
        _ => true,
    }
}

fn skill_matches(want: &Option<String>, skills: &[String]) -> bool {
    match want {
        Some(want) if !want.trim().is_empty() => skills
            .iter()
            .any(|skill| skill.trim().eq_ignore_ascii_case(want.trim())),
        _ => true,
    }
}
