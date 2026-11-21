# Changelog

All notable changes. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] - 2026-11-20

### Added
- `bagz-engine` crate.
- Ingestion from the public Greenhouse, Lever, Ashby and Workable job-board APIs and from RSS 2.0 feeds, normalized into one `Posting` model.
- Cross-source dedupe (normalized company + title + location, or canonical URL) and deterministic ranking against a profile.
- Recorded live run: 59 boards, 18,088 postings fetched, 14,228 indexed (`data/ingest-run.json`, sample in `data/sample-snapshot.json`).
- Six agents behind a typed approval gate (`Pending<T>` -> `Approved<T>`): scorer, résumé tailor, cover-letter writer, form pre-filler, receipt tracker, follow-up drafter. Nothing submits or sends.
- `Llm` trait with a deterministic `MockLlm` and an `AnthropicLlm` backend.
- Guards that refuse model drafts with placeholders or with skills the profile does not list.
- ATS-safe `.docx` writer with no dependencies, plus structure and text checks.
- Opt-in, local-first, anonymous usage counter (off by default; nothing is sent).
- `bagz` CLI (`ingest`, `top`, `usage`, `docx-check`), `metrics` binary and `metrics.json`, `cargo run --example demo`.
- Golden-file parser tests, recorded-HTTP fixtures, property tests; CI coverage job with `cargo-llvm-cov`.

### Fixed
- Large boards (over 10 MB of JSON) no longer fail: the live client streams the body.

## [0.1.0]

- Rust core (`bagz-core`), PyO3 bindings (`bagz-py`) and the embedded web demo (`bagz-web`) with 12 example jobs.
