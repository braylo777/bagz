//! Swipe deck: bag, pass, undo, and the applied queue.

use std::collections::HashSet;

use chrono::NaiveDate;
use serde_json::Value;

use crate::catalog::Catalog;
use crate::error::Error;
use crate::job::Job;
use crate::persist::{Decision, DecisionRecord, PersistedState, Plan, sanitize_state};

/// Experience added by a normal bag.
pub const BAG_XP: u32 = 10;
/// Experience added by a super-bag.
pub const SUPER_XP: u32 = 25;
/// Experience that reaches the Closer rank.
pub const CLOSER_XP: u32 = 40;
/// Experience that reaches the Bag Secured rank.
pub const BAG_SECURED_XP: u32 = 120;
/// Rows in the chat timeline.
pub const AGENT_QUEUE_LIMIT: usize = 4;
/// Companies shown in the deck's watcher row.
pub const WATCHER_LIMIT: usize = 3;

/// What the screen asked the deck to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swipe {
    /// Bag the role for 10 xp.
    Bag,
    /// Pass. Clears the combo and does not score.
    Pass,
    /// Bag the role for 25 xp. The screen, not this crate, checks VIP.
    Super,
}

/// Rank label on the deck, from experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    /// Below 40 xp.
    Rookie,
    /// From 40 xp up to, but not including, 120.
    Closer,
    /// 120 xp or more.
    BagSecured,
}

impl Rank {
    /// Label drawn in the HUD.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rookie => "Rookie",
            Self::Closer => "Closer",
            Self::BagSecured => "Bag Secured",
        }
    }
}

/// Rank for an experience total.
#[must_use]
pub fn rank_for(xp: u32) -> Rank {
    if xp >= BAG_SECURED_XP {
        Rank::BagSecured
    } else if xp >= CLOSER_XP {
        Rank::Closer
    } else {
        Rank::Rookie
    }
}

/// Result of one accepted swipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwipeOutcome {
    /// Decision written to history. Super is [`Decision::Bag`].
    pub decision: Decision,
    /// Experience added by this swipe.
    pub xp_gain: u32,
    /// Experience after the swipe.
    pub xp: u32,
    /// Streak after the swipe.
    pub streak: u32,
    /// Combo after the swipe.
    pub combo: u32,
    /// Rank after the swipe.
    pub rank: Rank,
}

/// A catalog plus the swipe state from the demo store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    catalog: Catalog,
    history: Vec<DecisionRecord>,
    plan: Plan,
    xp: u32,
    streak: u32,
    last_day: String,
    combo: u32,
}

