//! End-to-end ingestion over recorded HTTP. No test here can reach the
//! network: `FixtureClient` errors on any URL it did not record.

use std::path::PathBuf;
use std::process::Command;

use bagz_engine::http::FixtureClient;
use bagz_engine::ingest::{parse_board_list, run};
use bagz_engine::model::Source;
use bagz_engine::rank::rank;
use bagz_engine::text::days_from_civil;
use bagz_engine::Profile;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/http")
}

fn boards() -> Vec<bagz_engine::ingest::Board> {
    parse_board_list(&std::fs::read_to_string(fixtures().join("boards.txt")).unwrap()).unwrap()
}

fn client() -> FixtureClient {
    FixtureClient::from_dir(fixtures()).unwrap()
}

#[test]
fn index_maps_every_fixture_file() {
    assert_eq!(client().urls().count(), 5);
}

#[test]
fn run_counts_and_dedupes() {
    let (postings, report) = run(&client(), &boards());
    // 3 GH + 2 Lever + 2 Ashby + 2 Workable + 3 RSS = 12 parsed.
    assert_eq!(report.fetched, 12);
    // The RSS copy of the Acme backend job is a duplicate of the GH one.
    assert_eq!(report.duplicates_removed, 1);
    assert_eq!(report.indexed, 11);
    assert_eq!(postings.len(), 11);
    assert_eq!(report.by_source.get("rss"), Some(&2));
}

#[test]
fn duplicate_keeps_the_ats_copy() {
    let (postings, _) = run(&client(), &boards());
    let backend: Vec<_> = postings
        .iter()
        .filter(|p| p.title == "Senior Backend Engineer")
        .collect();
    assert_eq!(backend.len(), 1);
    assert_eq!(backend[0].source, Source::Greenhouse);
}

#[test]
fn unrecorded_board_fails_alone() {
    let (_, report) = run(&client(), &boards());
    let failed: Vec<_> = report.boards.iter().filter(|b| b.error.is_some()).collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].board, "greenhouse:not-recorded");
    assert!(failed[0]
        .error
        .as_deref()
        .unwrap()
        .contains("no recorded fixture"));
}

#[test]
fn every_source_is_represented() {
    let (postings, _) = run(&client(), &boards());
    for s in Source::ALL {
        assert!(postings.iter().any(|p| p.source == s), "missing {s}");
    }
}

#[test]
fn ids_are_unique_after_dedupe() {
    let (postings, _) = run(&client(), &boards());
    let ids: std::collections::BTreeSet<_> = postings.iter().map(|p| &p.id).collect();
    assert_eq!(ids.len(), postings.len());
}

#[test]
fn ranking_the_recorded_run_puts_backend_roles_first() {
    let (postings, _) = run(&client(), &boards());
    let ranked = rank(
        &postings,
        &Profile::example(),
        days_from_civil(2026, 11, 20),
    );
    let top = ranked[0].0;
    assert!(
        top.title.contains("Backend")
            || top.title.contains("Rust")
            || top.title.contains("Engineer"),
        "top was {}",
        top.title
    );
    assert!(ranked.windows(2).all(|w| w[0].1.total >= w[1].1.total));
}

#[test]
fn run_is_deterministic() {
    let a = run(&client(), &boards()).0;
    let b = run(&client(), &boards()).0;
    assert_eq!(a, b);
}

#[test]
fn cli_ingest_with_fixtures_writes_manifest_and_sample() {
    let out = std::env::temp_dir().join(format!("bagz-cli-{}", std::process::id()));
    let index = out.join("index.jsonl");
    let status = Command::new(env!("CARGO_BIN_EXE_bagz"))
        .env("BAGZ_HOME", out.join("home"))
        .args(["ingest", "--boards"])
        .arg(fixtures().join("boards.txt"))
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--out")
        .arg(&out)
        .arg("--index")
        .arg(&index)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("ingest-run.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["report"]["indexed"], 11);
    assert_eq!(manifest["index_sha256"].as_str().unwrap().len(), 64);
    let sample: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(out.join("sample-snapshot.json")).unwrap())
            .unwrap();
    assert_eq!(sample.len(), 11);
    assert_eq!(std::fs::read_to_string(&index).unwrap().lines().count(), 11);
    // Usage counting is off by default: no file appears.
    assert!(!out.join("home/usage.json").exists());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn cli_without_command_prints_usage() {
    let o = Command::new(env!("CARGO_BIN_EXE_bagz"))
        .env("BAGZ_HOME", std::env::temp_dir().join("bagz-none"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("usage: bagz"));
}
