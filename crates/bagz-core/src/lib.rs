//! Domain logic for the BAGZ swipe-to-apply demo.
//!
//! The React app still runs the same rules from `src/store.tsx` and
//! `src/components/CardArt.tsx`. This crate is the Rust copy: the job model,
//! `jobs.json` validation, the swipe deck, and the score, sticker, and filter
//! helpers. Python calls it through the `bagz` extension in `crates/bagz-py`.
//!
//! Job records come only from the example file `src/data/jobs.json`.

#![forbid(unsafe_code)]

mod date;
mod deck;
mod error;
mod job;

pub use date::{utc_date, utc_today};
pub use deck::{
    apply_phase, ApplyPhase, Decision, DecisionRecord, Deck, Plan, Swipe, CHAT_QUEUE_LIMIT,
};
pub use error::Error;
pub use job::{Catalog, Job, JobFilter, Rarity, SkillMatch};