impl Session {
    /// Empty history on top of `catalog`.
    #[must_use]
    pub fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            history: Vec::new(),
            plan: Plan::Free,
            xp: 0,
            streak: 0,
            last_day: String::new(),
            combo: 0,
        }
    }

    /// Build a session from stored JSON.
    ///
    /// Combo starts at zero because the demo does not persist it. History is
    /// filtered to ids in `catalog`.
    #[must_use]
    pub fn restore(catalog: Catalog, raw: &Value) -> Self {
        let state = {
            let known = catalog
                .jobs()
                .iter()
                .map(|job| job.id.as_str())
                .collect::<HashSet<_>>();
            sanitize_state(raw, &known)
        };
        Self::from_state(catalog, state)
    }

    fn from_state(catalog: Catalog, state: PersistedState) -> Self {
        Self {
            catalog,
            history: state.history,
            plan: state.plan,
            xp: state.xp,
            streak: state.streak,
            last_day: state.last_day,
            combo: 0,
        }
    }

    /// Catalog this session swipes against.
    #[must_use]
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Oldest decision first.
    #[must_use]
    pub fn history(&self) -> &[DecisionRecord] {
        &self.history
    }

    /// Current plan.
    #[must_use]
    pub fn plan(&self) -> Plan {
        self.plan
    }

    /// Replace the plan. This does not change swipe history.
    pub fn set_plan(&mut self, plan: Plan) {
        self.plan = plan;
    }

    /// Experience points.
    #[must_use]
    pub fn xp(&self) -> u32 {
        self.xp
    }

    /// Current streak.
    #[must_use]
    pub fn streak(&self) -> u32 {
        self.streak
    }

    /// In-memory combo. A bag or super-bag increments it. A pass sets it to zero.
    #[must_use]
    pub fn combo(&self) -> u32 {
        self.combo
    }

    /// Last day that scored, or empty.
    #[must_use]
    pub fn last_day(&self) -> &str {
        &self.last_day
    }

    /// Rank from the current experience.
    #[must_use]
    pub fn rank(&self) -> Rank {
        rank_for(self.xp)
    }

    /// Record a swipe on `day` (`YYYY-MM-DD`).
    ///
    /// Re-swiping an id replaces the older decision and appends the new one.
    /// A bag or super-bag adds experience and advances the streak when `day`
    /// differs from the last scored day. A pass does neither. Super is not
    /// plan-gated here; the deck screen checks VIP before it calls super.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadDay`] when `day` is not a real `YYYY-MM-DD` date,
    /// or [`Error::UnknownJob`] when `job_id` is not in the catalog. Neither
    /// error changes the session.
    pub fn swipe(&mut self, job_id: &str, action: Swipe, day: &str) -> Result<SwipeOutcome, Error> {
        let parsed = NaiveDate::parse_from_str(day, "%Y-%m-%d")
            .map_err(|_| Error::BadDay(day.to_string()))?;
        if self.catalog.get(job_id).is_none() {
            return Err(Error::UnknownJob(job_id.to_string()));
        }
        let canonical = parsed.format("%Y-%m-%d").to_string();
        self.history.retain(|record| record.id != job_id);
        let (decision, gain) = match action {
            Swipe::Bag => (Decision::Bag, BAG_XP),
            Swipe::Super => (Decision::Bag, SUPER_XP),
            Swipe::Pass => (Decision::Pass, 0),
        };
        self.history.push(DecisionRecord {
            id: job_id.to_string(),
            decision,
        });
        if gain > 0 {
            self.xp = self.xp.saturating_add(gain);
            if self.last_day != canonical {
                self.streak = self.streak.saturating_add(1);
                self.last_day = canonical;
            }
            self.combo = self.combo.saturating_add(1);
        } else {
            self.combo = 0;
        }
        Ok(SwipeOutcome {
            decision,
            xp_gain: gain,
            xp: self.xp,
            streak: self.streak,
            combo: self.combo,
            rank: rank_for(self.xp),
        })
    }

    /// Drop the newest decision.
    ///
    /// Experience, streak, and combo stay, matching the demo's undo.
    pub fn undo(&mut self) -> Option<DecisionRecord> {
        self.history.pop()
    }

    /// Clear swipe history.
    ///
    /// Experience and the in-memory combo stay, matching `resetSwipes`, which
    /// only empties history.
    pub fn reset_swipes(&mut self) {
        self.history.clear();
    }

    /// Bagged jobs, newest first.
    #[must_use]
    pub fn applied(&self) -> Vec<&Job> {
        self.history
            .iter()
            .rev()
            .filter(|record| record.decision == Decision::Bag)
            .filter_map(|record| self.catalog.get(&record.id))
            .collect()
    }

    /// Passed jobs, in the order they were decided.
    #[must_use]
    pub fn passed(&self) -> Vec<&Job> {
        self.history
            .iter()
            .filter(|record| record.decision == Decision::Pass)
            .filter_map(|record| self.catalog.get(&record.id))
            .collect()
    }

    /// Jobs that have not been bagged or passed, in catalog order.
    #[must_use]
    pub fn remaining(&self) -> Vec<&Job> {
        let decided = self
            .history
            .iter()
            .map(|record| record.id.as_str())
            .collect::<HashSet<_>>();
        self.catalog
            .jobs()
            .iter()
            .filter(|job| !decided.contains(job.id.as_str()))
            .collect()
    }

    /// The card on top of the deck.
    #[must_use]
    pub fn top(&self) -> Option<&Job> {
        self.remaining().into_iter().next()
    }

    /// Sum of `chase` across bagged jobs.
    #[must_use]
    pub fn chased(&self) -> u64 {
        self.applied().iter().map(|job| u64::from(job.chase)).sum()
    }

    /// The last three jobs still in the deck, in catalog order.
    #[must_use]
    pub fn watchers(&self) -> Vec<&Job> {
        let remaining = self.remaining();
        let start = remaining.len().saturating_sub(WATCHER_LIMIT);
        remaining[start..].to_vec()
    }

    /// Jobs the agent timeline files.
    ///
    /// Bagged jobs come first, newest first, then the catalog fills the rest.
    /// Ids are unique. `limit` caps the list; the chat screen uses
    /// [`AGENT_QUEUE_LIMIT`].
    #[must_use]
    pub fn agent_queue(&self, limit: usize) -> Vec<&Job> {
        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        for job in self.applied().into_iter().chain(self.catalog.jobs().iter()) {
            if rows.len() == limit {
                break;
            }
            if !seen.insert(job.id.as_str()) {
                continue;
            }
            rows.push(job);
        }
        rows
    }
}
