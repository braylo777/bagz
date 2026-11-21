use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Where a posting came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Greenhouse,
    Lever,
    Ashby,
    Workable,
    Rss,
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::Greenhouse,
        Source::Lever,
        Source::Ashby,
        Source::Workable,
        Source::Rss,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Greenhouse => "greenhouse",
            Self::Lever => "lever",
            Self::Ashby => "ashby",
            Self::Workable => "workable",
            Self::Rss => "rss",
        }
    }

    /// Lower number wins when two sources list the same job: a direct ATS
    /// feed has the apply form, an RSS aggregator only links to it.
    pub fn priority(self) -> u8 {
        match self {
            Self::Greenhouse | Self::Lever | Self::Ashby | Self::Workable => 0,
            Self::Rss => 1,
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Source {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "greenhouse" | "gh" => Ok(Self::Greenhouse),
            "lever" => Ok(Self::Lever),
            "ashby" => Ok(Self::Ashby),
            "workable" => Ok(Self::Workable),
            "rss" => Ok(Self::Rss),
            other => Err(format!("unknown source `{other}`")),
        }
    }
}

/// Where the work happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Workplace {
    Remote,
    Hybrid,
    Onsite,
    #[default]
    Unknown,
}

impl Workplace {
    /// Reads the free-text labels the feeds use.
    pub fn from_label(label: &str) -> Self {
        let l = label.to_ascii_lowercase();
        if l.contains("hybrid") {
            Self::Hybrid
        } else if l.contains("remote") || l.contains("anywhere") || l.contains("telecommut") {
            Self::Remote
        } else if l.contains("onsite") || l.contains("on-site") || l.contains("in office") {
            Self::Onsite
        } else {
            Self::Unknown
        }
    }
}

/// Yearly pay range in whole currency units, when the feed states one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Salary {
    pub min: u64,
    pub max: u64,
    pub currency: String,
}

/// One normalized job posting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    /// `source:board:native_id`, stable across runs.
    pub id: String,
    pub source: Source,
    /// Board token, company slug, or feed host.
    pub board: String,
    pub company: String,
    pub title: String,
    pub location: String,
    pub workplace: Workplace,
    pub department: Option<String>,
    pub employment_type: Option<String>,
    /// Posting page.
    pub url: String,
    /// Apply form, when the feed gives one apart from `url`.
    pub apply_url: Option<String>,
    /// ISO-8601 date (`YYYY-MM-DD`), when the feed gives one.
    pub posted: Option<String>,
    pub salary: Option<Salary>,
    /// Plain-text description, HTML removed, capped at
    /// [`crate::text::DESCRIPTION_CAP`] characters.
    pub description: String,
}

impl Posting {
    pub fn make_id(source: Source, board: &str, native: &str) -> String {
        format!("{}:{}:{}", source.as_str(), board, native)
    }
}
