//! Receipt tracker agent. The seeker submits each application by hand; the
//! tracker records that, reads the confirmation and status emails that
//! follow, and proposes status changes for approval.

use serde::{Deserialize, Serialize};

use super::follow_up::FollowUp;
use super::prefill::FormDraft;
use crate::approval::{Approved, Pending};
use crate::error::{Error, Result};
use crate::model::Source;
use crate::text::{days_from_iso, tokens};

pub const NAME: &str = "receipt_tracker";

/// Where an application stands. Order matters: a status only moves
/// forward, except `Rejected` and `Withdrawn`, which end it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Submitted,
    Received,
    Interview,
    Offer,
    Rejected,
    Withdrawn,
}

impl AppStatus {
    pub fn is_final(self) -> bool {
        matches!(self, Self::Offer | Self::Rejected | Self::Withdrawn)
    }

    /// `true` when moving from `self` to `next` is allowed.
    pub fn can_move_to(self, next: AppStatus) -> bool {
        if self.is_final() {
            return false;
        }
        match next {
            Self::Rejected | Self::Withdrawn => true,
            Self::Submitted => false,
            _ => next > self,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    pub posting_id: String,
    pub company: String,
    pub title: String,
    pub ats: Source,
    pub applied_on: String,
    pub status: AppStatus,
    pub last_update: String,
    pub followed_up_on: Option<String>,
    pub history: Vec<(String, AppStatus)>,
}

impl Application {
    pub fn age_days(&self, today: i64) -> Option<i64> {
        days_from_iso(&self.applied_on).map(|d| today - d)
    }
}

/// What a receipt email says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub ats: Option<Source>,
    pub company: Option<String>,
    pub status: AppStatus,
}

/// Reads the sender and body of an email and decides what it reports.
/// Returns `None` for mail that is not about an application.
pub fn parse_email(from: &str, subject: &str, body: &str) -> Option<Receipt> {
    let from_l = from.to_lowercase();
    let ats = if from_l.contains("greenhouse") {
        Some(Source::Greenhouse)
    } else if from_l.contains("lever.co") {
        Some(Source::Lever)
    } else if from_l.contains("ashbyhq") {
        Some(Source::Ashby)
    } else if from_l.contains("workable") {
        Some(Source::Workable)
    } else {
        None
    };
    let text = format!("{subject}\n{body}").to_lowercase();
    let has = |phrases: &[&str]| phrases.iter().any(|p| text.contains(p));
    let status = if has(&[
        "not moving forward",
        "decided not to",
        "will not be moving",
        "other candidates",
        "unfortunately",
        "position has been filled",
    ]) {
        AppStatus::Rejected
    } else if has(&["pleased to offer", "offer letter", "extend an offer"]) {
        AppStatus::Offer
    } else if has(&[
        "schedule an interview",
        "schedule a call",
        "interview with",
        "next step",
        "phone screen",
        "availability for",
    ]) {
        AppStatus::Interview
    } else if has(&[
        "received your application",
        "thank you for applying",
        "thanks for applying",
        "application has been received",
        "we have received",
        "application was submitted",
    ]) {
        AppStatus::Received
    } else {
        return None;
    };
    Some(Receipt {
        ats,
        company: company_from(subject, body),
        status,
    })
}

/// Company named after `at` / `to` / `with` in a subject such as
/// `Thank you for applying to Acme`.
fn company_from(subject: &str, body: &str) -> Option<String> {
    for text in [subject, body] {
        for marker in [
            " applying to ",
            " application to ",
            " interest in ",
            " update from ",
            " at ",
            " with ",
            " from ",
        ] {
            if let Some(i) = text.to_ascii_lowercase().find(marker) {
                let rest = &text[i + marker.len()..];
                let name: String = rest
                    .split(['!', '.', ',', '\n', '-', '|', '('])
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !name.is_empty() && name.split_whitespace().count() <= 5 {
                    return Some(name);
                }
            }
        }
    }
    None
}

/// A proposed change to one application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerUpdate {
    pub posting_id: String,
    pub from: AppStatus,
    pub to: AppStatus,
    pub on: String,
}

