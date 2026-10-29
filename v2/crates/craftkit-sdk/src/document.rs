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

use crate::Format;
use crate::contribution::Contribution;
use crate::format::FormatValue;

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

/// One segment of a path.
///
/// A line, or a cubic curve. Two kinds, because those are the two a
/// bezier path is made of, and because a backend that draws a path has to
/// know which it is drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    /// A straight line to a point.
    Line {
        /// Where it ends.
        to: Point,
    },
    /// A curve: two control points and an end.
    Cubic {
        /// The control point the curve leaves by.
        c1: Point,
        /// The control point the curve arrives by.
        c2: Point,
        /// Where it ends.
        to: Point,
    },
}

/// How a path's insides are decided.
///
/// A path can have more than one contour, and a contour can be inside
/// another one. The rule is what says which parts are inside the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    /// Inside where the winding number is not zero.
    ///
    /// The default: a contour inside another one makes a hole when the two
    /// run in opposite directions.
    #[default]
    NonZero,
    /// Inside where the winding number is odd.
    ///
    /// The rule that does not care which way round a contour runs: every
    /// contour inside another one is a hole, however deep.
    EvenOdd,
}

/// One contour of a path.
///
/// A contour is a start point, some segments, and whether it closes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubPath {
    /// Where it starts.
    pub start: Point,
    /// The segments, in order.
    pub segments: Vec<Segment>,
    /// Whether the last segment joins back to the first.
    pub closed: bool,
}

impl SubPath {
    /// The contour's points: where it starts, and where each segment ends.
    pub fn points(&self) -> Vec<Point> {
        let mut points = Vec::with_capacity(self.segments.len() + 1);
        points.push(self.start);
        for segment in &self.segments {
            points.push(segment.end());
        }
        points
    }

    /// The contour's last point.
    pub fn end(&self) -> Point {
        self.segments.last().map(|segment| segment.end()).unwrap_or(self.start)
    }
}

/// A path: contours, and the rule that says which parts are inside.
///
/// The path is **the shape**, and it is the SDK's: a plugin that draws a
/// curve builds one of these, and the backend decides how to draw it — a
/// rasteriser flattens it, a PDF writer writes it as a bezier, a recorder
/// counts its segments.
///
/// A path has more than one contour because a shape can have a hole, and a
/// hole is a contour inside another one. That is not a theory: it is what
/// a boolean operation produces, and the port of a real application is
/// what found it (§12).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    /// The contours, in order.
    pub subpaths: Vec<SubPath>,
    /// Which parts are inside.
    pub rule: FillRule,
}

impl Path {
    /// A path of one contour: straight lines through the points.
    pub fn polygon(points: &[Point]) -> Path {
        let mut segments = Vec::with_capacity(points.len());
        for point in points.iter().skip(1) {
            segments.push(Segment::Line { to: *point });
        }
        let start = points.first().copied().unwrap_or_default();
        Path { subpaths: vec![SubPath { start, segments, closed: true }], rule: FillRule::default() }
    }

    /// A path of one contour.
    pub fn single(subpath: SubPath) -> Path {
        Path { subpaths: vec![subpath], rule: FillRule::default() }
    }

    /// Whether the path has anything in it.
    pub fn is_empty(&self) -> bool {
        self.subpaths.iter().all(|subpath| subpath.segments.is_empty())
    }

    /// The path's points, contour by contour.
    pub fn points(&self) -> Vec<Vec<Point>> {
        self.subpaths.iter().map(|subpath| subpath.points()).collect()
    }
}

impl Segment {
    /// Where the segment ends.
    pub fn end(&self) -> Point {
        match self {
            Segment::Line { to } => *to,
            Segment::Cubic { c1: _, c2: _, to } => *to,
        }
    }

    /// The control points, if the segment is a curve.
    pub fn controls(&self) -> Option<(Point, Point)> {
        match self {
            Segment::Line { .. } => None,
            Segment::Cubic { c1, c2, .. } => Some((*c1, *c2)),
        }
    }
}

/// A stroke: a colour and a width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// The colour of the line.
    pub colour: Colour,
    /// How wide it is.
    pub width: f64,
}

