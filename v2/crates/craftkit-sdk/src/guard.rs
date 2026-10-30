//! The **guard**: what happens when code this tree does not own panics.
//!
//! # The rule
//!
//! A **contribution** — a kind, an evaluator, a command — is code that
//! craftkit does not own. When that code panics, the panic must not
//! take the program with it, and it must not leave anything behind that
//! is half-changed.
//!
//! There are two places where that matters, and they are the two trust
//! boundaries:
//!
//! 1. **A command.** `Session::execute` clones the document, runs the
//!    command under the guard, and on a panic **restores the pre-command
//!    document**. A command that panics is a command that did not happen.
//! 2. **An evaluation.** A kind's `evaluate` and an evaluator's
//!    `evaluate` are called under the guard too. A contribution that
//!    panics is **skipped**, and the **plan says so**: it carries the
//!    failure, the contribution's name and what it said when it
//!    panicked. A plan that is short says it is short, rather than
//!    pretending the document has no meaning.
//!
//! # What is not here
//!
//! The guard does not make the panic go away. It catches it, names it
//! and moves on: the program keeps running, the plan says what it could
//! not say, and the failure is visible instead of silent.

/// What a panic said.
///
/// A panic carries a payload: a string, or something else. The message
/// is what can be read out of it, and a payload that is not a string
/// gets a message that says so.
pub fn message_of(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    "it panicked, and it did not say why".to_string()
}

/// Run something under the guard.
///
/// The result is the value, or the panic's message. It is what the
/// command path and the evaluation path both use.
pub fn guard<T>(what: impl FnOnce() -> T) -> Result<T, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(what)) {
        Ok(value) => Ok(value),
        Err(panic) => Err(message_of(&panic)),
    }
}
