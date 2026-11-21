//! Writes `metrics.json` at the repo root.
//!
//! ```text
//! cargo run --bin metrics              # runs cargo llvm-cov if installed, else cargo test
//! cargo run --bin metrics -- --no-cov  # cargo test only, coverage = null
//! ```
//!
//! Every value is measured: tests come from a fresh test run, coverage from
//! `cargo llvm-cov`, LOC from the source tree, agents from the registry,
//! and jobs from `data/ingest-run.json` (written by `bagz ingest`).

use std::path::{Path, PathBuf};
use std::process::Command;

use bagz_engine::agents::REGISTRY;
use bagz_engine::metrics::{jobs_indexed, parse_llvm_cov, parse_test_totals, rust_loc, Metrics};

fn repo_root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.ancestors().nth(2).unwrap_or(here).to_path_buf()
}

fn run(cmd: &mut Command) -> Option<String> {
    let out = cmd.output().ok()?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Some(s)
}

fn has_llvm_cov() -> bool {
    Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn main() {
    let root = repo_root();
    let no_cov = std::env::args().any(|a| a == "--no-cov");
    // A separate target dir keeps the nested cargo from waiting on the
    // lock that `cargo run` holds.
    let target = root.join("target").join("metrics");
    let (output, coverage) = if !no_cov && has_llvm_cov() {
        let cov_path = target.join("cov.json");
        let out = run(Command::new("cargo")
            .current_dir(&root)
            .env("CARGO_TARGET_DIR", &target)
            .args([
                "llvm-cov",
                "--workspace",
                "--json",
                "--summary-only",
                "--output-path",
            ])
            .arg(&cov_path))
        .unwrap_or_default();
        let cov = std::fs::read_to_string(&cov_path)
            .ok()
            .and_then(|j| parse_llvm_cov(&j));
        (out, cov)
    } else {
        let out = run(Command::new("cargo")
            .current_dir(&root)
            .env("CARGO_TARGET_DIR", &target)
            .args(["test", "--workspace", "--no-fail-fast"]))
        .unwrap_or_default();
        (out, None)
    };
    let totals = parse_test_totals(&output);
    let m = Metrics {
        jobs_indexed: jobs_indexed(&root).unwrap_or(0),
        agents: REGISTRY.len(),
        tests: totals.passed + totals.failed,
        tests_passing: totals.passed,
        loc_rust: rust_loc(&root).unwrap_or(0),
        coverage,
    };
    let json = serde_json::to_string_pretty(&m).expect("metrics serialize");
    std::fs::write(root.join("metrics.json"), format!("{json}\n")).expect("write metrics.json");
    println!("{json}");
    if totals.failed > 0 {
        std::process::exit(1);
    }
}
