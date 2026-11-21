Ingest facts that are not obvious from the code.

- Large boards (SpaceX, Anduril on Greenhouse; OpenAI on Ashby) send over 10 MB with `content=true`. ureq's `into_string` stops at 10 MB, so `LiveClient` streams the body (cap `MAX_BODY`, 256 MB).
- Dedupe removes about 21 % of a live run. Most hits are the same title and city posted as many requisitions on one board, not cross-source copies.
- Lever and Ashby feeds do not name the company; the board slug is title-cased (`company_from_slug`).
- Workable's widget API only includes descriptions with `?details=true`.
- `metrics` runs nested cargo with `CARGO_TARGET_DIR=target/metrics` so it does not wait on the lock of the outer `cargo run`.
- Tests must never reach the network: use `FixtureClient`; it errors on any URL it did not record.