/// What a backend is given to draw with.
///
/// A trait, not an enum: a plugin that draws with its own backend does not
/// add a variant to the core.
///
/// # The vocabulary is the SDK's
///
/// The trait is open for **backends** — anything can implement it — and
/// closed for **kinds**: the set of things a kind can ask for is what this
/// trait says, and every addition to it is a change to the SDK. A vector
/// document needs more than a rectangle and a line of text; it needs a
/// path, a stroke, a gradient, a clip and a transform, and each of those
/// is a fifth argument somewhere. That is the shape of the seam as it
/// stands, and it is written down rather than pretended away.
pub trait Canvas {
    /// Draw a rectangle.
    fn rect(&mut self, rect: Rect, fill: Colour);
    /// Draw a line of text.
    fn text(&mut self, at: Point, text: &str);
    /// Draw a path: filled, and stroked on its outline.
    fn path(&mut self, path: &Path, fill: Colour, stroke: Option<Stroke>);
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

    fn path(&mut self, path: &Path, fill: Colour, stroke: Option<Stroke>) {
        let segments: usize = path.subpaths.iter().map(|subpath| subpath.segments.len()).sum();
        let mut drawn = format!("path({segments} segment(s), {} contour(s)", path.subpaths.len());
        if path.rule == FillRule::EvenOdd {
            drawn.push_str(", even-odd");
        }
        if fill != Colour::NONE {
            drawn.push_str(", filled");
        }
        if stroke.is_some() {
            drawn.push_str(", stroked");
        }
        self.drawn.push(drawn);
    }
}

impl FormatValue for Point {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "x": self.x, "y": self.y })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let number = |name: &str| {
            value.get(name).and_then(|value| value.as_f64()).ok_or_else(|| format!("a point has no `{name}`"))
        };
        Ok(Point::new(number("x")?, number("y")?))
    }
}

impl FormatValue for Colour {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "r": self.0[0], "g": self.0[1], "b": self.0[2], "a": self.0[3] })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let number = |name: &str| {
            value
                .get(name)
                .and_then(|value| value.as_f64())
                .ok_or_else(|| format!("a colour has no `{name}`"))
        };
        Ok(Colour([number("r")? as f32, number("g")? as f32, number("b")? as f32, number("a")? as f32]))
    }
}

impl FormatValue for Stroke {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "colour": self.colour.to_json(), "width": self.width })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let colour = value.get("colour").ok_or_else(|| "a stroke has no colour".to_string())?;
        let width = value
            .get("width")
            .and_then(|width| width.as_f64())
            .ok_or_else(|| "a stroke has no width".to_string())?;
        Ok(Stroke { colour: Colour::from_json(colour)?, width })
    }
}

impl FormatValue for Segment {
    fn to_json(&self) -> serde_json::Value {
        match self {
            Segment::Line { to } => serde_json::json!({ "line": { "to": to.to_json() } }),
            Segment::Cubic { c1, c2, to } => serde_json::json!({
                "cubic": { "c1": c1.to_json(), "c2": c2.to_json(), "to": to.to_json() }
            }),
        }
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        if let Some(line) = value.get("line") {
            let to = line.get("to").ok_or_else(|| "a line has no end".to_string())?;
            return Ok(Segment::Line { to: Point::from_json(to)? });
        }
        if let Some(cubic) = value.get("cubic") {
            let c1 = cubic.get("c1").ok_or_else(|| "a curve has no first control point".to_string())?;
            let c2 = cubic.get("c2").ok_or_else(|| "a curve has no second control point".to_string())?;
            let to = cubic.get("to").ok_or_else(|| "a curve has no end".to_string())?;
            return Ok(Segment::Cubic {
                c1: Point::from_json(c1)?,
                c2: Point::from_json(c2)?,
                to: Point::from_json(to)?,
            });
        }
        Err("a segment is a line or a curve".to_string())
    }
}

impl FormatValue for FillRule {
    fn to_json(&self) -> serde_json::Value {
        serde_json::Value::String(
            match self {
                FillRule::NonZero => "non-zero",
                FillRule::EvenOdd => "even-odd",
            }
            .to_string(),
        )
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        match value.as_str() {
            Some("non-zero") | Some("nonzero") | None => Ok(FillRule::NonZero),
            Some("even-odd") | Some("evenodd") => Ok(FillRule::EvenOdd),
            Some(other) => Err(format!("`{other}` is not a fill rule")),
        }
    }
}

