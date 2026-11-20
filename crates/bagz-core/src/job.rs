use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::Error;

/// Card rarity from the demo's `rarityOf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    /// Match of 90 or more.
    Legendary,
    /// Match of 85 to 89.
    Rare,
    /// Match under 85.
    Common,
}

impl Rarity {
    /// The class name the demo puts on a card.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Legendary => "legendary",
            Self::Rare => "rare",
            Self::Common => "common",
        }
    }
}

/// One example role from `src/data/jobs.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    /// Stable id used by swipes, for example `froth`.
    pub id: String,
    /// Role title shown on the card.
    pub title: String,
    /// Example company name.
    pub company: String,
    /// Pay label, already formatted (`$17.85/hr` or `$46,250/yr`).
    pub pay: String,
    /// Pay bar, 0 to 100.
    pub pay_score: u8,
    /// Growth bar, 0 to 100.
    pub growth: u8,
    /// Vibe bar, 0 to 100.
    pub vibe: u8,
    /// Dollar figure summed into the deck's "chased" total.
    pub chase: u32,
    /// Neighborhood or city label.
    pub area: String,
    /// `Onsite`, `Hybrid`, or `Remote` in the example file.
    pub mode: String,
    /// Shift label, for example `Mornings` or `15 hrs`.
    pub schedule: String,
    /// Relative post time, for example `3h ago` or `2d ago`.
    pub posted: String,
    /// Applicant label, for example `14 applied`.
    pub applicants: String,
    /// Team size label.
    pub size: String,
    /// Company stage label.
    pub stage: String,
    /// Fit percentage shipped with the example job.
    pub match_pct: u8,
    /// Short reasons the card flips to.
    pub reasons: Vec<String>,
    /// Skill chips.
    pub skills: Vec<String>,
    /// Perk chips.
    pub perks: Vec<String>,
    /// Reply-time label.
    pub reply: String,
    /// What the agent sends, shown in chat.
    pub sends: String,
}

impl Job {
    /// Rounded mean of pay, fit, growth, and vibe. Same as `scoreOf`.
    pub fn score(&self) -> u8 {
        rounded_mean([self.pay_score, self.match_pct, self.growth, self.vibe])
    }

    /// `legendary` at 90, `rare` at 85, otherwise `common`.
    pub fn rarity(&self) -> Rarity {
        if self.match_pct >= 90 {
            Rarity::Legendary
        } else if self.match_pct >= 85 {
            Rarity::Rare
        } else {
            Rarity::Common
        }
    }

    /// Corner sticker. Empty string means the demo draws no sticker.
    ///
    /// Priority matches `stickerFor`: HOT, then REMOTE, then NEW (`h ago`),
    /// then `$$$` when pay is at least 80.
    pub fn sticker(&self) -> &'static str {
        if self.match_pct >= 90 {
            "HOT"
        } else if self.mode == "Remote" {
            "REMOTE"
        } else if self.posted.ends_with("h ago") {
            "NEW"
        } else if self.pay_score >= 80 {
            "$$$"
        } else {
            ""
        }
    }

    /// Whether this job satisfies a deck filter.
    pub fn matches(&self, filter: &JobFilter) -> bool {
        if let Some(mode) = filled(&filter.mode) {
            if !self.mode.eq_ignore_ascii_case(mode) {
                return false;
            }
        }
        if let Some(area) = filled(&filter.area) {
            if !contains_ignore_ascii_case(&self.area, area) {
                return false;
            }
        }
        if let Some(skill) = filled(&filter.skill) {
            if !self
                .skills
                .iter()
                .any(|item| item.eq_ignore_ascii_case(skill))
            {
                return false;
            }
        }
        if let Some(min_match) = filter.min_match {
            if self.match_pct < min_match {
                return false;
            }
        }
        true
    }
}

/// Optional filters over the example catalog.
///
/// Empty strings are ignored. Mode and skill are case-insensitive exact
/// matches. Area is a case-insensitive substring, so `east` matches
/// `East Austin` and `East 6th`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobFilter {
    /// Work mode, for example `Remote`.
    pub mode: Option<String>,
    /// Substring of the area label.
    pub area: Option<String>,
    /// One skill chip.
    pub skill: Option<String>,
    /// Minimum shipped `matchPct`.
    pub min_match: Option<u8>,
}

/// A job ranked by how many of its skills a seeker listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMatch<'a> {
    /// The catalog job.
    pub job: &'a Job,
    /// Job skills the seeker also listed, in catalog order.
    pub shared: Vec<String>,
    /// `shared / distinct job skills`, as an integer percent from 0 to 100.
    pub overlap_pct: u8,
}

/// The example catalog, in file order.
#[derive(Debug, Clone)]
pub struct Catalog {
    jobs: Vec<Job>,
    index: HashMap<String, usize>,
}

impl Catalog {
    /// The `jobs.json` compiled into this crate.
    pub fn bundled() -> Result<Self, Error> {
        Self::from_json(include_str!("../../../src/data/jobs.json"))
    }

