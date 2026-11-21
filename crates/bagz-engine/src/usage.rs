//! Opt-in, local-first, anonymous usage counter.
//!
//! - Off by default. While off, [`UsageState::record`] stores nothing.
//! - When on, it keeps a random install id and the set of `YYYY-MM`
//!   months and `YYYY-MM-DD` days the app ran. No names, emails, jobs,
//!   IP addresses or device data.
//! - Nothing is sent. [`UsageState::export`] makes a small JSON report the
//!   user can read and share by hand; [`monthly_active`] counts MAU across
//!   reports someone chose to share.
//! - Turning it off deletes the id and all counts.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Days kept in the local day list; older days roll up into months only.
pub const DAY_WINDOW: usize = 62;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageState {
    pub enabled: bool,
    pub install_id: Option<String>,
    pub months: BTreeSet<String>,
    pub days: BTreeSet<String>,
    pub runs: u64,
}

/// What a user may choose to share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageReport {
    pub install_id: String,
    pub months: Vec<String>,
    pub runs: u64,
    pub version: String,
}

/// A random-looking id from the clock and process id, hashed. It does not
/// identify a person or a device.
pub fn new_install_id(seed: u128) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(seed.to_le_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn clock_seed() -> u128 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    nanos ^ (u128::from(std::process::id()) << 64)
}

impl UsageState {
    pub fn enable(&mut self, seed: u128) {
        if !self.enabled {
            self.enabled = true;
            self.install_id = Some(new_install_id(seed));
        }
    }

    /// Turns counting off and deletes everything it stored.
    pub fn disable(&mut self) {
        *self = Self::default();
    }

    /// Notes one run on `today` (`YYYY-MM-DD`). Does nothing while off.
    pub fn record(&mut self, today: &str) {
        if !self.enabled || today.len() < 7 {
            return;
        }
        self.runs += 1;
        self.months.insert(today[..7].to_string());
        self.days.insert(today.to_string());
        while self.days.len() > DAY_WINDOW {
            let first = self.days.iter().next().cloned();
            if let Some(f) = first {
                self.days.remove(&f);
            }
        }
    }

    /// The shareable report, or `None` while off.
    pub fn export(&self) -> Option<UsageReport> {
        Some(UsageReport {
            install_id: self.install_id.clone()?,
            months: self.months.iter().cloned().collect(),
            runs: self.runs,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

/// Distinct installs active in `month` (`YYYY-MM`) across shared reports.
pub fn monthly_active(reports: &[UsageReport], month: &str) -> usize {
    reports
        .iter()
        .filter(|r| r.months.iter().any(|m| m == month))
        .map(|r| r.install_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

/// The state file on disk: `$BAGZ_HOME/usage.json`, else `~/.bagz/usage.json`.
pub struct UsageStore {
    path: PathBuf,
}

impl UsageStore {
    pub fn at(dir: impl AsRef<Path>) -> Self {
        Self {
            path: dir.as_ref().join("usage.json"),
        }
    }

    pub fn default_location() -> Self {
        let dir = std::env::var_os("BAGZ_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".bagz")))
            .unwrap_or_else(|| PathBuf::from(".bagz"));
        Self::at(dir)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<UsageState> {
        match fs::read_to_string(&self.path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UsageState::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, s: &UsageState) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&self.path, serde_json::to_string_pretty(s)?)?;
        Ok(())
    }

    pub fn enable(&self) -> Result<UsageState> {
        let mut s = self.load()?;
        s.enable(clock_seed());
        self.save(&s)?;
        Ok(s)
    }

    /// Turns counting off and removes the file.
    pub fn disable(&self) -> Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Records a run. While off, it does not create or touch the file.
    pub fn record(&self, today: &str) -> Result<()> {
        let mut s = self.load()?;
        if !s.enabled {
            return Ok(());
        }
        s.record(today);
        self.save(&s)
    }
}
