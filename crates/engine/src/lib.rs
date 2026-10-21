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
//!    parameter object, anywhere, ever.

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
pub use session::{Outcome, Session};

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
}
