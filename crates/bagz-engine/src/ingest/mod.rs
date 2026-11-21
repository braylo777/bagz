//! Ingestion from public job-board APIs and RSS.

pub mod ashby;
pub mod greenhouse;
pub mod lever;
pub mod rss;
pub mod workable;

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::dedupe::dedupe;
use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::model::{Posting, Source};

/// One board to read: a source plus its token (Greenhouse board token, Lever
/// or Ashby company slug, Workable subdomain, or a full RSS URL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub source: Source,
    pub token: String,
}

impl Board {
    pub fn new(source: Source, token: impl Into<String>) -> Self {
        Self {
            source,
            token: token.into(),
        }
    }

    /// The public endpoint for this board.
    pub fn url(&self) -> String {
        match self.source {
            Source::Greenhouse => greenhouse::url(&self.token),
            Source::Lever => lever::url(&self.token),
            Source::Ashby => ashby::url(&self.token),
            Source::Workable => workable::url(&self.token),
            Source::Rss => self.token.clone(),
        }
    }

    /// Parses a response body for this board.
    pub fn parse(&self, body: &str) -> Result<Vec<Posting>> {
        match self.source {
            Source::Greenhouse => greenhouse::parse(body, &self.token),
            Source::Lever => lever::parse(body, &self.token),
            Source::Ashby => ashby::parse(body, &self.token),
            Source::Workable => workable::parse(body, &self.token),
            Source::Rss => rss::parse(body, &self.token),
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.token)
    }
}

impl FromStr for Board {
    type Err = Error;
    /// `greenhouse:stripe`, `lever:spotify`, `rss:https://host/feed.rss`.
    fn from_str(s: &str) -> Result<Self> {
        let (src, token) = s
            .split_once(':')
            .ok_or_else(|| Error::BoardSpec(s.to_string()))?;
        let source = Source::from_str(src.trim()).map_err(|_| Error::BoardSpec(s.to_string()))?;
        let token = token.trim();
        if token.is_empty() {
            return Err(Error::BoardSpec(s.to_string()));
        }
        Ok(Self::new(source, token))
    }
}

/// Reads a board list: one `source:token` per line, `#` comments allowed.
pub fn parse_board_list(text: &str) -> Result<Vec<Board>> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(Board::from_str)
        .collect()
}

/// What happened to one board in a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardResult {
    pub board: String,
    pub fetched: usize,
    pub error: Option<String>,
}

/// The result of an ingestion run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub boards: Vec<BoardResult>,
    /// Postings parsed before dedupe.
    pub fetched: usize,
    /// Postings left after dedupe.
    pub indexed: usize,
    pub duplicates_removed: usize,
    pub by_source: BTreeMap<String, usize>,
}

/// Fetches every board, parses, then dedupes. A failing board is recorded
/// in the report and does not stop the run.
pub fn run(client: &dyn HttpClient, boards: &[Board]) -> (Vec<Posting>, RunReport) {
    let mut all = Vec::new();
    let mut results = Vec::with_capacity(boards.len());
    for board in boards {
        let outcome = client.get(&board.url()).and_then(|body| board.parse(&body));
        match outcome {
            Ok(mut postings) => {
                results.push(BoardResult {
                    board: board.to_string(),
                    fetched: postings.len(),
                    error: None,
                });
                all.append(&mut postings);
            }
            Err(e) => results.push(BoardResult {
                board: board.to_string(),
                fetched: 0,
                error: Some(e.to_string()),
            }),
        }
    }
    let fetched = all.len();
    let unique = dedupe(all);
    let mut by_source = BTreeMap::new();
    for p in &unique {
        *by_source.entry(p.source.as_str().to_string()).or_insert(0) += 1;
    }
    let report = RunReport {
        boards: results,
        fetched,
        indexed: unique.len(),
        duplicates_removed: fetched - unique.len(),
        by_source,
    };
    (unique, report)
}

