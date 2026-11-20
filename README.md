# BAGZ

Clickable demo of BAGZ, a swipe-to-apply job app. The screen is a static React, TypeScript, and Vite front end. Job rules — the catalog, swipe state, and matching — live in a Rust crate and are also available as a Python module.

The demo has **12 example jobs** and **five screens** (splash, swipe deck, chat, profile, and checkout). Companies, pay figures, and the seeker profile are examples. No accounts and no real payments.

Live: https://bagz-demo.braylo777.workers.dev

The live Workers site stays as it is. These packages do not change its deploy.

## Architecture

```
crates/bagz-core     Rust domain crate
bindings/python      PyO3 / maturin module `bagz`
src/                 React demo (TypeScript store, unchanged)
src/data/jobs.json   The only job catalog
```

`bagz-core` loads and validates `jobs.json`, records bag / pass / super-bag swipes, and answers the questions the demo already asks: which jobs are left, which were applied to (newest first), match score, rarity, sticker, area / mode / skill filters, the four-row agent queue, and the five hash routes.

The browser keeps the TypeScript store in `src/store.tsx`, so the Vite build and the screens stay on the code they already ship. The Rust crate is the shared core for tests and for Python. WASM was left out: compiling through wasm-bindgen would add a second toolchain and a bundler plugin to the demo build.

Swipe names match the app. `bag` (also `like` or `apply` from Python) and `super` land in the applied queue, newest first. `super` is worth 25 xp and `bag` is worth 10. `pass` does not score. Undo and reset clear decisions and leave experience in place, the same way the demo store does.

## Rust

The toolchain is pinned in `rust-toolchain.toml` (stable 1.100.0, with rustfmt and clippy).

```bash
cargo test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

`cargo test` covers the core and compiles the Python crate. It needs a Python interpreter with development headers so PyO3 can link (`python3-dev` on Debian/Ubuntu).

## Python

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install maturin pytest
maturin develop --manifest-path bindings/python/Cargo.toml
pytest bindings/python/tests
```

Build a wheel instead of installing an editable copy:

```bash
maturin build --manifest-path bindings/python/Cargo.toml --out dist
python -m pip install dist/bagz-*.whl
```

```python
import bagz

jobs = bagz.load_example_jobs()
deck = bagz.Deck.example()
deck.swipe("froth", "like")
deck.applied()
bagz.filter_jobs(jobs, mode="remote", min_match=80)
```

Stubs ship in the package as `bagz/__init__.pyi`.

## Web

```bash
npm install
npm run dev    # http://localhost:43123
npm run build
```

Output directory: `dist/`

## CI

`.github/workflows/ci.yml` runs Rust fmt, clippy, and tests; builds the Python wheel with maturin and runs pytest; and runs `npm run build`.
