//! Form pre-filler agent. It maps profile data onto each ATS's own field
//! names and marks what a human must answer. It has no submit step: the
//! approved draft is a checklist the seeker uses in the real form.

use serde::{Deserialize, Serialize};

use crate::approval::Pending;
use crate::model::{Posting, Source};
use crate::profile::Profile;

pub const NAME: &str = "form_prefill";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    /// The ATS's field name.
    pub name: String,
    pub label: String,
    pub value: Option<String>,
    /// `true` when a person must enter or confirm this field.
    pub needs_human: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormDraft {
    pub posting_id: String,
    pub ats: Source,
    pub apply_url: String,
    pub fields: Vec<FormField>,
}

impl FormDraft {
    /// Fields a person still has to deal with.
    pub fn needs_human(&self) -> Vec<&FormField> {
        self.fields.iter().filter(|f| f.needs_human).collect()
    }

    pub fn get(&self, name: &str) -> Option<&FormField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

fn auto(name: &str, label: &str, value: &str) -> FormField {
    let v = value.trim();
    FormField {
        name: name.into(),
        label: label.into(),
        value: (!v.is_empty()).then(|| v.to_string()),
        needs_human: v.is_empty(),
    }
}

fn human(name: &str, label: &str, value: Option<String>) -> FormField {
    FormField {
        name: name.into(),
        label: label.into(),
        value,
        needs_human: true,
    }
}

fn yes_no(v: Option<bool>) -> Option<String> {
    v.map(|b| if b { "Yes" } else { "No" }.to_string())
}

/// Field names per ATS, as their public apply forms use them.
pub fn fields_for(ats: Source, p: &Profile) -> Vec<FormField> {
    let current = p
        .experience
        .first()
        .map(|e| e.company.as_str())
        .unwrap_or("");
    let linkedin = p.linkedin.as_deref().unwrap_or("");
    let github = p.github.as_deref().unwrap_or("");
    let mut f = match ats {
        Source::Greenhouse => vec![
            auto("first_name", "First name", &p.first_name),
            auto("last_name", "Last name", &p.last_name),
            auto("email", "Email", &p.email),
            auto("phone", "Phone", &p.phone),
            auto("location", "Location (City)", &p.location),
            human(
                "resume",
                "Resume/CV",
                Some("attach the approved .docx".into()),
            ),
            human(
                "cover_letter",
                "Cover letter",
                Some("attach the approved letter".into()),
            ),
            auto("urls[LinkedIn]", "LinkedIn profile", linkedin),
        ],
        Source::Lever => vec![
            auto("name", "Full name", &p.full_name()),
            auto("email", "Email", &p.email),
            auto("phone", "Phone", &p.phone),
            auto("org", "Current company", current),
            auto("location", "Current location", &p.location),
            auto("urls[LinkedIn]", "LinkedIn URL", linkedin),
            auto("urls[GitHub]", "GitHub URL", github),
            human(
                "resume",
                "Resume/CV",
                Some("attach the approved .docx".into()),
            ),
            human("comments", "Additional information", None),
        ],
        Source::Ashby => vec![
            auto("_systemfield_name", "Name", &p.full_name()),
            auto("_systemfield_email", "Email", &p.email),
            auto("_systemfield_phone", "Phone", &p.phone),
            auto("_systemfield_location", "Location", &p.location),
            auto("_systemfield_linkedin", "LinkedIn", linkedin),
            human(
                "_systemfield_resume",
                "Resume",
                Some("attach the approved .docx".into()),
            ),
        ],
        Source::Workable => vec![
            auto("firstname", "First name", &p.first_name),
            auto("lastname", "Last name", &p.last_name),
            auto("email", "Email", &p.email),
            auto("phone", "Phone", &p.phone),
            auto("address", "Address", &p.location),
            human("resume", "Resume", Some("attach the approved .docx".into())),
            human(
                "cover_letter",
                "Cover letter",
                Some("attach the approved letter".into()),
            ),
        ],
        Source::Rss => vec![human(
            "external_form",
            "Apply on the employer site (form unknown)",
            None,
        )],
    };
    // Legal and demographic questions: never answered by an agent alone.
    f.push(human(
        "work_authorization",
        "Are you legally authorized to work in this country?",
        yes_no(p.work_authorized),
    ));
    f.push(human(
        "sponsorship",
        "Will you now or in the future require visa sponsorship?",
        yes_no(p.needs_sponsorship),
    ));
    f.push(human(
        "eeo",
        "Voluntary self-identification (optional)",
        None,
    ));
    f
}

pub fn run(posting: &Posting, profile: &Profile) -> Pending<FormDraft> {
    let draft = FormDraft {
        posting_id: posting.id.clone(),
        ats: posting.source,
        apply_url: posting
            .apply_url
            .clone()
            .unwrap_or_else(|| posting.url.clone()),
        fields: fields_for(posting.source, profile),
    };
    let n = draft.needs_human().len();
    Pending::new(
        NAME,
        format!(
            "{} form for {} at {}: {} fields pre-filled, {} need you",
            posting.source,
            posting.title,
            posting.company,
            draft.fields.len() - n,
            n
        ),
        draft,
    )
}
