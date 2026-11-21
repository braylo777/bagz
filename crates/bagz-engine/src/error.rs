use thiserror::Error;

/// Engine result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure the engine reports.
#[derive(Debug, Error)]
pub enum Error {
    #[error("http error for {url}: {message}")]
    Http { url: String, message: String },
    #[error("no recorded fixture for {0}")]
    MissingFixture(String),
    #[error("parse error in {source_name} feed: {message}")]
    Parse {
        source_name: &'static str,
        message: String,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid board spec `{0}` (expected `source:token`)")]
    BoardSpec(String),
    #[error("proposal {0} is not pending")]
    NotPending(u64),
    #[error("unknown proposal {0}")]
    UnknownProposal(u64),
    #[error("llm error: {0}")]
    Llm(String),
    #[error("agent output rejected: {0}")]
    Guard(String),
}

impl Error {
    pub(crate) fn parse(source_name: &'static str, message: impl Into<String>) -> Self {
        Self::Parse {
            source_name,
            message: message.into(),
        }
    }
}
