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

use std::sync::Arc;

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

    /// Whether two rectangles overlap.
    ///
    /// Touching is not overlapping: a rectangle that ends where another
    /// begins shares no area, and a front end that draws both of them
    /// has drawn a seam rather than an overlap.
    pub fn intersects(&self, other: Rect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    /// The point that **resizes** the rectangle.
    ///
    /// It is the corner opposite the one the rectangle is anchored at:
    /// a drag on it changes the size and keeps the place.
    ///
    /// The point is in the **kit** and not in a front end, because every
    /// front end that resizes a node has to put its handle in the same
    /// corner, and a handle that moved would resize the other way.
    pub fn resize_handle(&self) -> Point {
        Point::new(self.x + self.w, self.y)
    }

    /// The rectangle that a resize handle at this point would make.
    ///
    /// The **anchor** stays where it is: the rectangle grows and shrinks
    /// from its corner and does not walk across the world.
    ///
    /// A handle on the wrong side of the anchor gives an **empty**
    /// rectangle and not a negative one: a size is not a direction, and
    /// a rectangle with a negative width is a rectangle that cannot be
    /// drawn and cannot be picked.
    pub fn resized_to(&self, handle: Point) -> Rect {
        Rect::new(self.x, self.y, (handle.x - self.x).max(0.0), (handle.y - self.y).max(0.0))
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

    /// A colour written as `#rrggbb`.
    ///
    /// The way a person writes a colour, and the way a command takes
    /// one. It is in the **kit** and not in a plugin: every battery that
    /// takes a colour takes it this way, and a second spelling of the
    /// same thing is a second thing to get wrong.
    pub fn from_hex(text: &str) -> Result<Colour, String> {
        let text = text.trim();
        let Some(hex) = text.strip_prefix('#') else {
            return Err(format!("`{text}` is not a colour: a colour is `#rrggbb`"));
        };
        if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
            return Err(format!("`{text}` is not a colour: a colour is `#rrggbb`"));
        }
        let byte = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).unwrap_or(0);
        Ok(Colour([byte(0) as f32 / 255.0, byte(2) as f32 / 255.0, byte(4) as f32 / 255.0, 1.0]))
    }
}

/// How a thing looks.
///
/// The appearance is the **kit's own**, and not a plugin's payload: a
/// path has one, a rectangle has one, and the kit says what an
/// appearance is, so that two kinds can be given the same one.
///
/// Before this, every plugin had its own fields for its own colours —
/// the vector battery had a fill and a stroke in its payload, and
/// nothing else could have either. The appearance is the one place a
/// colour lives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    /// What the thing is filled with.
    pub fill: Colour,
    /// The line on its outline, if it has one.
    pub stroke: Option<Stroke>,
}

impl Appearance {
    /// An appearance that is filled with a colour and has no outline.
    pub fn filled(fill: Colour) -> Appearance {
        Appearance { fill, stroke: None }
    }
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
    ///
    /// The text takes a **colour**, like everything else that is
    /// drawn: a canvas that could only draw text in one colour was a
    /// canvas that could not draw a layer's text.
    fn text(&mut self, at: Point, text: &str, colour: Colour);
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

    fn text(&mut self, at: Point, text: &str, colour: Colour) {
        self.drawn.push(format!("text({}, {}, {text})", at.x, at.y));
        let _ = colour;
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

impl FormatValue for Appearance {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "fill": self.fill.to_json(),
            "stroke": self.stroke.map(|stroke| stroke.to_json()),
        })
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let fill = match value.get("fill") {
            Some(fill) if !fill.is_null() => Colour::from_json(fill)?,
            _ => Colour::NONE,
        };
        let stroke = match value.get("stroke") {
            Some(stroke) if !stroke.is_null() => Some(Stroke::from_json(stroke)?),
            _ => None,
        };
        Ok(Appearance { fill, stroke })
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

impl Node {
    /// Read a node, and the nodes under it, within the **depth** the
    /// kit allows.
    ///
    /// The reader is **recursive**, and a recursion that runs out of
    /// stack is a crash the panic guard cannot catch: the guard catches
    /// a panic, and a stack overflow is not a panic. So the depth is
    /// checked here, on the way down, and it is **the kit's number** —
    /// not the number the JSON parser happens to have.
    fn read_at(value: &serde_json::Value, depth: usize) -> Result<Node, String> {
        if depth > MAX_DEPTH {
            return Err(format!("the document is deeper than {MAX_DEPTH} levels"));
        }
        Node::read(value, depth)
    }
}

impl FormatValue for Node {
    fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({
            "id": self.id,
            "name": self.name,
            "kind": self.kind,
            "rect": self.rect.to_json(),
            "muted": self.muted,
            "hidden": self.hidden,
            "appearance": self.appearance.map(|appearance| appearance.to_json()),
            "appearance_from": self.appearance_from,
            "data": self.data,
            "children": self.children.iter().map(|child| child.to_json()).collect::<Vec<_>>(),
            "stream": self.stream,
        });
        // The fields this program does not understand are written back.
        crate::format::merge_unknown(&mut value, &self.unknown);
        value
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        Node::read_at(value, 0)
    }
}

