//! ATS-safe .docx export and the opt-in usage counter.

use bagz_engine::agents::cover_letter::CoverLetter;
use bagz_engine::agents::tailor::Resume;
use bagz_engine::approval::Pending;
use bagz_engine::docx::{self, Para, Style};
use bagz_engine::usage::{monthly_active, UsageReport, UsageState, UsageStore, DAY_WINDOW};
use bagz_engine::Profile;

fn resume() -> Resume {
    Resume::from_profile(&Profile::example())
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("bagz-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---------- docx ----------

#[test]
fn resume_docx_has_required_parts() {
    let names: Vec<String> = docx::read_entries(&docx::resume_bytes(&resume()))
        .unwrap()
        .into_iter()
        .map(|e| e.0)
        .collect();
    assert_eq!(
        names,
        vec![
            "[Content_Types].xml",
            "_rels/.rels",
            "docProps/core.xml",
            "word/document.xml"
        ]
    );
}

#[test]
fn resume_docx_passes_ats_structure_checks() {
    assert!(docx::ats_violations(&docx::resume_bytes(&resume()))
        .unwrap()
        .is_empty());
}

#[test]
fn ats_check_catches_tables_and_columns() {
    let mut z = docx::ZipWriter::default();
    z.add(
        "word/document.xml",
        br#"<w:document><w:body><w:tbl></w:tbl><w:cols w:num="2"/></w:body></w:document>"#,
    );
    z.add("word/header1.xml", b"<w:hdr/>");
    let v = docx::ats_violations(&z.finish()).unwrap();
    assert!(
        v.contains(&"table")
            && v.contains(&"multiple columns")
            && v.contains(&"header or footer part")
    );
}

#[test]
fn resume_text_reads_in_order_with_standard_headings() {
    let text = docx::extract_text(&docx::resume_bytes(&resume())).unwrap();
    assert_eq!(text[0], "Avery Example");
    let pos = |h: &str| text.iter().position(|t| t == h).unwrap();
    assert!(pos("Summary") < pos("Skills"));
    assert!(pos("Skills") < pos("Experience"));
    assert!(pos("Experience") < pos("Education"));
    assert!(text
        .iter()
        .any(|t| t.starts_with("• Built a Rust ingestion service")));
    assert!(text
        .iter()
        .any(|t| t == "Rust, Python, PostgreSQL, Kafka, AWS, Kubernetes"));
}

#[test]
fn every_resume_line_survives_extraction() {
    let r = resume();
    let text = docx::extract_text(&docx::resume_bytes(&r))
        .unwrap()
        .join("\n");
    for e in &r.experience {
        for b in &e.bullets {
            assert!(text.contains(b.as_str()), "lost bullet {b}");
        }
    }
    assert!(text.contains("avery@example.com"));
}

#[test]
fn docx_escapes_special_characters() {
    let paras = vec![Para::new(Style::Normal, "R&D <lead> \"quoted\" O'Brien")];
    let text = docx::extract_text(&docx::build(&paras, "t", "a")).unwrap();
    assert_eq!(text, vec!["R&D <lead> \"quoted\" O'Brien"]);
}

#[test]
fn docx_output_is_byte_stable() {
    assert_eq!(docx::resume_bytes(&resume()), docx::resume_bytes(&resume()));
}

#[test]
fn empty_sections_are_left_out() {
    let mut r = resume();
    r.summary.clear();
    r.education.clear();
    let text = docx::extract_text(&docx::resume_bytes(&r)).unwrap();
    assert!(!text.iter().any(|t| t == "Summary" || t == "Education"));
}

#[test]
fn export_requires_approved_values_and_writes_files() {
    let dir = tmp("docx");
    let approved = Pending::new("resume_tailor", "", resume())
        .approve("Avery")
        .unwrap();
    let path = dir.join("resume.docx");
    docx::export_resume(&approved, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..2], b"PK");
    let letter = CoverLetter {
        posting_id: "x".into(),
        company: "Acme".into(),
        role: "Engineer".into(),
        body: "Dear Acme,\n\nFirst paragraph.\n\nSincerely,\nAvery".into(),
    };
    let ok = Pending::new("cover_letter", "", letter)
        .approve("Avery")
        .unwrap();
    let lp = dir.join("letter.docx");
    docx::export_letter(&ok, &Profile::example(), &lp).unwrap();
    let text = docx::extract_text(&std::fs::read(&lp).unwrap()).unwrap();
    assert_eq!(text[2], "Dear Acme,");
    assert_eq!(text.len(), 5);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_docx_check_reads_back_text() {
    let dir = tmp("cli-docx");
    let path = dir.join("r.docx");
    std::fs::write(&path, docx::resume_bytes(&resume())).unwrap();
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_bagz"))
        .env("BAGZ_HOME", dir.join("home"))
        .arg("docx-check")
        .arg(&path)
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("Avery Example"));
    assert!(String::from_utf8_lossy(&o.stderr).contains("ATS check: ok"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------- usage ----------

#[test]
fn usage_is_off_by_default_and_records_nothing() {
    let mut s = UsageState::default();
    assert!(!s.enabled);
    s.record("2026-11-20");
    assert_eq!(s, UsageState::default());
    assert!(s.export().is_none());
}

#[test]
fn usage_enable_sets_anonymous_id() {
    let mut s = UsageState::default();
    s.enable(42);
    let id = s.install_id.clone().unwrap();
    assert_eq!(id.len(), 16);
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    s.enable(43);
    assert_eq!(s.install_id.unwrap(), id, "re-enable keeps the id");
}

#[test]
fn usage_records_months_and_days() {
    let mut s = UsageState::default();
    s.enable(1);
    for d in ["2026-10-31", "2026-11-01", "2026-11-01"] {
        s.record(d);
    }
    assert_eq!(s.runs, 3);
    assert_eq!(s.months.len(), 2);
    assert_eq!(s.days.len(), 2);
    let r = s.export().unwrap();
    assert_eq!(r.months, vec!["2026-10", "2026-11"]);
}

#[test]
fn usage_day_window_rolls_off_old_days() {
    let mut s = UsageState::default();
    s.enable(1);
    for i in 0..100 {
        let (y, m, d) = bagz_engine::text::civil_from_days(20_000 + i);
        s.record(&format!("{y:04}-{m:02}-{d:02}"));
    }
    assert_eq!(s.days.len(), DAY_WINDOW);
    assert_eq!(s.runs, 100);
}

#[test]
fn usage_disable_wipes_everything() {
    let mut s = UsageState::default();
    s.enable(1);
    s.record("2026-11-20");
    s.disable();
    assert_eq!(s, UsageState::default());
}

#[test]
fn usage_store_does_not_create_file_while_off() {
    let dir = tmp("usage-off");
    let store = UsageStore::at(&dir);
    store.record("2026-11-20").unwrap();
    assert!(!store.path().exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn usage_store_round_trip_and_disable_removes_file() {
    let dir = tmp("usage-on");
    let store = UsageStore::at(&dir);
    store.enable().unwrap();
    store.record("2026-11-20").unwrap();
    store.record("2026-11-21").unwrap();
    let s = store.load().unwrap();
    assert!(s.enabled);
    assert_eq!(s.runs, 2);
    store.disable().unwrap();
    assert!(!store.path().exists());
    store.disable().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn monthly_active_counts_distinct_installs() {
    let r = |id: &str, months: &[&str]| UsageReport {
        install_id: id.into(),
        months: months.iter().map(|m| m.to_string()).collect(),
        runs: 1,
        version: "x".into(),
    };
    let reports = vec![
        r("a", &["2026-10", "2026-11"]),
        r("b", &["2026-11"]),
        r("a", &["2026-11"]),
        r("c", &["2026-09"]),
    ];
    assert_eq!(monthly_active(&reports, "2026-11"), 2);
    assert_eq!(monthly_active(&reports, "2026-09"), 1);
    assert_eq!(monthly_active(&reports, "2026-01"), 0);
}

#[test]
fn cli_usage_enable_status_export_disable() {
    let dir = tmp("cli-usage");
    let run = |args: &[&str]| {
        let o = std::process::Command::new(env!("CARGO_BIN_EXE_bagz"))
            .env("BAGZ_HOME", &dir)
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success());
        String::from_utf8_lossy(&o.stdout).to_string()
    };
    assert!(run(&["usage", "status"]).contains("off (default)"));
    assert!(run(&["usage", "export"]).contains("nothing to export"));
    assert!(run(&["usage", "enable"]).contains("Nothing is sent"));
    assert!(run(&["usage", "export"]).contains("install_id"));
    assert!(run(&["usage", "disable"]).contains("deleted"));
    assert!(!dir.join("usage.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
