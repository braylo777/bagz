//! The seeker's own data. It stays local; agents read it, nothing sends it.

use serde::{Deserialize, Serialize};

/// One role on the résumé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Experience {
    pub title: String,
    pub company: String,
    pub start: String,
    pub end: String,
    pub bullets: Vec<String>,
}

/// The seeker profile and résumé source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub linkedin: Option<String>,
    pub github: Option<String>,
    pub website: Option<String>,
    pub headline: String,
    pub summary: String,
    pub skills: Vec<String>,
    pub experience: Vec<Experience>,
    pub education: Vec<String>,
    /// Titles the seeker wants, for ranking.
    pub target_titles: Vec<String>,
    /// Places the seeker can work from (city or country names).
    pub locations: Vec<String>,
    pub remote_ok: bool,
    /// Lowest yearly pay the seeker accepts, if any.
    pub min_salary: Option<u64>,
    pub work_authorized: Option<bool>,
    pub needs_sponsorship: Option<bool>,
}

impl Profile {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// A fictional profile for demos and tests. No real person.
    pub fn example() -> Self {
        Self {
            first_name: "Avery".into(),
            last_name: "Example".into(),
            email: "avery@example.com".into(),
            phone: "+1 555 0100".into(),
            location: "Austin, TX".into(),
            linkedin: Some("https://www.linkedin.com/in/avery-example".into()),
            github: Some("https://github.com/avery-example".into()),
            website: None,
            headline: "Backend engineer, Rust and data pipelines".into(),
            summary: "Backend engineer who ships reliable data services.".into(),
            skills: vec![
                "Rust".into(),
                "Python".into(),
                "PostgreSQL".into(),
                "Kafka".into(),
                "AWS".into(),
                "Kubernetes".into(),
            ],
            experience: vec![
                Experience {
                    title: "Software Engineer".into(),
                    company: "Northwind Data".into(),
                    start: "2022-03".into(),
                    end: "Present".into(),
                    bullets: vec![
                        "Built a Rust ingestion service that handles 40K events per second.".into(),
                        "Cut PostgreSQL query time 60% with partial indexes.".into(),
                        "Ran the on-call rotation for six Kubernetes services.".into(),
                    ],
                },
                Experience {
                    title: "Data Engineer".into(),
                    company: "Contoso Retail".into(),
                    start: "2019-06".into(),
                    end: "2022-02".into(),
                    bullets: vec![
                        "Moved nightly Python batch jobs to Kafka streams.".into(),
                        "Wrote the AWS cost report the finance team still uses.".into(),
                    ],
                },
            ],
            education: vec!["B.S. Computer Science, Example State University, 2019".into()],
            target_titles: vec![
                "Backend Engineer".into(),
                "Software Engineer".into(),
                "Data Engineer".into(),
            ],
            locations: vec!["Austin".into(), "United States".into()],
            remote_ok: true,
            min_salary: Some(120_000),
            work_authorized: Some(true),
            needs_sponsorship: Some(false),
        }
    }
}
