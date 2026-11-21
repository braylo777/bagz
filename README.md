# BAGZ

BAGZ is a swipe-to-apply job app. This repo holds two parts:

1. **`bagz-engine`** (new in 0.2.0): real job ingestion, six approval-gated agents, and ATS-safe `.docx` export.
2. **The clickable demo**: 12 example jobs, five screens, served from Rust (`bagz-web`), with Python bindings (`bagz-py`).

Tracked source is Rust and Python only. No npm.

## Numbers you can rerun

`metrics.json` holds the current figures. Every value is measured by `cargo run --bin metrics`:

| Field | Source |
| --- | --- |
| `jobs_indexed` | `report.indexed` in `data/ingest-run.json`, written by the last live `bagz ingest` run |
| `agents` | length of `bagz_engine::agents::REGISTRY` |
| `tests`, `tests_passing` | a fresh `cargo llvm-cov --workspace` (or `cargo test`) run |
| `loc_rust` | non-blank, non-comment `.rs` lines under `crates/`, generated files excluded |
| `coverage` | line coverage from `cargo llvm-cov`; `null` with `--no-cov` |

Recorded run on 2026-11-20: 59 public boards, 18,088 postings fetched, **14,228 indexed** after dedupe (3,860 duplicates removed), 0 boards failed, 38 s.

## Engine quick start

```bash
cargo run --example demo                       # offline: fixtures + mock model, all six agents
cargo run --bin bagz -- ingest                 # live: reads data/boards.txt, writes data/ and target/bagz/index.jsonl
cargo run --bin bagz -- top -n 20              # rank the local index for the example profile
cargo run --bin bagz -- top --profile me.json  # rank for your own profile (see Profile in src/profile.rs)
cargo run --bin bagz -- docx-check resume.docx # print the text an ATS sees, and check the structure
cargo run --bin metrics                        # regenerate metrics.json
```

### Ingestion

| Source | Endpoint (public, no key) |
| --- | --- |
| Greenhouse | `boards-api.greenhouse.io/v1/boards/{token}/jobs?content=true` |
| Lever | `api.lever.co/v0/postings/{company}?mode=json` |
| Ashby | `api.ashbyhq.com/posting-api/job-board/{name}?includeCompensation=true` |
| Workable | `apply.workable.com/api/v1/widget/accounts/{subdomain}?details=true` |
| RSS 2.0 | any feed URL (`Company: Role` titles are split) |

Each posting becomes one `Posting` (`src/model.rs`): HTML removed, dates as `YYYY-MM-DD`, yearly salary when the feed states one.

- **Dedupe** (`src/dedupe.rs`): same company + title + first location after normalization (`Acme Inc.` = `Acme`, `Sr.` = `Senior`), or the same canonical URL. The direct ATS copy wins over RSS, then the longer description, then the smaller id. The result does not depend on input order.
- **Rank** (`src/rank.rs`): skills 40 %, title 25 %, location 15 %, recency 10 %, salary 10 %. Pure and deterministic.
- A failing board is logged in the run report and does not stop the run.
- `data/boards.txt` is the board list. `data/sample-snapshot.json` keeps 250 postings from the run (descriptions cut to 280 characters). The full index stays local in `target/bagz/index.jsonl`; its SHA-256 is in `data/ingest-run.json`.

### Agents and the approval gate

Every agent returns `Pending<T>`. Only `Pending::approve(name)` makes an `Approved<T>`, and it refuses an empty name. Export and tracker updates take `Approved<T>` only, so the compiler stops unapproved output. **No code path submits an application or sends an email.**

| Agent | Does | Model |
| --- | --- | --- |
| `scorer` | Computes the fit score and proposes apply, maybe, or skip. The model only writes the one-line reason. | yes |
| `resume_tailor` | Moves matching skills and bullets first. Never adds a skill, role, or bullet. | yes (summary) |
| `cover_letter` | Drafts a letter. Refuses drafts outside 30–450 words, with placeholders, without the company name, or with skills the profile lacks. | yes |
| `form_prefill` | Maps the profile onto each ATS's own field names. Work authorization, sponsorship and EEO always go to the human. | no |
| `receipt_tracker` | Reads confirmation and status emails and proposes status changes. Statuses only move forward. | no |
| `follow_up` | Finds applications with no news after 7 days and drafts one short note. | yes |

Model calls go through the `Llm` trait (`src/llm.rs`). `MockLlm` is deterministic and offline; tests and the demo use it. `AnthropicLlm` reads `ANTHROPIC_API_KEY` from the environment at call time.

### ATS-safe .docx

`src/docx.rs` writes Word files with no dependencies: one column, no tables, text boxes, images, headers or footers, standard headings (`Summary`, `Skills`, `Experience`, `Education`), Calibri, and `•` bullets as plain text. Same input gives the same bytes. `ats_violations` and `extract_text` check a file the way an ATS reads it.

### Opt-in usage counter

Off by default. While off, nothing is written.

```bash
cargo run --bin bagz -- usage enable    # random install id + months/days used, stored in ~/.bagz/usage.json
cargo run --bin bagz -- usage export    # prints the small JSON report; you decide whether to share it
cargo run --bin bagz -- usage disable   # deletes the file
```

