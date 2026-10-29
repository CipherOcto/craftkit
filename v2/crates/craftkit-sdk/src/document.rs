//! The document: what a command changes, and what a kind draws.
//!
//! # Small on purpose
//!
//! The document model is the one part of a tree like this that is *not* a
//! seam: it is the thing every plugin agrees about, and the design of it
//! wants thought that a spike cannot give. So it is small, and it is
//! **open**: a node names a **kind**, and the kind is contributed.
//!
//! Nothing in this file knows what a kind is. The core never matches on a
//! kind's name, never draws one and never hit-tests one: it asks the
//! registry. That is the whole of the open-kind seam, and it is why a
//! plugin can add a kind of thing to a document that was written before the
//! plugin existed.

use crate::contribution::Contribution;

/// A point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Across.
    pub x: f64,
    /// Down.
    pub y: f64,
}

impl Point {
    /// A point.
    pub const fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}

/// A rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Where it starts.
    pub x: f64,
    /// Where it starts.
    pub y: f64,
    /// How wide it is.
    pub w: f64,
    /// How tall it is.
    pub h: f64,
}

impl Rect {
    /// A rectangle.
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Rect { x, y, w, h }
    }

    /// Whether a point is inside.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x <= self.x + self.w && point.y >= self.y && point.y <= self.y + self.h
    }
}

/// A colour, at paint time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colour(pub [f32; 4]);

impl Colour {
    /// Black.
    pub const BLACK: Colour = Colour([0.0, 0.0, 0.0, 1.0]);
    /// White.
    pub const WHITE: Colour = Colour([1.0, 1.0, 1.0, 1.0]);
    /// Transparent.
    pub const NONE: Colour = Colour([0.0, 0.0, 0.0, 0.0]);
}

/// What a backend is given to draw with.
///
/// A trait, not an enum: a plugin that draws with its own backend does not
/// add a variant to the core.
pub trait Canvas {
    /// Draw a rectangle.
    fn rect(&mut self, rect: Rect, fill: Colour);
    /// Draw a line of text.
    fn text(&mut self, at: Point, text: &str);
}

/// A backend that records what it was asked to draw.
///
/// The spike's backend. A real application has a display list, a rasteriser
/// and a PDF writer, and the seam is the same.
#[derive(Debug, Clone, Default)]
pub struct RecordingCanvas {
    /// What was drawn.
    pub drawn: Vec<String>,
}

impl Canvas for RecordingCanvas {
    fn rect(&mut self, rect: Rect, fill: Colour) {
        self.drawn.push(format!("rect({}, {}, {}, {})", rect.x, rect.y, rect.w, rect.h));
        let _ = fill;
    }

    fn text(&mut self, at: Point, text: &str) {
        self.drawn.push(format!("text({}, {}, {text})", at.x, at.y));
    }
}

/// A node in the document.
///
/// The node names a **kind**. What that kind means — how it is drawn, what
/// it is hit by — is the kind's business, and the kind is contributed.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Its id.
    pub id: u64,
    /// Its name.
    pub name: String,
    /// The kind of thing it is.
    pub kind: String,
    /// Where it is.
    pub rect: Rect,
    /// Whether it is muted.
    pub muted: bool,
}

/// The document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The document's name.
    pub name: String,
    /// Its nodes.
    pub nodes: Vec<Node>,
}

impl Document {
    /// An empty document with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Document { name: name.into(), nodes: Vec::new() }
    }

    /// The next free id.
    pub fn next_id(&self) -> u64 {
        self.nodes.iter().map(|node| node.id).max().unwrap_or(0) + 1
    }

    /// Add a node, and give back its id.
    pub fn add_node(&mut self, name: impl Into<String>, kind: impl Into<String>, rect: Rect) -> u64 {
        let id = self.next_id();
        self.nodes.push(Node { id, name: name.into(), kind: kind.into(), rect, muted: false });
        id
    }

    /// The nodes, in order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    /// The node with an id.
    pub fn node(&self, id: u64) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// The node with an id, to change.
    pub fn node_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// The node with a name.
    pub fn node_by_name(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.name == name)
    }
}

/// One kind of node, as the ecosystem sees it.
///
/// The core owns the document and the order of the nodes; a plugin owns what
/// its own kind means. There is no `match` on a kind anywhere in the core.
#[derive(Clone, Copy)]
pub struct NodeKindSpec {
    /// The kind's name: what a node's `kind` field says.
    pub kind: &'static str,
    /// What it is called in a menu.
    pub label: &'static str,
    /// The layer that owns it.
    pub layer: &'static str,
    /// How it draws itself.
    pub paint: fn(&Node, &mut dyn Canvas),
    /// Whether a point is inside it.
    pub hit: fn(&Node, Point) -> bool,
}

// The link collects node kinds here.
inventory::collect!(crate::contribution::Registration<NodeKindSpec>);

impl fmt_debug::Debug for NodeKindSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NodeKindSpec").field("kind", &self.kind).field("layer", &self.layer).finish()
    }
}

impl Contribution for NodeKindSpec {
    fn id(&self) -> &'static str {
        self.kind
    }

    fn capability(&self) -> &'static str {
        "node kind"
    }

    fn layer(&self) -> &'static str {
        self.layer
    }
}

/// A tiny module so the `Debug` impl above can name one type.
mod fmt_debug {
    pub use std::fmt::Debug;
}
