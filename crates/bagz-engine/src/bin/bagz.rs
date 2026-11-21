//! BAGZ command line.
//!
//! ```text
//! bagz ingest [--boards data/boards.txt] [--out data] [--fixtures DIR]
//! bagz top    [--index target/bagz/index.jsonl] [--profile profile.json] [-n 20]
//! bagz usage  status|enable|disable|export
//! bagz docx-check FILE.docx
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use bagz_engine::http::{FixtureClient, HttpClient, LiveClient};
use bagz_engine::ingest::{parse_board_list, run};
use bagz_engine::rank::rank;
use bagz_engine::text::{today_days, today_iso, truncate_chars};
use bagz_engine::usage::UsageStore;
use bagz_engine::{Posting, Profile};
use sha2::{Digest, Sha256};

/// Postings kept in the committed sample snapshot.
const SAMPLE_SIZE: usize = 250;

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn die(msg: impl std::fmt::Display) -> ! {
    eprintln!("bagz: {msg}");
    std::process::exit(2)
}

fn ingest(args: &[String]) {
    let boards_path = arg(args, "--boards").unwrap_or_else(|| "data/boards.txt".into());
    let out = PathBuf::from(arg(args, "--out").unwrap_or_else(|| "data".into()));
    let index_path =
        PathBuf::from(arg(args, "--index").unwrap_or_else(|| "target/bagz/index.jsonl".into()));
    let boards = parse_board_list(&fs::read_to_string(&boards_path).unwrap_or_else(|e| die(e)))
        .unwrap_or_else(|e| die(e));
    let client: Box<dyn HttpClient> = match arg(args, "--fixtures") {
        Some(dir) => Box::new(FixtureClient::from_dir(dir).unwrap_or_else(|e| die(e))),
        None => Box::new(LiveClient::new()),
    };
    eprintln!("bagz: reading {} boards", boards.len());
    let started = std::time::Instant::now();
    let (postings, report) = run(client.as_ref(), &boards);
    let secs = started.elapsed().as_secs_f64();

    // Full index (local, gitignored): one JSON posting per line.
    if let Some(dir) = index_path.parent() {
        fs::create_dir_all(dir).unwrap_or_else(|e| die(e));
    }
    let mut lines = String::new();
    for p in &postings {
        lines.push_str(&serde_json::to_string(p).expect("posting json"));
        lines.push('\n');
    }
    fs::write(&index_path, &lines).unwrap_or_else(|e| die(e));
    let sha = Sha256::digest(lines.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    // Committed sample: every Nth posting, description cut to 280 chars.
    let step = (postings.len() / SAMPLE_SIZE).max(1);
    let sample: Vec<Posting> = postings
        .iter()
        .step_by(step)
        .take(SAMPLE_SIZE)
        .cloned()
        .map(|mut p| {
            p.description = truncate_chars(&p.description, 280);
            p
        })
        .collect();
    fs::create_dir_all(&out).unwrap_or_else(|e| die(e));
    let manifest = serde_json::json!({
        "run_date": today_iso(),
        "boards_file": boards_path,
        "seconds": (secs * 10.0).round() / 10.0,
        "index_sha256": sha,
        "sample_size": sample.len(),
        "report": report,
    });
    fs::write(
        out.join("ingest-run.json"),
        serde_json::to_string_pretty(&manifest).expect("manifest json") + "\n",
    )
    .unwrap_or_else(|e| die(e));
    fs::write(
        out.join("sample-snapshot.json"),
        serde_json::to_string_pretty(&sample).expect("sample json") + "\n",
    )
    .unwrap_or_else(|e| die(e));
    let failed = report.boards.iter().filter(|b| b.error.is_some()).count();
    println!(
        "fetched {} postings, indexed {} after dedupe ({} duplicates), {} boards failed, {:.1}s",
        report.fetched, report.indexed, report.duplicates_removed, failed, secs
    );
    for (src, n) in &report.by_source {
        println!("  {src:<10} {n}");
    }
    println!("index: {} (sha256 {sha})", index_path.display());
}

fn top(args: &[String]) {
    let index = arg(args, "--index").unwrap_or_else(|| "target/bagz/index.jsonl".into());
    let n: usize = arg(args, "-n").and_then(|s| s.parse().ok()).unwrap_or(20);
    let profile = match arg(args, "--profile") {
        Some(p) => serde_json::from_str(&fs::read_to_string(p).unwrap_or_else(|e| die(e)))
            .unwrap_or_else(|e| die(e)),
        None => Profile::example(),
    };
    let postings: Vec<Posting> = fs::read_to_string(&index)
        .unwrap_or_else(|e| die(format!("{index}: {e} (run `bagz ingest` first)")))
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    for (p, s) in rank(&postings, &profile, today_days()).into_iter().take(n) {
        println!(
            "{:5.1}  {} | {} | {} | {}",
            s.total, p.company, p.title, p.location, p.url
        );
    }
}

fn usage(args: &[String]) {
    let store = UsageStore::default_location();
    match args.first().map(String::as_str).unwrap_or("status") {
        "enable" => {
            let s = store.enable().unwrap_or_else(|e| die(e));
            println!(
                "usage counting is ON (id {}). Stored only in {}. Nothing is sent.",
                s.install_id.unwrap_or_default(),
                store.path().display()
            );
        }
        "disable" => {
            store.disable().unwrap_or_else(|e| die(e));
            println!("usage counting is OFF; local data deleted.");
        }
        "export" => match store.load().unwrap_or_else(|e| die(e)).export() {
            Some(r) => println!("{}", serde_json::to_string_pretty(&r).expect("json")),
            None => println!("usage counting is off; nothing to export."),
        },
        _ => {
            let s = store.load().unwrap_or_else(|e| die(e));
            println!(
                "usage counting: {} ({} runs over {} months) at {}",
                if s.enabled { "on" } else { "off (default)" },
                s.runs,
                s.months.len(),
                store.path().display()
            );
        }
    }
}

fn docx_check(args: &[String]) {
    let path = args
        .first()
        .unwrap_or_else(|| die("docx-check needs a file"));
    let bytes = fs::read(Path::new(path)).unwrap_or_else(|e| die(e));
    let issues = bagz_engine::docx::ats_violations(&bytes).unwrap_or_else(|e| die(e));
    let text = bagz_engine::docx::extract_text(&bytes).unwrap_or_else(|e| die(e));
    for line in &text {
        println!("{line}");
    }
    if issues.is_empty() {
        eprintln!("ATS check: ok ({} paragraphs)", text.len());
    } else {
        die(format!("ATS check failed: {}", issues.join(", ")));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let _ = UsageStore::default_location().record(&today_iso());
    match args.first().map(String::as_str) {
        Some("ingest") => ingest(&args[1..]),
        Some("top") => top(&args[1..]),
        Some("usage") => usage(&args[1..]),
        Some("docx-check") => docx_check(&args[1..]),
        _ => {
            eprintln!("usage: bagz ingest|top|usage|docx-check  (see README)");
            std::process::exit(2);
        }
    }
}
