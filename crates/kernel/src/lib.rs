//! # craftkit-kernel
//!
//! **The trust boundary.** Every call into code craftkit does not own goes
//! through [`guard`].
//!
//! ## Why a whole crate for this
//!
//! The rule "a crash loses the user's work" is easy to state and easy to break.
//! It breaks the moment someone adds a call into a third-party crate *outside*
//! the function that catches. Then one bad input unwinds past the guard, and the
//! "never crash" promise quietly stops being true.
//!
//! Making the boundary a crate with exactly one exported function — and giving it
//! a lint-friendly signature — means the rule is checkable:
//!
//! - Every unsafe-ish call site is a single expression: `guard("name", || …)?`.
//! - `cargo xtask layers` keeps the boundary *below* the engine, so nothing can
//!   reach past it.
//! - A test proves a panic inside becomes a `KernelError` and leaves the input
//!   untouched.
//!
//! ## The rule
//!
//! **If a call can panic and its code is not in this workspace, it goes through
//! [`guard`].** No exceptions, no "this one is obviously fine".
//!
//! What the boundary does *not* do is roll back. Callers pass immutable input
//! and get a new value back, so there is nothing to roll back. A caller that
//! mutates in place and is not under a guard has a bug the boundary cannot see.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod measure;
mod ops;

pub use measure::{RunStats, measure_run};
pub use ops::{ChainInput, ChainStep, MAX_STEPS, apply_gain, evaluate_chain};

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    /// How deep we are inside a guarded call. A `bool` would be wrong: guarded
    /// calls nest, and a flag that is cleared by an inner return would leave an
    /// outer call believing it was unguarded.
    ///
    /// **Per-thread on purpose.** "Am I inside a guarded call?" is a question
    /// about the call stack of *this* thread; a call on another thread is not
    /// inside this one. §62.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Panics recovered by this boundary since the process started, and the two
/// operations on that number.
///
/// **Process-wide, not per-thread** — and that is the whole point of it. The
/// thread that would panic is never the thread that asks: the document is
/// evaluated on the UI thread, and the statistic is read from the control
/// channel's thread. A counter that lived in a thread-local cell could only ever
/// report recoveries that happened on the thread doing the reading, which in the
/// desktop app is none of them. §62.
///
/// It is an atomic rather than a lock because the increment happens on the
/// failure path, where the cost of taking a mutex is exactly what we do not
/// want, and because `wasm32-unknown-unknown` has no threads to speak of but
/// still compiles this crate.
///
/// The three items below are kept together on purpose: the mutation suite
/// turns this whole block back into a thread-local, to prove that the test
/// catches the difference rather than merely passing on a good tree.
static RECOVERED: AtomicU64 = AtomicU64::new(0);

/// Count one recovered panic.
///
/// Saturating rather than wrapping: the count is a lifetime statistic, and a
/// number that silently went back to zero would be read as "nothing has ever
/// gone wrong".
fn note_recovered() {
    // A compare-and-exchange loop, and not `fetch_update`.
    //
    // `fetch_update` was **deprecated** in the newest Rust — *"renamed to
    // `try_update` for consistency"*, since 1.99 — and the workflow builds
    // with `RUSTFLAGS: -D warnings`, so a deprecation is an **error** there.
    // And `try_update` does not exist on the floor this tree supports: it
    // arrived with the rename. So the one call that works on both is the one
    // that was always there. §217.
    let mut current = RECOVERED.load(Ordering::Relaxed);
    loop {
        let next = current.saturating_add(1);
        match RECOVERED.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            // Another thread counted first: take its number and try again.
            Err(actual) => current = actual,
        }
    }
}

/// How many panics this boundary has recovered since the process started.
///
/// The count is **process-wide**: a recovery on any thread is visible from any
/// other. See the note on [`RECOVERED`] for why it cannot be per-thread.
pub fn recovered_count() -> u64 {
    RECOVERED.load(Ordering::Relaxed)
}

/// RAII depth guard. `Drop` decrements even when the closure returns early with
/// `?` or unwinds, which a manual decrement would miss.
struct Depth;

impl Depth {
    fn enter() -> Self {
        DEPTH.with(|d| d.set(d.get().saturating_add(1)));
        Self
    }
}

