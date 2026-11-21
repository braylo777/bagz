//! Lever Postings API: `GET /v0/postings/{company}?mode=json`.

use serde_json::Value;

use super::{company_from_slug, str_field};
use crate::error::{Error, Result};
use crate::model::{Posting, Salary, Source, Workplace};
use crate::text::{date_from_epoch_ms, html_to_text, truncate_chars, DESCRIPTION_CAP};

pub fn url(company: &str) -> String {
    format!("https://api.lever.co/v0/postings/{company}?mode=json")
}

pub fn parse(body: &str, board: &str) -> Result<Vec<Posting>> {
    let root: Value =
        serde_json::from_str(body).map_err(|e| Error::parse("lever", e.to_string()))?;
    let jobs = root
        .as_array()
        .ok_or_else(|| Error::parse("lever", "expected a JSON array"))?;
    let company = company_from_slug(board);
    let mut out = Vec::with_capacity(jobs.len());
    for j in jobs {
        let (Some(id), Some(title)) = (str_field(j, "id"), str_field(j, "text")) else {
            continue;
        };
        let cats = j.get("categories").cloned().unwrap_or(Value::Null);
        let location = str_field(&cats, "location").unwrap_or("").to_string();
        let workplace = match str_field(j, "workplaceType") {
            Some(w) => Workplace::from_label(w),
            None => Workplace::from_label(&location),
        };
        let description = str_field(j, "descriptionPlain")
            .map(crate::text::collapse_spaces)
            .or_else(|| str_field(j, "description").map(html_to_text))
            .unwrap_or_default();
        let salary = j.get("salaryRange").and_then(|s| {
            let min = s.get("min")?.as_u64()?;
            let max = s.get("max")?.as_u64()?;
            let currency = str_field(s, "currency").unwrap_or("USD").to_string();
            let yearly = str_field(s, "interval").is_none_or(|i| i.contains("year"));
            yearly.then_some(Salary { min, max, currency })
        });
        out.push(Posting {
            id: Posting::make_id(Source::Lever, board, id),
            source: Source::Lever,
            board: board.to_string(),
            company: company.clone(),
            title: title.trim().to_string(),
            location,
            workplace,
            department: str_field(&cats, "department")
                .or_else(|| str_field(&cats, "team"))
                .map(str::to_string),
            employment_type: str_field(&cats, "commitment").map(str::to_string),
            url: str_field(j, "hostedUrl").unwrap_or("").to_string(),
            apply_url: str_field(j, "applyUrl").map(str::to_string),
            posted: j
                .get("createdAt")
                .and_then(Value::as_i64)
                .map(date_from_epoch_ms),
            salary,
            description: truncate_chars(&description, DESCRIPTION_CAP),
        });
    }
    Ok(out)
}
