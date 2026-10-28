//! What a snapshot is, for a stack that keeps them.
//!
//! **The one home for the shape.** §278: the undo stack counted a
//! document's memory with the document's own method, so the stack could
//! only ever hold documents. That is the one thing a stack of snapshots
//! should not care about: a PDF editor's undo stack holds pages, a video
//! editor's holds timelines, and a vector illustrator's holds trees — and
//! all of them need the same stack.
//!
//! So the stack asks for one thing — *what does one of these cost?* — and
//! the thing it holds says.

/// A thing an undo stack can hold.
///
/// **Clone, and a size.** The clone is what the stack does when a step is
/// pushed ✓; the size is what the budget counts ✓ — and the budget is the
/// stack's reason for existing ✓ (§52: a count of entries is not a bound on
/// memory ✓).
pub trait Snapshot: Clone {
    /// What one snapshot holds, in bytes, as the budget counts it.
    ///
    /// A number, not a walk ✓: the stack counts a snapshot when it is
    /// pushed ✓, so dropping the oldest step is a subtraction ✓ and not a
    /// walk of the whole thing ✓.
    fn estimated_bytes(&self) -> usize;
}