impl Drop for Depth {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// True when the current thread is inside at least one guarded call.
///
/// **Nothing outside this crate calls it.** It is the per-thread counterpart
/// of [`recovered_count()`] — "is *this* thread inside a guarded call?" beside
/// "has the kernel recovered anywhere?" — and the tree has one rule for a
/// recovered kernel panic: it is a bug, it is reported, and it is never quiet.
/// The engine surfaces every one of them as an internal error, and the
/// application's panic hook writes a crash report for each, because a panic
/// that was recovered is still a panic.
///
/// The doc comment here used to say *"The engine uses this to decide whether a
/// recovered panic should be surfaced to the user as a warning or can be
/// logged quietly"*, and there is no such decision anywhere: the tree has no
/// speculative background pass to keep quiet about. §114.
pub fn guarded() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

/// Why a call into the kernel could not be completed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KernelError {
    /// The caller asked for something outside the domain's limits.
    #[error("{what}: {detail}")]
    Refused { what: &'static str, detail: String },
    /// The work itself failed for a domain reason.
    #[error("{what} failed: {detail}")]
    Failed { what: &'static str, detail: String },
    /// Something below the boundary panicked. The input was not modified.
    ///
    /// Deliberately a distinct variant: this means "a bug, not your input",
    /// and the engine reports it differently — it writes a crash record, because
    /// a recovered kernel panic is the thing worth reproducing.
    #[error("{what}: the implementation panicked ({detail}); your document is unchanged")]
    Internal { what: &'static str, detail: String },
}

impl KernelError {
    /// True for a recovered panic, as opposed to an ordinary refusal.
    pub fn is_internal(&self) -> bool {
        matches!(self, KernelError::Internal { .. })
    }
}

/// Run `f`, converting an escaped panic into [`KernelError::Internal`].
///
/// The `what` string names the operation and appears verbatim in the message,
/// so a report reads *"evaluating the chain: the implementation panicked …"*
/// rather than *"error"*. It is `&'static str` precisely so it can name the call
/// site without allocating in the failure path.
///
/// `AssertUnwindSafe` is a deliberate, reviewed decision: the closure may touch
/// shared state, and the safety of continuing after a caught panic rests on the
/// caller's discipline — which is why the rule is "the closure does not mutate
/// anything the caller will read afterwards". Every call site in this crate
/// obeys it, because they all build a new value from immutable input.
pub fn guard<T>(what: &'static str, f: impl FnOnce() -> Result<T, KernelError>) -> Result<T, KernelError> {
    let _depth = Depth::enter();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(payload) => {
            note_recovered();
            let detail = panic_message(&payload);
            log::warn!("kernel: {what} panicked and was recovered: {detail}");
            Err(KernelError::Internal { what, detail })
        }
    }
}

/// The message from a panic payload, or a placeholder when there is none.
///
/// A panic payload is not guaranteed to be a string: `panic_any(42)` is legal.
/// Falling back to a fixed message keeps the error `Eq`, which matters because
/// tests compare them.
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "no message".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(v: i32) -> Result<i32, KernelError> {
        Ok(v)
    }

    #[test]
    fn a_clean_call_is_transparent() {
        assert_eq!(guard("adding", || ok(2)), Ok(2));
    }

    #[test]
    fn an_ordinary_error_passes_through_unchanged() {
        let e = KernelError::Refused { what: "adding", detail: "no".to_string() };
        let got: Result<i32, KernelError> = guard("adding", || Err(e.clone()));
        assert_eq!(got, Err(e));
    }

    #[test]
    fn a_panic_becomes_an_internal_error_naming_the_call() {
        let r: Result<i32, KernelError> =
            guard("evaluating the chain", || -> Result<i32, KernelError> { panic!("deliberate") });
        let err = r.expect_err("should have been caught");
        assert!(err.is_internal(), "a panic must be Internal, not Refused");
        let msg = err.to_string();
        assert!(msg.contains("evaluating the chain"), "message was {msg}");
        assert!(msg.contains("your document is unchanged"), "message was {msg}");
    }

    #[test]
    fn every_panic_payload_shape_is_handled() {
        // `panic_any` with a non-string payload is legal and must not crash the
        // formatter, or the recovery path is not actually total. Every shape has
        // to go *through* the boundary — panicking beside it would panic the
        // test thread and prove nothing about recovery.
        for label in ["str", "string", "unit", "int"] {
            let r: Result<i32, KernelError> = match label {
                "str" => guard("x", || -> Result<i32, KernelError> { panic!("s") }),
                "string" => guard("x", || -> Result<i32, KernelError> { panic!("{}", String::from("s")) }),
                "unit" => guard("x", || -> Result<i32, KernelError> { std::panic::panic_any(()) }),
                _ => guard("x", || -> Result<i32, KernelError> { std::panic::panic_any(42u32) }),
            };
            let err = r.expect_err("caught");
            assert!(err.is_internal(), "{label} payload was not caught");
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn a_non_string_payload_reports_a_usable_message() {
        // The formatter's `else` arm: `panic_any(42u32)` has no message at all,
        // so it must fall back to something an operator can act on rather than
        // to an empty string.
        let r: Result<i32, KernelError> =
            guard("subtracting", || -> Result<i32, KernelError> { std::panic::panic_any(42u32) });
        let msg = r.expect_err("caught").to_string();
        assert!(msg.contains("no message"), "message was {msg}");
        assert!(msg.contains("subtracting"), "message was {msg}");
    }

    #[test]
    fn depth_is_balanced_across_nesting_and_early_return() {
        assert!(!guarded());
        let inner: Result<i32, KernelError> = guard("inner", || -> Result<i32, KernelError> {
            assert!(guarded(), "we should be inside the boundary");
            // An early `?` from a nested guarded call must still unwind the
            // depth, or every later call believes it is guarded.
            let e = KernelError::Failed { what: "inner", detail: "x".to_string() };
            guard("deeper", || Err(e))
        });
        assert!(inner.is_err());
        assert!(!guarded(), "depth leaked after an early return");
    }

    #[test]
    fn depth_is_balanced_after_a_panic() {
        let before = recovered_count();
        let _: Result<i32, KernelError> = guard("boom", || -> Result<i32, KernelError> { panic!("boom") });
        assert!(!guarded(), "depth leaked after a panic");
        assert!(recovered_count() > before, "the recovery counter should have moved");
    }

    /// A recovery on one thread must be visible from another.
    ///
    /// The counter exists so the engine can tell "the kernel recovered from
    /// something" from "nothing unusual happened" — and the two are always on
    /// different threads in the running app. The document is evaluated on the
    /// UI thread; the statistic is read from the control channel's thread. A
    /// counter kept in a thread-local cell could therefore only ever report
    /// recoveries that happened on the thread doing the reading, which in the
    /// desktop app is none of them: the counter was structurally incapable of
    /// reporting the thing it exists to report. §62.
    #[test]
    fn a_recovery_on_one_thread_is_counted_on_another() {
        let before = recovered_count();
        let worker = std::thread::spawn(|| {
            let r: Result<i32, KernelError> =
                guard("boom on a worker", || -> Result<i32, KernelError> { panic!("boom") });
            assert!(r.expect_err("caught").is_internal());
            recovered_count()
        });
        let seen_on_worker = worker.join().expect("the worker should not panic");
        assert!(seen_on_worker > before, "the worker should have counted its own recovery");
        assert!(
            recovered_count() > before,
            "a recovery on one thread must be visible on another: the counter is process-wide"
        );
    }

    /// The depth counter is a different question and stays per-thread.
    ///
    /// "Am I inside a guarded call?" is about *this* thread's stack. A guarded
    /// call on another thread does not put this thread inside the boundary, and
    /// making it do so would make `guarded()` a lie in the other direction.
    #[test]
    fn being_guarded_on_one_thread_does_not_guard_another() {
        let worker = std::thread::spawn(|| {
            let r: Result<i32, KernelError> =
                guard("held open", || -> Result<i32, KernelError> { panic!("boom") });
            assert!(r.is_err());
            assert!(!guarded(), "the depth must be unwound after the panic");
        });
        worker.join().expect("the worker should not panic");
        assert!(!guarded(), "another thread's guarded call must not guard this one");
    }

    #[test]
    fn the_input_is_untouched_because_nothing_is_mutated_in_place() {
        // The property the boundary guarantees: the caller's data survives a
        // panic inside the kernel, because the kernel only ever builds new
        // values from borrowed input.
        let input = vec![1u8, 2, 3];
        let snapshot = input.clone();
        let r: Result<Vec<u8>, KernelError> = guard("summing", || {
            // Borrow, never move: the caller's data must still be readable
            // afterwards, and that is exactly the property under test.
            let borrowed: &[u8] = input.as_slice();
            assert_eq!(borrowed.len(), 3);
            panic!("mid-operation");
        });
        assert!(r.is_err());
        assert_eq!(input, snapshot, "the caller's input was modified by a failed call");
    }
}
