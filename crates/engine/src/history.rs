//! The undo stack's budget in this tree's terms.
//!
//! **The stack itself is not here.** §280: the stack asks its snapshots one
//! question — *what does one of these cost?* — and its budget is the
//! caller's, so the stack lives in the foundation
//! ([`craftkit_foundation::history`]) and this module is what the tree's own
//! caller says the budget is.
//!
//! What a document is worth is the **model's** question — the model's limit
//! table owns `MAX_DOCUMENT_BYTES` — and the tree derives its budget from it
//! rather than choosing a number. §144: two caps with the same value and no
//! relation between them drift apart the moment one of them is changed.

pub use craftkit_foundation::history::{History, MAX_ENTRIES};

/// How many documents' worth of memory the undo and redo stacks may hold.
///
/// Two, because a session has to be able to undo more than one step, and
/// because the stacks are the only part of a session that can be given back.
/// §144.
///
/// The number is the same as it has always been — 64 MiB — and what it *means*
/// is now stated: **two documents**. It used to be written out as
/// `64 * 1024 * 1024`, which is a number somebody chose that happened to be
/// the same as the design file's cap — and §141 had already derived that one
/// from this budget. Two caps with the same value and no relation between
/// them are two caps that drift apart the moment one of them is changed.
pub const HISTORY_EXPANSION: usize = 2;

/// How much memory the undo and redo stacks together may hold.
///
/// The budget is in bytes, because bytes are what the memory is. When the
/// document is small this is never reached — 128 small snapshots are far
/// below it — and when the document is large the stack keeps fewer steps
/// rather than more memory. §52.
pub const MAX_HISTORY_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION;
