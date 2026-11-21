//! Cover-letter agent. The model drafts; guards refuse drafts that are too
//! short or long, keep placeholders, skip the company name, or claim skills
//! the profile does not list.

use serde::{Deserialize, Serialize};

use super::{check_invented, check_placeholders, matching_skills};
use crate::approval::Pending;
use crate::error::{Error, Result};
use crate::llm::{Llm, Prompt};
use crate::model::Posting;
use crate::profile::Profile;

pub const NAME: &str = "cover_letter";

pub const MIN_WORDS: usize = 30;
pub const MAX_WORDS: usize = 450;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverLetter {
    pub posting_id: String,
    pub company: String,
    pub role: String,
    pub body: String,
}

/// The guard every draft must pass.
pub fn check(body: &str, posting: &Posting, profile: &Profile) -> Result<()> {
    let words = body.split_whitespace().count();
    if !(MIN_WORDS..=MAX_WORDS).contains(&words) {
        return Err(Error::Guard(format!(
            "letter has {words} words; allowed {MIN_WORDS}..={MAX_WORDS}"
        )));
    }
    check_placeholders(body)?;
    if !body
        .to_lowercase()
        .contains(&posting.company.to_lowercase())
    {
        return Err(Error::Guard("letter does not name the company".into()));
    }
    check_invented(body, profile)
}

pub fn run(llm: &dyn Llm, posting: &Posting, profile: &Profile) -> Result<Pending<CoverLetter>> {
    let matched = matching_skills(
        &format!("{} {}", posting.title, posting.description),
        profile,
    );
    let prompt = Prompt {
        task: "cover_letter".into(),
        system: "Write a short, plain cover letter. Use only facts from the candidate profile. No placeholders.".into(),
        user: format!(
            "Role: {}\nCompany: {}\nCandidate: {}\nMatching skills: {}\nHeadline: {}\nPosting excerpt: {}\n",
            posting.title,
            posting.company,
            profile.full_name(),
            if matched.is_empty() { "general engineering".to_string() } else { matched.join(", ") },
            profile.headline,
            crate::text::truncate_chars(&posting.description, 600),
        ),
        max_tokens: 700,
    };
    let body = llm.complete(&prompt)?.trim().to_string();
    check(&body, posting, profile)?;
    Ok(Pending::new(
        NAME,
        format!("Cover letter for {} at {}", posting.title, posting.company),
        CoverLetter {
            posting_id: posting.id.clone(),
            company: posting.company.clone(),
            role: posting.title.clone(),
            body,
        },
    ))
}
