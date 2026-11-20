//! Restore demo state the way `sanitize` does in `src/store.tsx`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A stored decision. Super-bag is recorded as a bag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Applied. Shown in the bag, newest first.
    Bag,
    /// Skipped.
    Pass,
}

/// One entry in swipe history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Job id.
    pub id: String,
    /// Bag or pass.
    pub decision: Decision,
}

/// Paid plan stored with the deck.
///
/// The demo's old `agent` value is read as [`Plan::Vip`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    /// Swipe is free.
    Free,
    /// Pro badge.
    Pro,
    /// VIP, including a stored `agent` plan.
    Vip,
}

/// State the demo writes to `localStorage` under `bagz-demo-v2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    /// Current plan.
    pub plan: Plan,
    /// Bag and pass decisions, oldest first. Each id appears once.
    pub history: Vec<DecisionRecord>,
    /// Experience points. Bag is worth 10 and super-bag is worth 25.
    pub xp: u32,
    /// Days on which a bag or super-bag scored.
    pub streak: u32,
    /// Last day a bag or super-bag scored, or empty.
    pub last_day: String,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            plan: Plan::Free,
            history: Vec::new(),
            xp: 0,
            streak: 0,
            last_day: String::new(),
        }
    }
}

/// Keep only decisions the demo would keep.
///
/// Unknown ids, duplicate ids, and decisions other than `bag` or `pass` are
/// dropped. The first valid decision for an id wins. A non-object `value`
/// becomes the empty free state. `xp` and `streak` keep the whole part of a
/// non-negative number that fits in `u32`, and are otherwise zero.
#[must_use]
pub fn sanitize_state(value: &Value, known_ids: &HashSet<&str>) -> PersistedState {
    let Some(obj) = value.as_object() else {
        return PersistedState::default();
    };
    PersistedState {
        plan: parse_plan(obj.get("plan")),
        history: parse_history(obj.get("history"), known_ids),
        xp: parse_u32(obj.get("xp")),
        streak: parse_u32(obj.get("streak")),
        last_day: parse_last_day(obj.get("lastDay")),
    }
}

fn parse_plan(value: Option<&Value>) -> Plan {
    match value.and_then(Value::as_str) {
        Some("pro") => Plan::Pro,
        Some("vip" | "agent") => Plan::Vip,
        _ => Plan::Free,
    }
}

fn parse_history(value: Option<&Value>, known_ids: &HashSet<&str>) -> Vec<DecisionRecord> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut history = Vec::new();
    for item in items {
        let Some(row) = item.as_object() else {
            continue;
        };
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !known_ids.contains(id) || seen.contains(id) {
            continue;
        }
        let Some(decision) = row.get("decision").and_then(Value::as_str) else {
            continue;
        };
        let decision = match decision {
            "bag" => Decision::Bag,
            "pass" => Decision::Pass,
            _ => continue,
        };
        seen.insert(id.to_string());
        history.push(DecisionRecord {
            id: id.to_string(),
            decision,
        });
    }
    history
}

fn parse_u32(value: Option<&Value>) -> u32 {
    let Some(number) = value.and_then(Value::as_f64) else {
        return 0;
    };
    if !number.is_finite() || number < 0.0 || number > f64::from(u32::MAX) {
        return 0;
    }
    number as u32
}

fn parse_last_day(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_drops_unknown_and_keeps_the_first_valid_row() {
        let ids = HashSet::from(["froth"]);
        let raw = json!({
            "plan": "agent",
            "xp": 10.9,
            "streak": -3,
            "lastDay": "2026-11-20",
            "history": [
                {"id": "missing", "decision": "bag"},
                {"id": "froth", "decision": "hold"},
                {"id": "froth", "decision": "pass"},
                {"id": "froth", "decision": "bag"},
                "nope",
                null
            ]
        });
        let state = sanitize_state(&raw, &ids);
        assert_eq!(state.plan, Plan::Vip);
        assert_eq!(state.xp, 10);
        assert_eq!(state.streak, 0);
        assert_eq!(state.last_day, "2026-11-20");
        assert_eq!(
            state.history,
            vec![DecisionRecord {
                id: "froth".to_string(),
                decision: Decision::Pass,
            }]
        );
    }

    #[test]
    fn non_objects_and_bad_numbers_reset() {
        let ids = HashSet::new();
        assert_eq!(
            sanitize_state(&Value::Null, &ids),
            PersistedState::default()
        );
        assert_eq!(
            sanitize_state(&json!(["bag"]), &ids),
            PersistedState::default()
        );
        let raw = json!({"plan": "pro", "xp": "10", "streak": 1e20, "lastDay": 4});
        let state = sanitize_state(&raw, &ids);
        assert_eq!(state.plan, Plan::Pro);
        assert_eq!(state.xp, 0);
        assert_eq!(state.streak, 0);
        assert_eq!(state.last_day, "");
    }
}
