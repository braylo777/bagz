//! Follow-up drafter agent. Finds applications with no news after a set
//! number of days and drafts one short note each. The seeker sends it.

use serde::{Deserialize, Serialize};

use super::receipt::{AppStatus, Application, Tracker};
use super::{check_invented, check_placeholders};
use crate::approval::Pending;
use crate::error::{Error, Result};
use crate::llm::{Llm, Prompt};
use crate::profile::Profile;

pub const NAME: &str = "follow_up";

/// Days after applying before a follow-up is due.
pub const DEFAULT_WAIT_DAYS: i64 = 7;
pub const MAX_WORDS: usize = 150;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowUp {
    pub posting_id: String,
    pub company: String,
    pub subject: String,
    pub body: String,
}

/// Applications still open, never followed up, and at least `wait` days old.
pub fn due(tracker: &Tracker, today: i64, wait: i64) -> Vec<&Application> {
    tracker
        .applications
        .iter()
        .filter(|a| matches!(a.status, AppStatus::Submitted | AppStatus::Received))
        .filter(|a| a.followed_up_on.is_none())
        .filter(|a| a.age_days(today).is_some_and(|d| d >= wait))
        .collect()
}

/// Splits a `Subject: ...` first line from the body.
pub fn split_subject(text: &str, fallback: &str) -> (String, String) {
    let text = text.trim();
    match text.split_once('\n') {
        Some((first, rest)) if first.trim().to_ascii_lowercase().starts_with("subject:") => (
            first.trim()["subject:".len()..].trim().to_string(),
            rest.trim().to_string(),
        ),
        None if text.to_ascii_lowercase().starts_with("subject:") => {
            (text["subject:".len()..].trim().to_string(), String::new())
        }
        _ => (fallback.to_string(), text.to_string()),
    }
}

pub fn run(llm: &dyn Llm, app: &Application, profile: &Profile) -> Result<Pending<FollowUp>> {
    let prompt = Prompt {
        task: "follow_up".into(),
        system:
            "Write a brief, polite follow-up email. Start with a `Subject:` line. No placeholders."
                .into(),
        user: format!(
            "Role: {}\nCompany: {}\nCandidate: {}\nApplied on: {}\n",
            app.title,
            app.company,
            profile.full_name(),
            app.applied_on
        ),
        max_tokens: 300,
    };
    let raw = llm.complete(&prompt)?;
    let (subject, body) = split_subject(&raw, &format!("Following up: {}", app.title));
    let words = body.split_whitespace().count();
    if words == 0 || words > MAX_WORDS {
        return Err(Error::Guard(format!(
            "follow-up has {words} words; allowed 1..={MAX_WORDS}"
        )));
    }
    check_placeholders(&body)?;
    check_invented(&body, profile)?;
    Ok(Pending::new(
        NAME,
        format!("Follow-up to {} about {}", app.company, app.title),
        FollowUp {
            posting_id: app.posting_id.clone(),
            company: app.company.clone(),
            subject,
            body,
        },
    ))
}
