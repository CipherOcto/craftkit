//! The group: a node that holds other nodes.
//!
//! # What a group is
//!
//! **Structural**, not a kind: a node with children is a group, and the
//! core walks into it. The group still has a kind — the kind says what
//! the group *means* (how it is evaluated, what it is called) — and this
//! module provides the plain one: a group that holds things and says
//! nothing about them.
//!
//! # Why the core has one
//!
//! Because a node's kind has to be in the registry: the document may only
//! hold kinds the program knows how to draw. A group is a kind of thing,
//! so it is a kind, and the plain one is the core's.
//!
//! A plugin may add kinds of group — a layer, a frame, a symbol — and the
//! core never matches on them.

use craftkit_derive::NodeKind;

use crate::document::{Canvas, Node, Point};

/// A group: nodes inside nodes.
///
/// The core walks into a group and draws what is in it. The group itself
/// draws nothing, and it is hit by nothing: what is in it is hit by its
/// own kinds, and the core asks them.
#[derive(NodeKind)]
#[node_kind(kind = "core.group", label = "Group", layer = "core")]
pub struct GroupKind;

impl GroupKind {
    /// How a group is drawn.
    ///
    /// Nothing: the core draws the group's children.
    fn paint(_node: &Node, _document: &crate::Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a group.
    ///
    /// Nothing: the core asks the kinds of the nodes in the group, and
    /// they answer for themselves.
    fn hit(_node: &Node, _document: &crate::document::Document, _point: Point) -> bool {
        false
    }
}
