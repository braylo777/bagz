# BAGZ

Clickable demo of BAGZ, a swipe-to-apply job app. The browser demo is React, TypeScript, and Vite. The domain rules also live in a Rust crate, and Python calls that crate through a PyO3 module named `bagz`.

The demo has **12 example jobs** and **five screens** (splash, swipe deck, chat, profile, and checkout). Companies, pay figures, and the seeker profile are examples. No accounts and no real payments.

Live: https://bagz-demo.braylo777.workers.dev

That deployment is the static front end. The Rust crate and the Python module are local libraries in this repo. They do not change the live demo.

## Architecture

```
src/data/jobs.json          example catalog (the only job data)
        |
        v
crates/bagz-core            job model, validation, swipe deck, score, filters
        |
        +-- PyO3 / maturin -> import bagz     (crates/bagz-py, bagz.pyi)
        |
        +-- same rules, not linked at build time
                |
                v
        src/store.tsx        React deck, chat, profile, checkout
```

| Path | Role |
| --- | --- |
| `src/` | React demo. Screens, motion, and layout stay here. |
| `src/data/jobs.json` | The 12 example jobs. Rust reads this file. It is not copied into a second catalog. |
| `crates/bagz-core` | Job model, JSON validation, swipe/apply state, scoring, and filters. |
| `crates/bagz-py` | PyO3 bindings compiled with maturin into an importable `bagz` module. |
| `bagz.pyi` | Type stubs. Maturin ships them next to the extension. |

The web app still runs the TypeScript store in `src/store.tsx`. WASM is not wired into Vite. A wasm-bindgen step would make `npm run build` depend on Rust, and the demo does not need it to stay the same. The Rust tests lock the swipe, score, queue, and plan rules that store uses.

Shared rules:

- Right swipe (`bag`) applies and awards 10 XP. Super awards 25 XP and is stored as a bag. Left swipe (`pass`) awards nothing and clears the combo.
- The streak increments once per distinct day, and only on a bag or super.
- Applied jobs are newest first. The chat queue takes those first, then the rest of the catalog, up to four rows.
- Undo drops the last swipe and does not refund XP. Reset clears history only.
- Card score is the rounded mean of pay, fit, growth, and vibe. Rarity is legendary at 90 and rare at 85. Stickers are HOT, then REMOTE, then NEW (`h ago`), then `$$$`.
- A saved `agent` plan loads as `vip`. Unknown job ids in saved history are dropped.

## Web

```bash
npm install
npm run dev    # http://localhost:43123
npm run build
```

Output directory: `dist/`

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

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`
- `maturin build --release`, install the wheel, and `pytest`
- `npm ci` and `npm run build`

CI does not deploy.
