use std::collections::HashSet;

use serde::Serialize;
use serde_json::Value;

use crate::error::Error;
use crate::job::Catalog;
use crate::job::Job;

/// How many chat rows the demo fills from the apply queue.
pub const CHAT_QUEUE_LIMIT: usize = 4;

const XP_BAG: i64 = 10;
const XP_SUPER: i64 = 25;

/// A stored swipe. Super is not stored separately: it is a bag with more XP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Swipe right, or super. The job is in the applied queue.
    Bag,
    /// Swipe left.
    Pass,
}

/// One entry in the demo's persisted history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecisionRecord {
    /// Catalog id.
    pub id: String,
    /// `bag` or `pass`.
    pub decision: Decision,
}

/// Live swipe. `Super` is stored as [`Decision::Bag`] and awards 25 XP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swipe {
    /// Apply. Awards 10 XP and adds one to the combo.
    Bag,
    /// Pass. Awards no XP and clears the combo.
    Pass,
    /// Apply with the VIP super gesture. Awards 25 XP.
    Super,
}

impl Swipe {
    /// Parse the Python/API names `bag`, `pass`, and `super`.
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "bag" => Ok(Self::Bag),
            "pass" => Ok(Self::Pass),
            "super" => Ok(Self::Super),
            _ => Err(Error::BadDecision),
        }
    }
}

/// Demo plan. Checkout can set `pro` or `vip`. A saved `agent` plan loads as `vip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Default plan.
    Free,
    /// Pro checkout.
    Pro,
    /// VIP checkout, including a legacy saved `agent` plan.
    Vip,
}

impl Plan {
    /// Wire value stored in the demo.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Vip => "vip",
        }
    }

    /// Strict parse used when a caller sets the plan.
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "free" => Ok(Self::Free),
            "pro" => Ok(Self::Pro),
            "vip" => Ok(Self::Vip),
            _ => Err(Error::BadPlan),
        }
    }

    fn parse_saved(value: &str) -> Self {
        match value {
            "pro" => Self::Pro,
            "vip" | "agent" => Self::Vip,
            _ => Self::Free,
        }
    }
}

/// Chat row phase from the demo's `phaseFor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyPhase {
    /// Before 450 ms.
    Queued,
    /// From 450 ms until the row is filed.
    Applying,
    /// Filed, and not one of the first two reply rows yet.
    Applied,
    /// First two rows, once 1900 ms have elapsed.
    Reply,
}

impl ApplyPhase {
    /// Label the demo prints on the row.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Applying => "Applying",
            Self::Applied => "Applied",
            Self::Reply => "Reply",
        }
    }
}

/// Phase of one chat row.
///
/// `elapsed_ms` is the row's own clock (`now - index * 700` in the demo).
/// The first two rows become `Reply` at 1900 ms. Later rows stay `Applied`.
pub fn apply_phase(elapsed_ms: i64, index: usize) -> ApplyPhase {
    if elapsed_ms < 450 {
        ApplyPhase::Queued
    } else if elapsed_ms < 1100 {
        ApplyPhase::Applying
    } else if index < 2 && elapsed_ms >= 1900 {
        ApplyPhase::Reply
    } else if elapsed_ms >= 1100 {
        ApplyPhase::Applied
    } else {
        ApplyPhase::Applying
    }
}

/// Swipe session: history, XP, streak, combo, and plan.
///
/// The rules match `src/store.tsx`. Passing does not change XP or streak.
/// Bag awards 10 XP, super awards 25. The streak increments once per distinct
/// `today` value, and only on a bag or super. Undo drops the last history
/// entry and does not refund XP. Reset clears history and leaves XP alone.
#[derive(Debug, Clone)]
pub struct Deck {
    catalog: Catalog,
    plan: Plan,
    history: Vec<DecisionRecord>,
    xp: i64,
    streak: i64,
    last_day: String,
    combo: i64,
}