/// The reading of a node, at a depth.
impl Node {
    /// Read a node and its children, at a known depth.
    fn read(value: &serde_json::Value, depth: usize) -> Result<Node, String> {
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
        // A node may have **no place**: a clip in a sound document is at a
        // time, not at a point in a picture. A file that says nothing about
        // a node's place, or says `null`, is read as a node with none.
        let rect = match value.get("rect") {
            Some(rect) => Option::<Rect>::from_json(rect)?,
            None => None,
        };
        let muted = value.get("muted").and_then(|muted| muted.as_bool()).unwrap_or(false);
        // A node may be **hidden**. A file that says nothing about it is
        // read as a node that is shown: the ninth version is the one
        // that writes the field, and a file before it has no word for
        // it.
        let hidden = value.get("hidden").and_then(|hidden| hidden.as_bool()).unwrap_or(false);
        // A node may have an **appearance** of its own, and it may point
        // at one that has. A file that says nothing about either is read
        // as a node with neither.
        let appearance = match value.get("appearance") {
            Some(appearance) if !appearance.is_null() => Some(Appearance::from_json(appearance)?),
            _ => None,
        };
        let appearance_from = value.get("appearance_from").and_then(|from| from.as_u64());
        let data = value.get("data").cloned().unwrap_or(serde_json::Value::Null);
        let children = match value.get("children") {
            Some(children) => children
                .as_array()
                .ok_or_else(|| "a node's children are a list".to_string())?
                .iter()
                .map(|child| Node::read_at(child, depth + 1).map(Arc::new))
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        let unknown = crate::format::keep_unknown(
            value,
            &[
                "id",
                "name",
                "kind",
                "rect",
                "muted",
                "hidden",
                "appearance",
                "appearance_from",
                "data",
                "children",
                "stream",
            ],
        );
        // A node may have a **stream**: the file its bytes are in. A node
        // that does not have one says so.
        let stream = value.get("stream").and_then(|stream| stream.as_str()).map(|path| path.to_string());
        Ok(Node {
            id,
            name,
            kind,
            rect,
            muted,
            hidden,
            appearance,
            appearance_from,
            data,
            children,
            unknown,
            stream,
        })
    }
}

/// The deepest a document's **tree** may be.
///
/// It is the kit's number and not the JSON parser's: serde_json refuses
/// a document nested deeper than a hundred and twenty-eight levels, and
/// that is a backstop, not a rule. A value that was **built** rather
/// than parsed — a command's raw parameter, a plugin's own JSON — is
/// not bounded by the parser at all, and the reader that walks it is
/// **recursive**: a recursion that runs out of stack is a crash the
/// panic guard cannot catch.
pub const MAX_DEPTH: usize = 64;

/// The most **nodes** a document may hold.
pub const MAX_NODES: usize = 10_000;

/// The most **relations** a document may hold.
pub const MAX_RELATIONS: usize = 10_000;

/// How much memory **one document** may hold.
///
/// Every other limit here is a **count**, and a count does not say how
/// much memory the thing counted holds: ten thousand nodes, each with a
/// payload, is a number no machine has. This is the number that says
/// how much memory a document takes, and it is the same number the
/// family uses — a document is a document in either tree.
///
/// It is measured with an **estimate** that never under-estimates: a
/// budget a document can lie about is not a budget.
pub const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;

/// A cheap, conservative estimate of the memory a JSON value holds.
///
/// It never under-estimates: the strings' **capacities** are counted and
/// not their lengths, and the lists' capacities are counted as well as
/// their contents.
pub fn estimated_bytes(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Null => 0,
        serde_json::Value::Bool(_) => std::mem::size_of::<bool>(),
        serde_json::Value::Number(_) => std::mem::size_of::<serde_json::Number>(),
        serde_json::Value::String(text) => text.capacity(),
        serde_json::Value::Array(items) => {
            items.capacity() * std::mem::size_of::<serde_json::Value>()
                + items.iter().map(estimated_bytes).sum::<usize>()
        }
        serde_json::Value::Object(table) => {
            table.len() * (std::mem::size_of::<String>() + std::mem::size_of::<serde_json::Value>())
                + table.iter().map(|(key, value)| key.capacity() + estimated_bytes(value)).sum::<usize>()
        }
    }
}

/// A **shared** node writes and reads as the node it holds.
///
/// The sharing is an implementation detail of the document: a file says
/// what a node is, not how many programs are holding it.
impl FormatValue for Arc<Node> {
    fn to_json(&self) -> serde_json::Value {
        (**self).to_json()
    }

    fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        Node::from_json(value).map(Arc::new)
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
    /// Where it is, if it is anywhere.
    ///
    /// A node in a picture is somewhere: it has a box. A node in a sound
    /// document is **at a time**, and a time is not a place — so the place
    /// is **optional**, and a node that has none says so.
    ///
    /// Before this, every node had a box, and the core said so: the field
    /// was not optional and a node had to have one. A host that added a node
    /// of a kind it did not know had to invent a box for it — a tenth of a
    /// unit square at the origin — and that invention was in the file. The
    /// port of a sound application is what found this, the way the port of
    /// a vector application found the payload.
    pub rect: Option<Rect>,
    /// Whether it is muted.
    pub muted: bool,
    /// Whether it is **hidden**.
    ///
    /// A hidden node is not drawn and not picked: it is in the
    /// document and not on the screen. Every drawing program has the
    /// idea — a layer that is turned off, a guide that is in the way,
    /// a shape that is not part of the picture yet — and before this
    /// the only way to have it was to take the node **out** of the
    /// document, which loses it.
    ///
    /// Hiding a **group** hides what is in it: the group is the thing
    /// that is hidden, and what is inside it is behind it.
    pub hidden: bool,
    /// How it looks, if it has an appearance of its own.
    ///
    /// It is **optional**: most things in a document are not seen — a
    /// sound clip is at a time and not at a place — and a thing with no
    /// appearance says so rather than pretending to be white.
    pub appearance: Option<Appearance>,
    /// The node whose appearance it uses, if it uses one.
    ///
    /// A **reference** and not a copy: a document that keeps twenty
    /// copies of the same blue has twenty places to change it, and one
    /// of them will be forgotten. The reference is followed once, by
    /// [`Document::appearance_of`].
    pub appearance_from: Option<u64>,
    /// The kind's own data.
    ///
    /// `null` when the kind has no payload. The core never looks inside it.
    pub data: serde_json::Value,
    /// The nodes under this one.
    ///
    /// A node with children is a **group**, and that is a structural fact
    /// rather than a kind: the core draws a group by drawing what is in
    /// it, and the group's own kind says what the group *means*.
    ///
    /// Before this, the document was a list — and a list cannot hold a
    /// group, a layer, or anything else that holds other things. The port
    /// of a real application found that, the way it found the payload.
    ///
    /// The children are **shared**: each one is behind an `Arc`, so a
    /// document that is copied — and the history copies one for every
    /// state — shares the parts that were not changed. That is what
    /// makes the undo cheap: § the session's history.
    pub children: Vec<Arc<Node>>,
    /// The fields of this node that the reader did **not** understand.
    ///
    /// A program that writes a node with a field this one does not know
    /// would have that field dropped, and the node would come back
    /// smaller. So the fields are **kept**, and written back as they were.
    pub unknown: serde_json::Value,
    /// Where this node's **stream** is, if it has one.
    ///
    /// A node has a **dictionary** — its payload — and it may also have a
    /// **stream**: bytes that belong to the node and are not JSON. That
    /// is the shape of a PDF object, and of a great many other formats
    /// that grew out of PostScript.
    ///
    /// The bytes are **not** in the document: JSON cannot hold them
    /// without making them a third bigger, and a document that carries
    /// fifty megabytes of streams would carry sixty-seven megabytes of
    /// text. They travel beside the document, in the **bundle**, and this
    /// field says which file is this node's stream.
    ///
    /// Before this there was nowhere to say it: the only answer was to
    /// put the bytes in the payload as text, or to leave them out and
    /// lose them.
    pub stream: Option<String>,
}

/// A **relation** between two nodes.
///
/// The document is a **tree**: a node holds other nodes, and that is
/// containment. A tree cannot say everything a real document has to say:
/// a clip plays a source and is not *in* the source, a track feeds a bus
/// and is not *in* the bus, a track is under a controller and is not *in*
/// it either.
///
/// Those are **relations**: a typed edge from one node to another. The
/// kind of the relation is a **string**, and not a variant, because the
/// kinds of relation are as open as the kinds of node: a sound document
/// has sources and routes and loops, a vector document has symbols and
/// instances, a document that carries its media has assets.
///
/// The relation is in the **document** and not in the node, because it is
/// between two nodes and a node does not know what it is attached to.
/// The port of a sound application is what found this: a mixer is a
/// graph, and a tree cannot be a mixer.
#[derive(Debug, Clone, PartialEq, Format)]
pub struct Relation {
    /// The node the relation goes **from**.
    pub from: u64,
    /// The node the relation goes **to**.
    pub to: u64,
    /// What the relation is: `source`, `route`, `vca`, `loop`, …
    pub kind: String,
    /// What the edge **says**.
    ///
    /// An edge is not always only an edge. A PDF's `/Resources 12 0 R`
    /// says which **key** the reference is under and which **generation**
    /// of the object it names; a symbol instance says the **transform** it
    /// was placed with; a track's send says the **level** it sends at.
    ///
    /// None of that fits in the kind — the kind is the *name* of the edge,
    /// and two edges of the same kind can say different things. So the
    /// edge carries a **payload**, and the core does not read it.
    ///
    /// Before this there was nowhere to put it, and a plugin had to
    /// squeeze it into the kind — `ref/Resources` — which made the kind
    /// mean two things at once.
    #[format(default)]
    pub payload: serde_json::Value,
}

