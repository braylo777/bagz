//! Six agents. Each returns a [`crate::approval::Pending`] proposal; none
//! of them submits, sends, or changes anything on its own.

pub mod cover_letter;
pub mod follow_up;
pub mod prefill;
pub mod receipt;
pub mod scorer;
pub mod tailor;

use std::collections::BTreeSet;

use crate::error::{Error, Result};
use crate::profile::Profile;
use crate::text::{token_set, tokens};

/// Name and job of one agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentInfo {
    pub name: &'static str,
    pub does: &'static str,
    pub uses_llm: bool,
}

/// Every agent in the engine. `metrics.json` counts this list.
pub const REGISTRY: &[AgentInfo] = &[
    AgentInfo {
        name: scorer::NAME,
        does: "Scores a posting against the profile and proposes apply, maybe, or skip.",
        uses_llm: true,
    },
    AgentInfo {
        name: tailor::NAME,
        does: "Reorders résumé skills and bullets for one posting; never adds a skill.",
        uses_llm: true,
    },
    AgentInfo {
        name: cover_letter::NAME,
        does: "Drafts a cover letter and rejects drafts with placeholders or invented skills.",
        uses_llm: true,
    },
    AgentInfo {
        name: prefill::NAME,
        does: "Maps profile data onto the ATS form fields and flags questions a human must answer.",
        uses_llm: false,
    },
    AgentInfo {
        name: receipt::NAME,
        does: "Reads confirmation and status emails and proposes tracker updates.",
        uses_llm: false,
    },
    AgentInfo {
        name: follow_up::NAME,
        does: "Finds stale applications and drafts a polite follow-up for review.",
        uses_llm: true,
    },
];

/// Skills and tools the guards know by name. If a model draft names one of
/// these and the profile does not list it, the draft is refused.
pub const KNOWN_SKILLS: &[&str] = &[
    "airflow",
    "android",
    "angular",
    "ansible",
    "aws",
    "azure",
    "bigquery",
    "c#",
    "c++",
    "cassandra",
    "clojure",
    "css",
    "dart",
    "databricks",
    "dbt",
    "django",
    "docker",
    "elasticsearch",
    "elixir",
    "excel",
    "figma",
    "flask",
    "flutter",
    "gcp",
    "go",
    "golang",
    "graphql",
    "hadoop",
    "haskell",
    "html",
    "ios",
    "java",
    "javascript",
    "jenkins",
    "kafka",
    "kotlin",
    "kubernetes",
    "laravel",
    "looker",
    "matlab",
    "mongodb",
    "mysql",
    "nextjs",
    "nodejs",
    "objective-c",
    "ocaml",
    "perl",
    "php",
    "postgresql",
    "powerbi",
    "python",
    "pytorch",
    "rails",
    "react",
    "redis",
    "ruby",
    "rust",
    "salesforce",
    "sass",
    "scala",
    "snowflake",
    "solidity",
    "spark",
    "sql",
    "svelte",
    "swift",
    "tableau",
    "tensorflow",
    "terraform",
    "typescript",
    "vue",
    "zig",
];

/// Known skills named in `text` that `profile` does not list.
pub fn invented_skills(text: &str, profile: &Profile) -> Vec<String> {
    let have: BTreeSet<String> = profile.skills.iter().flat_map(|s| tokens(s)).collect();
    let said = token_set(text);
    KNOWN_SKILLS
        .iter()
        .filter(|k| said.contains(**k) && !have.contains(**k))
        .map(|k| k.to_string())
        .collect()
}

/// Refuses model text that still holds template placeholders.
pub fn check_placeholders(text: &str) -> Result<()> {
    for bad in ["[", "{{", "<insert", "lorem ipsum", "TODO"] {
        if text.to_lowercase().contains(&bad.to_lowercase()) {
            return Err(Error::Guard(format!("draft holds a placeholder (`{bad}`)")));
        }
    }
    Ok(())
}

/// Refuses model text that names skills the profile does not have.
pub fn check_invented(text: &str, profile: &Profile) -> Result<()> {
    let extra = invented_skills(text, profile);
    if extra.is_empty() {
        Ok(())
    } else {
        Err(Error::Guard(format!(
            "draft claims skills not on the profile: {}",
            extra.join(", ")
        )))
    }
}

/// Profile skills the posting names, in profile order.
pub fn matching_skills(text: &str, profile: &Profile) -> Vec<String> {
    let hay = token_set(text);
    profile
        .skills
        .iter()
        .filter(|s| {
            let t = tokens(s);
            !t.is_empty() && t.iter().all(|w| hay.contains(w))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_six_unique_agents() {
        assert_eq!(REGISTRY.len(), 6);
        let names: BTreeSet<_> = REGISTRY.iter().map(|a| a.name).collect();
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn invented_skills_flags_only_unlisted_known_skills() {
        let p = Profile::example();
        assert_eq!(
            invented_skills("I love Rust and Java and teamwork", &p),
            vec!["java"]
        );
        assert!(invented_skills("Rust, Python, AWS", &p).is_empty());
    }

    #[test]
    fn placeholder_guard() {
        assert!(check_placeholders("Dear [Hiring Manager]").is_err());
        assert!(check_placeholders("Hello {{name}}").is_err());
        assert!(check_placeholders("Dear Acme team").is_ok());
    }

    #[test]
    fn matching_skills_keeps_profile_order() {
        let p = Profile::example();
        assert_eq!(
            matching_skills("kafka aws rust", &p),
            vec!["Rust", "Kafka", "AWS"]
        );
    }
}