impl Deck {
    /// A fresh free deck over `catalog`.
    pub fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            plan: Plan::Free,
            history: Vec::new(),
            xp: 0,
            streak: 0,
            last_day: String::new(),
            combo: 0,
        }
    }

    /// Catalog this deck swipes against.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Record a swipe. Unknown ids are rejected and do not change the deck.
    ///
    /// Swiping a job that is already decided replaces that decision and moves
    /// it to the end of history. That is the live path. [`Deck::import_state`]
    /// is the page-load path and keeps the first valid saved record instead.
    pub fn swipe(&mut self, id: &str, swipe: Swipe, today: &str) -> Result<(), Error> {
        if self.catalog.get(id).is_none() {
            return Err(Error::UnknownJob(id.to_string()));
        }
        self.history.retain(|record| record.id != id);
        let decision = match swipe {
            Swipe::Pass => Decision::Pass,
            Swipe::Bag | Swipe::Super => Decision::Bag,
        };
        self.history.push(DecisionRecord {
            id: id.to_string(),
            decision,
        });
        match swipe {
            Swipe::Pass => self.combo = 0,
            Swipe::Bag => self.gain(XP_BAG, today),
            Swipe::Super => self.gain(XP_SUPER, today),
        }
        Ok(())
    }

    /// Drop the last swipe. XP, streak, and combo stay where they are.
    pub fn undo(&mut self) {
        self.history.pop();
    }

    /// Clear swipe history. XP, streak, plan, and combo stay.
    pub fn reset(&mut self) {
        self.history.clear();
    }

    /// Saved history, oldest first.
    pub fn history(&self) -> &[DecisionRecord] {
        &self.history
    }

    /// Applied jobs, newest first. This is the demo's `bags` list.
    pub fn applied(&self) -> Vec<&Job> {
        self.history
            .iter()
            .rev()
            .filter(|record| record.decision == Decision::Bag)
            .filter_map(|record| self.catalog.get(&record.id))
            .collect()
    }

    /// Passed jobs, oldest first.
    pub fn passed(&self) -> Vec<&Job> {
        self.history
            .iter()
            .filter(|record| record.decision == Decision::Pass)
            .filter_map(|record| self.catalog.get(&record.id))
            .collect()
    }

    /// Jobs that have not been bagged or passed, in file order.
    pub fn remaining(&self) -> Vec<&Job> {
        let decided: HashSet<&str> = self
            .history
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        self.catalog
            .jobs()
            .iter()
            .filter(|job| !decided.contains(job.id.as_str()))
            .collect()
    }

    /// Chat queue: applied jobs newest-first, then the rest of the catalog.
    ///
    /// Duplicates are skipped. The demo shows four rows (`CHAT_QUEUE_LIMIT`).
    pub fn queue(&self, limit: usize) -> Vec<&Job> {
        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        let applied: Vec<&str> = self.applied().iter().map(|job| job.id.as_str()).collect();
        let mut ordered = applied;
        ordered.extend(self.catalog.jobs().iter().map(|job| job.id.as_str()));
        for id in ordered {
            if !seen.insert(id) {
                continue;
            }
            let Some(job) = self.catalog.get(id) else {
                continue;
            };
            rows.push(job);
            if rows.len() == limit {
                break;
            }
        }
        rows
    }

    /// Last three still-undecided jobs. The deck shows these as watchers.
    pub fn watchers(&self) -> Vec<&Job> {
        let remaining = self.remaining();
        let start = remaining.len().saturating_sub(3);
        remaining[start..].to_vec()
    }

    /// Sum of `chase` across applied jobs.
    pub fn chased(&self) -> u64 {
        self.applied().iter().map(|job| u64::from(job.chase)).sum()
    }

    /// Current plan.
    pub fn plan(&self) -> Plan {
        self.plan
    }

    /// Set the plan. This is the checkout confirmation, not the saved-state loader.
    pub fn set_plan(&mut self, plan: Plan) {
        self.plan = plan;
    }

    /// Total XP. Bag is 10, super is 25, pass is 0.
    pub fn xp(&self) -> i64 {
        self.xp
    }

    /// Streak of distinct days on which a bag or super landed.
    pub fn streak(&self) -> i64 {
        self.streak
    }

    /// Day of the last bag or super, or empty before the first one.
    pub fn last_day(&self) -> &str {
        &self.last_day
    }

    /// Current apply combo. Pass resets it. Undo does not.
    pub fn combo(&self) -> i64 {
        self.combo
    }

    /// Rank label from the deck header.
    pub fn rank(&self) -> &'static str {
        if self.xp >= 120 {
            "Bag Secured"
        } else if self.xp >= 40 {
            "Closer"
        } else {
            "Rookie"
        }
    }

    /// JSON object in the demo's `localStorage` shape.
    pub fn export_state(&self) -> Result<String, Error> {
        let state = PersistedOut {
            plan: self.plan.as_str(),
            history: &self.history,
            xp: self.xp,
            streak: self.streak,
            last_day: &self.last_day,
        };
        Ok(serde_json::to_string(&state)?)
    }

    /// Replace the session from a saved JSON object.
    ///
    /// Invalid JSON returns an error and leaves the deck alone. A parsed value
    /// is sanitized the way the demo sanitizes `localStorage`: non-objects
    /// become an empty free session, unknown job ids and bad decisions are
    /// dropped, the first valid record for an id wins, and a legacy `agent`
    /// becomes `vip`. Combo resets, because the demo does not persist it.
    pub fn import_state(&mut self, raw: &str) -> Result<(), Error> {
        let value = serde_json::from_str(raw)?;
        let state = sanitize(&self.catalog, &value);
        self.plan = state.plan;
        self.history = state.history;
        self.xp = state.xp;
        self.streak = state.streak;
        self.last_day = state.last_day;
        self.combo = 0;
        Ok(())
    }

    fn gain(&mut self, amount: i64, today: &str) {
        self.xp += amount;
        if self.last_day != today {
            self.streak += 1;
            self.last_day = today.to_string();
        }
        self.combo += 1;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedOut<'a> {
    plan: &'a str,
    history: &'a [DecisionRecord],
    xp: i64,
    streak: i64,
    last_day: &'a str,
}

