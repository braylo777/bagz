//! Focused checks on public helpers: salary text, labels, metrics, HTTP
//! fixtures, ranking weights.

use bagz_engine::http::{FixtureClient, HttpClient};
use bagz_engine::ingest::salary_from_text;
use bagz_engine::metrics::{count_code_lines, jobs_indexed, parse_test_totals, rust_loc};
use bagz_engine::model::{Posting, Source, Workplace};
use bagz_engine::rank;
use bagz_engine::usage::new_install_id;
use bagz_engine::Profile;

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("bagz-units-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn salary_dollars_with_commas() {
    let s = salary_from_text("Range: $120,000 - $150,000 USD").unwrap();
    assert_eq!(
        (s.min, s.max, s.currency.as_str()),
        (120_000, 150_000, "USD")
    );
}

#[test]
fn salary_k_suffix_and_en_dash() {
    let s = salary_from_text("$130K – $290K • Offers Equity").unwrap();
    assert_eq!((s.min, s.max), (130_000, 290_000));
}

#[test]
fn salary_pounds_with_to() {
    let s = salary_from_text("£50k to £70k").unwrap();
    assert_eq!((s.min, s.max, s.currency.as_str()), (50_000, 70_000, "GBP"));
}

#[test]
fn salary_ignores_hourly_and_inverted_and_single_numbers() {
    assert!(salary_from_text("$22 - $28 per hour").is_none());
    assert!(salary_from_text("$200,000 - $100,000").is_none());
    assert!(salary_from_text("We raised $50,000,000.").is_none());
    assert!(salary_from_text("").is_none());
}

#[test]
fn salary_second_dollar_sign_optional() {
    let s = salary_from_text("$90,000-120,000").unwrap();
    assert_eq!((s.min, s.max), (90_000, 120_000));
}

#[test]
fn workplace_labels() {
    assert_eq!(Workplace::from_label("Remote - US"), Workplace::Remote);
    assert_eq!(Workplace::from_label("HYBRID"), Workplace::Hybrid);
    assert_eq!(Workplace::from_label("On-site"), Workplace::Onsite);
    assert_eq!(
        Workplace::from_label("Anywhere in the World"),
        Workplace::Remote
    );
    assert_eq!(Workplace::from_label("Remote or hybrid"), Workplace::Hybrid);
    assert_eq!(Workplace::from_label("Paris"), Workplace::Unknown);
}

#[test]
fn source_names_round_trip() {
    for s in Source::ALL {
        assert_eq!(s.as_str().parse::<Source>().unwrap(), s);
        assert_eq!(s.to_string(), s.as_str());
    }
    assert_eq!("GH".parse::<Source>().unwrap(), Source::Greenhouse);
    assert!("indeed".parse::<Source>().is_err());
}

#[test]
fn source_priority_prefers_ats_over_rss() {
    assert!(Source::Greenhouse.priority() < Source::Rss.priority());
    assert_eq!(Source::Lever.priority(), Source::Workable.priority());
}

#[test]
fn posting_ids_are_namespaced() {
    assert_eq!(Posting::make_id(Source::Ashby, "co", "1"), "ashby:co:1");
}

