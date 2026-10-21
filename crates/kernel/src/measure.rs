//! Measuring what a walk cost.
//!
//! The kernel is the only layer that touches implementation detail, so it is the
//! only place that can report *how much work happened*.
//!
//! **Nothing in the tree asks.** The engine's evaluator cache is
//! *content-addressed* (§80): an entry is reused when the node it was computed
//! from still hashes the same, and a cost would not change what it decides —
//! the expensive node is the one whose content changed, whatever it cost to
//! compute. So this is the seam where a cost-based rule would live, and there
//! is no such rule. §114.
//!
//! The doc comment here used to say *"The engine uses that to decide whether
//! an evaluation is worth caching"*, and the engine has never used it. The
//! counter it does report — `session.stats`'s `kernel_recoveries` — comes
//! from `recovered_count()`, and that one is about panics, not about time.

use super::KernelError;
use std::time::Instant;

/// What one guarded operation cost.
#[derive(Debug, Clone, PartialEq)]
pub struct RunStats {
    pub steps: usize,
    pub micros: u128,
}

/// Run `f`, returning both its value and what it cost.
///
/// Under the same boundary as everything else: a panic inside `f` is still
/// recovered, and the timing is discarded because the measurement did not
/// complete.
pub fn measure_run<T>(
    what: &'static str,
    f: impl FnOnce() -> Result<T, KernelError>,
) -> Result<(T, RunStats), KernelError> {
    let start = Instant::now();
    let value = super::guard(what, f)?;
    Ok((value, RunStats { steps: 1, micros: start.elapsed().as_micros() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_run_reports_its_cost_and_its_value() {
        let (v, stats): (i32, RunStats) = measure_run("t", || Ok(7i32)).expect("ok");
        assert_eq!(v, 7);
        assert_eq!(stats.steps, 1);
    }

    #[test]
    fn a_panicking_run_produces_no_measurement() {
        let r: Result<(i32, RunStats), KernelError> =
            measure_run("t", || -> Result<i32, KernelError> { panic!("boom") });
        assert!(r.is_err());
        assert!(r.expect_err("err").is_internal());
    }

    #[test]
    fn an_ordinary_error_produces_no_measurement_either() {
        let e = KernelError::Refused { what: "t", detail: "d".to_string() };
        let r: Result<(i32, RunStats), KernelError> = measure_run("t", || Err(e.clone()));
        assert_eq!(r.expect_err("err"), e);
    }
}
