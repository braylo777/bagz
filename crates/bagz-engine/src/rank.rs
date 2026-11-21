//! Ranks postings against a profile. Pure and deterministic: the same
//! postings, profile and date always give the same order.

use serde::{Deserialize, Serialize};

use crate::model::{Posting, Workplace};
use crate::profile::Profile;
use crate::text::{days_from_iso, token_set, tokens};

/// Component scores, each 0 to 100, and the weighted total.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RankScore {
    pub total: f64,
    pub skills: f64,
    pub title: f64,
    pub location: f64,
    pub recency: f64,
    pub salary: f64,
}

pub const W_SKILLS: f64 = 0.40;
pub const W_TITLE: f64 = 0.25;
pub const W_LOCATION: f64 = 0.15;
pub const W_RECENCY: f64 = 0.10;
pub const W_SALARY: f64 = 0.10;

/// Share of profile skills named in the title or description.
pub fn skill_score(p: &Posting, profile: &Profile) -> f64 {
    if profile.skills.is_empty() {
        return 0.0;
    }
    let hay = token_set(&format!("{} {}", p.title, p.description));
    let hits = profile
        .skills
        .iter()
        .filter(|s| {
            let t = tokens(s);
            !t.is_empty() && t.iter().all(|w| hay.contains(w))
        })
        .count();
    100.0 * hits as f64 / profile.skills.len() as f64
}

/// Best word overlap between the posting title and any target title.
pub fn title_score(p: &Posting, profile: &Profile) -> f64 {
    let have = token_set(&crate::text::title_key(&p.title));
    profile
        .target_titles
        .iter()
        .map(|t| {
            let want = token_set(&crate::text::title_key(t));
            if want.is_empty() {
                0.0
            } else {
                100.0 * want.intersection(&have).count() as f64 / want.len() as f64
            }
        })
        .fold(0.0, f64::max)
}

/// 100 for remote when the seeker accepts remote, or a named place match.
pub fn location_score(p: &Posting, profile: &Profile) -> f64 {
    if p.workplace == Workplace::Remote && profile.remote_ok {
        return 100.0;
    }
    let loc = token_set(&p.location);
    let hit = profile.locations.iter().any(|l| {
        let t = tokens(l);
        !t.is_empty() && t.iter().all(|w| loc.contains(w))
    });
    match (hit, p.workplace) {
        (true, _) => 100.0,
        (false, Workplace::Remote) => 40.0,
        (false, Workplace::Unknown) => 30.0,
        _ => 0.0,
    }
}

/// Linear decay from 100 (today) to 0 (60 days old). Unknown dates get 50.
pub fn recency_score(p: &Posting, today: i64) -> f64 {
    match p.posted.as_deref().and_then(days_from_iso) {
        Some(d) => {
            let age = (today - d).max(0) as f64;
            (100.0 * (1.0 - age / 60.0)).clamp(0.0, 100.0)
        }
        None => 50.0,
    }
}

/// 100 when the top of the range meets the seeker's minimum, 0 when it is
/// below, 50 when either side is unknown.
pub fn salary_score(p: &Posting, profile: &Profile) -> f64 {
    match (&p.salary, profile.min_salary) {
        (Some(s), Some(min)) if s.max >= min => 100.0,
        (Some(_), Some(_)) => 0.0,
        _ => 50.0,
    }
}

pub fn score(p: &Posting, profile: &Profile, today: i64) -> RankScore {
    let skills = skill_score(p, profile);
    let title = title_score(p, profile);
    let location = location_score(p, profile);
    let recency = recency_score(p, today);
    let salary = salary_score(p, profile);
    let total = W_SKILLS * skills
        + W_TITLE * title
        + W_LOCATION * location
        + W_RECENCY * recency
        + W_SALARY * salary;
    RankScore {
        total,
        skills,
        title,
        location,
        recency,
        salary,
    }
}