impl Relation {
    /// A relation from one node to another.
    pub fn new(from: u64, to: u64, kind: impl Into<String>) -> Self {
        Relation { from, to, kind: kind.into(), payload: serde_json::Value::Null }
    }

    /// A relation that says something.
    pub fn with_payload(from: u64, to: u64, kind: impl Into<String>, payload: serde_json::Value) -> Self {
        Relation { from, to, kind: kind.into(), payload }
    }
}

/// A file the document needs.
///
/// The document **references** files: a sound document points at audio,
/// a vector document points at a font, a document that carries its
/// pictures points at them. A reference is not the file — it is the
/// document saying what it needs, so that a program can say "this
/// document is missing two things" instead of failing later, in the
/// middle of an edit.
///
/// The bytes are not in the document. They travel beside it, in a
/// **bundle**: one file with the document and the files it names.
#[derive(Debug, Clone, PartialEq, Format)]
pub struct Asset {
    /// Where the file is, relative to the document.
    pub path: String,
    /// What kind of file it is: `audio`, `font`, `image`, …
    ///
    /// A string, because the kinds are open.
    pub kind: String,
    /// How long it is, in bytes, if anybody knows.
    ///
    /// `None` when nobody does: a document that names a file it has not
    /// read does not know how long the file is, and saying so is better
    /// than guessing.
    pub length: Option<u64>,
}

impl Asset {
    /// A file the document needs.
    pub fn new(path: impl Into<String>, kind: impl Into<String>, length: Option<u64>) -> Self {
        Asset { path: path.into(), kind: kind.into(), length }
    }
}

/// The document.
///
/// The **compiler writes its format**: the two functions a file needs come
/// from the fields, so a field that is added is a field that is written.
#[derive(Debug, Clone, PartialEq, Format)]
pub struct Document {
    /// The document's name.
    pub name: String,
    /// The document's own payload.
    ///
    /// The **node** has a payload, and the document has one too: the same
    /// idea, one level up. A sound document's sample rate and timecode
    /// belong to the session rather than to any one node, and before this
    /// there was nowhere to put them that was not some node's data.
    ///
    /// The core carries it and does not read it, and a plugin owns the
    /// keys it writes: the sound battery keeps its session settings under
    /// one name, so two plugins cannot collide by accident.
    #[format(default)]
    pub data: serde_json::Value,
    /// Its nodes.
    ///
    /// The nodes are **shared** the same way the children are, so that
    /// copying a document is cheap and the history's states share what
    /// they have in common.
    pub nodes: Vec<Arc<Node>>,
    /// The relations between the nodes.
    ///
    /// The tree says what **holds** what; the relations say what is
    /// **attached** to what. A list, and not a map, because the order is
    /// the order they were made in and a document is written in the order
    /// it was built.
    #[format(default)]
    pub relations: Vec<Relation>,
    /// The files the document needs.
    ///
    /// The **references**, and not the files: a document that holds a
    /// sound says which sound it wants, and the bytes travel beside it in
    /// a bundle (§ the SDK's `bundle` module).
    ///
    /// Before this there was nowhere to say it: the media was in the
    /// nodes' payloads as paths, and a program that wanted to know what a
    /// document needed had to look in every payload and know what a path
    /// meant.
    #[format(default)]
    pub assets: Vec<Asset>,
    /// The fields of the file that the reader did **not** understand.
    ///
    /// A file written by a newer program may carry fields this one has
    /// never heard of. Dropping them would make the document come back
    /// **smaller** than it was, and the next program to open it would
    /// find the work gone. So they are kept, and written back as they
    /// were.
    ///
    /// The field is **merged** rather than written under its own name: the
    /// file keeps the shape it had.
    #[format(merge)]
    pub unknown: serde_json::Value,
}

impl Document {
    /// A cheap, conservative estimate of the memory this document holds.
    ///
    /// Never under-estimates, and it counts the **children**: a
    /// document's memory is its nodes' memory.
    pub fn estimated_bytes(&self) -> usize {
        let mut total = std::mem::size_of::<Document>()
            + self.name.capacity()
            + estimated_bytes(&self.data)
            + self.nodes.capacity() * std::mem::size_of::<Arc<Node>>()
            + self.relations.capacity() * std::mem::size_of::<Relation>()
            + self
                .relations
                .iter()
                .map(|relation| relation.kind.capacity() + estimated_bytes(&relation.payload))
                .sum::<usize>()
            + self.assets.capacity() * std::mem::size_of::<Asset>()
            + self.assets.iter().map(|asset| asset.path.capacity() + asset.kind.capacity()).sum::<usize>();
        for node in &self.nodes {
            total += node.estimated_bytes();
        }
        total
    }

