//! BAGZ engine: real job ingestion, approval-gated agents, and ATS-safe export.
//!
//! - [`ingest`] reads the public job-board APIs of Greenhouse, Lever, Ashby and
//!   Workable plus plain RSS feeds, normalizes every posting into [`Posting`],
//!   then [`dedupe`]s and [`rank`]s them.
//! - [`agents`] holds six agents. Each one returns a [`approval::Pending`]
//!   proposal. Only a human approval turns it into an [`approval::Approved`]
//!   value, and nothing in this crate submits an application.
//! - [`llm`] puts every model call behind the [`llm::Llm`] trait, with a
//!   deterministic [`llm::MockLlm`] for tests and demos.
//! - [`docx`] writes single-column, table-free Word files that ATS parsers read.
//! - [`usage`] is an opt-in, local-first, anonymous usage counter. It is off by
//!   default and never sends data anywhere.

#![forbid(unsafe_code)]

pub mod agents;
pub mod approval;
pub mod dedupe;
pub mod docx;
pub mod error;
pub mod http;
pub mod ingest;
pub mod llm;
pub mod metrics;
pub mod model;
pub mod profile;
pub mod rank;
pub mod text;
pub mod usage;

pub use error::{Error, Result};
pub use model::{Posting, Source, Workplace};
pub use profile::{Experience, Profile};
