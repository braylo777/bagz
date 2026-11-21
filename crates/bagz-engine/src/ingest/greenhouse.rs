//! Greenhouse Job Board API: `GET /v1/boards/{token}/jobs?content=true`.

use serde_json::Value;

use super::str_field;
use crate::error::{Error, Result};
use crate::model::{Posting, Source, Workplace};
use crate::text::{html_to_text, iso_date, truncate_chars, DESCRIPTION_CAP};

pub fn url(token: &str) -> String {
    format!("https://boards-api.greenhouse.io/v1/boards/{token}/jobs?content=true")
}

pub fn parse(body: &str, board: &str) -> Result<Vec<Posting>> {
    let root: Value =
        serde_json::from_str(body).map_err(|e| Error::parse("greenhouse", e.to_string()))?;
    let jobs = root
        .get("jobs")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::parse("greenhouse", "missing `jobs` array"))?;
    let mut out = Vec::with_capacity(jobs.len());
    for j in jobs {
        let Some(id) = j.get("id").and_then(|v| v.as_u64().map(|n| n.to_string())) else {
            continue;
        };
        let Some(title) = str_field(j, "title") else {
            continue;
        };
        let location = j
            .get("location")
            .and_then(|l| str_field(l, "name"))
            .unwrap_or("")
            .trim()
            .to_string();
        let description = truncate_chars(
            &html_to_text(str_field(j, "content").unwrap_or("")),
            DESCRIPTION_CAP,
        );
        let company = str_field(j, "company_name")
            .map(str::to_string)
            .unwrap_or_else(|| board.to_string());
        let department = j
            .get("departments")
            .and_then(Value::as_array)
            .and_then(|d| d.first())
            .and_then(|d| str_field(d, "name"))
            .map(str::to_string);
        let posted = str_field(j, "first_published")
            .or_else(|| str_field(j, "updated_at"))
            .and_then(iso_date);
        let url = str_field(j, "absolute_url").unwrap_or("").to_string();
        let mut workplace = Workplace::from_label(&location);
        if workplace == Workplace::Unknown && !location.is_empty() {
            workplace = Workplace::Onsite;
        }
        out.push(Posting {
            id: Posting::make_id(Source::Greenhouse, board, &id),
            source: Source::Greenhouse,
            board: board.to_string(),
            salary: super::salary_from_text(&description),
            company,
            title: title.trim().to_string(),
            location,
            workplace,
            department,
            employment_type: None,
            apply_url: None,
            url,
            posted,
            description,
        });
    }
    Ok(out)
}
