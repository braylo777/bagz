//! Cross-source dedupe. Two postings are the same job when company, title
//! and first location match after normalization, or when they share a
//! canonical URL.

use std::collections::HashMap;

use crate::model::Posting;
use crate::text::{company_key, location_key, title_key};

/// The identity key of a posting.
pub fn job_key(p: &Posting) -> String {
    format!(
        "{}|{}|{}",
        company_key(&p.company),
        title_key(&p.title),
        location_key(&p.location)
    )
}

/// URL with scheme, `www.`, query, fragment and trailing slash removed.
pub fn canonical_url(url: &str) -> String {
    let u = url.trim();
    let u = u.split_once("://").map(|(_, r)| r).unwrap_or(u);
    let u = u.split(['?', '#']).next().unwrap_or(u);
    u.trim_start_matches("www.")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

/// `true` when `a` should be kept over `b`.
fn better(a: &Posting, b: &Posting) -> bool {
    (
        a.source.priority(),
        std::cmp::Reverse(a.description.len()),
        &a.id,
    ) < (
        b.source.priority(),
        std::cmp::Reverse(b.description.len()),
        &b.id,
    )
}

/// Removes duplicates. The kept copy is the direct ATS listing over RSS,
/// then the longer description, then the smaller id, so the result does
/// not depend on input order. Output keeps first-seen order.
pub fn dedupe(postings: Vec<Posting>) -> Vec<Posting> {
    let mut slots: Vec<Posting> = Vec::with_capacity(postings.len());
    let mut by_key: HashMap<String, usize> = HashMap::new();
    let mut by_url: HashMap<String, usize> = HashMap::new();
    for p in postings {
        let key = job_key(&p);
        let url = canonical_url(&p.url);
        let hit = by_key.get(&key).copied().or_else(|| {
            (!url.is_empty())
                .then(|| by_url.get(&url).copied())
                .flatten()
        });
        match hit {
            Some(i) => {
                if better(&p, &slots[i]) {
                    let old_url = canonical_url(&slots[i].url);
                    by_url.remove(&old_url);
                    if !url.is_empty() {
                        by_url.insert(url, i);
                    }
                    by_key.insert(key, i);
                    slots[i] = p;
                } else {
                    by_key.entry(key).or_insert(i);
                    if !url.is_empty() {
                        by_url.entry(url).or_insert(i);
                    }
                }
            }
            None => {
                let i = slots.len();
                by_key.insert(key, i);
                if !url.is_empty() {
                    by_url.insert(url, i);
                }
                slots.push(p);
            }
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Source, Workplace};

    fn p(id: &str, source: Source, company: &str, title: &str, url: &str, desc: &str) -> Posting {
        Posting {
            id: id.into(),
            source,
            board: "b".into(),
            company: company.into(),
            title: title.into(),
            location: "Austin, TX".into(),
            workplace: Workplace::Onsite,
            department: None,
            employment_type: None,
            url: url.into(),
            apply_url: None,
            posted: None,
            salary: None,
            description: desc.into(),
        }
    }

    #[test]
    fn canonical_url_strips_noise() {
        assert_eq!(
            canonical_url("https://www.Jobs.test/a/1/?gh_jid=1#top"),
            "jobs.test/a/1"
        );
    }

    #[test]
    fn same_job_across_sources_keeps_ats_copy() {
        let rss = p(
            "rss:1",
            Source::Rss,
            "Acme Inc.",
            "Sr. Data Eng",
            "https://r.test/1",
            "long long long",
        );
        let gh = p(
            "greenhouse:1",
            Source::Greenhouse,
            "Acme",
            "Senior Data Engineer",
            "https://g.test/1",
            "short",
        );
        let out = dedupe(vec![rss, gh]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source, Source::Greenhouse);
    }

    #[test]
    fn shared_url_is_a_duplicate() {
        let a = p(
            "a",
            Source::Lever,
            "One",
            "Role A",
            "https://x.test/j/9",
            "",
        );
        let b = p("b", Source::Rss, "Two", "Role B", "http://x.test/j/9/", "");
        assert_eq!(dedupe(vec![a, b]).len(), 1);
    }

    #[test]
    fn distinct_jobs_survive() {
        let a = p("a", Source::Lever, "One", "Role A", "https://x.test/1", "");
        let b = p("b", Source::Lever, "One", "Role B", "https://x.test/2", "");
        assert_eq!(dedupe(vec![a, b]).len(), 2);
    }

    #[test]
    fn longer_description_wins_within_same_tier() {
        let a = p(
            "a",
            Source::Lever,
            "One",
            "Role",
            "https://x.test/1",
            "short",
        );
        let b = p(
            "b",
            Source::Ashby,
            "One",
            "Role",
            "https://y.test/1",
            "much longer text",
        );
        let out = dedupe(vec![a, b]);
        assert_eq!(out[0].id, "b");
    }
}
