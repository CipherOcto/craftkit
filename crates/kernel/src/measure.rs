//! Measuring what a walk cost.
//!
//! The kernel is the only layer that touches implementation detail, so it is the
//! only place that can report *how much work happened*. The engine uses that to
//! decide whether an evaluation is worth caching.

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
