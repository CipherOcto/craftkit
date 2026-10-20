//! Identifiers.
//!
//! Two rules, both learned the hard way somewhere in this stack's history:
//!
//! 1. **Newtype the id.** A `DocumentId` cannot be passed where a `NodeId` is
//!    expected, so "did I mean the document or the node?" stops being a runtime
//!    question.
//! 2. **One counter per document, not per collection.** A single `next_id`
//!    means an id is unique across the whole document, so a log line can name
//!    any object without a kind prefix — and a copy from one collection to
//!    another cannot collide.

use serde::{Deserialize, Serialize};

/// A monotonic counter, saturating rather than wrapping.
///
/// Wrapping would let a document with 2^64 nodes mint an id that already exists.
/// Saturating stops the document changing at all, which is a far smaller lie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdCounter(u64);

impl IdCounter {
    pub const MAX: u64 = (1 << 52) - 1;

    pub fn new(start: u64) -> Self {
        Self(start.min(Self::MAX))
    }

    // Named `next` because that is what an id allocator does; the lint is
    // silenced because the type is not an iterator and cannot be used as one.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        let cur = self.0;
        if self.0 < Self::MAX {
            self.0 += 1;
        }
        cur
    }

    pub fn peek(&self) -> u64 {
        self.0
    }

    /// Raise the counter above `id`, so a hand-edited file cannot reuse one.
    pub fn observe(&mut self, id: u64) {
        if id >= self.0 {
            self.0 = id.saturating_add(1).min(Self::MAX);
        }
    }
}

/// An id within a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u64);

impl NodeId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "n{}", self.0)
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "d{}", self.0)
    }
}

/// The id of a document within a workspace. Always `d0` today; a newtype so
/// that widening it later is not a breaking change.
///
/// `Default` **is** derived here, unlike [`FrameRate`]: the derived default
/// `DocumentId(0)` is the valid first id, so there is nothing to be careful
/// about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sequential_and_saturate() {
        let mut c = IdCounter::new(1);
        assert_eq!(c.next(), 1);
        assert_eq!(c.next(), 2);
        c.observe(IdCounter::MAX);
        assert_eq!(c.next(), IdCounter::MAX);
        assert_eq!(c.next(), IdCounter::MAX, "must saturate, never wrap");
    }

    #[test]
    fn observe_never_lowers_the_counter() {
        let mut c = IdCounter::new(10);
        c.observe(3);
        assert_eq!(c.peek(), 10, "an older id must not rewind the counter");
        c.observe(40);
        assert_eq!(c.peek(), 41);
    }

    #[test]
    fn ids_round_trip_as_plain_numbers() {
        let j = serde_json::to_string(&NodeId(7)).unwrap();
        assert_eq!(j, "7");
        assert_eq!(serde_json::from_str::<NodeId>(&j).unwrap(), NodeId(7));
    }
}