    /// Read a jobs file from disk. The demo file is `src/data/jobs.json`.
    pub fn load_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let text = fs::read_to_string(path)?;
        Self::from_json(&text)
    }

    /// Parse and validate a jobs JSON array.
    pub fn from_json(text: &str) -> Result<Self, Error> {
        let jobs: Vec<Job> = serde_json::from_str(text)?;
        Self::from_jobs(jobs)
    }

    /// Jobs in file order.
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// Number of jobs.
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Always false for a catalog that passed validation.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Look up one job by id.
    pub fn get(&self, id: &str) -> Option<&Job> {
        self.index.get(id).map(|position| &self.jobs[*position])
    }

    /// Jobs that match every set filter, in file order.
    pub fn filter(&self, filter: &JobFilter) -> Vec<&Job> {
        self.jobs.iter().filter(|job| job.matches(filter)).collect()
    }

    /// Rank jobs by skill overlap, then by shipped match, then file order.
    ///
    /// `matchPct` stays the score shipped in the example file. Overlap is a
    /// separate matcher: the percent of that job's distinct skills the seeker
    /// listed. The profile screen's example skills are `Espresso`, `React`,
    /// and the rest of that chip list.
    pub fn rank_for_skills(&self, skills: &[impl AsRef<str>]) -> Vec<SkillMatch<'_>> {
        let have: HashSet<String> = skills
            .iter()
            .map(|skill| skill.as_ref().trim().to_ascii_lowercase())
            .filter(|skill| !skill.is_empty())
            .collect();

        let mut ranked: Vec<(usize, SkillMatch<'_>)> = self
            .jobs
            .iter()
            .enumerate()
            .map(|(position, job)| {
                let (shared, distinct) = shared_skills(job, &have);
                let overlap_pct = shared
                    .len()
                    .checked_mul(100)
                    .and_then(|value| value.checked_div(distinct))
                    .and_then(|value| u8::try_from(value).ok())
                    .unwrap_or(0);
                (
                    position,
                    SkillMatch {
                        job,
                        shared,
                        overlap_pct,
                    },
                )
            })
            .collect();

        ranked.sort_by(|left, right| {
            right
                .1
                .overlap_pct
                .cmp(&left.1.overlap_pct)
                .then(right.1.job.match_pct.cmp(&left.1.job.match_pct))
                .then(left.0.cmp(&right.0))
        });

        ranked.into_iter().map(|(_, hit)| hit).collect()
    }

    fn from_jobs(jobs: Vec<Job>) -> Result<Self, Error> {
        if jobs.is_empty() {
            return Err(Error::Invalid(
                "jobs file must contain at least one job".to_string(),
            ));
        }

        let mut index = HashMap::with_capacity(jobs.len());
        for (position, job) in jobs.iter().enumerate() {
            validate_job(position, job)?;
            if index.insert(job.id.clone(), position).is_some() {
                return Err(Error::Invalid(format!(
                    "jobs[{position}]: duplicate id `{}`",
                    job.id
                )));
            }
        }

        Ok(Self { jobs, index })
    }
}

fn validate_job(index: usize, job: &Job) -> Result<(), Error> {
    check_text(index, "id", &job.id)?;
    check_text(index, "title", &job.title)?;
    check_text(index, "company", &job.company)?;
    check_text(index, "pay", &job.pay)?;
    check_text(index, "area", &job.area)?;
    check_text(index, "mode", &job.mode)?;
    check_text(index, "schedule", &job.schedule)?;
    check_text(index, "posted", &job.posted)?;
    check_text(index, "applicants", &job.applicants)?;
    check_text(index, "size", &job.size)?;
    check_text(index, "stage", &job.stage)?;
    check_text(index, "reply", &job.reply)?;
    check_text(index, "sends", &job.sends)?;
    check_percent(index, "payScore", job.pay_score)?;
    check_percent(index, "growth", job.growth)?;
    check_percent(index, "vibe", job.vibe)?;
    check_percent(index, "matchPct", job.match_pct)?;
    check_list(index, "reasons", &job.reasons)?;
    check_list(index, "skills", &job.skills)?;
    check_list(index, "perks", &job.perks)?;
    Ok(())
}

fn check_text(index: usize, field: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        Err(invalid(index, format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn check_percent(index: usize, field: &str, value: u8) -> Result<(), Error> {
    if value > 100 {
        Err(invalid(index, format!("{field} must be between 0 and 100")))
    } else {
        Ok(())
    }
}

fn check_list(index: usize, field: &str, values: &[String]) -> Result<(), Error> {
    if values.is_empty() {
        return Err(invalid(index, format!("{field} must not be empty")));
    }
    for (item_index, value) in values.iter().enumerate() {
        if value.trim().is_empty() {
            return Err(invalid(
                index,
                format!("{field}[{item_index}] must not be empty"),
            ));
        }
    }
    Ok(())
}

fn invalid(index: usize, message: String) -> Error {
    Error::Invalid(format!("jobs[{index}]: {message}"))
}

fn filled(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn shared_skills(job: &Job, have: &HashSet<String>) -> (Vec<String>, usize) {
    let mut shared = Vec::new();
    let mut seen = HashSet::new();
    let mut distinct = 0;
    for skill in &job.skills {
        let key = skill.trim().to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key.clone()) {
            continue;
        }
        distinct += 1;
        if have.contains(&key) {
            shared.push(skill.clone());
        }
    }
    (shared, distinct)
}

fn rounded_mean(values: [u8; 4]) -> u8 {
    let sum = values.iter().copied().map(u16::from).sum::<u16>();
    let rounded = (f64::from(sum) / 4.0).round();
    if rounded <= 0.0 {
        0
    } else if rounded >= 100.0 {
        100
    } else {
        rounded as u8
    }
}
