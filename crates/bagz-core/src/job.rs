//! Job records stored in `jobs.json`.

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// One example role in the swipe deck.
///
/// Field names match `src/data/jobs.json`. Scores are percentages from 0 to 100.
/// `chase` is the dollar figure the deck adds when the role is bagged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Job {
    /// Stable id used by swipes. Unique within a catalog.
    pub id: String,
    /// Role name shown on the card.
    pub title: String,
    /// Example company.
    pub company: String,
    /// Pay label, such as `$17.85/hr` or `$46,250/yr`.
    pub pay: String,
    /// Pay bar, 0..=100.
    pub pay_score: u8,
    /// Growth bar, 0..=100.
    pub growth: u8,
    /// Vibe bar, 0..=100.
    pub vibe: u8,
    /// Dollars added to the chased total when this role is bagged.
    pub chase: u32,
    /// Neighborhood or city label.
    pub area: String,
    /// Where the work happens: Onsite, Hybrid, or Remote in the example file.
    pub mode: String,
    /// Hours label, such as `Mornings` or `Full-time`.
    pub schedule: String,
    /// Relative post time, such as `5h ago` or `2d ago`.
    pub posted: String,
    /// Applicant label, such as `14 applied`.
    pub applicants: String,
    /// Team size label.
    pub size: String,
    /// Company stage label.
    pub stage: String,
    /// Fit percentage, 0..=100. Drives rarity.
    pub match_pct: u8,
    /// Short reasons the card back shows.
    pub reasons: Vec<String>,
    /// Skill chips.
    pub skills: Vec<String>,
    /// Perk chips.
    pub perks: Vec<String>,
    /// Recruiter reply timing label.
    pub reply: String,
    /// What the agent sends when it applies.
    pub sends: String,
}

impl Job {
    /// Check the rules that [`crate::Catalog`] enforces on every job.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidJob`] when an identity field is blank, a score
    /// is above 100, or a chip list contains a blank entry.
    pub fn validate(&self) -> Result<(), Error> {
        validate_id(&self.id)?;
        for (field, value) in [
            ("title", self.title.as_str()),
            ("company", self.company.as_str()),
            ("pay", self.pay.as_str()),
            ("area", self.area.as_str()),
            ("mode", self.mode.as_str()),
            ("schedule", self.schedule.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(invalid(&self.id, format!("{field} must not be blank")));
            }
        }
        for (field, value) in [
            ("payScore", self.pay_score),
            ("growth", self.growth),
            ("vibe", self.vibe),
            ("matchPct", self.match_pct),
        ] {
            if value > 100 {
                return Err(invalid(
                    &self.id,
                    format!("{field} must be 0..=100, got {value}"),
                ));
            }
        }
        for (field, values) in [
            ("reasons", &self.reasons),
            ("skills", &self.skills),
            ("perks", &self.perks),
        ] {
            if values.iter().any(|value| value.trim().is_empty()) {
                return Err(invalid(
                    &self.id,
                    format!("{field} must not contain blank entries"),
                ));
            }
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), Error> {
    if id.trim().is_empty() {
        return Err(invalid(id, "id must not be blank"));
    }
    if id.chars().any(char::is_whitespace) {
        return Err(invalid(id, "id must not contain whitespace"));
    }
    Ok(())
}

fn invalid(id: &str, reason: impl Into<String>) -> Error {
    Error::InvalidJob {
        id: display_id(id),
        reason: reason.into(),
    }
}

fn display_id(id: &str) -> String {
    if id.trim().is_empty() {
        "<blank>".to_string()
    } else {
        id.to_string()
    }
}
