use thiserror::Error;

/// Failures from loading the catalog or changing a deck.
#[derive(Debug, Error)]
pub enum Error {
    /// `jobs.json` is not valid JSON, or a field has the wrong type.
    #[error("jobs json is not valid: {0}")]
    Json(#[from] serde_json::Error),

    /// The jobs file could not be read.
    #[error("could not read jobs file: {0}")]
    Io(#[from] std::io::Error),

    /// A job, or the catalog as a whole, failed validation.
    #[error("{0}")]
    Invalid(String),

    /// A swipe named a job that is not in the catalog.
    #[error("unknown job id `{0}`")]
    UnknownJob(String),

    /// A swipe name was not `bag`, `pass`, or `super`.
    #[error("decision must be bag, pass, or super")]
    BadDecision,

    /// A plan name was not `free`, `pro`, or `vip`.
    #[error("plan must be free, pro, or vip")]
    BadPlan,

    /// `min_match` was outside 0..=100.
    #[error("min_match must be between 0 and 100")]
    BadMinMatch,
}