/// `open-ai` -> `Open Ai`. Lever and Ashby feeds do not name the company,
/// so the board slug is the best label they give.
pub fn company_from_slug(slug: &str) -> String {
    slug.split(['-', '_', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn money(token: &str) -> Option<u64> {
    let t = token.trim().trim_start_matches(['$', '£', '€']);
    let (num, mult) = match t.chars().last()? {
        'k' | 'K' => (&t[..t.len() - 1], 1_000.0),
        _ => (t, 1.0),
    };
    let cleaned: String = num.chars().filter(|c| *c != ',').collect();
    let v: f64 = cleaned.parse().ok()?;
    Some((v * mult) as u64)
}

/// Finds a yearly pay range written in prose, such as `$130,000 - $290,000`,
/// `$130K – $290K` or `£50k to £70k`. Ranges under 10,000 (hourly pay) or
/// with `min > max` are ignored.
pub fn salary_from_text(text: &str) -> Option<crate::model::Salary> {
    let bytes: Vec<char> = text.chars().collect();
    let symbols = [('$', "USD"), ('£', "GBP"), ('€', "EUR")];
    let mut i = 0;
    while i < bytes.len() {
        if let Some((sym, cur)) = symbols.iter().find(|(s, _)| *s == bytes[i]) {
            let read_num = |start: usize| -> (String, usize) {
                let mut j = start;
                let mut s = String::new();
                while j < bytes.len()
                    && (bytes[j].is_ascii_digit() || bytes[j] == ',' || bytes[j] == '.')
                {
                    s.push(bytes[j]);
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] == 'k' || bytes[j] == 'K') {
                    s.push('k');
                    j += 1;
                }
                (s, j)
            };
            let (a, mut j) = read_num(i + 1);
            if !a.is_empty() {
                // separator: spaces, dash, en/em dash, or "to"
                let rest: String = bytes[j..bytes.len().min(j + 8)].iter().collect();
                let trimmed = rest.trim_start();
                let skipped = rest.chars().count() - trimmed.chars().count();
                let sep_len = if trimmed.starts_with('-')
                    || trimmed.starts_with('–')
                    || trimmed.starts_with('—')
                {
                    1
                } else if trimmed.starts_with("to ") {
                    2
                } else {
                    0
                };
                if sep_len > 0 {
                    j += skipped + sep_len;
                    while j < bytes.len() && bytes[j] == ' ' {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == *sym {
                        j += 1;
                    }
                    let (b, _) = read_num(j);
                    if let (Some(min), Some(max)) = (money(&a), money(&b)) {
                        if min >= 10_000 && min <= max {
                            return Some(crate::model::Salary {
                                min,
                                max,
                                currency: cur.to_string(),
                            });
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

pub(crate) fn str_field<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_spec_round_trips() {
        let b: Board = "greenhouse:acme".parse().unwrap();
        assert_eq!(b, Board::new(Source::Greenhouse, "acme"));
        assert_eq!(b.to_string(), "greenhouse:acme");
        let r: Board = "rss:https://feeds.test/jobs.rss".parse().unwrap();
        assert_eq!(r.token, "https://feeds.test/jobs.rss");
        assert_eq!(r.url(), "https://feeds.test/jobs.rss");
    }

    #[test]
    fn board_spec_rejects_bad_input() {
        assert!("acme".parse::<Board>().is_err());
        assert!("myspace:acme".parse::<Board>().is_err());
        assert!("lever:".parse::<Board>().is_err());
    }

    #[test]
    fn board_list_skips_comments_and_blanks() {
        let list = parse_board_list("# boards\ngreenhouse:a\n\nlever:b # trailing\n").unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[1], Board::new(Source::Lever, "b"));
    }

    #[test]
    fn board_urls_per_source() {
        assert_eq!(
            Board::new(Source::Greenhouse, "x").url(),
            "https://boards-api.greenhouse.io/v1/boards/x/jobs?content=true"
        );
        assert_eq!(
            Board::new(Source::Lever, "x").url(),
            "https://api.lever.co/v0/postings/x?mode=json"
        );
        assert_eq!(
            Board::new(Source::Ashby, "x").url(),
            "https://api.ashbyhq.com/posting-api/job-board/x?includeCompensation=true"
        );
        assert_eq!(
            Board::new(Source::Workable, "x").url(),
            "https://apply.workable.com/api/v1/widget/accounts/x?details=true"
        );
    }
}
