//! Failures from loading jobs or recording a swipe.

/// Failure while loading jobs or recording a swipe.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The jobs file could not be read.
    #[error("could not read jobs file {path}: {source}")]
    Read {
        /// Path that was opened.
        path: String,
        /// Filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// The document was not a JSON array of job objects.
    #[error("jobs JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// A job object broke a catalog rule.
    #[error("job {id} is invalid: {reason}")]
    InvalidJob {
        /// Job id, or `<blank>` when the id itself is empty.
        id: String,
        /// What failed validation.
        reason: String,
    },
    /// Two jobs in one catalog share an id.
    #[error("duplicate job id `{0}`")]
    DuplicateId(String),
    /// A swipe named a job that is not in the catalog.
    #[error("unknown job id `{0}`")]
    UnknownJob(String),
    /// A caller passed a day that is not a calendar date.
    #[error("day must be YYYY-MM-DD, got `{0}`")]
    BadDay(String),
    /// `min_match` was outside 0..=100.
    #[error("min_match must be an integer from 0 to 100, got {0}")]
    MatchOutOfRange(i64),
}
