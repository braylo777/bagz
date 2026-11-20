//! Load and validate a job catalog.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::filter::{JobFilter, filter_jobs};
use crate::job::Job;

const EXAMPLE_JOBS_JSON: &str = include_str!("../../../src/data/jobs.json");

/// The example `jobs.json` embedded from `src/data/jobs.json`.
#[must_use]
pub fn example_jobs_json() -> &'static str {
    EXAMPLE_JOBS_JSON
}

/// Parse the embedded example catalog.
///
/// # Errors
///
/// Returns [`Error`] if the embedded file no longer validates.
pub fn load_example_jobs() -> Result<Catalog, Error> {
    Catalog::parse(EXAMPLE_JOBS_JSON)
}

/// Validated jobs in file order, indexed by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    jobs: Vec<Job>,
    by_id: HashMap<String, usize>,
}

impl Catalog {
    /// Parse a JSON array of jobs.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Json`] for malformed JSON, [`Error::InvalidJob`] when
    /// a job fails validation, and [`Error::DuplicateId`] when two rows share
    /// an id.
    pub fn parse(json: &str) -> Result<Self, Error> {
        let jobs: Vec<Job> = serde_json::from_str(json)?;
        Self::from_jobs(jobs)
    }

    /// Read and parse a jobs file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Read`] when the file cannot be read, or the same
    /// errors as [`Self::parse`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    fn from_jobs(jobs: Vec<Job>) -> Result<Self, Error> {
        let mut by_id = HashMap::with_capacity(jobs.len());
        for (index, job) in jobs.iter().enumerate() {
            job.validate()?;
            if by_id.insert(job.id.clone(), index).is_some() {
                return Err(Error::DuplicateId(job.id.clone()));
            }
        }
        Ok(Self { jobs, by_id })
    }

    /// Jobs in catalog order.
    #[must_use]
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// Look up a job by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Job> {
        self.by_id.get(id).map(|index| &self.jobs[*index])
    }

    /// Number of jobs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Whether the catalog has no jobs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Jobs that match `query`, in catalog order.
    #[must_use]
    pub fn filter(&self, query: &JobFilter) -> Vec<&Job> {
        filter_jobs(&self.jobs, query)
    }
}
