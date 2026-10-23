//! # craftkit-engine
//!
//! The command registry, the session, undo, and the crash reporter.
//!
//! This crate is the reason an agent can drive craftkit. Everything a person
//! can do goes through [`Session::execute`]; so does everything a script, a
//! CLI subcommand, the control channel and the MCP server can do. One entry
//! point means one place where the panic boundary, the undo snapshot and the
//! log all live — and it means a tool call an agent makes cannot be a
//! second-class path that skipped a guard.
//!
//! ## The four rules in this crate
//!
//! 1. **Everything is a command.** User-visible behaviour is a
//!    [`crate::command::CommandSpec`] with our own dotted id. There is no second
//!    way to change a document — not a UI-only path, not a "just this once"
//!    helper that bypasses the registry.
//! 2. **Programmatic calls never open dialogs.** [`Session::execute`] is
//!    synchronous and total. Interactive work is a *different* entry point,
//!    which the front ends own: it asks the person for the parameters and
//!    then calls this. §118.
//! 3. **Clone before edit.** A mutating command runs against a snapshot, and the
//!    snapshot is restored on both an error and a panic.
//! 4. **Read parameters through [`craftkit_core::Params`].** Never index a
//!    parameter object, anywhere, ever. The one door out — the raw JSON — has
//!    two homes, and `cargo xtask lists` refuses the tree when a third file
//!    opens it. §136.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod command;
pub mod commands;
pub mod crash;
pub mod error;
pub mod history;
pub mod log;
pub mod params;
pub mod session;

pub use command::{Area, CommandSpec, Icon, ParamDoc, Registry, all, catalogue, get, ids};
pub use error::EngineError;
pub use history::History;
pub use log::LogBook;
pub use params::Doc;
pub use session::{MAX_SESSION_BYTES, Outcome, SESSION_DOCUMENTS, Session, sample_steps};

/// How much memory the command trace may hold.
///
/// The trace is in memory for the whole session, and what it holds is the
/// **parsed** parameters of every command — not the text they arrived as. A
/// parsed JSON value costs up to [`craftkit_core::JSON_EXPANSION`] times its
/// text, so a budget written in bytes of text is a budget in bytes of memory
/// that many times larger. §70.
pub const MAX_TRACE_MEMORY_BYTES: usize = 8 * 1024 * 1024;

/// How many bytes of command trace a session keeps for a crash report.
///
/// Bounded because the trace is in memory for the whole session — but bounded
/// by **size**, not by a count of commands. Sixty-four commands can be a few
/// hundred bytes or a few megabytes, and the number that decides whether a
/// report can still reproduce the session is the second one. A trace that
/// stops mid-way is not wrong, as long as it says how much it dropped.
///
/// **Derived from the memory budget**, because the trace holds parsed values
/// and not text: 8 MiB of memory is 256 KiB of text at the tree's own
/// measured expansion. The number is the same as it always was; what it
/// *means* is now stated. §70.
pub const MAX_TRACE_BYTES: usize = MAX_TRACE_MEMORY_BYTES / craftkit_core::JSON_EXPANSION;

/// How many commands one script or one replay may run.
///
/// **One bound for both doors.** A script is a list of commands and a replay
/// is a list of commands, and the tree hands one to the other: `crashes
/// replay --script` prints a replay **as a script**, and `craftkit-cli run`
/// reads a script. Two bounds would mean the tree could print a script it
/// would then refuse to run. §181.
///
/// Three derivations, and they agree on ten thousand.
///
/// - **The trace's capacity.** A crash report carries the commands that ran,
///   and the trace is bounded by [`MAX_TRACE_BYTES`] — by *bytes*, not by a
///   count. The shortest step the tree can produce is 34 bytes
///   (`{"command":"node.add","params":{}}`), so the trace holds at most 7 710
///   of them, and measured, that is exactly what it holds. A report with more
///   steps than that was not written by this tree.
/// - **The model's node budget.** A design may hold
///   [`craftkit_model::limits::MAX_NODES`] nodes. A script that asks for more
///   commands than the model allows nodes is asking for work the model
///   refuses at the last step, and refusing it at the last step is the
///   expensive way to refuse it.
/// - **What the tree already said.** §48 bounded the replay at "about ten
///   thousand steps", and the constant said twenty thousand: the prose and the
///   code disagreed, and the prose was right.
///
/// The cost is not linear. **Every** command clones the document — mutating
/// or not — so `n` commands that each grow the document cost O(n²) in the
/// nodes copied. Measured: 1 000 steps took 1.1 s, 2 000 took 4.7 s, 4 000
/// took 24.9 s and 10 000 took 165 s. Ten thousand steps is therefore
/// minutes, not seconds, and the bound is a bound and not a formality. §181.
///
/// The bound says *how many*, not *how fast*. Making the cost linear is a
/// different question and it has an owner: the per-command clone is the price
/// of putting the design back when a command fails or panics. §52, and the
/// suggested order's item 7.
pub const MAX_COMMAND_STEPS: usize = 10_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_is_not_empty_and_every_command_is_reachable() {
        assert!(!all().is_empty());
        for spec in all() {
            assert!(
                get(spec.id).is_some_and(|found| std::ptr::eq(found as *const _, *spec as *const _)),
                "`{}` is in the list but not findable",
                spec.id
            );
        }
        assert_eq!(ids().len(), all().len(), "ids() lost or duplicated a command");
    }

    /// The step bound is the tree's own capacity, in both directions.
    ///
    /// §181. A bound on a script or a replay has to be **at least** what the
    /// trace can hold — a report this tree wrote must be replayable — and
    /// **at most** what the model allows — a script must not ask for more work
    /// than building the largest design the model allows. The old bound of
    /// twenty thousand was over the second and under nothing: it was more
    /// than twice what a report can hold, and twice what the model allows.
    ///
    /// Both halves are checked against the tree's own numbers rather than
    /// against a number written here, so the test fails when the tree changes
    /// and the bound does not.
    #[test]
    fn the_step_bound_is_the_trace_s_capacity_and_the_model_s_node_budget() {
        // At least: the shortest step the registry can produce. The trace is
        // bounded by *bytes*, so the shortest step is the one that buys the
        // most steps — and the bound has to cover the most.
        let shortest = ids().iter().map(|id| id.len()).min().unwrap_or(0);
        let step = serde_json::to_string(&session::TraceStep {
            command: "x".repeat(shortest),
            params: serde_json::json!({}),
        })
        .map(|s| s.len())
        .unwrap_or(0);
        assert!(step > 0, "the registry has no command to measure a step with");
        assert!(
            MAX_TRACE_BYTES / step <= MAX_COMMAND_STEPS,
            "the trace holds {} step(s) of the shortest kind, and the bound is {MAX_COMMAND_STEPS}: \
             a report this tree wrote could not be replayed",
            MAX_TRACE_BYTES / step
        );

        // At most: the model's node budget.
        //
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same trick the model's limit test uses.
        let (steps, nodes) = (MAX_COMMAND_STEPS, craftkit_model::limits::MAX_NODES);
        assert!(
            steps <= nodes,
            "a script may run {steps} step(s), and the model allows {nodes} node(s): \
             a script may ask for more work than building the largest design the model allows"
        );
    }
}
