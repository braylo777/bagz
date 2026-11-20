//! Python module `bagz._core`, re-exported by the `bagz` package.

use pyo3::exceptions::{PyFileNotFoundError, PyValueError};
use pyo3::prelude::*;

fn to_py(err: bagz_core::Error) -> PyErr {
    let not_found = matches!(
        &err,
        bagz_core::Error::Read { source, .. } if source.kind() == std::io::ErrorKind::NotFound
    );
    let message = err.to_string();
    if not_found {
        PyFileNotFoundError::new_err(message)
    } else {
        PyValueError::new_err(message)
    }
}

/// Native half of the `bagz` package.
#[pymodule]
mod _core {
    use pyo3::prelude::*;

    /// One example role from the job catalog.
    #[derive(Clone, PartialEq, Eq)]
    #[pyclass(frozen, eq, from_py_object, module = "bagz", name = "Job")]
    struct Job {
        inner: bagz_core::Job,
    }

    #[pymethods]
    impl Job {
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
            bagz_core::score_of(&self.inner)
        }

        /// `common`, `rare`, or `legendary`.
        fn rarity(&self) -> &'static str {
            bagz_core::rarity_of(self.inner.match_pct).as_str()
        }

        /// `HOT`, `REMOTE`, `NEW`, `$$$`, or none.
        fn sticker(&self) -> Option<&'static str> {
            bagz_core::sticker_for(&self.inner).map(bagz_core::Sticker::as_str)
        }

        fn __repr__(&self) -> String {
            format!(
                "Job(id={:?}, title={:?}, company={:?})",
                self.inner.id, self.inner.title, self.inner.company
            )
        }
    }

    impl Job {
        fn new(inner: bagz_core::Job) -> Self {
            Self { inner }
        }
    }

    /// Swipe state for one catalog.
    #[pyclass(module = "bagz", name = "Deck")]
    struct Deck {
        session: bagz_core::Session,
    }

    #[pymethods]
    impl Deck {
        /// Load and validate a jobs JSON file.
        #[staticmethod]
        fn load(path: &str) -> PyResult<Self> {
            let catalog = bagz_core::Catalog::load(path).map_err(super::to_py)?;
            Ok(Self {
                session: bagz_core::Session::new(catalog),
            })
        }

        /// Deck over the 12 embedded example jobs.
        #[staticmethod]
        fn example() -> PyResult<Self> {
            let catalog = bagz_core::load_example_jobs().map_err(super::to_py)?;
            Ok(Self {
                session: bagz_core::Session::new(catalog),
            })
        }

        /// Record a swipe.
        ///
        /// `decision` is `bag`, `like`, `apply`, `pass`, or `super`.
        /// `day` is `YYYY-MM-DD`. The default is today's UTC date.
        #[pyo3(signature = (job_id, decision, day=None))]
        fn swipe(&mut self, job_id: &str, decision: &str, day: Option<&str>) -> PyResult<()> {
            let action = parse_swipe(decision)?;
            let owned;
            let day = match day {
                Some(day) => day,
                None => {
                    owned = bagz_core::utc_today();
                    owned.as_str()
                }
            };
            self.session
                .swipe(job_id, action, day)
                .map_err(super::to_py)?;
            Ok(())
        }

        /// Bagged jobs, newest first.
        fn applied(&self) -> Vec<Job> {
            self.session
                .applied()
                .into_iter()
                .cloned()
                .map(Job::new)
                .collect()
        }

        /// Passed jobs, oldest first.
        fn passed(&self) -> Vec<Job> {
            self.session
                .passed()
                .into_iter()
                .cloned()
                .map(Job::new)
                .collect()
        }

        /// Jobs not yet swiped, in catalog order.
        fn remaining(&self) -> Vec<Job> {
            self.session
                .remaining()
                .into_iter()
                .cloned()
                .map(Job::new)
                .collect()
        }

        /// Drop the newest decision and return its id.
        fn undo(&mut self) -> Option<String> {
            self.session.undo().map(|record| record.id)
        }

        /// Clear swipe history. Experience stays.
        fn reset(&mut self) {
            self.session.reset_swipes();
        }

        /// Experience from bags (10) and super-bags (25).
        #[getter]
        fn xp(&self) -> u32 {
            self.session.xp()
        }

        fn __repr__(&self) -> String {
            format!(
                "Deck(applied={}, remaining={}, xp={})",
                self.session.applied().len(),
                self.session.remaining().len(),
                self.session.xp()
            )
        }
    }

    /// Load and validate a jobs JSON file.
    #[pyfunction]
    fn load_jobs(path: &str) -> PyResult<Vec<Job>> {
        let catalog = bagz_core::Catalog::load(path).map_err(super::to_py)?;
        Ok(catalog.jobs().iter().cloned().map(Job::new).collect())
    }

    /// The 12 embedded example jobs, in file order.
    #[pyfunction]
    fn load_example_jobs() -> PyResult<Vec<Job>> {
        let catalog = bagz_core::load_example_jobs().map_err(super::to_py)?;
        Ok(catalog.jobs().iter().cloned().map(Job::new).collect())
    }

    /// Keep jobs that match every set constraint, in the input order.
    #[pyfunction]
    #[pyo3(signature = (jobs, *, mode=None, area=None, schedule=None, skill=None, min_match=None))]
    fn filter_jobs(
        jobs: Vec<Job>,
        mode: Option<String>,
        area: Option<String>,
        schedule: Option<String>,
        skill: Option<String>,
        min_match: Option<i64>,
    ) -> PyResult<Vec<Job>> {
        let query = bagz_core::JobFilter::try_new(mode, area, schedule, skill, min_match)
            .map_err(super::to_py)?;
        Ok(jobs
            .into_iter()
            .filter(|job| query.matches(&job.inner))
            .collect())
    }

    fn parse_swipe(decision: &str) -> Result<bagz_core::Swipe, PyErr> {
        match decision.trim().to_ascii_lowercase().as_str() {
            "bag" | "like" | "apply" => Ok(bagz_core::Swipe::Bag),
            "pass" => Ok(bagz_core::Swipe::Pass),
            "super" => Ok(bagz_core::Swipe::Super),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "decision must be bag, like, apply, pass, or super, got `{decision}`"
            ))),
        }
    }
}
