//! Model calls behind one trait. Tests and the demo use [`MockLlm`], which
//! is deterministic and offline. [`AnthropicLlm`] is the live backend; it
//! reads its key from the environment at call time and is never used in
//! tests.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// One request to a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompt {
    /// Which agent is asking, for example `cover_letter`.
    pub task: String,
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
}

/// A text-completion backend.
pub trait Llm {
    fn complete(&self, prompt: &Prompt) -> Result<String>;
    fn name(&self) -> &str;
}

/// FNV-1a, used to make mock output depend on the whole prompt.
pub fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Deterministic offline model. With a script, it returns the queued
/// replies in order; without one, it builds a reply from the prompt so the
/// same prompt always gives the same text.
#[derive(Debug, Default)]
pub struct MockLlm {
    script: RefCell<VecDeque<String>>,
    calls: RefCell<Vec<Prompt>>,
}

impl MockLlm {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues fixed replies, returned first-in first-out.
    pub fn scripted<I: IntoIterator<Item = S>, S: Into<String>>(replies: I) -> Self {
        Self {
            script: RefCell::new(replies.into_iter().map(Into::into).collect()),
            calls: RefCell::default(),
        }
    }

    /// Every prompt this mock has received.
    pub fn calls(&self) -> Vec<Prompt> {
        self.calls.borrow().clone()
    }
}

impl Llm for MockLlm {
    fn complete(&self, prompt: &Prompt) -> Result<String> {
        self.calls.borrow_mut().push(prompt.clone());
        if let Some(r) = self.script.borrow_mut().pop_front() {
            return Ok(r);
        }
        let tag = fnv1a(&format!(
            "{}\n{}\n{}",
            prompt.task, prompt.system, prompt.user
        ));
        let reply = match prompt.task.as_str() {
            "cover_letter" => {
                let role = field(&prompt.user, "Role:").unwrap_or("this role");
                let company = field(&prompt.user, "Company:").unwrap_or("your team");
                let name = field(&prompt.user, "Candidate:").unwrap_or("The candidate");
                let skills = field(&prompt.user, "Matching skills:").unwrap_or("the skills listed");
                format!(
                    "Dear {company} hiring team,\n\n\
                     I am applying for the {role} role. My work with {skills} matches what the posting asks for, \
                     and my résumé lists the results.\n\n\
                     I would welcome a short call to talk about how I can help {company}.\n\n\
                     Sincerely,\n{name}"
                )
            }
            "summary" => {
                let role = field(&prompt.user, "Role:").unwrap_or("this role");
                let skills = field(&prompt.user, "Matching skills:").unwrap_or("core skills");
                format!("Engineer focused on {skills}, applying for {role}.")
            }
            "follow_up" => {
                let role = field(&prompt.user, "Role:").unwrap_or("the role");
                let company = field(&prompt.user, "Company:").unwrap_or("your team");
                let name = field(&prompt.user, "Candidate:").unwrap_or("The candidate");
                format!(
                    "Subject: Following up on my {role} application\n\n\
                     Hello {company} team,\n\n\
                     I applied for the {role} role and remain very interested. \
                     Is there any update on the next step?\n\n\
                     Thank you,\n{name}"
                )
            }
            "rationale" => format!("Fit summary #{:04x}: see component scores.", tag & 0xffff),
            _ => format!("mock-{:016x}", tag),
        };
        Ok(reply)
    }

    fn name(&self) -> &str {
        "mock"
    }
}

/// Value after `label` on its own line in a prompt.
pub fn field<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(label))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Live backend: the Anthropic Messages API. The key comes from
/// `ANTHROPIC_API_KEY` when a call is made; it is never logged or stored.
pub struct AnthropicLlm {
    pub model: String,
    agent: ureq::Agent,
}

impl AnthropicLlm {
    pub const DEFAULT_MODEL: &'static str = "claude-sonnet-5-5";

    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(120))
                .build(),
        }
    }
}

impl Llm for AnthropicLlm {
    fn complete(&self, prompt: &Prompt) -> Result<String> {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::Llm("ANTHROPIC_API_KEY is not set".into()))?;
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": prompt.max_tokens,
            "system": prompt.system,
            "messages": [{"role": "user", "content": prompt.user}],
        });
        let resp: serde_json::Value = self
            .agent
            .post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &key)
            .set("anthropic-version", "2023-06-01")
            .send_json(body)
            .map_err(|e| Error::Llm(e.to_string()))?
            .into_json()
            .map_err(|e| Error::Llm(e.to_string()))?;
        parse_anthropic(&resp)
    }

    fn name(&self) -> &str {
        &self.model
    }
}

/// Joins the text blocks of a Messages API response.
pub fn parse_anthropic(resp: &serde_json::Value) -> Result<String> {
    let blocks = resp
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::Llm("response has no content".into()))?;
    let text: Vec<&str> = blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
        .collect();
    if text.is_empty() {
        return Err(Error::Llm("response has no text block".into()));
    }
    Ok(text.join(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(task: &str, user: &str) -> Prompt {
        Prompt {
            task: task.into(),
            system: "sys".into(),
            user: user.into(),
            max_tokens: 100,
        }
    }

    #[test]
    fn mock_is_deterministic() {
        let a = MockLlm::new().complete(&prompt("other", "x")).unwrap();
        let b = MockLlm::new().complete(&prompt("other", "x")).unwrap();
        let c = MockLlm::new().complete(&prompt("other", "y")).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn mock_script_runs_first_then_falls_back() {
        let m = MockLlm::scripted(["one"]);
        assert_eq!(m.complete(&prompt("t", "u")).unwrap(), "one");
        assert!(m.complete(&prompt("t", "u")).unwrap().starts_with("mock-"));
        assert_eq!(m.calls().len(), 2);
    }

    #[test]
    fn mock_cover_letter_uses_prompt_fields() {
        let out = MockLlm::new()
            .complete(&prompt(
                "cover_letter",
                "Role: Data Engineer\nCompany: Acme\nCandidate: Avery Example",
            ))
            .unwrap();
        assert!(
            out.contains("Data Engineer") && out.contains("Acme") && out.ends_with("Avery Example")
        );
    }

    #[test]
    fn field_reads_labels() {
        assert_eq!(field("A: 1\n  B:  two \nC:", "B:"), Some("two"));
        assert_eq!(field("C:", "C:"), None);
    }

    #[test]
    fn parse_anthropic_joins_text_blocks() {
        let v = serde_json::json!({"content":[{"type":"text","text":"Hi "},{"type":"tool_use"},{"type":"text","text":"there"}]});
        assert_eq!(parse_anthropic(&v).unwrap(), "Hi there");
        assert!(parse_anthropic(&serde_json::json!({})).is_err());
        assert!(parse_anthropic(&serde_json::json!({"content":[]})).is_err());
    }

    #[test]
    fn fnv1a_known_value() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }
}
