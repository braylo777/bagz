//! Scorer agent: deterministic fit score plus a short model rationale.

use serde::{Deserialize, Serialize};

use super::matching_skills;
use crate::approval::Pending;
use crate::error::Result;
use crate::llm::{Llm, Prompt};
use crate::model::Posting;
use crate::profile::Profile;
use crate::rank::{score, RankScore};

pub const NAME: &str = "scorer";

/// Score at or above which the scorer proposes applying.
pub const APPLY_AT: f64 = 60.0;
/// Score at or above which the scorer proposes a closer look.
pub const MAYBE_AT: f64 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Apply,
    Maybe,
    Skip,
}

impl Verdict {
    pub fn from_total(total: f64) -> Self {
        if total >= APPLY_AT {
            Self::Apply
        } else if total >= MAYBE_AT {
            Self::Maybe
        } else {
            Self::Skip
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreCard {
    pub posting_id: String,
    pub score: RankScore,
    pub verdict: Verdict,
    pub matched_skills: Vec<String>,
    pub rationale: String,
}

/// Scores one posting. The number is computed, not generated; the model
/// only writes the one-line rationale.
pub fn run(
    llm: &dyn Llm,
    posting: &Posting,
    profile: &Profile,
    today: i64,
) -> Result<Pending<ScoreCard>> {
    let s = score(posting, profile, today);
    let matched = matching_skills(
        &format!("{} {}", posting.title, posting.description),
        profile,
    );
    let verdict = Verdict::from_total(s.total);
    let prompt = Prompt {
        task: "rationale".into(),
        system: "Explain a job-fit score in one sentence. Use only the facts given.".into(),
        user: format!(
            "Role: {}\nCompany: {}\nScore: {:.0}\nMatching skills: {}\n",
            posting.title,
            posting.company,
            s.total,
            matched.join(", ")
        ),
        max_tokens: 120,
    };
    let rationale = llm.complete(&prompt)?.trim().to_string();
    let summary = format!(
        "{:?} {} at {} ({:.0}/100)",
        verdict, posting.title, posting.company, s.total
    );
    Ok(Pending::new(
        NAME,
        summary,
        ScoreCard {
            posting_id: posting.id.clone(),
            score: s,
            verdict,
            matched_skills: matched,
            rationale,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_thresholds() {
        assert_eq!(Verdict::from_total(60.0), Verdict::Apply);
        assert_eq!(Verdict::from_total(59.9), Verdict::Maybe);
        assert_eq!(Verdict::from_total(40.0), Verdict::Maybe);
        assert_eq!(Verdict::from_total(39.9), Verdict::Skip);
    }
}
