//! Ashby Posting API: `GET /posting-api/job-board/{name}?includeCompensation=true`.

use serde_json::Value;

use super::{company_from_slug, str_field};
use crate::error::{Error, Result};
use crate::model::{Posting, Salary, Source, Workplace};
use crate::text::{collapse_spaces, html_to_text, iso_date, truncate_chars, DESCRIPTION_CAP};

pub fn url(name: &str) -> String {
    format!("https://api.ashbyhq.com/posting-api/job-board/{name}?includeCompensation=true")
}

fn salary(job: &Value) -> Option<Salary> {
    let tiers = job
        .get("compensation")?
        .get("compensationTiers")?
        .as_array()?;
    let mut best: Option<Salary> = None;
    for tier in tiers {
        for c in tier.get("components")?.as_array()? {
            if str_field(c, "compensationType") != Some("Salary") {
                continue;
            }
            if !str_field(c, "interval").is_some_and(|i| i.contains("YEAR")) {
                continue;
            }
            let (Some(min), Some(max)) = (
                c.get("minValue").and_then(Value::as_f64),
                c.get("maxValue").and_then(Value::as_f64),
            ) else {
                continue;
            };
            let currency = str_field(c, "currencyCode").unwrap_or("USD").to_string();
            let s = Salary {
                min: min as u64,
                max: max as u64,
                currency,
            };
            best = Some(match best {
                Some(b) if b.max >= s.max => b,
                _ => s,
            });
        }
    }
    best
}

pub fn parse(body: &str, board: &str) -> Result<Vec<Posting>> {
    let root: Value =
        serde_json::from_str(body).map_err(|e| Error::parse("ashby", e.to_string()))?;
    let jobs = root
        .get("jobs")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::parse("ashby", "missing `jobs` array"))?;
    let company = company_from_slug(board);
    let mut out = Vec::with_capacity(jobs.len());
    for j in jobs {
        if j.get("isListed").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let (Some(id), Some(title)) = (str_field(j, "id"), str_field(j, "title")) else {
            continue;
        };
        let location = str_field(j, "location").unwrap_or("").to_string();
        let workplace = match str_field(j, "workplaceType") {
            Some(w) => Workplace::from_label(w),
            None if j.get("isRemote").and_then(Value::as_bool) == Some(true) => Workplace::Remote,
            None => Workplace::from_label(&location),
        };
        let description = str_field(j, "descriptionPlain")
            .map(collapse_spaces)
            .or_else(|| str_field(j, "descriptionHtml").map(html_to_text))
            .unwrap_or_default();
        out.push(Posting {
            id: Posting::make_id(Source::Ashby, board, id),
            source: Source::Ashby,
            board: board.to_string(),
            company: company.clone(),
            title: title.trim().to_string(),
            location,
            workplace,
            department: str_field(j, "department").map(str::to_string),
            employment_type: str_field(j, "employmentType").map(str::to_string),
            url: str_field(j, "jobUrl").unwrap_or("").to_string(),
            apply_url: str_field(j, "applyUrl").map(str::to_string),
            posted: str_field(j, "publishedAt").and_then(iso_date),
            salary: salary(j),
            description: truncate_chars(&description, DESCRIPTION_CAP),
        });
    }
    Ok(out)
}
