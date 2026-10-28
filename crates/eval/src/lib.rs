//! # craftkit-eval
//!
//! Evaluation: `(document, instant)` → [`Plan`].
//!
//! ## The dependency is sequential, and that is the whole point
//!
//! A *level* flows through the node list: a source sets it, a filter scales it,
//! a sink reads it and passes it on. So node `i`'s result depends on nodes
//! `0..i` **and** on the instant.
//!
//! That shape is why this crate owns an [`Evaluator`] rather than a free
//! function. Editing node 7 of 40 must not cost 40 evaluations, and dragging the
//! playhead across a static document must not cost more than the nodes that
//! actually move. [`Evaluator::evaluate`] reuses the longest unchanged prefix.
//!
//! ## What this crate does *not* do
//!
//! It does not validate hostile input. Every [`Value`] that reaches here was
//! built through `craftkit_core`'s checked constructors, and every command
//! parameter was read through [`craftkit_foundation::Params`]. Re-checking is not
//! defence in depth, it is a second, drifting copy of the rules.
//!
//! It does not decide what is *valid* either. A node whose `amount` is the
//! wrong type produces a [`Diagnostic`], not an error, and the plan still
//! carries on to the nodes behind it — one broken node must not hide the eleven
//! behind it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod evaluate;
pub mod plan;

pub use evaluate::{EvalError, Evaluator, MAX_LEVEL, START_LEVEL, evaluate_once, state_of};
pub use plan::{Diagnostic, Diagnostics, NodeState, Plan, Resolved, Severity};

/// How many diagnostics one evaluation collects before counting the rest.
///
/// A document with a broken property on every node would otherwise produce a
/// megabyte of messages *per frame*.
pub const MAX_DIAGNOSTICS: usize = 256;

/// The largest document this crate will evaluate.
///
/// Re-exported from `craftkit-model::limits` so an error message can quote one
/// number, and so the limit the evaluator enforces is visibly the same one the
/// document validator enforces.
pub const MAX_NODES: usize = craftkit_model::limits::MAX_NODES;
