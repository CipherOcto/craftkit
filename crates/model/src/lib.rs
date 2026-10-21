//! # craftkit-model
//!
//! The document: an ordered list of nodes, each with a property tree, and the
//! path grammar used to address anything inside it.
//!
//! Three rules shape this crate, and each was learned somewhere it hurt:
//!
//! 1. **A property is addressable by a path and by a stable id.** A path is for
//!    humans and scripts; an id survives a rename and a reorder. Commands take
//!    either.
//! 2. **Every keyframe time is a [`Tick`].** Never seconds. See `craftkit-core`.
//! 3. **The document is plain data.** It has no behaviour beyond validation, no
//!    I/O, and no reference to the engine. That is what makes undo a snapshot
//!    and the file format trivial.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod document;
pub mod fingerprint;
pub mod keyframe;
pub mod path;
pub mod property;

pub use document::{DocError, Document, DocumentMeta, Node, NodeKind};
pub use keyframe::{Easing, Key, Keyframes};
pub use path::{PathSeg, PropPath};
pub use property::{PropGroup, PropHint, PropNode, Property};

/// Hard limits on the document, checked on construction and on load.
///
/// These are the numbers that keep a hostile or corrupt file from reaching an
/// allocation. They are deliberately generous for a hand-authored document and
/// deliberately finite.
pub mod limits {
    /// Nodes in one document.
    pub const MAX_NODES: usize = 10_000;
    /// The fastest a document may run.
    ///
    /// The frame rate a document may carry. It used to be written in five
    /// places — the core's constructor, the `doc.range` command, that
    /// command's parameter doc, the UI's comment and this table's neighbour in
    /// the prose — and the two that decided anything disagreed: the reader
    /// accepted a thousand frames per second and the command accepted two
    /// hundred and forty, so a document could hold a rate no command could
    /// set. §89.
    ///
    /// The *other* check on a frame rate — that it is not zero, because the
    /// ticks per frame divide by it — lives in the core, next to the type.
    pub const MAX_FRAME_RATE: u32 = 1_000;
    /// Nesting depth of the **node graph**.
    ///
    /// How deep a node may sit below another node: the evaluator walks the
    /// graph and refuses to go deeper. It is **not** the depth of the property
    /// tree — a property is named by a path, and the path is the shorter
    /// limit: a property tree deeper than [`MAX_PATH_SEGMENTS`] holds
    /// properties no path can name, so it is refused instead. §85.
    pub const MAX_DEPTH: usize = 32;
    /// Properties in one node, across the whole tree.
    pub const MAX_PROPERTIES: usize = 50_000;
    /// Keyframes on one property.
    pub const MAX_KEYS: usize = 10_000;
    /// The longest property path, in segments.
    pub const MAX_PATH_SEGMENTS: usize = 16;
    /// The longest node name, in bytes.
    pub const MAX_NAME: usize = 255;
    /// The longest document note, in bytes.
    ///
    /// A note is a paragraph: it is what a person writes next to a design to
    /// say why the design is the way it is. It used to have **no** bound at
    /// all, and no way out either — the note was in the file and in the model
    /// and in nothing else, so it could not be read by anything that was not
    /// the file. A bound makes it readable: the note can be carried whole,
    /// and a file with a longer one is refused rather than half-kept. §110.
    pub const MAX_COMMENT_BYTES: usize = 4_096;
    /// The longest property path, in bytes.
    ///
    /// A path is made of names, and a name is at most [`MAX_NAME`] bytes, so a
    /// path of [`MAX_PATH_SEGMENTS`] segments is at most this long: the names
    /// plus the slashes between them.
    ///
    /// The number is here so that **every layer that carries a path agrees on
    /// how long one may be**. The grammar accepted a path of four kilobytes,
    /// the commands refused anything over 255 bytes, and the model did not
    /// limit a group's id at all — so a file could hold a property that the
    /// tree published a path to and no command would accept. §97.
    pub const MAX_PATH_BYTES: usize = MAX_PATH_SEGMENTS * (MAX_NAME + 1) - 1;
    /// How much memory one document may hold.
    ///
    /// Every other limit here is a **count**, and a count does not say how
    /// much memory the thing counted holds: 10 000 nodes × 50 000 properties
    /// × 10 000 keyframes is a number no machine has. The engine clones the
    /// whole document for every command and keeps 128 of them for undo, so
    /// this is the number that says what the process actually uses. §52.
    ///
    /// 32 MiB is far more than any hand-made plan needs and is small enough
    /// that a clone of it is quick.
    pub const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;
}

// The relationships between the limits matter: a "smaller" limit that is
// actually larger than the one it guards would make the larger one unreachable.
// Asserted with locals so clippy does not fold them into a constant comparison.
#[cfg(test)]
mod tests {
    #[test]
    fn the_limits_are_ordered_so_a_smaller_one_is_never_larger() {
        let (nodes, props) = (super::limits::MAX_NODES, super::limits::MAX_PROPERTIES);
        let (keys, segs, depth) =
            (super::limits::MAX_KEYS, super::limits::MAX_PATH_SEGMENTS, super::limits::MAX_DEPTH);
        assert!(nodes <= props, "the node limit must not exceed the property limit");
        assert!(keys <= props, "the keyframe limit must not exceed the property limit");
        assert!(segs <= depth, "a path may not be longer than the node graph is deep");
        // The property tree is bounded the other way round: it is **not**
        // deeper than a path, because a property deeper than a path can name
        // is a property no path can reach, and the tree is refused rather than
        // truncated. The bound is `MAX_PATH_SEGMENTS` itself, and
        // `property::tests::a_tree_deeper_than_a_path_is_refused_rather_than_truncated`
        // is where it is checked. §85.
    }
}
