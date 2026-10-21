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

/// A monotonic counter that stops rather than repeats.
///
/// Wrapping would let a document with 2^64 nodes mint an id that already
/// exists. So does repeating — and repeating is what a **saturating** counter
/// does: it hands out its last value for ever. The comment here used to say
/// that saturating "stops the document changing at all", and it did not: a
/// counter that repeats does not stop the document changing, it makes the
/// document **unbuildable**, because two nodes that share an id are a document
/// the file format refuses.
///
/// So the counter stops one id short of its ceiling: the last id it hands out
/// is `MAX - 1`, and after that it hands out nothing at all. §137.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdCounter(u64);

impl IdCounter {
    pub const MAX: u64 = (1 << 52) - 1;

    pub fn new(start: u64) -> Self {
        Self(start.min(Self::MAX))
    }

    /// The next id, or `None` when the counter cannot promise a fresh one.
    ///
    /// Named `next` because that is what an id allocator does; the lint is
    /// silenced because the type is not an iterator and cannot be used as one.
    ///
    /// The refusal is the point. A counter that hands out its last id twice
    /// breaks the one promise this type makes — *an id is unique across the
    /// whole document* — and the document it breaks is the one the file format
    /// refuses, so the failure lands on a command the user cannot fix. §137.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<u64> {
        if self.0 >= Self::MAX {
            return None;
        }
        let cur = self.0;
        self.0 += 1;
        Some(cur)
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
    fn ids_are_sequential_and_stop_at_the_ceiling() {
        let mut c = IdCounter::new(1);
        assert_eq!(c.next(), Some(1));
        assert_eq!(c.next(), Some(2));
        c.observe(IdCounter::MAX);
        assert_eq!(c.next(), None, "the counter must stop, not repeat");
        assert_eq!(c.next(), None, "and it must keep stopping");
    }

    /// The counter never hands out the same id twice.
    ///
    /// §137: it used to be *saturating*, which meant it handed out its last
    /// value for ever — and a document with two nodes that share an id is a
    /// document the file format refuses.
    #[test]
    fn the_counter_never_hands_out_the_same_id_twice() {
        let mut c = IdCounter::new(IdCounter::MAX - 1);
        assert_eq!(c.next(), Some(IdCounter::MAX - 1), "the last id it may hand out");
        assert_eq!(c.next(), None, "the counter must stop, not repeat");
        assert_eq!(c.next(), None, "and it must keep stopping");
        // And the same id cannot come back through `observe` either.
        c.observe(IdCounter::MAX - 1);
        assert_eq!(c.next(), None, "an observed id must not make the counter repeat");
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