struct Persisted {
    plan: Plan,
    history: Vec<DecisionRecord>,
    xp: i64,
    streak: i64,
    last_day: String,
}

fn sanitize(catalog: &Catalog, value: &Value) -> Persisted {
    let Some(object) = value.as_object() else {
        return empty_state();
    };

    let plan = object
        .get("plan")
        .and_then(Value::as_str)
        .map(Plan::parse_saved)
        .unwrap_or(Plan::Free);

    let mut history = Vec::new();
    let mut seen = HashSet::new();
    if let Some(items) = object.get("history").and_then(Value::as_array) {
        for item in items {
            let Some(row) = item.as_object() else {
                continue;
            };
            let Some(id) = row.get("id").and_then(Value::as_str) else {
                continue;
            };
            if catalog.get(id).is_none() || seen.contains(id) {
                continue;
            }
            let decision = match row.get("decision").and_then(Value::as_str) {
                Some("bag") => Decision::Bag,
                Some("pass") => Decision::Pass,
                _ => continue,
            };
            seen.insert(id.to_string());
            history.push(DecisionRecord {
                id: id.to_string(),
                decision,
            });
        }
    }

    Persisted {
        plan,
        history,
        xp: json_i64(object.get("xp")).unwrap_or(0),
        streak: json_i64(object.get("streak")).unwrap_or(0),
        last_day: object
            .get("lastDay")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    }
}

fn empty_state() -> Persisted {
    Persisted {
        plan: Plan::Free,
        history: Vec::new(),
        xp: 0,
        streak: 0,
        last_day: String::new(),
    }
}

fn json_i64(value: Option<&Value>) -> Option<i64> {
    let Value::Number(number) = value? else {
        return None;
    };
    if let Some(integer) = number.as_i64() {
        return Some(integer);
    }
    if let Some(integer) = number.as_u64() {
        return i64::try_from(integer).ok();
    }
    number
        .as_f64()
        .filter(|value| value.is_finite())
        .map(|value| value.trunc() as i64)
}