/// Postings sorted best first. Ties break on id so the order is total.
pub fn rank<'a>(
    postings: &'a [Posting],
    profile: &Profile,
    today: i64,
) -> Vec<(&'a Posting, RankScore)> {
    let mut scored: Vec<_> = postings
        .iter()
        .map(|p| (p, score(p, profile, today)))
        .collect();
    scored.sort_by(|a, b| {
        b.1.total
            .partial_cmp(&a.1.total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Salary, Source};
    use crate::text::days_from_civil;

    fn posting(title: &str, desc: &str, loc: &str, wp: Workplace) -> Posting {
        Posting {
            id: format!("lever:t:{title}"),
            source: Source::Lever,
            board: "t".into(),
            company: "T".into(),
            title: title.into(),
            location: loc.into(),
            workplace: wp,
            department: None,
            employment_type: None,
            url: String::new(),
            apply_url: None,
            posted: Some("2026-11-10".into()),
            salary: None,
            description: desc.into(),
        }
    }

    #[test]
    fn skills_count_whole_words() {
        let p = posting(
            "Backend Engineer",
            "We use Rust, Kafka and AWS.",
            "Austin, TX",
            Workplace::Onsite,
        );
        let s = skill_score(&p, &Profile::example());
        assert!((s - 50.0).abs() < 1e-9, "{s}");
    }

    #[test]
    fn title_overlap_takes_best_target() {
        let p = posting("Senior Backend Engineer", "", "", Workplace::Remote);
        assert_eq!(title_score(&p, &Profile::example()), 100.0);
        let q = posting("Account Executive", "", "", Workplace::Remote);
        assert_eq!(title_score(&q, &Profile::example()), 0.0);
    }

    #[test]
    fn location_rules() {
        let prof = Profile::example();
        assert_eq!(
            location_score(&posting("x", "", "Anywhere", Workplace::Remote), &prof),
            100.0
        );
        assert_eq!(
            location_score(&posting("x", "", "Austin, TX", Workplace::Onsite), &prof),
            100.0
        );
        assert_eq!(
            location_score(&posting("x", "", "Berlin", Workplace::Onsite), &prof),
            0.0
        );
        assert_eq!(
            location_score(&posting("x", "", "", Workplace::Unknown), &prof),
            30.0
        );
        let mut no_remote = prof.clone();
        no_remote.remote_ok = false;
        assert_eq!(
            location_score(&posting("x", "", "Berlin", Workplace::Remote), &no_remote),
            40.0
        );
    }

    #[test]
    fn recency_decays_and_clamps() {
        let today = days_from_civil(2026, 11, 20);
        let mut p = posting("x", "", "", Workplace::Remote);
        assert!((recency_score(&p, today) - 100.0 * (1.0 - 10.0 / 60.0)).abs() < 1e-9);
        p.posted = Some("2025-01-01".into());
        assert_eq!(recency_score(&p, today), 0.0);
        p.posted = Some("2027-01-01".into());
        assert_eq!(recency_score(&p, today), 100.0);
        p.posted = None;
        assert_eq!(recency_score(&p, today), 50.0);
    }

    #[test]
    fn salary_rules() {
        let prof = Profile::example();
        let mut p = posting("x", "", "", Workplace::Remote);
        assert_eq!(salary_score(&p, &prof), 50.0);
        p.salary = Some(Salary {
            min: 100_000,
            max: 150_000,
            currency: "USD".into(),
        });
        assert_eq!(salary_score(&p, &prof), 100.0);
        p.salary = Some(Salary {
            min: 50_000,
            max: 90_000,
            currency: "USD".into(),
        });
        assert_eq!(salary_score(&p, &prof), 0.0);
    }

    #[test]
    fn rank_orders_best_first() {
        let today = days_from_civil(2026, 11, 20);
        let good = posting(
            "Backend Engineer",
            "Rust Python PostgreSQL Kafka AWS Kubernetes",
            "Remote",
            Workplace::Remote,
        );
        let bad = posting(
            "Account Executive",
            "Sell things",
            "Berlin",
            Workplace::Onsite,
        );
        let list = vec![bad, good];
        let ranked = rank(&list, &Profile::example(), today);
        assert_eq!(ranked[0].0.title, "Backend Engineer");
        assert!(ranked[0].1.total > ranked[1].1.total);
    }
}
