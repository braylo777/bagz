//! HTTP behind a trait, so tests replay recorded responses and never touch
//! the network.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};

/// A GET-only HTTP client.
pub trait HttpClient {
    fn get(&self, url: &str) -> Result<String>;
}

/// Largest response body read, in bytes.
pub const MAX_BODY: u64 = 256 * 1024 * 1024;

/// Live client for real runs. Sends a plain `bagz-ingest` User-Agent and no
/// personal data.
pub struct LiveClient {
    agent: ureq::Agent,
}

impl LiveClient {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("bagz-ingest/", env!("CARGO_PKG_VERSION")))
            .build();
        Self { agent }
    }
}

impl Default for LiveClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for LiveClient {
    fn get(&self, url: &str) -> Result<String> {
        let resp = self.agent.get(url).call().map_err(|e| Error::Http {
            url: url.to_string(),
            message: e.to_string(),
        })?;
        // `into_string` caps bodies at 10 MB; large boards send more.
        let mut body = String::new();
        resp.into_reader()
            .take(MAX_BODY)
            .read_to_string(&mut body)
            .map_err(|e| Error::Http {
                url: url.to_string(),
                message: e.to_string(),
            })?;
        Ok(body)
    }
}

/// Replays recorded responses. An index file maps each URL to a body file.
/// A URL that was not recorded is an error, so a test can never fall back to
/// the network.
#[derive(Debug, Default, Clone)]
pub struct FixtureClient {
    bodies: BTreeMap<String, String>,
}

impl FixtureClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `body` as the response for `url`.
    pub fn with(mut self, url: impl Into<String>, body: impl Into<String>) -> Self {
        self.bodies.insert(url.into(), body.into());
        self
    }

    /// Loads `dir/index.json`, a JSON object of `url -> file name`.
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let index: BTreeMap<String, String> =
            serde_json::from_str(&fs::read_to_string(dir.join("index.json"))?)?;
        let mut bodies = BTreeMap::new();
        for (url, file) in index {
            let path: PathBuf = dir.join(file);
            bodies.insert(url, fs::read_to_string(path)?);
        }
        Ok(Self { bodies })
    }

    pub fn urls(&self) -> impl Iterator<Item = &str> {
        self.bodies.keys().map(String::as_str)
    }
}

impl HttpClient for FixtureClient {
    fn get(&self, url: &str) -> Result<String> {
        self.bodies
            .get(url)
            .cloned()
            .ok_or_else(|| Error::MissingFixture(url.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_client_replays_and_refuses_unknown_urls() {
        let c = FixtureClient::new().with("https://a.test/x", "body");
        assert_eq!(c.get("https://a.test/x").unwrap(), "body");
        assert!(matches!(
            c.get("https://a.test/y"),
            Err(Error::MissingFixture(u)) if u == "https://a.test/y"
        ));
        assert_eq!(c.urls().count(), 1);
    }
}
