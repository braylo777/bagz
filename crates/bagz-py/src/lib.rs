//! Python module `bagz`. Built with PyO3 and maturin.
//!
//! The typed surface is `load_jobs`, `Catalog`, and `Deck` (swipe, then
//! `applied`). Stubs live in `bagz.pyi` and ship next to the extension.

use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

use bagz_core::{Catalog, Deck, Job, JobFilter, Plan, SkillMatch, Swipe};

#[pyclass(name = "Job", frozen, module = "bagz", skip_from_py_object)]
#[derive(Clone)]
struct PyJob {
    inner: Job,
}

#[pymethods]
impl PyJob {
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }

    #[getter]
    fn company(&self) -> &str {
        &self.inner.company
    }

    #[getter]
    fn pay(&self) -> &str {
        &self.inner.pay
    }

    #[getter]
    fn pay_score(&self) -> u8 {
        self.inner.pay_score
    }

    #[getter]
    fn growth(&self) -> u8 {
        self.inner.growth
    }

    #[getter]
    fn vibe(&self) -> u8 {
        self.inner.vibe
    }

    #[getter]
    fn chase(&self) -> u32 {
        self.inner.chase
    }

    #[getter]
    fn area(&self) -> &str {
        &self.inner.area
    }

    #[getter]
    fn mode(&self) -> &str {
        &self.inner.mode
    }

    #[getter]
    fn schedule(&self) -> &str {
        &self.inner.schedule
    }

    #[getter]
    fn posted(&self) -> &str {
        &self.inner.posted
    }

    #[getter]
    fn applicants(&self) -> &str {
        &self.inner.applicants
    }

    #[getter]
    fn size(&self) -> &str {
        &self.inner.size
    }

    #[getter]
    fn stage(&self) -> &str {
        &self.inner.stage
    }

    #[getter]
    fn match_pct(&self) -> u8 {
        self.inner.match_pct
    }

    #[getter]
    fn reasons(&self) -> Vec<String> {
        self.inner.reasons.clone()
    }

    #[getter]
    fn skills(&self) -> Vec<String> {
        self.inner.skills.clone()
    }

    #[getter]
    fn perks(&self) -> Vec<String> {
        self.inner.perks.clone()
    }

    #[getter]
    fn reply(&self) -> &str {
        &self.inner.reply
    }

    #[getter]
    fn sends(&self) -> &str {
        &self.inner.sends
    }

    /// Rounded mean of pay, fit, growth, and vibe.
    fn score(&self) -> u8 {
        self.inner.score()
    }

    /// `legendary`, `rare`, or `common`.
    fn rarity(&self) -> &'static str {
        self.inner.rarity().as_str()
    }

    /// `HOT`, `REMOTE`, `NEW`, `$$$`, or `""` when the card has no sticker.
    fn sticker(&self) -> &'static str {
        self.inner.sticker()
    }

    fn __repr__(&self) -> String {
        format!(
            "Job(id={:?}, title={:?}, company={:?})",
            self.inner.id, self.inner.title, self.inner.company
        )
    }
}

impl From<Job> for PyJob {
    fn from(inner: Job) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "SkillMatch", frozen, module = "bagz")]
struct PySkillMatch {
    job: Job,
    shared: Vec<String>,
    overlap_pct: u8,
}

#[pymethods]
impl PySkillMatch {
    #[getter]
    fn job(&self) -> PyJob {
        PyJob::from(self.job.clone())
    }

    #[getter]
    fn shared(&self) -> Vec<String> {
        self.shared.clone()
    }

    #[getter]
    fn overlap_pct(&self) -> u8 {
        self.overlap_pct
    }

    fn __repr__(&self) -> String {
        format!(
            "SkillMatch(job_id={:?}, overlap_pct={})",
            self.job.id, self.overlap_pct
        )
    }
}

impl From<SkillMatch<'_>> for PySkillMatch {
    fn from(hit: SkillMatch<'_>) -> Self {
        Self {
            job: hit.job.clone(),
            shared: hit.shared,
            overlap_pct: hit.overlap_pct,
        }
    }
}

#[pyclass(name = "Catalog", frozen, module = "bagz", skip_from_py_object)]
#[derive(Clone)]
struct PyCatalog {
    inner: Catalog,
}

#[pymethods]
impl PyCatalog {
    /// Load and validate a jobs JSON file.
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        Catalog::load_path(path).map(Self::from).map_err(to_py)
    }

    /// Catalog compiled from the repo's example `jobs.json`.
    #[staticmethod]
    fn bundled() -> PyResult<Self> {
        Catalog::bundled().map(Self::from).map_err(to_py)
    }

    fn jobs(&self) -> Vec<PyJob> {
        self.inner.jobs().iter().cloned().map(PyJob::from).collect()
    }

    fn get(&self, job_id: &str) -> Option<PyJob> {
        self.inner.get(job_id).cloned().map(PyJob::from)
    }

    #[pyo3(signature = (*, mode=None, area=None, skill=None, min_match=None))]
    fn filter(
        &self,
        mode: Option<String>,
        area: Option<String>,
        skill: Option<String>,
        min_match: Option<i64>,
    ) -> PyResult<Vec<PyJob>> {
        let min_match = checked_min_match(min_match)?;
        let query = JobFilter {
            mode,
            area,
            skill,
            min_match,
        };
        Ok(to_py_jobs(self.inner.filter(&query)))
    }

    /// Rank by skill overlap, then shipped match, then file order.
    fn rank_for_skills(&self, skills: Vec<String>) -> Vec<PySkillMatch> {
        self.inner
            .rank_for_skills(&skills)
            .into_iter()
            .map(PySkillMatch::from)
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("Catalog(jobs={})", self.inner.len())
    }
}

