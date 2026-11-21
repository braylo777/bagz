//! Helpers behind `cargo run --bin metrics`. Each number in `metrics.json`
//! is computed from the repo or from a test run, never typed in.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    pub jobs_indexed: usize,
    pub agents: usize,
    pub tests: usize,
    pub tests_passing: usize,
    pub loc_rust: usize,
    /// Line coverage in percent, or `null` when `cargo llvm-cov` was not run.
    pub coverage: Option<f64>,
}

/// Counts from `test result:` lines in `cargo test` output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TestTotals {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

fn number_before(line: &str, word: &str) -> usize {
    line.split(';')
        .find_map(|part| {
            let part = part
                .trim()
                .trim_start_matches("test result: ok.")
                .trim_start_matches("test result: FAILED.");
            let mut it = part.split_whitespace();
            let n = it.next()?.parse::<usize>().ok()?;
            (it.next()? == word).then_some(n)
        })
        .unwrap_or(0)
}

pub fn parse_test_totals(output: &str) -> TestTotals {
    let mut t = TestTotals::default();
    for line in output
        .lines()
        .filter(|l| l.trim_start().starts_with("test result:"))
    {
        t.passed += number_before(line, "passed");
        t.failed += number_before(line, "failed");
        t.ignored += number_before(line, "ignored");
    }
    t
}

/// Line percent from `cargo llvm-cov --json --summary-only` output.
pub fn parse_llvm_cov(json: &str) -> Option<f64> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let pct = v
        .get("data")?
        .get(0)?
        .get("totals")?
        .get("lines")?
        .get("percent")?
        .as_f64()?;
    Some((pct * 10.0).round() / 10.0)
}

/// Non-blank lines that are not only a `//` comment.
pub fn count_code_lines(src: &str) -> usize {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .count()
}

/// Rust lines of code under `root/crates`, skipping `target/` and files
/// marked `linguist-generated` in `.gitattributes`.
pub fn rust_loc(root: &Path) -> Result<usize> {
    let generated: Vec<String> = fs::read_to_string(root.join(".gitattributes"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.contains("linguist-generated"))
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();
    let mut total = 0;
    let mut stack = vec![root.join("crates")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if name.ends_with(".rs") {
                let rel = path
                    .strip_prefix(root)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                if generated.iter().any(|g| g == &rel) {
                    continue;
                }
                total += count_code_lines(&fs::read_to_string(&path)?);
            }
        }
    }
    Ok(total)
}

/// `indexed` from the newest recorded ingest run in `data/`.
pub fn jobs_indexed(root: &Path) -> Result<usize> {
    let s = fs::read_to_string(root.join("data/ingest-run.json"))?;
    let v: serde_json::Value = serde_json::from_str(&s)?;
    Ok(v.get("report")
        .and_then(|r| r.get("indexed"))
        .and_then(|n| n.as_u64())
        .unwrap_or(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_sum_all_result_lines() {
        let out = "running 3 tests\n\
            test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\
            test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out\n";
        assert_eq!(
            parse_test_totals(out),
            TestTotals {
                passed: 8,
                failed: 2,
                ignored: 1
            }
        );
        assert_eq!(parse_test_totals("nothing"), TestTotals::default());
    }

    #[test]
    fn llvm_cov_percent() {
        let j = r#"{"data":[{"totals":{"lines":{"count":10,"covered":8,"percent":81.234}}}]}"#;
        assert_eq!(parse_llvm_cov(j), Some(81.2));
        assert_eq!(parse_llvm_cov("{}"), None);
    }

    #[test]
    fn code_lines_skip_blanks_and_comments() {
        assert_eq!(
            count_code_lines("// a\n\nfn x() {}\n  // b\nlet y = 1; // c\n"),
            2
        );
    }
}