    /// Check the document against the kit's **own** bounds.
    ///
    /// The depth, the number of nodes and the number of relations are
    /// the shape of a document, and the shape is the kit's business: a
    /// program that reads a file and a program that writes one must
    /// agree on what a document may hold, or the round trip that decides
    /// which version to write (§57) cannot mean anything.
    pub fn check_bounds(&self) -> Result<(), String> {
        let bytes = self.estimated_bytes();
        if bytes > MAX_DOCUMENT_BYTES {
            return Err(format!("the document is {bytes} bytes, and the limit is {MAX_DOCUMENT_BYTES}"));
        }
        if self.relations.len() > MAX_RELATIONS {
            return Err(format!(
                "the document holds {} relations, and the limit is {MAX_RELATIONS}",
                self.relations.len()
            ));
        }
        let mut stack: Vec<(&Node, usize)> = self.nodes.iter().rev().map(|node| (&**node, 1usize)).collect();
        let mut nodes = 0usize;
        while let Some((node, depth)) = stack.pop() {
            nodes += 1;
            if nodes > MAX_NODES {
                return Err(format!("the document holds more than {MAX_NODES} nodes"));
            }
            if depth > MAX_DEPTH {
                return Err(format!("the document is deeper than {MAX_DEPTH} levels"));
            }
            for child in node.children.iter().rev() {
                stack.push((&**child, depth + 1));
            }
        }
        Ok(())
    }