impl From<Catalog> for PyCatalog {
    fn from(inner: Catalog) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "Deck", module = "bagz")]
struct PyDeck {
    inner: Deck,
}

#[pymethods]
impl PyDeck {
    #[new]
    fn new(catalog: &PyCatalog) -> Self {
        Self {
            inner: Deck::new(catalog.inner.clone()),
        }
    }

    /// Swipe `bag`, `pass`, or `super`.
    ///
    /// `today` is the UTC day (`YYYY-MM-DD`) used for the streak. When it is
    /// omitted, the streak uses today's UTC date.
    #[pyo3(signature = (job_id, decision, today=None))]
    fn swipe(&mut self, job_id: &str, decision: &str, today: Option<&str>) -> PyResult<()> {
        let swipe = Swipe::parse(decision).map_err(to_py)?;
        let owned;
        let day = match today {
            Some(day) => day,
            None => {
                owned = bagz_core::utc_today();
                &owned
            }
        };
        self.inner.swipe(job_id, swipe, day).map_err(to_py)
    }

    fn undo(&mut self) {
        self.inner.undo();
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Applied jobs, newest first.
    fn applied(&self) -> Vec<PyJob> {
        to_py_jobs(self.inner.applied())
    }

    /// Passed jobs, oldest first.
    fn passed(&self) -> Vec<PyJob> {
        to_py_jobs(self.inner.passed())
    }

    /// Jobs not yet bagged or passed, in file order.
    fn remaining(&self) -> Vec<PyJob> {
        to_py_jobs(self.inner.remaining())
    }

    /// Applied jobs first, then the rest of the catalog, capped at `limit`.
    #[pyo3(signature = (limit=4))]
    fn queue(&self, limit: usize) -> Vec<PyJob> {
        to_py_jobs(self.inner.queue(limit))
    }

    fn watchers(&self) -> Vec<PyJob> {
        to_py_jobs(self.inner.watchers())
    }

    /// `(job_id, "bag" | "pass")` oldest first.
    fn history(&self) -> Vec<(String, String)> {
        self.inner
            .history()
            .iter()
            .map(|record| {
                let decision = match record.decision {
                    bagz_core::Decision::Bag => "bag",
                    bagz_core::Decision::Pass => "pass",
                };
                (record.id.clone(), decision.to_string())
            })
            .collect()
    }

    #[getter]
    fn xp(&self) -> i64 {
        self.inner.xp()
    }

    #[getter]
    fn streak(&self) -> i64 {
        self.inner.streak()
    }

    #[getter]
    fn combo(&self) -> i64 {
        self.inner.combo()
    }

    #[getter]
    fn plan(&self) -> &'static str {
        self.inner.plan().as_str()
    }

    #[getter]
    fn last_day(&self) -> &str {
        self.inner.last_day()
    }

    #[getter]
    fn chased(&self) -> u64 {
        self.inner.chased()
    }

    #[getter]
    fn rank(&self) -> &'static str {
        self.inner.rank()
    }

    fn set_plan(&mut self, plan: &str) -> PyResult<()> {
        self.inner.set_plan(Plan::parse(plan).map_err(to_py)?);
        Ok(())
    }

    fn export_state(&self) -> PyResult<String> {
        self.inner.export_state().map_err(to_py)
    }

    fn import_state(&mut self, raw: &str) -> PyResult<()> {
        self.inner.import_state(raw).map_err(to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Deck(applied={}, xp={}, plan={:?})",
            self.inner.applied().len(),
            self.inner.xp(),
            self.inner.plan().as_str()
        )
    }
}

/// Load and validate a jobs JSON file. Same as `Catalog.load`.
#[pyfunction]
fn load_jobs(path: &str) -> PyResult<PyCatalog> {
    PyCatalog::load(path)
}

/// Chat-row phase label: `Queued`, `Applying`, `Applied`, or `Reply`.
#[pyfunction]
fn apply_phase(elapsed_ms: i64, index: usize) -> &'static str {
    bagz_core::apply_phase(elapsed_ms, index).as_str()
}

#[pymodule]
fn bagz(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyJob>()?;
    m.add_class::<PySkillMatch>()?;
    m.add_class::<PyCatalog>()?;
    m.add_class::<PyDeck>()?;
    m.add_function(wrap_pyfunction!(load_jobs, m)?)?;
    m.add_function(wrap_pyfunction!(apply_phase, m)?)?;
    Ok(())
}

fn to_py(error: bagz_core::Error) -> PyErr {
    let message = error.to_string();
    match error {
        bagz_core::Error::Io(_) => PyOSError::new_err(message),
        _ => PyValueError::new_err(message),
    }
}

fn checked_min_match(value: Option<i64>) -> PyResult<Option<u8>> {
    match value {
        None => Ok(None),
        Some(value) if (0..=100).contains(&value) => Ok(Some(value as u8)),
        Some(_) => Err(to_py(bagz_core::Error::BadMinMatch)),
    }
}

fn to_py_jobs(jobs: Vec<&Job>) -> Vec<PyJob> {
    jobs.into_iter().cloned().map(PyJob::from).collect()
}