It never sends data. `usage::monthly_active` counts MAU across reports that users chose to share.

### Tests

- Golden files per ATS parser: `tests/golden/*.json` (regenerate with `BAGZ_UPDATE_GOLDEN=1 cargo test`).
- Recorded HTTP: `tests/fixtures/http/` with fictional companies. `FixtureClient` errors on any URL it did not record, so tests never reach the network.
- Property tests (`proptest`): parsers never panic, dedupe is idempotent and order-independent, rank is sorted and bounded, `.docx` and zip round-trip, the approval queue decides each proposal once.

## Demo app

The clickable demo has **12 example jobs** and **five screens** (splash, swipe deck, chat, profile, and checkout). Companies, pay figures, and the seeker profile are examples. No accounts and no real payments.

Live: https://bagz-demo.braylo777.workers.dev

That deployment is the static front end. `cargo run -p bagz-web --bin export -- dist` writes the exact same files (SHA-256 checked in tests).

## Architecture

```
src/data/jobs.json          example catalog (the only job data)
        |
        v
crates/bagz-core            job model, validation, swipe deck, score, filters
        |
        +-- PyO3 / maturin -> import bagz     (crates/bagz-py, bagz.pyi)
        |
        +-- crates/bagz-web          embedded web app (serve / export), checked against the catalog
```

| Path | Role |
| --- | --- |
| `crates/bagz-engine` | Ingestion, dedupe, ranking, six agents, approval gate, `.docx` export, usage counter, `bagz` and `metrics` binaries. |
| `data/` | Board list, last ingest run manifest, 250-posting sample snapshot. |
| `crates/bagz-web` | The web app embedded in Rust: `serve` and `export` binaries, SHA-256 checked. |
| `scripts/embed_web.py` | Regenerates `bagz-web/src/bundle.rs` from a built bundle. |
| `src/data/jobs.json` | The 12 example jobs. Rust reads this file. It is not copied into a second catalog. |
| `crates/bagz-core` | Job model, JSON validation, swipe/apply state, scoring, and filters. |
| `crates/bagz-py` | PyO3 bindings compiled with maturin into an importable `bagz` module. |
| `bagz.pyi` | Type stubs. Maturin ships them next to the extension. |

The shipped app still runs the swipe store that was compiled from TypeScript (now embedded in `bagz-web`). The Rust tests lock the swipe, score, queue, and plan rules that store uses, and a `bagz-web` test checks the app ships every job in the Rust catalog.

Shared rules:

- Right swipe (`bag`) applies and awards 10 XP. Super awards 25 XP and is stored as a bag. Left swipe (`pass`) awards nothing and clears the combo.
- The streak increments once per distinct day, and only on a bag or super.
- Applied jobs are newest first. The chat queue takes those first, then the rest of the catalog, up to four rows.
- Undo drops the last swipe and does not refund XP. Reset clears history only.
- Card score is the rounded mean of pay, fit, growth, and vibe. Rarity is legendary at 90 and rare at 85. Stickers are HOT, then REMOTE, then NEW (`h ago`), then `$$$`.
- A saved `agent` plan loads as `vip`. Unknown job ids in saved history are dropped.

## Web (Rust)

```bash
cargo run -p bagz-web --bin serve            # http://127.0.0.1:43123
cargo run -p bagz-web --bin export -- dist   # static site for deployment
```

`crates/bagz-web/src/bundle.rs` holds the page, script, styles and brand art, generated by `scripts/embed_web.py`. Tests check every file against its recorded SHA-256, check that `index.html` links the embedded script and styles, and check that all 12 jobs in the Rust catalog (`src/data/jobs.json`) appear in the shipped app.

The React/TypeScript source of the UI is kept on the `legacy-ts` branch (tag `pre-pycrust` marks the last commit before the port). To change the UI: edit it there, build it, then run `python3 scripts/embed_web.py <dist>` here.

## Rust

Rust 1.83 or newer.

```bash
cargo test
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

`Catalog::bundled()` and `Catalog::load_path("src/data/jobs.json")` both read the example catalog. `Deck` is the swipe session: `swipe`, `applied`, `passed`, `remaining`, and `queue`.

## Python

Python 3.9 or newer. The module is built from this repo. It is not published to PyPI.

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install maturin pytest
maturin develop
pytest
```

The CI job builds a wheel instead of an editable install:

```bash
maturin build --release
python -m pip install target/wheels/*.whl
pytest
```

```python
import bagz

catalog = bagz.load_jobs("src/data/jobs.json")
deck = bagz.Deck(catalog)
deck.swipe("froth", "bag")
deck.swipe("rack", "pass")
applied = deck.applied()  # newest first
```

`bag` applies, `pass` skips, and `super` applies for 25 XP. `Catalog.filter` narrows by mode, area, skill, and minimum match. `Catalog.rank_for_skills` orders jobs by how many of that job's skills were listed.

## CI

`.github/workflows/ci.yml` runs three jobs on pull requests and on pushes to `main`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo run --example demo`
- `cargo llvm-cov` coverage plus `cargo run --bin metrics`; `lcov.info` and `metrics.json` are uploaded as artifacts
- `maturin build --release`, install the wheel, and `pytest`

CI does not deploy and does not reach job boards: every test uses recorded fixtures.