    /// An empty document with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Document {
            name: name.into(),
            data: serde_json::Value::Null,
            nodes: Vec::new(),
            relations: Vec::new(),
            assets: Vec::new(),
            unknown: serde_json::Value::Null,
        }
    }

    /// The next free id.
    ///
    /// It walks the **whole tree**, not only the top level: a child has an
    /// id too, and an id that a child already holds is not free. Before
    /// this, the tree was a list, and a list is one level deep, and the
    /// count was right by accident. The port of a sound document found it:
    /// a track holds a playlist, a playlist holds a clip, and the clip
    /// took the id the next top-level node was going to have.
    pub fn next_id(&self) -> u64 {
        self.walk().map(|node| node.id).max().unwrap_or(0) + 1
    }

    /// Add a node at the top level, and give back its id.
    pub fn add_node(&mut self, name: impl Into<String>, kind: impl Into<String>, rect: Option<Rect>) -> u64 {
        self.add_node_with(name, kind, rect, serde_json::Value::Null)
    }

    /// Add a node under another one, and give back its id.
    ///
    /// The parent is a node's id. `None` means the top level.
    pub fn add_node_under(
        &mut self,
        parent: Option<u64>,
        name: impl Into<String>,
        kind: impl Into<String>,
        rect: Option<Rect>,
    ) -> u64 {
        self.add_node_with_under(parent, name, kind, rect, serde_json::Value::Null)
    }

    /// Add a node with a payload under another one, and give back its id.
    pub fn add_node_with_under(
        &mut self,
        parent: Option<u64>,
        name: impl Into<String>,
        kind: impl Into<String>,
        rect: Option<Rect>,
        data: serde_json::Value,
    ) -> u64 {
        let id = self.next_id();
        let node = Node {
            id,
            name: name.into(),
            kind: kind.into(),
            rect,
            muted: false,
            hidden: false,
            appearance: None,
            appearance_from: None,
            data,
            children: Vec::new(),
            unknown: serde_json::Value::Null,
            stream: None,
        };
        match parent.and_then(|parent| self.node_mut(parent)) {
            Some(parent) => parent.children.push(Arc::new(node)),
            None => self.nodes.push(Arc::new(node)),
        }
        id
    }

    /// Take a node out of the document, and give it back.
    ///
    /// The node goes wherever it is — at the top level or under another
    /// one — and it comes back whole, with whatever is in it.
    pub fn take_node(&mut self, id: u64) -> Option<Node> {
        if let Some(index) = self.nodes.iter().position(|node| node.id == id) {
            return Some(unwrap(self.nodes.remove(index)));
        }
        for node in self.nodes.iter_mut() {
            if let Some(taken) = Arc::make_mut(node).take_child(id) {
                return Some(taken);
            }
        }
        None
    }

    /// The **bounds** of the document: the box that holds everything
    /// that is somewhere.
    ///
    /// It is the union of the places of the nodes, and a node with no
    /// place is in none of it: a clip in a sound document is at a time
    /// and not at a place.
    ///
    /// A document with nothing in it has **no** bounds, and the caller
    /// says what to do about that.
    pub fn bounds(&self) -> Option<Rect> {
        let mut bounds: Option<Rect> = None;
        for node in self.walk() {
            let Some(rect) = node.rect else { continue };
            bounds = Some(match bounds {
                None => rect,
                Some(current) => Rect::new(
                    current.x.min(rect.x),
                    current.y.min(rect.y),
                    (current.x + current.w).max(rect.x + rect.w) - current.x.min(rect.x),
                    (current.y + current.h).max(rect.y + rect.h) - current.y.min(rect.y),
                ),
            });
        }
        bounds
    }

    /// The nodes at the top level.
    pub fn roots(&self) -> &[Arc<Node>] {
        &self.nodes
    }

    /// Bring a node to the **front**: the last of its brothers.
    ///
    /// The **order of the tree is the order of the drawing**: a node
    /// that is drawn later is on top, and the picture and the picking
    /// agree because both walk the tree in this order.
    ///
    /// So the order is not a number a node carries: it is where the
    /// node is among its brothers, and bringing a node to the front is
    /// moving it to the end of the list it is in.
    ///
    /// A node that is the only one of its brothers is already at the
    /// front, and the command says so rather than doing nothing in
    /// silence.
    pub fn bring_to_front(&mut self, id: u64) -> bool {
        self.reorder(id, true)
    }

    /// Send a node to the **back**: the first of its brothers.
    pub fn send_to_back(&mut self, id: u64) -> bool {
        self.reorder(id, false)
    }

    /// Move a node to the front or to the back of its brothers.
    fn reorder(&mut self, id: u64, front: bool) -> bool {
        if let Some(index) = self.nodes.iter().position(|node| node.id == id) {
            let node = unwrap(self.nodes.remove(index));
            let node = Arc::new(node);
            if front {
                self.nodes.push(node);
            } else {
                self.nodes.insert(0, node);
            }
            return true;
        }
        for node in self.nodes.iter_mut() {
            let parent = Arc::make_mut(node);
            let Some(index) = parent.children.iter().position(|child| child.id == id) else { continue };
            let child = unwrap(parent.children.remove(index));
            let child = Arc::new(child);
            if front {
                parent.children.push(child);
            } else {
                parent.children.insert(0, child);
            }
            return true;
        }
        false
    }

    /// Every node, in the order they are drawn: a node, then what is in it.
    pub fn walk(&self) -> Descendants<'_> {
        Descendants { stack: self.nodes.iter().rev().map(|node| &**node).collect() }
    }

    /// The nodes that are **not hidden**, in the order they are in.
    ///
    /// A hidden node is not drawn and not picked, and what is inside a
    /// hidden group is behind it: the group is the thing that is
    /// hidden, so the walk does not go into it.
    ///
    /// This is the walk a front end draws with and the walk it picks
    /// with, so that "hidden" means one thing and not two.
    pub fn walk_visible(&self) -> Visible<'_> {
        Visible { document: self, stack: self.nodes.iter().rev().map(|node| &**node).collect() }
    }

    /// Whether a node is **shown**.
    ///
    /// A node is shown when it is not hidden itself and when the layer
    /// it is on is not hidden: a layer that is turned off takes its
    /// members with it, and that is what turning a layer off means.
    pub fn is_visible(&self, node: &Node) -> bool {
        if node.hidden {
            return false;
        }
        self.layer_of(node).map(|layer| !layer.hidden).unwrap_or(true)
    }

    /// Take a node out of the document, and say whether it was there.
    ///
    /// The node goes wherever it is — at the top level or under another
    /// one — and its children go with it.
    pub fn remove_node(&mut self, id: u64) -> bool {
        if let Some(index) = self.nodes.iter().position(|node| node.id == id) {
            self.nodes.remove(index);
            return true;
        }
        for node in self.nodes.iter_mut() {
            if Arc::make_mut(node).remove_child(id) {
                return true;
            }
        }
        false
    }

    /// Add a node with a payload, and give back its id.
    ///
    /// The payload is the kind's own data. The core carries it and does not
    /// look at it; the kind reads it and writes it.
    pub fn add_node_with(
        &mut self,
        name: impl Into<String>,
        kind: impl Into<String>,
        rect: Option<Rect>,
        data: serde_json::Value,
    ) -> u64 {
        let id = self.next_id();
        self.nodes.push(Arc::new(Node {
            id,
            name: name.into(),
            kind: kind.into(),
            rect,
            muted: false,
            hidden: false,
            appearance: None,
            appearance_from: None,
            data,
            children: Vec::new(),
            unknown: serde_json::Value::Null,
            stream: None,
        }));
        id
    }

    /// The nodes, in order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().map(|node| &**node)
    }

    /// Add a relation between two nodes, and say whether it was new.
    ///
    /// A relation is **not** added twice: the same edge with the same
    /// kind is the same relation, and a document that says it twice is a
    /// document that says two things about one edge.
    pub fn add_relation(&mut self, relation: Relation) -> bool {
        if self.relations.contains(&relation) {
            return false;
        }
        self.relations.push(relation);
        true
    }

    /// Take the relations that match away, and say how many there were.
    ///
    /// The matcher is a closure, so a caller may leave out any part of the
    /// relation it does not care about: remove every relation of a kind,
    /// or every relation from a node, or one exact edge.
    pub fn remove_relations(&mut self, matches: impl Fn(&Relation) -> bool) -> usize {
        let before = self.relations.len();
        self.relations.retain(|relation| !matches(relation));
        before - self.relations.len()
    }

    /// The relations from a node, of a kind if one is named.
    pub fn relations_from(&self, node: u64, kind: Option<&str>) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|relation| relation.from == node)
            .filter(|relation| kind.is_none_or(|kind| relation.kind == kind))
            .collect()
    }

    /// Say that the document needs a file.
    ///
    /// The same file twice is the same file: a document that names a
    /// file twice is a document that says two things about one file.
    pub fn add_asset(&mut self, asset: Asset) -> bool {
        if self.assets.iter().any(|known| known.path == asset.path) {
            return false;
        }
        self.assets.push(asset);
        true
    }

    /// Say that the document does not need a file any more.
    ///
    /// It gives back how many there were, and a document that did not
    /// need the file is not changed.
    pub fn remove_assets(&mut self, matches: impl Fn(&Asset) -> bool) -> usize {
        let before = self.assets.len();
        self.assets.retain(|asset| !matches(asset));
        before - self.assets.len()
    }

    /// The files the document needs that are not in `available`.
    ///
    /// The paths, in the order the document names them.
    pub fn missing_assets(&self, available: &[String]) -> Vec<String> {
        self.assets
            .iter()
            .filter(|asset| !available.contains(&asset.path))
            .map(|asset| asset.path.clone())
            .collect()
    }

    /// The files the nodes' streams need that are not among the ones
    /// that are there.
    ///
    /// A node's stream is a file, and a document that names a file it has
    /// not got is a document that is not finished. This is the same
    /// question the assets answer, asked about the streams.
    pub fn missing_streams(&self, available: &[String]) -> Vec<String> {
        let mut missing: Vec<String> = Vec::new();
        for node in self.walk() {
            let Some(path) = node.stream.clone() else { continue };
            if available.contains(&path) || missing.contains(&path) {
                continue;
            }
            missing.push(path);
        }
        missing
    }

    /// The relations to a node, of a kind if one is named.
    pub fn relations_to(&self, node: u64, kind: Option<&str>) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|relation| relation.to == node)
            .filter(|relation| kind.is_none_or(|kind| relation.kind == kind))
            .collect()
    }

    /// The node with an id, wherever it is.
    pub fn node(&self, id: u64) -> Option<&Node> {
        self.walk().find(|node| node.id == id)
    }

    /// The **appearance** of a node: its own, or the one it points at.
    ///
    /// The reference is followed **once**: a chain of appearances is a
    /// document nobody can follow, and one hop is enough for a style
    /// that shapes use.
    ///
    /// A node with neither has **no** appearance, and the caller says
    /// what to do about that — a kind that has an appearance of its own
    /// in its payload (the vector battery's fill and stroke, before the
    /// appearance existed) keeps it.
    pub fn appearance_of(&self, node: &Node) -> Option<Appearance> {
        if node.appearance.is_some() {
            return node.appearance;
        }
        if let Some(from) = node.appearance_from
            && let Some(appearance) = self.node(from).and_then(|node| node.appearance)
        {
            return Some(appearance);
        }
        // And the **layer's**: a node with no appearance of its own, on
        // a layer that has one, is drawn as the layer says. The layer
        // is followed **once**: a layer on a layer is not a chain to
        // walk.
        self.layer_of(node).and_then(|layer| layer.appearance)
    }

    /// The **layer** a node is on, if it is on one.
    ///
    /// The membership is an **edge** — `core.on_layer` — and the layer
    /// is the node at the other end of it.
    ///
    /// A node is on **one** layer: the edge says which, and a second
    /// edge would be two answers to one question. So the first edge is
    /// the one that is read.
    pub fn layer_of(&self, node: &Node) -> Option<&Node> {
        let relations = self.relations_from(node.id, Some(crate::layer::ON_LAYER));
        let relation = relations.first()?;
        self.node(relation.to)
    }

    /// The **layers** of the document: the nodes that things are put on.
    pub fn layers(&self) -> Vec<&Node> {
        let mut found: Vec<&Node> = Vec::new();
        for relation in self.relations.iter().filter(|relation| relation.kind == crate::layer::ON_LAYER) {
            if let Some(layer) = self.node(relation.to)
                && !found.iter().any(|found| found.id == layer.id)
            {
                found.push(layer);
            }
        }
        found
    }

    /// The node with an id, wherever it is, to change.
    pub fn node_mut(&mut self, id: u64) -> Option<&mut Node> {
        // The node is **made mutable** on the way: a node that is shared
        // with a state in the history is copied first, so that the state
        // keeps what it was. That is the whole of copy-on-write, and it
        // is why copying a document is cheap and changing one is not.
        fn find(nodes: &mut [Arc<Node>], id: u64) -> Option<&mut Node> {
            for node in nodes.iter_mut() {
                if node.id == id {
                    return Some(Arc::make_mut(node));
                }
                if let Some(found) = find(&mut Arc::make_mut(node).children, id) {
                    return Some(found);
                }
            }
            None
        }
        find(&mut self.nodes, id)
    }

    /// The node with a name, wherever it is.
    pub fn node_by_name(&self, name: &str) -> Option<&Node> {
        self.walk().find(|node| node.name == name)
    }
}