#[test]
fn posting_json_round_trip() {
    let p = Posting {
        id: "lever:x:1".into(),
        source: Source::Lever,
        board: "x".into(),
        company: "X".into(),
        title: "T".into(),
        location: "L".into(),
        workplace: Workplace::Remote,
        department: None,
        employment_type: None,
        url: "u".into(),
        apply_url: None,
        posted: Some("2026-01-01".into()),
        salary: None,
        description: String::new(),
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains(r#""source":"lever""#) && j.contains(r#""workplace":"remote""#));
    assert_eq!(serde_json::from_str::<Posting>(&j).unwrap(), p);
}

#[test]
fn rank_weights_sum_to_one() {
    let sum = rank::W_SKILLS + rank::W_TITLE + rank::W_LOCATION + rank::W_RECENCY + rank::W_SALARY;
    assert!((sum - 1.0).abs() < 1e-12);
}

#[test]
fn empty_profile_scores_zero_skills_and_title() {
    let p = Posting {
        id: "a".into(),
        source: Source::Lever,
        board: "b".into(),
        company: "c".into(),
        title: "Engineer".into(),
        location: String::new(),
        workplace: Workplace::Unknown,
        department: None,
        employment_type: None,
        url: String::new(),
        apply_url: None,
        posted: None,
        salary: None,
        description: "rust".into(),
    };
    let s = rank::score(&p, &Profile::default(), 0);
    assert_eq!((s.skills, s.title), (0.0, 0.0));
}

#[test]
fn profile_example_is_fictional_and_complete() {
    let p = Profile::example();
    assert!(p.email.ends_with("@example.com"));
    assert!(p.phone.contains("555"));
    assert_eq!(p.full_name(), "Avery Example");
    assert!(!p.skills.is_empty() && !p.experience.is_empty());
}

#[test]
fn profile_json_defaults_missing_fields() {
    let p: Profile = serde_json::from_str(r#"{"first_name":"Sam","skills":["Go"]}"#).unwrap();
    assert_eq!(p.full_name(), "Sam");
    assert!(!p.remote_ok);
    assert!(p.experience.is_empty());
}

#[test]
fn install_id_is_seeded_and_hex() {
    assert_eq!(new_install_id(5), new_install_id(5));
    assert_ne!(new_install_id(5), new_install_id(6));
}

#[test]
fn fixture_client_from_dir() {
    let d = tmp("fixtures");
    std::fs::write(d.join("index.json"), r#"{"https://x.test/a":"a.txt"}"#).unwrap();
    std::fs::write(d.join("a.txt"), "A").unwrap();
    let c = FixtureClient::from_dir(&d).unwrap();
    assert_eq!(c.get("https://x.test/a").unwrap(), "A");
    std::fs::write(
        d.join("index.json"),
        r#"{"https://x.test/b":"missing.txt"}"#,
    )
    .unwrap();
    assert!(FixtureClient::from_dir(&d).is_err());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn metrics_loc_skips_generated_and_target() {
    let d = tmp("loc");
    std::fs::create_dir_all(d.join("crates/a/src")).unwrap();
    std::fs::create_dir_all(d.join("crates/a/target")).unwrap();
    std::fs::write(
        d.join("crates/a/src/lib.rs"),
        "// c\nfn a() {}\n\nfn b() {}\n",
    )
    .unwrap();
    std::fs::write(d.join("crates/a/src/gen.rs"), "fn g() {}\n").unwrap();
    std::fs::write(d.join("crates/a/target/x.rs"), "fn t() {}\n").unwrap();
    std::fs::write(
        d.join(".gitattributes"),
        "crates/a/src/gen.rs linguist-generated=true\n",
    )
    .unwrap();
    assert_eq!(rust_loc(&d).unwrap(), 2);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn metrics_jobs_indexed_reads_manifest() {
    let d = tmp("jobs");
    std::fs::create_dir_all(d.join("data")).unwrap();
    std::fs::write(
        d.join("data/ingest-run.json"),
        r#"{"report":{"indexed":12345}}"#,
    )
    .unwrap();
    assert_eq!(jobs_indexed(&d).unwrap(), 12345);
    let _ = std::fs::remove_dir_all(&d);
    assert!(jobs_indexed(&d).is_err());
}

#[test]
fn metrics_test_totals_ignore_unrelated_lines() {
    let t = parse_test_totals(
        "test foo ... ok\nrunning 2 tests\ntest result: ok. 2 passed; 0 failed; 0 ignored;",
    );
    assert_eq!((t.passed, t.failed), (2, 0));
}

#[test]
fn metrics_code_lines_count_block_code() {
    assert_eq!(count_code_lines("/// doc\nfn a() {\n}\n"), 2);
}

#[test]
fn committed_metrics_json_matches_schema() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(s) = std::fs::read_to_string(root.join("metrics.json")) else {
        return; // not generated yet in a fresh checkout
    };
    let m: bagz_engine::metrics::Metrics = serde_json::from_str(&s).unwrap();
    assert_eq!(m.agents, bagz_engine::agents::REGISTRY.len());
    assert!(m.tests_passing <= m.tests);
}

#[test]
fn committed_sample_snapshot_parses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let s = std::fs::read_to_string(root.join("data/sample-snapshot.json")).unwrap();
    let sample: Vec<Posting> = serde_json::from_str(&s).unwrap();
    assert!(!sample.is_empty());
    assert!(sample.iter().all(|p| p.description.chars().count() <= 280));
    let ids: std::collections::BTreeSet<_> = sample.iter().map(|p| &p.id).collect();
    assert_eq!(ids.len(), sample.len());
}

#[test]
fn committed_run_manifest_meets_target() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let n = jobs_indexed(&root).unwrap();
    assert!(n >= 10_000, "recorded run indexed {n}");
}

#[test]
fn committed_board_list_parses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let boards = bagz_engine::ingest::parse_board_list(
        &std::fs::read_to_string(root.join("data/boards.txt")).unwrap(),
    )
    .unwrap();
    for s in Source::ALL {
        assert!(boards.iter().any(|b| b.source == s), "no {s} board");
    }
}