impl FormatValue for SubPath {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "start": self.start.to_json(),
            "segments": self.segments.iter().map(|segment| segment.to_json()).collect::<Vec<_>>(),
            "closed": self.closed,
        })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let start = value.get("start").ok_or_else(|| "a contour has no start".to_string())?;
        let segments = match value.get("segments") {
            Some(segments) => segments
                .as_array()
                .ok_or_else(|| "a contour's segments are a list".to_string())?
                .iter()
                .map(Segment::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        let closed = value.get("closed").and_then(|closed| closed.as_bool()).unwrap_or(false);
        Ok(SubPath { start: Point::from_json(start)?, segments, closed })
    }
}

impl FormatValue for Path {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "rule": self.rule.to_json(),
            "subpaths": self.subpaths.iter().map(|subpath| subpath.to_json()).collect::<Vec<_>>(),
        })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let rule = match value.get("rule") {
            Some(rule) => FillRule::from_json(rule)?,
            None => FillRule::NonZero,
        };
        let subpaths = match value.get("subpaths") {
            Some(subpaths) => subpaths
                .as_array()
                .ok_or_else(|| "a path's contours are a list".to_string())?
                .iter()
                .map(SubPath::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        Ok(Path { subpaths, rule })
    }
}

impl FormatValue for Rect {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "x": self.x, "y": self.y, "w": self.w, "h": self.h })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let number = |name: &str| {
            value
                .get(name)
                .and_then(|value| value.as_f64())
                .ok_or_else(|| format!("a rectangle has no `{name}`"))
        };
        Ok(Rect::new(number("x")?, number("y")?, number("w")?, number("h")?))
    }
}

impl FormatValue for Node {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "kind": self.kind,
            "rect": self.rect.to_json(),
            "muted": self.muted,
            "data": self.data,
        })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let id = value.get("id").and_then(|id| id.as_u64()).ok_or_else(|| "a node has no id".to_string())?;
        let name = value
            .get("name")
            .and_then(|name| name.as_str())
            .ok_or_else(|| "a node has no name".to_string())?
            .to_string();
        let kind = value
            .get("kind")
            .and_then(|kind| kind.as_str())
            .ok_or_else(|| "a node has no kind".to_string())?
            .to_string();
        let rect = value.get("rect").ok_or_else(|| "a node has no rectangle".to_string())?;
        let muted = value.get("muted").and_then(|muted| muted.as_bool()).unwrap_or(false);
        let data = value.get("data").cloned().unwrap_or(serde_json::Value::Null);
        Ok(Node { id, name, kind, rect: Rect::from_json(rect)?, muted, data })
    }
}

impl Node {
    /// The node as it is written.
    pub fn to_json(&self) -> serde_json::Value {
        <Self as FormatValue>::to_json(self)
    }

    /// The node as it is read.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        <Self as FormatValue>::from_json(value)
    }
}

/// A node in the document.
///
/// The node names a **kind**. What that kind means — how it is drawn, what
/// it is hit by — is the kind's business, and the kind is contributed.
///
/// The node also carries a **payload**: the kind's own data, as JSON. The
/// core does not know what it means, does not read it and does not write it
/// — it carries it, and the file carries it, so a document written by a
/// program that had the plugin is still readable by one that has it.
///
/// That is the seam a real document model needs and this one did not have.
/// A rectangle needs nothing beyond a rectangle; a path needs anchors, a
/// fill needs a colour model, and a text object needs a font. Before the
/// payload, the only way to say any of that was to add a field to the core
/// — which is how the tree this replaces ended up with a thirteen-armed
/// enum and three thousand match sites.
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
    /// The kind's own data.
    ///
    /// `null` when the kind has no payload. The core never looks inside it.
    pub data: serde_json::Value,
}

/// The document.
///
/// The **compiler writes its format**: the two functions a file needs come
/// from the fields, so a field that is added is a field that is written.
#[derive(Debug, Clone, PartialEq, Format)]
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
        self.add_node_with(name, kind, rect, serde_json::Value::Null)
    }

    /// Add a node with a payload, and give back its id.
    ///
    /// The payload is the kind's own data. The core carries it and does not
    /// look at it; the kind reads it and writes it.
    pub fn add_node_with(
        &mut self,
        name: impl Into<String>,
        kind: impl Into<String>,
        rect: Rect,
        data: serde_json::Value,
    ) -> u64 {
        let id = self.next_id();
        self.nodes.push(Node { id, name: name.into(), kind: kind.into(), rect, muted: false, data });
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
    /// What the node **means**, in the plan.
    ///
    /// The kind's own evaluation: it turns a node into a value, or says
    /// nothing, and the core never matches on a kind to work it out.
    pub evaluate: Option<fn(&Node, &Document) -> Option<serde_json::Value>>,
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