/// Every node under another one, in the order they are drawn.
///
/// A node comes before what is in it, and what is in a node comes before
/// what is in that.
pub struct Descendants<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for Descendants<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<&'a Node> {
        let node = self.stack.pop()?;
        for child in node.children.iter().rev() {
            self.stack.push(&**child);
        }
        Some(node)
    }
}

/// The nodes of a document that are **not hidden**.
pub struct Visible<'a> {
    /// The document the nodes are in: a node's **layer** is in it, and
    /// a layer that is hidden hides what is on it.
    document: &'a Document,
    /// The nodes still to come.
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for Visible<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<&'a Node> {
        loop {
            let node = self.stack.pop()?;
            if !self.document.is_visible(node) {
                // The node is hidden, and so is everything in it: the
                // walk does not go into a hidden group. And a node on a
                // hidden layer is hidden too.
                continue;
            }
            for child in node.children.iter().rev() {
                self.stack.push(&**child);
            }
            return Some(node);
        }
    }
}

impl Node {
    /// A cheap, conservative estimate of the memory this node holds.
    ///
    /// Never under-estimates: a budget a document can lie about is not a
    /// budget.
    pub fn estimated_bytes(&self) -> usize {
        let mut total = std::mem::size_of::<Node>()
            + self.name.capacity()
            + self.kind.capacity()
            + self.stream.as_ref().map_or(0, |stream| stream.capacity())
            + estimated_bytes(&self.data)
            + estimated_bytes(&self.unknown);
        // The children are walked with a stack and not by recursing: a
        // tree that is deep is a tree the kit has a bound for, and a
        // bound that is checked by overflowing the stack is not a bound.
        let mut stack: Vec<&Node> = self.children.iter().map(|child| &**child).collect();
        while let Some(child) = stack.pop() {
            total += std::mem::size_of::<Node>()
                + child.name.capacity()
                + child.kind.capacity()
                + child.stream.as_ref().map_or(0, |stream| stream.capacity())
                + estimated_bytes(&child.data)
                + estimated_bytes(&child.unknown)
                + child.children.capacity() * std::mem::size_of::<Arc<Node>>();
            for grandchild in &child.children {
                stack.push(grandchild);
            }
        }
        total
    }

