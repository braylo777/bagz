//! Domain logic for the Bagz swipe-to-apply demo.
//!
//! The crate loads the example jobs, validates them, and tracks bag, pass,
//! and super-bag decisions. The React demo keeps its TypeScript store so the
//! browser bundle does not depend on this crate.
//!
//! ```
//! let catalog = bagz_core::load_example_jobs().expect("example jobs");
//! assert_eq!(catalog.len(), 12);
//! ```

#![warn(missing_docs)]

mod catalog;
mod error;
mod filter;
mod job;
mod persist;
mod queue;
mod route;
mod score;
mod session;

pub use catalog::{Catalog, example_jobs_json, load_example_jobs};
pub use error::Error;
pub use filter::{JobFilter, filter_jobs};
pub use job::Job;
pub use persist::{Decision, DecisionRecord, PersistedState, Plan, sanitize_state};
pub use queue::{Phase, hours_saved_label, phase_for, stamp_label};
pub use route::{Route, parse_route};
pub use score::{Rarity, Sticker, rarity_of, score_of, sticker_for};
pub use session::{
    AGENT_QUEUE_LIMIT, BAG_SECURED_XP, BAG_XP, CLOSER_XP, Rank, SUPER_XP, Session, Swipe,
    SwipeOutcome, WATCHER_LIMIT, rank_for,
};

/// Today's date in UTC, as `YYYY-MM-DD`.
#[must_use]
pub fn utc_today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}
