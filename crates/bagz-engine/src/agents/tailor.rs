//! Résumé tailor agent. It reorders what is already on the résumé so the
//! lines that match the posting come first, and asks the model for a
//! two-line summary. It never adds a skill, a role, or a bullet.

use serde::{Deserialize, Serialize};

use super::{check_invented, check_placeholders, matching_skills};
use crate::approval::Pending;
use crate::error::{Error, Result};
use crate::llm::{Llm, Prompt};
use crate::model::Posting;
use crate::profile::{Experience, Profile};
use crate::text::{token_set, tokens};

pub const NAME: &str = "resume_tailor";

/// Longest summary the tailor accepts, in characters.
pub const SUMMARY_MAX: usize = 400;

/// A résumé ready for export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resume {
    pub name: String,
    pub contact: Vec<String>,
    pub headline: String,
    pub summary: String,
    pub skills: Vec<String>,
    pub experience: Vec<Experience>,
    pub education: Vec<String>,
    /// Posting this version targets.
    pub target: Option<String>,
}

impl Resume {
    /// The untailored résumé, straight from the profile.
    pub fn from_profile(p: &Profile) -> Self {
        let mut contact = vec![p.email.clone(), p.phone.clone(), p.location.clone()];
        contact.extend(p.linkedin.iter().cloned());
        contact.extend(p.github.iter().cloned());
        contact.extend(p.website.iter().cloned());
        contact.retain(|c| !c.trim().is_empty());
        Self {
            name: p.full_name(),
            contact,
            headline: p.headline.clone(),
            summary: p.summary.clone(),
            skills: p.skills.clone(),
            experience: p.experience.clone(),
            education: p.education.clone(),
            target: None,
        }
    }
}

/// How many posting words a line shares.
pub fn relevance(line: &str, posting_words: &std::collections::BTreeSet<String>) -> usize {
    tokens(line)
        .into_iter()
        .filter(|t| t.len() > 2 && posting_words.contains(t))
        .count()
}

/// Reorders skills (matches first) and each role's bullets (most relevant
/// first). Stable: equal lines keep résumé order. Roles stay in date order.
pub fn reorder(profile: &Profile, posting: &Posting) -> Resume {
    let mut r = Resume::from_profile(profile);
    let text = format!("{} {}", posting.title, posting.description);
    let words = token_set(&text);
    let matched = matching_skills(&text, profile);
    let mut skills = matched.clone();
    skills.extend(
        profile
            .skills
            .iter()
            .filter(|s| !matched.contains(s))
            .cloned(),
    );
    r.skills = skills;
    for role in &mut r.experience {
        role.bullets
            .sort_by_key(|b| std::cmp::Reverse(relevance(b, &words)));
    }
    r.target = Some(posting.id.clone());
    r
}

pub fn run(llm: &dyn Llm, posting: &Posting, profile: &Profile) -> Result<Pending<Resume>> {
    let mut r = reorder(profile, posting);
    let matched = matching_skills(
        &format!("{} {}", posting.title, posting.description),
        profile,
    );
    let prompt = Prompt {
        task: "summary".into(),
        system: "Write a résumé summary of at most two sentences. Use only skills and facts from the candidate profile. Do not invent anything.".into(),
        user: format!(
            "Role: {}\nCompany: {}\nMatching skills: {}\nProfile skills: {}\nCurrent summary: {}\n",
            posting.title,
            posting.company,
            matched.join(", "),
            profile.skills.join(", "),
            profile.summary
        ),
        max_tokens: 160,
    };
    let summary = llm.complete(&prompt)?.trim().to_string();
    if summary.is_empty() || summary.chars().count() > SUMMARY_MAX {
        return Err(Error::Guard(format!(
            "summary must be 1..={SUMMARY_MAX} characters"
        )));
    }
    check_placeholders(&summary)?;
    check_invented(&summary, profile)?;
    r.summary = summary;
    let note = format!(
        "Résumé for {} at {}: {} matching skills moved first",
        posting.title,
        posting.company,
        matched.len()
    );
    Ok(Pending::new(NAME, note, r))
}