/// The seeker's applications. Every change goes through an approved value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tracker {
    pub applications: Vec<Application>,
}

impl Tracker {
    pub fn get(&self, posting_id: &str) -> Option<&Application> {
        self.applications
            .iter()
            .find(|a| a.posting_id == posting_id)
    }

    /// Records that the seeker submitted an approved form themselves.
    pub fn record_submission(
        &mut self,
        form: &Approved<FormDraft>,
        company: &str,
        title: &str,
        on: &str,
    ) -> Result<()> {
        let d = form.payload();
        if self.get(&d.posting_id).is_some() {
            return Err(Error::Guard(format!("{} is already tracked", d.posting_id)));
        }
        self.applications.push(Application {
            posting_id: d.posting_id.clone(),
            company: company.to_string(),
            title: title.to_string(),
            ats: d.ats,
            applied_on: on.to_string(),
            status: AppStatus::Submitted,
            last_update: on.to_string(),
            followed_up_on: None,
            history: vec![(on.to_string(), AppStatus::Submitted)],
        });
        Ok(())
    }

    /// Finds the application an email is about: same ATS (when known) and
    /// a company name that shares a word with it. Most recent first.
    pub fn match_receipt(&self, r: &Receipt) -> Option<&Application> {
        let want: Vec<String> = r.company.as_deref().map(tokens).unwrap_or_default();
        self.applications
            .iter()
            .rev()
            .filter(|a| !a.status.is_final())
            .filter(|a| r.ats.is_none_or(|s| s == a.ats))
            .find(|a| {
                let have = tokens(&a.company);
                !want.is_empty() && want.iter().any(|w| have.contains(w))
            })
    }

    /// The agent step: turns an email into a proposed update, or `None`
    /// when the email matches nothing or would move a status backward.
    pub fn propose(
        &self,
        from: &str,
        subject: &str,
        body: &str,
        on: &str,
    ) -> Option<Pending<TrackerUpdate>> {
        let r = parse_email(from, subject, body)?;
        let app = self.match_receipt(&r)?;
        if !app.status.can_move_to(r.status) {
            return None;
        }
        Some(Pending::new(
            NAME,
            format!(
                "{} at {}: {:?} -> {:?}",
                app.title, app.company, app.status, r.status
            ),
            TrackerUpdate {
                posting_id: app.posting_id.clone(),
                from: app.status,
                to: r.status,
                on: on.to_string(),
            },
        ))
    }

    /// Applies an approved update. Refuses stale updates (status changed
    /// since the proposal) and illegal moves.
    pub fn apply(&mut self, update: Approved<TrackerUpdate>) -> Result<()> {
        let u = update.into_payload();
        let app = self
            .applications
            .iter_mut()
            .find(|a| a.posting_id == u.posting_id)
            .ok_or_else(|| Error::Guard(format!("{} is not tracked", u.posting_id)))?;
        if app.status != u.from {
            return Err(Error::Guard("update is stale".into()));
        }
        if !app.status.can_move_to(u.to) {
            return Err(Error::Guard(format!(
                "cannot move {:?} -> {:?}",
                u.from, u.to
            )));
        }
        app.status = u.to;
        app.last_update = u.on.clone();
        app.history.push((u.on, u.to));
        Ok(())
    }

    /// Records that the seeker sent an approved follow-up.
    pub fn mark_followed_up(&mut self, f: &Approved<FollowUp>, on: &str) -> Result<()> {
        let id = &f.payload().posting_id;
        let app = self
            .applications
            .iter_mut()
            .find(|a| &a.posting_id == id)
            .ok_or_else(|| Error::Guard(format!("{id} is not tracked")))?;
        app.followed_up_on = Some(on.to_string());
        Ok(())
    }
}
