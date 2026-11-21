//! Workable widget API: `GET /api/v1/widget/accounts/{subdomain}?details=true`.

use serde_json::Value;

use super::{company_from_slug, str_field};
use crate::error::{Error, Result};
use crate::model::{Posting, Source, Workplace};
use crate::text::{html_to_text, iso_date, truncate_chars, DESCRIPTION_CAP};

pub fn url(subdomain: &str) -> String {
    format!("https://apply.workable.com/api/v1/widget/accounts/{subdomain}?details=true")
}

pub fn parse(body: &str, board: &str) -> Result<Vec<Posting>> {
    let root: Value =
        serde_json::from_str(body).map_err(|e| Error::parse("workable", e.to_string()))?;
    let jobs = root
        .get("jobs")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::parse("workable", "missing `jobs` array"))?;
    let company = str_field(&root, "name")
        .map(str::to_string)
        .unwrap_or_else(|| company_from_slug(board));
    let mut out = Vec::with_capacity(jobs.len());
    for j in jobs {
        let (Some(code), Some(title)) = (str_field(j, "shortcode"), str_field(j, "title")) else {
            continue;
        };
        let location = ["city", "state", "country"]
            .iter()
            .filter_map(|k| str_field(j, k))
            .collect::<Vec<_>>()
            .join(", ");
        let workplace = if j.get("telecommuting").and_then(Value::as_bool) == Some(true) {
            Workplace::Remote
        } else if location.is_empty() {
            Workplace::Unknown
        } else {
            Workplace::Onsite
        };
        let description = truncate_chars(
            &html_to_text(str_field(j, "description").unwrap_or("")),
            DESCRIPTION_CAP,
        );
        out.push(Posting {
            id: Posting::make_id(Source::Workable, board, code),
            source: Source::Workable,
            board: board.to_string(),
            company: company.clone(),
            title: title.trim().to_string(),
            location,
            workplace,
            department: str_field(j, "department").map(str::to_string),
            employment_type: str_field(j, "employment_type").map(str::to_string),
            url: str_field(j, "url")
                .or_else(|| str_field(j, "shortlink"))
                .unwrap_or("")
                .to_string(),
            apply_url: str_field(j, "application_url").map(str::to_string),
            posted: str_field(j, "published_on")
                .or_else(|| str_field(j, "created_at"))
                .and_then(iso_date),
            salary: super::salary_from_text(&description),
            description,
        });
    }
    Ok(out)
}