    /// Put a node under this one.
    ///
    /// The sharing is the document's business: a caller that has a node
    /// in its hand gives it over, and does not know how the document
    /// holds it.
    pub fn add_child(&mut self, node: Node) {
        self.children.push(Arc::new(node));
    }

    /// Take a child out of this node, and give it back.
    pub fn take_child(&mut self, id: u64) -> Option<Node> {
        if let Some(index) = self.children.iter().position(|node| node.id == id) {
            return Some(unwrap(self.children.remove(index)));
        }
        for node in self.children.iter_mut() {
            if let Some(taken) = Arc::make_mut(node).take_child(id) {
                return Some(taken);
            }
        }
        None
    }

    /// Take a child out of this node, and say whether it was there.
    pub fn remove_child(&mut self, id: u64) -> bool {
        if let Some(index) = self.children.iter().position(|node| node.id == id) {
            self.children.remove(index);
            return true;
        }
        for node in self.children.iter_mut() {
            if Arc::make_mut(node).remove_child(id) {
                return true;
            }
        }
        false
    }

    /// The node and everything under it, in the order they are drawn.
    pub fn descendants(&self) -> Descendants<'_> {
        Descendants { stack: vec![self] }
    }
}

/// The node an `Arc` holds, copied out if it is shared.
///
/// A node that is still shared belongs to a state in the history, so it
/// is copied rather than taken: the state keeps what it was.
fn unwrap(node: Arc<Node>) -> Node {
    Arc::try_unwrap(node).unwrap_or_else(|shared| (*shared).clone())
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
    ///
    /// It is given the **document**, and that is the point: a kind that
    /// follows a reference — a shape that takes its colour from a style
    /// node, an entity that takes its colour from its layer — cannot
    /// follow it without the document, and a kind that cannot follow a
    /// reference is a kind whose every value is a copy.
    pub paint: fn(&Node, &Document, &mut dyn Canvas),
    /// Whether a point is inside it.
    ///
    /// It is given the **document**, and that is the point: a kind that
    /// follows a reference — an insert that draws the block it
    /// inserts, a shape that takes its size from a style — cannot
    /// follow it without the document, and a kind that cannot follow a
    /// reference is a kind whose every value is a copy.
    pub hit: fn(&Node, &Document, Point) -> bool,
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
