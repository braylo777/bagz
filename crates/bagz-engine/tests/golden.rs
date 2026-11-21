//! Per-ATS parser golden files. Each recorded response in
//! `tests/fixtures/http` is parsed and compared with `tests/golden/*.json`.
//! To regenerate after an intended change: `BAGZ_UPDATE_GOLDEN=1 cargo test`.

use std::fs;
use std::path::PathBuf;

use bagz_engine::ingest::{ashby, greenhouse, lever, rss, workable};
use bagz_engine::model::{Posting, Source, Workplace};

fn dir(sub: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(sub)
}

fn fixture(name: &str) -> String {
    fs::read_to_string(dir("fixtures/http").join(name)).unwrap()
}

fn golden(name: &str, postings: &[Posting]) {
    let path = dir("golden").join(name);
    let actual = serde_json::to_string_pretty(postings).unwrap() + "\n";
    if std::env::var_os("BAGZ_UPDATE_GOLDEN").is_some() {
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing {}; run with BAGZ_UPDATE_GOLDEN=1", path.display()));
    assert_eq!(actual, expected, "golden mismatch for {name}");
}

fn gh() -> Vec<Posting> {
    greenhouse::parse(&fixture("greenhouse-acme.json"), "acme").unwrap()
}
fn lv() -> Vec<Posting> {
    lever::parse(&fixture("lever-globex.json"), "globex").unwrap()
}
fn ab() -> Vec<Posting> {
    ashby::parse(&fixture("ashby-initech.json"), "initech").unwrap()
}
fn wk() -> Vec<Posting> {
    workable::parse(&fixture("workable-umbrella.json"), "umbrella").unwrap()
}
fn rs() -> Vec<Posting> {
    rss::parse(
        &fixture("rss-remote.xml"),
        "https://feeds.example.test/remote.rss",
    )
    .unwrap()
}

#[test]
fn greenhouse_golden() {
    golden("greenhouse.json", &gh());
}
#[test]
fn lever_golden() {
    golden("lever.json", &lv());
}
#[test]
fn ashby_golden() {
    golden("ashby.json", &ab());
}
#[test]
fn workable_golden() {
    golden("workable.json", &wk());
}
#[test]
fn rss_golden() {
    golden("rss.json", &rs());
}

// Greenhouse
#[test]
fn greenhouse_skips_rows_without_id_or_title() {
    assert_eq!(gh().len(), 3);
}
#[test]
fn greenhouse_decodes_escaped_html() {
    let d = &gh()[0].description;
    assert!(
        d.starts_with("Build Rust services on AWS & Kubernetes."),
        "{d}"
    );
    assert!(!d.contains('<') && !d.contains("&lt;"));
}
#[test]
fn greenhouse_reads_salary_from_prose() {
    let s = gh()[0].salary.clone().unwrap();
    assert_eq!(
        (s.min, s.max, s.currency.as_str()),
        (150_000, 190_000, "USD")
    );
}
#[test]
fn greenhouse_dates_ids_and_department() {
    let p = &gh()[0];
    assert_eq!(p.id, "greenhouse:acme:1001");
    assert_eq!(p.posted.as_deref(), Some("2026-11-10"));
    assert_eq!(p.department.as_deref(), Some("Engineering"));
    assert_eq!(gh()[1].posted.as_deref(), Some("2026-11-01"));
}
#[test]
fn greenhouse_workplace_and_company_fallback() {
    let all = gh();
    assert_eq!(all[0].workplace, Workplace::Onsite);
    assert_eq!(all[1].workplace, Workplace::Remote);
    assert_eq!(all[2].company, "acme");
}
#[test]
fn greenhouse_rejects_non_board_json() {
    assert!(greenhouse::parse("{}", "x").is_err());
    assert!(greenhouse::parse("not json", "x").is_err());
    assert!(greenhouse::parse(r#"{"jobs":[]}"#, "x").unwrap().is_empty());
}

// Lever
#[test]
fn lever_skips_rows_without_text() {
    assert_eq!(lv().len(), 2);
}
#[test]
fn lever_uses_plain_description_and_collapses_space() {
    assert_eq!(
        lv()[0].description,
        "Globex builds data pipelines in Python and Kafka. You will run Spark jobs."
    );
    assert_eq!(lv()[1].description, "Help customers & partners.");
}
#[test]
fn lever_salary_yearly_only() {
    let all = lv();
    assert_eq!(all[0].salary.as_ref().unwrap().max, 165_000);
    assert!(
        all[1].salary.is_none(),
        "hourly ranges are not yearly salary"
    );
}
#[test]
fn lever_fields() {
    let p = &lv()[0];
    assert_eq!(p.company, "Globex");
    assert_eq!(p.workplace, Workplace::Hybrid);
    assert_eq!(p.posted.as_deref(), Some("2025-11-12"));
    assert_eq!(p.employment_type.as_deref(), Some("Full-time"));
    assert!(p.apply_url.as_deref().unwrap().ends_with("/apply"));
    assert_eq!(lv()[1].department.as_deref(), Some("Support"));
}
#[test]
fn lever_rejects_object_root() {
    assert!(lever::parse("{}", "x").is_err());
}

// Ashby
#[test]
fn ashby_skips_unlisted() {
    let all = ab();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|p| p.title != "Hidden Role"));
}
#[test]
fn ashby_takes_widest_salary_tier() {
    let s = ab()[0].salary.clone().unwrap();
    assert_eq!((s.min, s.max), (160_000, 210_000));
}
#[test]
fn ashby_workplace_prefers_explicit_type() {
    let all = ab();
    assert_eq!(all[0].workplace, Workplace::Hybrid);
    assert_eq!(all[1].workplace, Workplace::Remote);
    assert_eq!(all[1].description, "Hire great people.");
}
#[test]
fn ashby_company_from_slug() {
    assert_eq!(ab()[0].company, "Initech");
    assert_eq!(
        bagz_engine::ingest::company_from_slug("open-ai_labs"),
        "Open Ai Labs"
    );
}

// Workable
#[test]
fn workable_company_from_account_name() {
    assert!(wk().iter().all(|p| p.company == "Umbrella Health"));
}
#[test]
fn workable_location_and_remote() {
    let all = wk();
    assert_eq!(all[0].location, "Paris, Île-de-France, France");
    assert_eq!(all[0].workplace, Workplace::Remote);
    assert_eq!(all[1].location, "Athens, Greece");
    assert_eq!(all[1].workplace, Workplace::Onsite);
}
#[test]
fn workable_date_fallback_and_euro_salary() {
    let all = wk();
    assert_eq!(all[1].posted.as_deref(), Some("2026-10-01"));
    let s = all[0].salary.clone().unwrap();
    assert_eq!((s.min, s.max, s.currency.as_str()), (60_000, 80_000, "EUR"));
}

// RSS
#[test]
fn rss_splits_company_from_title() {
    let all = rs();
    assert_eq!(all[0].company, "Acme Robotics");
    assert_eq!(all[0].title, "Senior Backend Engineer");
}
#[test]
fn rss_uses_creator_when_present_and_skips_empty_titles() {
    let all = rs();
    assert_eq!(all.len(), 3);
    assert_eq!(all[2].company, "Vandelay Industries");
    assert_eq!(all[2].title, "Plain title without company");
}
#[test]
fn rss_reads_cdata_entities_dates_and_pay() {
    let all = rs();
    assert!(all[0]
        .description
        .starts_with("Same job as the Greenhouse listing"));
    assert_eq!(all[1].description, "Write Rust & Go. Pay $180K - $220K.");
    assert_eq!(all[1].posted.as_deref(), Some("2026-11-19"));
    assert_eq!(all[1].salary.as_ref().unwrap().min, 180_000);
    assert_eq!(all[1].workplace, Workplace::Remote);
    assert_eq!(all[1].source, Source::Rss);
    assert_eq!(all[1].board, "feeds.example.test");
}
#[test]
fn rss_rejects_non_rss() {
    assert!(rss::parse("<html><body/></html>", "https://x.test").is_err());
    assert!(rss::parse("<rss><channel><item><title>x</rss>", "https://x.test").is_err());
}
#[test]
fn rss_host_helper() {
    assert_eq!(
        rss::host("https://www.example.test/a/b.rss?x=1"),
        "example.test"
    );
    assert_eq!(rss::host("example.test/feed"), "example.test");
}
