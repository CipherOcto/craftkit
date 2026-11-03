//! `craftkit-plugin-vector` — the **vector battery**.
//!
//! # What this is
//!
//! A port, into v2's seams, of the shape of a vector illustration program:
//! a **path** in the document, **shapes** that make paths, a **colour** on
//! a path, and a **plan** that says what the paths add up to.
//!
//! It is the first port of a real application into v2, and it is here
//! because the ports are how the seams get tested. Everything in this
//! crate is a plugin: it depends on the SDK and the macro, and on nothing
//! else, and an application that does not want a vector battery does not
//! link it in.
//!
//! # What it adds
//!
//! - a **layer**, `vector`;
//! - a **node kind**, `vector.path`, whose payload is a path, a fill and a
//!   stroke;
//! - **thirty-five commands**: `vector.rectangle`, `vector.ellipse` and
//!   `vector.polygon` make shapes, and `vector.star`, `vector.line` and
//!   `vector.spiral` make the shapes the tree makes that are not
//!   polygons; `vector.path.unite`,
//!   `vector.path.subtract`, `vector.path.intersect`,
//!   `vector.path.exclude`, `vector.path.simplify` and
//!   `vector.path.offset` work on whole paths; `vector.path.move` and
//!   `vector.path.handle` move a point of a path and set the handles of
//!   the curves that leave and arrive at it; `vector.path.average`
//!   puts the points of a path on one line, in one column, or all in
//!   one place; `vector.path.anchor.remove` takes an anchor away, and
//!   the contour becomes the stretches that are left; `vector.path.fill`
//!   and
//!   `vector.path.stroke` say what the path is painted with, and
//!   `vector.path.gradient` says what the colours are, and
//!   `vector.path.blend` says how the path mixes with what is under
//!   it; and `vector.path.grid`, `vector.path.mirror` and
//!   `vector.path.radial` make the copies of a path — in a grid, over a
//!   mirror and round a centre;
//!   `vector.path.join` and `vector.path.cap` say how the line's
//!   corners and ends are finished; `vector.path.reshape` grabs the
//!   path at a point and drags it, and the anchors around follow; `vector.path.smooth` makes a point
//!   of a path smooth, and `vector.path.corner` makes it a corner; `vector.path.transform` says where
//!   the path is put; `vector.path.outline` makes the shape a line
//!   covers into a path of its own; `vector.path.order` says which
//!   path is in front of the others; and `vector.path.rule` says which
//!   parts of it are inside;
//! - a **tool**, `tool.vector`;
//! - a **panel**, `panel.vector`;
//! - an **evaluator**, `vector.totalarea`.
//!
//! # The payload
//!
//! A node in v2 carries a rectangle and a kind. A path needs more than a
//! rectangle — it needs anchors, curves, a fill and a stroke — so the node
//! carries a **payload**: JSON the core carries and does not read, and the
//! kind reads and writes.
//!
//! The payload here is a [`VectorNode`], and it is written by the SDK's
//! `FormatValue` trait, so a file written by this plugin is read back by
//! this plugin and by nothing else — which is the arrangement the whole
//! design rests on.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod reshape;
mod shapes;

pub use shapes::{AddLine, AddSpiral, AddStar};

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel, Tool};
use craftkit_sdk::{
    BlendMode, Canvas, Cap, Colour, Document, EngineError, Fill, FillRule, FormatValue, Gradient,
    GradientGeometry, GradientKind, GradientStop, Join, Node, NodeRef, Number, Outcome, Path, Plan, PlanItem,
    Point, Rect, Segment, Session, Stroke, SubPath, Text,
};

use craftkit_vector_geom as geom;

/// The most **copies** one shape may be given.
///
/// A grid of a million copies is a million nodes in the document, and a
/// document that holds a million nodes is a document nobody can open.
/// The number is this battery's, and a shape that asks for more is
/// refused by name.
const MAX_COPIES: i64 = 1000;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;

/// The vector layer.
///
/// It needs the core, so it says so, and the ecosystem checks the graph
/// when the program starts.
#[derive(Layer)]
#[layer(id = "vector", name = "Vector", requires("core"))]
pub struct VectorLayer;

/// What a vector node carries.
///
/// The payload of a `vector.path` node: the path itself, the colour it is
/// filled with, the stroke on its outline, and the **transform** it is
/// placed with.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorNode {
    /// The outline.
    pub path: Path,
    /// What it is filled with: a colour, or a gradient.
    pub fill: Fill,
    /// The stroke on its outline, if it has one.
    pub stroke: Option<Stroke>,
    /// How the path mixes with what is under it.
    pub blend: BlendMode,
    /// The transform the path is placed with.
    pub transform: geom::Matrix,
}

impl VectorNode {
    /// The path **where it is**: the path with the transform put on it.
    ///
    /// The path in the payload is the path as it was drawn, and the
    /// transform says where that path is put — so the path the world
    /// sees is the transformed one.
    pub fn placed(&self) -> Path {
        if self.transform.is_identity() {
            return self.path.clone();
        }
        self.transform.transform_path(&self.path)
    }
}

impl FormatValue for VectorNode {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "path": self.path.to_json(),
            "fill": self.fill.to_json(),
            "stroke": self.stroke.as_ref().map(|stroke| stroke.to_json()),
            "blend": self.blend.label(),
            "transform": self.transform.to_json(),
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let path = value.get("path").ok_or_else(|| "a vector node has no path".to_string())?;
        let fill = match value.get("fill") {
            Some(fill) => Fill::from_json(fill)?,
            None => Fill::Solid(Colour::BLACK),
        };
        let stroke = match value.get("stroke") {
            Some(stroke) if !stroke.is_null() => Some(Stroke::from_json(stroke)?),
            _ => None,
        };
        // A path written before the **blend mode** was in the payload
        // has none, and a path that says nothing is laid over what is
        // under it.
        let blend = match value.get("blend").and_then(|blend| blend.as_str()) {
            Some(name) => {
                BlendMode::parse(name).ok_or_else(|| format!("there is no blend mode called `{name}`"))?
            }
            None => BlendMode::Normal,
        };
        // A path written before the **transform** was in the payload
        // has none, and a path with no transform is where it is.
        let transform = match value.get("transform") {
            Some(transform) if !transform.is_null() => geom::Matrix::from_json(transform)?,
            _ => geom::Matrix::identity(),
        };
        Ok(VectorNode { path: Path::from_json(path)?, fill, stroke, blend, transform })
    }
}

/// The path kind.
///
/// The kind owns the meaning of the payload: it draws it, it is hit by it,
/// and it says what it means.
#[derive(NodeKind)]
#[node_kind(kind = "vector.path", label = "Path", layer = "vector", evaluates)]
pub struct VectorPathKind;

impl VectorPathKind {
    /// How a path is drawn.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Ok(vector) = VectorNode::from_json(&node.data) else { return };
        // The **appearance** wins over the payload's own colours: it is
        // the kit's own, it is the one that can be **shared**, and a
        // path whose colours live in its own payload cannot be given
        // the same appearance as another path.
        let (fill, stroke, blend) = match document.appearance_of(node) {
            Some(appearance) => (appearance.fill, appearance.stroke, appearance.blend),
            None => (vector.fill.clone(), vector.stroke, vector.blend),
        };
        let fill = if node.muted { Fill::Solid(Colour::NONE) } else { fill };
        // The **transform** is where the path is put, and the canvas
        // draws the path it is given.
        canvas.path(&vector.placed(), fill, blend, stroke);
    }

    /// Whether a point is inside a path.
    ///
    /// The path is flattened to a polygon and the point is tested against
    /// it. A curve is tested by its samples, which is an approximation, and
    /// it is written down rather than hidden.
    fn hit(node: &Node, _document: &Document, point: Point) -> bool {
        let Ok(vector) = VectorNode::from_json(&node.data) else { return false };
        // The point is in the **world**, and the path is in its own
        // place: the point is moved back into the path's space before
        // it is tested.
        let Some(inverse) = vector.transform.inverse() else { return false };
        let local = inverse.apply(point);
        vector.path.subpaths.iter().any(|subpath| polygon_contains(&geom::flatten(subpath, 0.05), local))
    }

    /// What a path **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let vector = VectorNode::from_json(&node.data).ok()?;
        let segments: usize = vector.path.subpaths.iter().map(|subpath| subpath.segments.len()).sum();
        Some(serde_json::json!({
            "area": geom::area(&vector.placed()),
            "segments": segments,
            "contours": vector.path.subpaths.len(),
        }))
    }
}

/// The vector tool.
///
/// A tool is a gesture that runs a command. It is not a command: it has no
/// parameters of its own, it has an icon, and it says which command it
/// runs.
#[derive(Tool)]
#[tool(id = "tool.vector", label = "Path", layer = "vector", icon = "rect", command = "vector.rectangle")]
pub struct VectorTool;

/// Put a rectangle in the document.
#[derive(Command)]
#[command(
    id = "vector.rectangle",
    area = "vector",
    layer = "vector",
    icon = "rect",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddRectangle {
    /// Where it starts, across.
    pub x: Number,
    /// Where it starts, down.
    pub y: Number,
    /// How wide it is.
    pub width: Number,
    /// How tall it is.
    pub height: Number,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl AddRectangle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let rect = Rect::new(self.x.0, self.y.0, self.width.0, self.height.0);
        let path = Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.w, rect.y),
            Point::new(rect.x + rect.w, rect.y + rect.h),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("rectangle {}", rect.w),
        };
        let id = add_path(session, name, path, self.fill.as_ref())?;
        Ok(Outcome::changed(format!("added a rectangle `{id}`")).with_data(serde_json::json!({ "id": id })))
    }
}

/// Put an ellipse in the document.
///
/// The ellipse is four cubic curves, the way a circle has always been
/// drawn on a screen: the control points are the corner of the square the
/// circle is inscribed in, pulled in by the circle's constant.
#[derive(Command)]
#[command(
    id = "vector.ellipse",
    area = "vector",
    layer = "vector",
    icon = "ellipse",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddEllipse {
    /// Its centre, across.
    pub x: Number,
    /// Its centre, down.
    pub y: Number,
    /// How wide it is, across the centre.
    pub width: Number,
    /// How tall it is, down the centre.
    pub height: Number,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl AddEllipse {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = ellipse(self.x.0, self.y.0, self.width.0, self.height.0);
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("ellipse {}", self.width.0),
        };
        let id = add_path(session, name, path, self.fill.as_ref())?;
        Ok(Outcome::changed(format!("added an ellipse `{id}`")).with_data(serde_json::json!({ "id": id })))
    }
}

/// Put a regular polygon in the document.
#[derive(Command)]
#[command(
    id = "vector.polygon",
    area = "vector",
    layer = "vector",
    icon = "polygon",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddPolygon {
    /// Its centre, across.
    pub x: Number,
    /// Its centre, down.
    pub y: Number,
    /// How far its corners are from the centre.
    pub radius: Number,
    /// How many corners it has.
    pub sides: Number,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl AddPolygon {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sides = self.sides.0;
        if !sides.is_finite() || sides < 3.0 || sides > 64.0 {
            return Err(EngineError::Failed {
                id: "vector.polygon".to_string(),
                message: "a polygon has between three and sixty-four sides".to_string(),
            });
        }
        let path = polygon(self.x.0, self.y.0, self.radius.0, sides as usize);
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("polygon {sides}"),
        };
        let id = add_path(session, name, path, self.fill.as_ref())?;
        Ok(Outcome::changed(format!("added a polygon `{id}`")).with_data(serde_json::json!({ "id": id })))
    }
}

/// The panel that shows the vector commands.
#[derive(Panel)]
#[panel(
    id = "panel.vector",
    label = "Vector",
    layer = "vector",
    commands(
        "vector.rectangle",
        "vector.ellipse",
        "vector.polygon",
        "vector.path.unite",
        "vector.path.subtract",
        "vector.path.intersect",
        "vector.path.simplify",
        "vector.path.offset",
        "vector.path.gradient",
        "vector.path.blend",
        "vector.path.grid",
        "vector.path.mirror",
        "vector.path.radial"
    )
)]
pub struct VectorPanel;

/// The total area of every path in the document.
#[derive(Evaluator)]
#[evaluator(id = "vector.totalarea", label = "Total area", layer = "vector")]
pub struct TotalAreaEvaluator;

impl TotalAreaEvaluator {
    /// Add up the area of every path.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        let mut total = 0.0;
        for node in document.walk() {
            if node.kind != "vector.path" {
                continue;
            }
            let Ok(vector) = VectorNode::from_json(&node.data) else { continue };
            total += geom::area(&vector.path);
        }
        plan.items.push(PlanItem {
            node: 0,
            kind: "vector.totalarea".to_string(),
            value: serde_json::json!({ "area": total }),
        });
        plan
    }
}

/// Combine two paths into one.
///
/// The **union**: everything either path covers. The result is one node,
/// and the second node is gone.
#[derive(Command)]
#[command(
    id = "vector.path.unite",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct UnitePaths {
    /// The first path.
    ///
    /// Leave it out and the **selection** is used instead.
    pub first: Option<NodeRef>,
    /// The second path.
    ///
    /// Leave it out and the **selection** is used instead.
    pub second: Option<NodeRef>,
}

impl UnitePaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let (first, second) = two_of(session, self.first, self.second)?;
        combine(session, "vector.path.unite", first, second, geom::BoolOp::Union)
    }
}

/// Take one path from another.
///
/// The **difference**: what the first path covers and the second does not.
/// The result may have a hole, and a hole is a contour of its own.
#[derive(Command)]
#[command(
    id = "vector.path.subtract",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct SubtractPaths {
    /// The path to take from.
    ///
    /// Leave it out and the **selection** is used instead.
    pub first: Option<NodeRef>,
    /// The path to take away.
    ///
    /// Leave it out and the **selection** is used instead.
    pub second: Option<NodeRef>,
}

impl SubtractPaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let (first, second) = two_of(session, self.first, self.second)?;
        combine(session, "vector.path.subtract", first, second, geom::BoolOp::Difference)
    }
}

/// Keep what both paths cover.
///
/// The **intersection**: what the two paths share.
#[derive(Command)]
#[command(
    id = "vector.path.intersect",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct IntersectPaths {
    /// The first path.
    ///
    /// Leave it out and the **selection** is used instead.
    pub first: Option<NodeRef>,
    /// The second path.
    ///
    /// Leave it out and the **selection** is used instead.
    pub second: Option<NodeRef>,
}

impl IntersectPaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let (first, second) = two_of(session, self.first, self.second)?;
        combine(session, "vector.path.intersect", first, second, geom::BoolOp::Intersect)
    }
}

/// Take the points a path does not need.
///
/// The path is simplified: the points that a flat line would serve are
/// removed and the rest are left alone.
#[derive(Command)]
#[command(
    id = "vector.path.simplify",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct SimplifyPath {
    /// The path.
    pub node: NodeRef,
    /// How far the result may be from the original.
    pub tolerance: Option<Number>,
}

impl SimplifyPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let tolerance = match &self.tolerance {
            Some(tolerance) => tolerance.0,
            None => 0.05,
        };
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(EngineError::Failed {
                id: "vector.path.simplify".to_string(),
                message: "the tolerance is not a distance".to_string(),
            });
        }
        let mut vector = vector_of(session, self.node.0)?;
        vector.path = geom::simplify(&vector.path, tolerance);
        write_back(session, self.node.0, &vector)?;
        Ok(Outcome::changed("the path is simpler").with_data(serde_json::json!({ "id": self.node.0 })))
    }
}

/// **Close** a path: its last open contour is closed.
///
/// A contour that is open has a loose end, and a loose end is a place
/// where the paint of the path stops in the middle of nowhere. Closing
/// it joins the end back to the beginning.
///
/// A path with **no open contour** is refused: there is nothing to
/// close.
#[derive(Command)]
#[command(
    id = "vector.path.close",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct ClosePath {
    /// The path.
    pub node: NodeRef,
}

impl ClosePath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.node.0)?;
        let Some(subpath) = vector.path.subpaths.iter_mut().rev().find(|subpath| !subpath.closed) else {
            return Err(EngineError::Failed {
                id: "vector.path.close".to_string(),
                message: "the path has no open contour: there is nothing to close".to_string(),
            });
        };
        subpath.closed = true;
        write_back(session, self.node.0, &vector)?;
        Ok(Outcome::changed("the path is closed").with_data(serde_json::json!({ "id": self.node.0 })))
    }
}

/// **Turn** a path the other way round.
///
/// The contour runs backwards: it starts where it ended, and every
/// curve keeps its shape and turns the other way. The path is the same
/// shape and it winds the other way, which matters when the shape has
/// a **hole** in it or when the path is filled by a rule that says
/// which way it winds.
#[derive(Command)]
#[command(
    id = "vector.path.reverse",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct ReversePath {
    /// The path.
    pub node: NodeRef,
}

impl ReversePath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.node.0)?;
        let subpaths: Vec<SubPath> = vector.path.subpaths.iter().map(reverse_subpath).collect();
        vector.path.subpaths = subpaths;
        write_back(session, self.node.0, &vector)?;
        Ok(Outcome::changed("the path is turned round").with_data(serde_json::json!({ "id": self.node.0 })))
    }
}

/// **Add an anchor** to a path: a segment of a contour is split in two.
///
/// The anchor goes in at a place along the segment — `t`, from zero at
/// its beginning to one at its end — and the two halves are the same
/// curve: the shape of the path does not change, and the path has one
/// more point to move.
#[derive(Command)]
#[command(
    id = "vector.path.anchor.add",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct AddAnchor {
    /// The path.
    pub node: NodeRef,
    /// Which contour of the path.
    pub subpath: Option<Number>,
    /// Which segment of the contour.
    pub segment: Number,
    /// Where along the segment the anchor goes: from zero to one.
    pub t: Option<Number>,
}

impl AddAnchor {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.anchor.add".to_string();
        let index = self.segment.0;
        if !index.is_finite() || index < 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a segment is a number and not a negative one".to_string(),
            });
        }
        let t = self.t.map(|t| t.0).unwrap_or(0.5);
        if !t.is_finite() {
            return Err(EngineError::Failed {
                id,
                message: "a place along a segment is a number between zero and one".to_string(),
            });
        }
        let mut vector = vector_of(session, self.node.0)?;
        let which = match &self.subpath {
            Some(subpath) => subpath.0,
            None => 0.0,
        };
        if !which.is_finite() || which < 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a contour is a number and not a negative one".to_string(),
            });
        }
        let which = which as usize;
        let Some(subpath) = vector.path.subpaths.get(which).cloned() else {
            return Err(EngineError::Failed { id, message: format!("the path has no contour {which}") });
        };
        let split = geom::split_segment(&subpath, index as usize, t)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        vector.path.subpaths[which] = split;
        write_back(session, self.node.0, &vector)?;
        Ok(Outcome::changed("the path has one more anchor")
            .with_data(serde_json::json!({ "id": self.node.0, "subpath": which, "segment": index })))
    }
}

/// Take an **anchor** away.
///
/// The tree's rule: the segments that touch the anchor go with it, and
/// the contour **splits** into the runs that are left — the surviving
/// stretches stay separate rather than joining across the gap. A run
/// with fewer than two points is not a run, and it is gone; and a
/// contour that was closed is open afterwards.
///
/// The anchors are given one at a time: this tree has no selection,
/// and a selection is not a fact.
#[derive(Command)]
#[command(
    id = "vector.path.anchor.remove",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct RemoveAnchor {
    /// The path.
    pub node: NodeRef,
    /// Which contour of the path.
    pub subpath: Option<Number>,
    /// Which anchor of the contour.
    pub anchor: Number,
}

impl RemoveAnchor {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.anchor.remove".to_string();
        let index = self.anchor.0;
        if !index.is_finite() || index < 0.0 || index.fract() != 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "an anchor is a number and not a negative one".to_string(),
            });
        }
        let anchor = index as usize;
        let mut vector = vector_of(session, self.node.0)?;
        let which = match &self.subpath {
            Some(subpath) => subpath.0,
            None => 0.0,
        };
        if !which.is_finite() || which < 0.0 || which.fract() != 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a contour is a number and not a negative one".to_string(),
            });
        }
        let which = which as usize;
        let Some(subpath) = vector.path.subpaths.get(which).cloned() else {
            return Err(EngineError::Failed { id, message: format!("the path has no contour {which}") });
        };
        let anchors = subpath.segments.len() + 1;
        if anchor >= anchors {
            return Err(EngineError::Failed {
                id,
                message: format!("the contour has {anchors} anchors and not an anchor {anchor}"),
            });
        }
        // The **runs** that are left: the stretches that survive stay
        // separate rather than joining across the gap.
        let runs = without_anchor(&subpath, anchor);
        let mut contours = Vec::with_capacity(vector.path.subpaths.len() + runs.len());
        for (index, contour) in vector.path.subpaths.iter().enumerate() {
            if index != which {
                contours.push(contour.clone());
            }
        }
        for run in runs {
            contours.push(run);
        }
        vector.path.subpaths = contours;
        write_back(session, self.node.0, &vector)?;
        Ok(Outcome::changed("the path has one anchor less")
            .with_data(serde_json::json!({ "id": self.node.0, "subpath": which, "anchor": anchor })))
    }
}

/// The **runs** of a contour that are left when an anchor is taken away.
///
/// The segments that touch the anchor go with it: the one that arrives
/// at it and the one that leaves it.
///
/// A closed contour is a **cycle**, so one anchor away leaves **one**
/// stretch, and the edge that closed the contour becomes an edge of
/// its own. An open contour is a **line** of points, so one anchor
/// away leaves the stretches on either side of it — two stretches and
/// not one, because the tree keeps them apart rather than joining
/// across the gap.
///
/// A stretch with fewer than two points is not a stretch, and it is
/// gone.
fn without_anchor(subpath: &SubPath, anchor: usize) -> Vec<SubPath> {
    let segments = &subpath.segments;
    let count = segments.len();
    if subpath.closed {
        // A closed contour is a **cycle**, so taking one anchor away
        // leaves **one** stretch: the edge that closed the contour
        // becomes an edge of its own, and in this model that edge is a
        // straight line.
        let mut kept: Vec<Segment> = Vec::new();
        let mut start = subpath.start;
        // The stretch **after** the anchor.
        if anchor < count {
            start = segments[anchor].end();
            kept.extend_from_slice(&segments[anchor + 1..]);
        }
        // The edge that **closed** the contour: it touches the first
        // point and the last one, so it goes when the anchor is one of
        // them — and when it is there, it is a straight line.
        if anchor > 0 && anchor < count {
            let from = kept.last().map(|segment| segment.end()).unwrap_or(start);
            if from != subpath.start {
                kept.push(Segment::Line { to: subpath.start });
            }
        }
        // And the stretch **before** the anchor.
        if anchor > 1 {
            kept.extend_from_slice(&segments[..anchor - 1]);
        }
        if kept.is_empty() {
            return Vec::new();
        }
        return vec![SubPath { start, segments: kept, closed: false }];
    }
    // An open contour is a **line** of points, so taking one anchor
    // away leaves the stretches that are on either side of it.
    let mut runs: Vec<SubPath> = Vec::new();
    // The stretch **before** the anchor.
    if anchor >= 2 {
        runs.push(SubPath { start: subpath.start, segments: segments[..anchor - 1].to_vec(), closed: false });
    }
    // And the stretch **after** it.
    if anchor < count {
        let kept = segments[anchor + 1..].to_vec();
        if !kept.is_empty() {
            runs.push(SubPath { start: segments[anchor].end(), segments: kept, closed: false });
        }
    }
    runs
}

/// A contour the other way round.
///
/// The contour starts where it ended, and every curve keeps its shape:
/// the two control points change places.
fn reverse_subpath(subpath: &SubPath) -> SubPath {
    let mut points = Vec::with_capacity(subpath.segments.len() + 1);
    points.push(subpath.start);
    for segment in &subpath.segments {
        points.push(segment.end());
    }
    let mut segments = Vec::with_capacity(subpath.segments.len());
    for index in (0..subpath.segments.len()).rev() {
        let from = points[index];
        segments.push(match &subpath.segments[index] {
            Segment::Line { .. } => Segment::Line { to: from },
            Segment::Cubic { c1, c2, .. } => Segment::Cubic { c1: *c2, c2: *c1, to: from },
        });
    }
    SubPath { start: subpath.end(), segments, closed: subpath.closed }
}

/// Push a path out, or in.
///
/// The path is offset by a distance: out for a positive one, in for a
/// negative one. The curves stay curves.
#[derive(Command)]
#[command(
    id = "vector.path.offset",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct OffsetPath {
    /// The path.
    pub node: NodeRef,
    /// How far to push it.
    pub distance: Number,
}

impl OffsetPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.node.0)?;
        vector.path = geom::offset(&vector.path, self.distance.0).map_err(|error| EngineError::Failed {
            id: "vector.path.offset".to_string(),
            message: error.to_string(),
        })?;
        write_back(session, self.node.0, &vector)?;
        Ok(Outcome::changed("the path is offset").with_data(serde_json::json!({ "id": self.node.0 })))
    }
}

/// The **outline** of a path's line.
///
/// The line of a path covers an area, and the outline is that area as
/// a shape: a path in its own right, which may be filled, measured and
/// cut. The command makes a **new** path and leaves the one it was
/// given alone.
#[derive(Command)]
#[command(
    id = "vector.path.outline",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path"),
    mutates,
    undoable
)]
pub struct OutlinePath {
    /// The path whose line is outlined.
    pub path: NodeRef,
    /// What to call the new path.
    pub name: Option<Text>,
    /// The colour of the new path, as `#rrggbb`.
    ///
    /// Leave it out and the new path is filled with the colour of the
    /// line it came from.
    pub fill: Option<Text>,
}

impl OutlinePath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let vector = vector_of(session, self.path.0)?;
        let Some(stroke) = vector.stroke else {
            return Err(EngineError::Failed {
                id: "vector.path.outline".to_string(),
                message: format!("the path `{}` has no line to outline", self.path.0),
            });
        };
        // The **miter limit** is the kit's: four widths.
        let outline = geom::outline_stroke(&vector.path, stroke.width, stroke.cap, stroke.join, 4.0);
        let colour = match &self.fill {
            Some(fill) => parse_colour(&fill.0)?,
            None => stroke.colour,
        };
        let name = self.name.clone().unwrap_or_else(|| Text(format!("outline {}", self.path.0)));
        let node = VectorNode {
            path: outline.clone(),
            fill: Fill::Solid(colour),
            stroke: None,
            blend: BlendMode::Normal,
            transform: geom::Matrix::identity(),
        };
        let rect = bounds(&outline);
        let id =
            session.document_mut()?.add_node_with(name.0.clone(), "vector.path", Some(rect), node.to_json());
        Ok(Outcome::changed("the line has an outline")
            .with_data(serde_json::json!({ "id": id, "from": self.path.0 })))
    }
}

///
/// The points of a contour are the point it starts at and the point
/// each segment ends at, counting from zero. The handles of the curves
/// move with the point they belong to, as they do in the tree: an
/// anchor carries its two handles.
#[derive(Command)]
#[command(
    id = "vector.path.move",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Points"),
    mutates,
    undoable
)]
pub struct MovePoint {
    /// The path.
    pub path: NodeRef,
    /// Which contour of the path, counting from zero. The first one if
    /// it is left out.
    pub contour: Option<Number>,
    /// Which point of the contour, counting from zero.
    pub point: Number,
    /// Where the point goes: a point, `{"x": …, "y": …}`.
    pub to: Json,
}

impl MovePoint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let contour = index_of(self.contour, "the contour")
            .map_err(|reason| EngineError::Failed { id: "vector.path.move".to_string(), message: reason })?;
        let point = index_of(Some(self.point), "the point")
            .map_err(|reason| EngineError::Failed { id: "vector.path.move".to_string(), message: reason })?;
        let to = Point::from_json(&self.to).map_err(|reason| EngineError::Failed {
            id: "vector.path.move".to_string(),
            message: format!("where the point goes is not a point: {reason}"),
        })?;
        move_point(&mut vector.path, contour, point, to)
            .map_err(|reason| EngineError::Failed { id: "vector.path.move".to_string(), message: reason })?;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the point has moved")
            .with_data(serde_json::json!({ "id": self.path.0, "contour": contour, "point": point })))
    }
}

/// **Average** the points of a path.
///
/// The points that are averaged are given **one per line**, as
/// `contour:point`, because the tree's average works on the points that
/// are selected and this tree has no selection: a range is a fact and
/// a selection is not.
///
/// The **axis** says what is averaged: `horizontal` puts the points on
/// one line, `vertical` puts them in one column, and `both` puts them
/// all in one place. The default is `both`.
///
/// The tree's average works on the selected points of **several**
/// paths at once; here a command works on one path, because a command
/// works on one node.
#[derive(Command)]
#[command(
    id = "vector.path.average",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Points"),
    mutates,
    undoable
)]
pub struct AveragePoints {
    /// The path.
    pub path: NodeRef,
    /// The points to average, one per line, each `contour:point`.
    pub points: Text,
    /// The axis: `horizontal`, `vertical` or `both`. The default is
    /// `both`.
    pub axis: Option<Text>,
}

impl AveragePoints {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.average".to_string();
        let mut vector = vector_of(session, self.path.0)?;
        let axis = match &self.axis {
            Some(axis) => match axis.0.trim().to_ascii_lowercase().as_str() {
                "horizontal" => Axis::Horizontal,
                "vertical" => Axis::Vertical,
                "both" => Axis::Both,
                other => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{other}` is not an axis: an axis is `horizontal`, `vertical` or `both`"
                        ),
                    });
                }
            },
            None => Axis::Both,
        };
        // The **points**: one per line, each `contour:point`.
        let mut wanted: Vec<(usize, usize)> = Vec::new();
        for line in self.points.0.lines().map(|line| line.trim()).filter(|line| !line.is_empty()) {
            let Some((contour, point)) = line.split_once(':') else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{line}` is not a point: a point is `contour:point`"),
                });
            };
            let (Ok(contour), Ok(point)) = (contour.trim().parse::<usize>(), point.trim().parse::<usize>())
            else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{line}` is not a point: a point is `contour:point`"),
                });
            };
            if wanted.contains(&(contour, point)) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the point `{line}` is in the list twice"),
                });
            }
            wanted.push((contour, point));
        }
        if wanted.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "there are no points to average".to_string(),
            });
        }
        // The **centre**: the mean of the points that are averaged, and
        // it is worked out before any point moves.
        let mut sum = Point::new(0.0, 0.0);
        for (contour, point) in &wanted {
            let at = point_at(&vector.path, *contour, *point).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the path has no point {contour}:{point}"),
            })?;
            sum = Point::new(sum.x + at.x, sum.y + at.y);
        }
        let count = wanted.len() as f64;
        let centre = Point::new(sum.x / count, sum.y / count);
        for (contour, point) in &wanted {
            let at = point_at(&vector.path, *contour, *point).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the path has no point {contour}:{point}"),
            })?;
            let to = match axis {
                Axis::Horizontal => Point::new(at.x, centre.y),
                Axis::Vertical => Point::new(centre.x, at.y),
                Axis::Both => centre,
            };
            move_point(&mut vector.path, *contour, *point, to)
                .map_err(|reason| EngineError::Failed { id: id.clone(), message: reason })?;
        }
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed(format!("{} points are averaged", wanted.len())).with_data(
            serde_json::json!({
                "id": self.path.0,
                "points": wanted.iter().map(|(contour, point)| serde_json::json!([contour, point])).collect::<Vec<_>>(),
                "centre": { "x": centre.x, "y": centre.y },
                "axis": axis.name(),
            }),
        ))
    }
}

/// The **axis** a set of points is averaged along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// The points end up on one line: the mean of their heights.
    Horizontal,
    /// The points end up in one column: the mean of their widths.
    Vertical,
    /// The points end up in one place.
    Both,
}

impl Axis {
    /// The name of the axis.
    fn name(self) -> &'static str {
        match self {
            Axis::Horizontal => "horizontal",
            Axis::Vertical => "vertical",
            Axis::Both => "both",
        }
    }
}

/// Where a **point** of a contour is.
fn point_at(path: &Path, contour: usize, point: usize) -> Option<Point> {
    let subpath = path.subpaths.get(contour)?;
    if point == 0 { Some(subpath.start) } else { Some(subpath.segments.get(point - 1)?.end()) }
}

/// Set the **handles** of a point.
///
/// The two handles of a point are the one the curve leaves by and the
/// one the curve arrives by, and a point with handles on it is a
/// curve: a straight segment with a handle on it becomes a curve, as
/// it does in the tree.
#[derive(Command)]
#[command(
    id = "vector.path.handle",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Points"),
    mutates,
    undoable
)]
pub struct HandlePoint {
    /// The path.
    pub path: NodeRef,
    /// Which contour of the path, counting from zero. The first one if
    /// it is left out.
    pub contour: Option<Number>,
    /// Which point of the contour, counting from zero.
    pub point: Number,
    /// The handle the curve leaves the point by.
    pub c1: Option<Json>,
    /// The handle the curve arrives at the point by.
    pub c2: Option<Json>,
}

impl HandlePoint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let contour = index_of(self.contour, "the contour").map_err(|reason| EngineError::Failed {
            id: "vector.path.handle".to_string(),
            message: reason,
        })?;
        let point = index_of(Some(self.point), "the point").map_err(|reason| EngineError::Failed {
            id: "vector.path.handle".to_string(),
            message: reason,
        })?;
        let c1 = match &self.c1 {
            Some(value) => Some(Point::from_json(value).map_err(|reason| EngineError::Failed {
                id: "vector.path.handle".to_string(),
                message: format!("the handle is not a point: {reason}"),
            })?),
            None => None,
        };
        let c2 = match &self.c2 {
            Some(value) => Some(Point::from_json(value).map_err(|reason| EngineError::Failed {
                id: "vector.path.handle".to_string(),
                message: format!("the handle is not a point: {reason}"),
            })?),
            None => None,
        };
        if c1.is_none() && c2.is_none() {
            return Err(EngineError::Failed {
                id: "vector.path.handle".to_string(),
                message: "a handle says where the curve goes: c1, c2, or both".to_string(),
            });
        }
        handle_point(&mut vector.path, contour, point, c1, c2).map_err(|reason| EngineError::Failed {
            id: "vector.path.handle".to_string(),
            message: reason,
        })?;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the handles have moved")
            .with_data(serde_json::json!({ "id": self.path.0, "contour": contour, "point": point })))
    }
}

/// Set the **fill** of a path.
///
/// The fill is what is inside the path, and the rule that says what is
/// inside is the path's own.
#[derive(Command)]
#[command(
    id = "vector.path.fill",
    area = "vector",
    layer = "vector",
    icon = "paint",
    menu("Path", "Paint"),
    mutates,
    undoable
)]
pub struct FillPath {
    /// The path.
    pub path: NodeRef,
    /// The colour, as `#rrggbb`.
    pub colour: Text,
}

impl FillPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let colour = parse_colour(&self.colour.0)?;
        vector.fill = Fill::Solid(colour);
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the path is filled")
            .with_data(serde_json::json!({ "id": self.path.0, "fill": self.colour.0 })))
    }
}

/// Put a path **in front** of the others, or **behind** them.
///
/// The order the paths are drawn in is the order they are in the
/// document: the first is drawn first and is behind, and the last is
/// drawn last and is in front.
///
/// The tree's layers hold an ordered list of their own, and this
/// tree's layers are **edges** — so a path moved here moves among
/// everything, and not only among the paths of its own layer.
#[derive(Command)]
#[command(
    id = "vector.path.order",
    area = "vector",
    layer = "vector",
    icon = "list",
    menu("Path", "Order"),
    mutates,
    undoable
)]
pub struct OrderPath {
    /// The path.
    pub path: NodeRef,
    /// `front` or `back`.
    pub place: Text,
}

impl OrderPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.order".to_string();
        let node = session.document().node(self.path.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.path.0),
        })?;
        if node.kind != "vector.path" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a path", self.path.0, node.kind),
            });
        }
        let place = self.place.0.trim().to_ascii_lowercase();
        let moved = match place.as_str() {
            "front" => session.document_mut()?.bring_to_front(self.path.0),
            "back" => session.document_mut()?.send_to_back(self.path.0),
            other => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a path goes to the front or to the back, and not to `{other}`"),
                });
            }
        };
        if !moved {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is not in the document", self.path.0),
            });
        }
        Ok(Outcome::changed(format!("the path is {place}"))
            .with_data(serde_json::json!({ "id": self.path.0, "place": place })))
    }
}

/// Make a **grid** of copies of a path.
///
/// The tree's repeat tool makes a grid of copies, and the shape is the
/// first cell of it. Here the copies are **paths of their own** — the
/// same shape, the same paint, and a **transform** that says where each
/// one is — and the original is where it was.
///
/// The copies are placed by the step: how far apart they are, across
/// and down.
#[derive(Command)]
#[command(
    id = "vector.path.grid",
    area = "vector",
    layer = "vector",
    icon = "plus",
    menu("Path", "Grid"),
    mutates,
    undoable
)]
pub struct GridPath {
    /// The path to copy.
    pub path: NodeRef,
    /// How many copies across, the original among them.
    pub columns: Number,
    /// How many copies down, the original among them.
    pub rows: Number,
    /// How far apart they are, across.
    pub step_x: Number,
    /// How far apart they are, down.
    pub step_y: Number,
}

impl GridPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.grid".to_string();
        let vector = vector_of(session, self.path.0)?;
        let columns = the_count(self.columns.0, "columns", &id)?;
        let rows = the_count(self.rows.0, "rows", &id)?;
        let step_x = self.step_x.0;
        let step_y = self.step_y.0;
        if !step_x.is_finite() || !step_y.is_finite() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the step between the copies is a number".to_string(),
            });
        }
        let cells = columns.saturating_mul(rows);
        if cells > MAX_COPIES {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a grid holds at most {MAX_COPIES} copies, and this one holds {cells}"),
            });
        }
        let mut made: Vec<u64> = Vec::with_capacity(cells.saturating_sub(1) as usize);
        for row in 0..rows {
            for column in 0..columns {
                if row == 0 && column == 0 {
                    continue;
                }
                let move_by = geom::Matrix::translate(column as f64 * step_x, row as f64 * step_y);
                let copy = a_copy(session, &vector, move_by, format!("copy {column}-{row}"))?;
                made.push(copy);
            }
        }
        let count = made.len();
        Ok(Outcome::changed(format!("made {count} cop(y|ies) of the path"))
            .with_data(serde_json::json!({ "id": self.path.0, "copies": made })))
    }
}

/// Make a **mirrored** copy of a path.
///
/// The copy is the path turned over a **line**: the two points say
/// where the line is. A line with no length mirrors nothing, and it is
/// refused.
#[derive(Command)]
#[command(
    id = "vector.path.mirror",
    area = "vector",
    layer = "vector",
    icon = "plus",
    menu("Path", "Mirror"),
    mutates,
    undoable
)]
pub struct MirrorPath {
    /// The path to copy.
    pub path: NodeRef,
    /// Where the line starts, across.
    pub from_x: Number,
    /// Where the line starts, down.
    pub from_y: Number,
    /// Where the line ends, across.
    pub to_x: Number,
    /// Where the line ends, down.
    pub to_y: Number,
}

impl MirrorPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.mirror".to_string();
        let vector = vector_of(session, self.path.0)?;
        let from = Point::new(self.from_x.0, self.from_y.0);
        let to = Point::new(self.to_x.0, self.to_y.0);
        if !self.from_x.0.is_finite()
            || !self.from_y.0.is_finite()
            || !self.to_x.0.is_finite()
            || !self.to_y.0.is_finite()
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the line is two points".to_string(),
            });
        }
        let Some(mirror) = geom::Matrix::mirror(from, to) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a line with no length mirrors nothing".to_string(),
            });
        };
        let copy = a_copy(session, &vector, mirror, "mirror".to_string())?;
        Ok(Outcome::changed("made a mirrored copy of the path")
            .with_data(serde_json::json!({ "id": self.path.0, "copies": [copy] })))
    }
}

/// Make a **ring** of copies of a path.
///
/// The copies are the path turned about a **centre**: each one by its
/// share of the whole turn, so that the last one is where the first
/// began.
#[derive(Command)]
#[command(
    id = "vector.path.radial",
    area = "vector",
    layer = "vector",
    icon = "plus",
    menu("Path", "Radial"),
    mutates,
    undoable
)]
pub struct RadialPath {
    /// The path to copy.
    pub path: NodeRef,
    /// How many copies there are, the original among them.
    pub count: Number,
    /// Where the centre is, across.
    pub centre_x: Number,
    /// Where the centre is, down.
    pub centre_y: Number,
}

impl RadialPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.radial".to_string();
        let vector = vector_of(session, self.path.0)?;
        let count = the_count(self.count.0, "copies", &id)?;
        if count > MAX_COPIES {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a ring holds at most {MAX_COPIES} copies, and this one holds {count}"),
            });
        }
        if !self.centre_x.0.is_finite() || !self.centre_y.0.is_finite() {
            return Err(EngineError::Failed { id: id.clone(), message: "the centre is a point".to_string() });
        }
        let centre = Point::new(self.centre_x.0, self.centre_y.0);
        let turn = std::f64::consts::TAU / count as f64;
        let mut made: Vec<u64> = Vec::with_capacity(count.saturating_sub(1) as usize);
        for step in 1..count {
            let turn_by = geom::Matrix::rotate_about(centre, turn * step as f64);
            let copy = a_copy(session, &vector, turn_by, format!("copy {step}"))?;
            made.push(copy);
        }
        let made_count = made.len();
        Ok(Outcome::changed(format!("made {made_count} cop(y|ies) of the path"))
            .with_data(serde_json::json!({ "id": self.path.0, "copies": made })))
    }
}

/// A **count** of things: a whole number, and at least one.
fn the_count(value: f64, what: &str, id: &str) -> Result<i64, EngineError> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the number of {what} is a whole number"),
        });
    }
    if value < 1.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is at least one of the {what}"),
        });
    }
    Ok(value as i64)
}

/// A **copy** of a path, put somewhere else.
///
/// The copy is a path of its own: the same shape, the same paint, and
/// the **transform** that says where it is. The tree's repeat is the
/// same thing — the copies are paths of their own, and the shape is not
/// a shape with copies in it.
fn a_copy(
    session: &mut Session,
    vector: &VectorNode,
    move_by: geom::Matrix,
    name: String,
) -> Result<u64, EngineError> {
    let mut copy = vector.clone();
    copy.transform = vector.transform.then(&move_by);
    let rect = bounds(&copy.placed());
    Ok(session.document_mut()?.add_node_with(name, "vector.path", Some(rect), copy.to_json()))
}

/// Set the **blend mode** of a path.
///
/// The mode says how the path mixes with what is under it: the plain
/// mode lays it over, and the others mix the two colours first. The
/// mode is the **kit's own**, so that a shape, a letter and a picture
/// all mix the same way.
///
/// The plain mode is the mode a path has when it says nothing, and it
/// is how a blend is taken away.
#[derive(Command)]
#[command(
    id = "vector.path.blend",
    area = "vector",
    layer = "vector",
    icon = "paint",
    menu("Path", "Paint"),
    mutates,
    undoable
)]
pub struct BlendPath {
    /// The path.
    pub path: NodeRef,
    /// The mode: `Normal`, `Multiply`, `Screen`, `Overlay` and the
    /// rest.
    pub mode: Text,
}

impl BlendPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.blend".to_string();
        let mut vector = vector_of(session, self.path.0)?;
        let mode = BlendMode::parse(self.mode.0.trim()).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no blend mode called `{}`", self.mode.0.trim()),
        })?;
        vector.blend = mode;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed(format!("the path mixes with `{}`", mode.label()))
            .with_data(serde_json::json!({ "id": self.path.0, "blend": mode.label() })))
    }
}

/// Fill a path with a **gradient**.
///
/// The gradient is the **kit's own** — the colours, the stops they are
/// at, and where the gradient is put — and the path is filled with it.
///
/// The stops are a list of `{"offset": 0, "colour": "#rrggbb"}`, and
/// the offset is where the colour is along the gradient: zero is the
/// start and one is the end. A stop may say how solid it is, with
/// `"opacity"`.
///
/// Where the gradient is put is the path's **own box** unless the
/// caller says otherwise: a linear gradient goes across the box, and a
/// radial one is centred in it.
#[derive(Command)]
#[command(
    id = "vector.path.gradient",
    area = "vector",
    layer = "vector",
    icon = "paint",
    menu("Path", "Paint"),
    mutates,
    undoable
)]
pub struct GradientPath {
    /// The path to fill with the gradient.
    pub path: NodeRef,
    /// `linear` — along a line — or `radial` — out from a centre.
    pub kind: Text,
    /// The stops: a list of `{"offset": 0, "colour": "#rrggbb"}`.
    pub stops: serde_json::Value,
    /// Where a **linear** gradient starts: `{"x": 0, "y": 0}`.
    pub from: Option<serde_json::Value>,
    /// Where a **linear** gradient ends.
    pub to: Option<serde_json::Value>,
    /// Where a **radial** gradient is centred.
    pub centre: Option<serde_json::Value>,
    /// How far a **radial** gradient reaches.
    pub radius: Option<Number>,
}

impl GradientPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.gradient".to_string();
        let mut vector = vector_of(session, self.path.0)?;
        // The **kind**: a line or a centre.
        let kind = match self.kind.0.trim().to_ascii_lowercase().as_str() {
            "linear" => GradientKind::Linear,
            "radial" => GradientKind::Radial,
            other => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a gradient is linear or radial, and this one is `{other}`"),
                });
            }
        };
        // The **stops**.
        let list = self.stops.as_array().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: "the stops of a gradient are a list".to_string(),
        })?;
        if list.len() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a gradient has two stops or more".to_string(),
            });
        }
        let mut stops: Vec<GradientStop> = Vec::with_capacity(list.len());
        for item in list {
            let offset = item.get("offset").and_then(|offset| offset.as_f64()).ok_or_else(|| {
                EngineError::Failed { id: id.clone(), message: "a stop says where it is".to_string() }
            })?;
            if !offset.is_finite() || !(0.0..=1.0).contains(&offset) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a stop's offset is between zero and one, and this one is {offset}"),
                });
            }
            let colour = item.get("colour").and_then(|colour| colour.as_str()).ok_or_else(|| {
                EngineError::Failed { id: id.clone(), message: "a stop says what colour it is".to_string() }
            })?;
            let colour = parse_colour(colour)?;
            let opacity = match item.get("opacity").and_then(|opacity| opacity.as_f64()) {
                Some(opacity) if opacity.is_finite() && (0.0..=1.0).contains(&opacity) => opacity,
                Some(_) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a stop's opacity is between zero and one".to_string(),
                    });
                }
                None => 1.0,
            };
            stops.push(GradientStop { offset, colour, opacity });
        }
        // The **geometry**: the path's own box, unless the caller says.
        let box_of = bounds(&vector.path);
        let geometry = match kind {
            GradientKind::Linear => {
                let from = match &self.from {
                    Some(from) => point_of(from, &id)?,
                    None => Point::new(box_of.x, box_of.y + box_of.h / 2.0),
                };
                let to = match &self.to {
                    Some(to) => point_of(to, &id)?,
                    None => Point::new(box_of.x + box_of.w, box_of.y + box_of.h / 2.0),
                };
                GradientGeometry::Linear { from, to }
            }
            GradientKind::Radial => {
                let centre = match &self.centre {
                    Some(centre) => point_of(centre, &id)?,
                    None => Point::new(box_of.x + box_of.w / 2.0, box_of.y + box_of.h / 2.0),
                };
                let radius = match &self.radius {
                    Some(radius) => radius.0,
                    None => (box_of.w * box_of.w + box_of.h * box_of.h).sqrt() / 2.0,
                };
                if !radius.is_finite() || radius <= 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a gradient's radius is a number greater than zero".to_string(),
                    });
                }
                GradientGeometry::Radial { centre, radius }
            }
        };
        vector.fill = Fill::Gradient(Gradient { kind, stops, geometry });
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the path is filled with a gradient")
            .with_data(serde_json::json!({ "id": self.path.0, "gradient": vector.fill.to_json() })))
    }
}

/// A **point**, from a value a caller sent.
fn point_of(value: &serde_json::Value, id: &str) -> Result<Point, EngineError> {
    let number =
        |name: &str| value.get(name).and_then(|value| value.as_f64()).filter(|value| value.is_finite());
    let (Some(x), Some(y)) = (number("x"), number("y")) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a point says where it is, across and down".to_string(),
        });
    };
    Ok(Point::new(x, y))
}

/// Set the **stroke** of a path, or take it away.
///
/// The stroke is the line on the path's outline, and it has a colour
/// and a width. A path with no stroke has no line.
#[derive(Command)]
#[command(
    id = "vector.path.stroke",
    area = "vector",
    layer = "vector",
    icon = "paint",
    menu("Path", "Paint"),
    mutates,
    undoable
)]
pub struct StrokePath {
    /// The path.
    pub path: NodeRef,
    /// The colour of the line, as `#rrggbb`.
    pub colour: Text,
    /// How wide the line is.
    pub width: Number,
    /// Take the line away instead.
    pub none: Option<craftkit_sdk::Bool>,
}

impl StrokePath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        if self.none.map(|none| none.0).unwrap_or(false) {
            vector.stroke = None;
            write_back(session, self.path.0, &vector)?;
            return Ok(Outcome::changed("the path has no line")
                .with_data(serde_json::json!({ "id": self.path.0, "stroke": Json::Null })));
        }
        let width = self.width.0;
        if !width.is_finite() || width < 0.0 {
            return Err(EngineError::Failed {
                id: "vector.path.stroke".to_string(),
                message: "a line's width is a number and not a negative one".to_string(),
            });
        }
        let colour = parse_colour(&self.colour.0)?;
        vector.stroke = Some(Stroke { colour, width, ..Default::default() });
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the path has a line")
            .with_data(serde_json::json!({ "id": self.path.0, "width": width })))
    }
}

/// Say how the **corners** of a path's line meet.
///
/// The join is `miter` — a pointed corner — `round` or `bevel`, which
/// is a corner that is cut off.
#[derive(Command)]
#[command(
    id = "vector.path.join",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Paint"),
    mutates,
    undoable
)]
pub struct JoinPath {
    /// The path.
    pub path: NodeRef,
    /// The join: `miter`, `round` or `bevel`.
    pub join: Text,
}

impl JoinPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let Some(mut stroke) = vector.stroke else {
            return Err(EngineError::Failed {
                id: "vector.path.join".to_string(),
                message: format!("the path `{}` has no line", self.path.0),
            });
        };
        let join = match self.join.0.trim().to_ascii_lowercase().as_str() {
            "miter" => Join::Miter,
            "round" => Join::Round,
            "bevel" => Join::Bevel,
            other => {
                return Err(EngineError::Failed {
                    id: "vector.path.join".to_string(),
                    message: format!("the join `{other}` is not one of the three"),
                });
            }
        };
        stroke.join = join;
        vector.stroke = Some(stroke);
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the corners of the line meet")
            .with_data(serde_json::json!({ "id": self.path.0, "join": self.join.0 })))
    }
}

/// Say how the **ends** of a path's line are finished.
///
/// The cap is `butt` — a flush end — `round` or `square`.
#[derive(Command)]
#[command(
    id = "vector.path.cap",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Paint"),
    mutates,
    undoable
)]
pub struct CapPath {
    /// The path.
    pub path: NodeRef,
    /// The cap: `butt`, `round` or `square`.
    pub cap: Text,
}

impl CapPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let Some(mut stroke) = vector.stroke else {
            return Err(EngineError::Failed {
                id: "vector.path.cap".to_string(),
                message: format!("the path `{}` has no line", self.path.0),
            });
        };
        let cap = match self.cap.0.trim().to_ascii_lowercase().as_str() {
            "butt" => Cap::Butt,
            "round" => Cap::Round,
            "square" => Cap::Square,
            other => {
                return Err(EngineError::Failed {
                    id: "vector.path.cap".to_string(),
                    message: format!("the cap `{other}` is not one of the three"),
                });
            }
        };
        stroke.cap = cap;
        vector.stroke = Some(stroke);
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the ends of the line are finished")
            .with_data(serde_json::json!({ "id": self.path.0, "cap": self.cap.0 })))
    }
}

/// Place a path with a **transform**.
///
/// The transform is the six numbers of an affine matrix — `a b c d e f`
/// — and it says where the path is put: a point `(x, y)` of the path
/// is drawn at `(a·x + c·y + e, b·x + d·y + f)`. Leave the numbers out
/// and the path is where it is.
#[derive(Command)]
#[command(
    id = "vector.path.transform",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Transform"),
    mutates,
    undoable
)]
pub struct TransformPath {
    /// The path.
    pub path: NodeRef,
    /// The six numbers of the matrix.
    pub matrix: Option<serde_json::Value>,
}

impl TransformPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let transform = match &self.matrix {
            Some(matrix) => geom::Matrix::from_json(matrix).map_err(|reason| EngineError::Failed {
                id: "vector.path.transform".to_string(),
                message: format!("the transform is not a transform: {reason}"),
            })?,
            None => geom::Matrix::identity(),
        };
        // A matrix that **collapses** the plane has no undoing, and a
        // path that cannot be undone cannot be hit.
        if transform.inverse().is_none() {
            return Err(EngineError::Failed {
                id: "vector.path.transform".to_string(),
                message: "a transform that cannot be undone is refused".to_string(),
            });
        }
        vector.transform = transform;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the path is placed")
            .with_data(serde_json::json!({ "id": self.path.0, "transform": transform.to_json() })))
    }
}

/// Make a point of a path **smooth**.
///
/// A smooth point's two handles are collinear and of the same length:
/// the curve passes through the point as one curve and not as two. The
/// command makes it so — the handles take the average of the two they
/// were, and a side that was a straight line becomes a curve.
#[derive(Command)]
#[command(
    id = "vector.path.smooth",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Points"),
    mutates,
    undoable
)]
pub struct SmoothPoint {
    /// The path.
    pub path: NodeRef,
    /// Which contour of the path, counting from zero. The first one if
    /// it is left out.
    pub contour: Option<Number>,
    /// Which point of the contour, counting from zero.
    pub point: Number,
}

impl SmoothPoint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let contour = self.contour.as_ref().map(|contour| contour.0).unwrap_or(0.0);
        let contour = usize::try_from(contour.max(0.0) as i64).unwrap_or(0);
        let point = usize::try_from(self.point.0.max(0.0) as i64).unwrap_or(0);
        smooth_point(&mut vector.path, contour, point).map_err(|reason| EngineError::Failed {
            id: "vector.path.smooth".to_string(),
            message: reason,
        })?;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the point is smooth")
            .with_data(serde_json::json!({ "id": self.path.0, "contour": contour, "point": point })))
    }
}

/// Make a point of a path **smooth**.
///
/// The two handles of the point become collinear and of the same
/// length, and a side that was a straight line becomes a curve with a
/// handle.
fn smooth_point(path: &mut Path, contour: usize, point: usize) -> Result<(), String> {
    let Some(subpath) = path.subpaths.get_mut(contour) else {
        return Err(format!("the path has no contour {contour}"));
    };
    let segments = subpath.segments.len();
    let count = segments + 1;
    if point >= count {
        return Err(format!("the contour has {count} points and not a point {point}"));
    }
    // A closed contour may leave its **closing edge** out, and a
    // handle needs a segment to hold it: the edge is made explicit
    // when the point needs one.
    if subpath.closed && closing_segment(subpath).is_none() && (point == 0 || point == segments) {
        subpath.segments.push(Segment::Line { to: subpath.start });
    }
    let leaving = leaving_segment(subpath, point);
    let arriving = arriving_segment(subpath, point);
    // The **point** itself: where the two sides meet.
    let at = match leaving {
        Some(index) => {
            if index == 0 {
                subpath.start
            } else {
                subpath.segments[index - 1].end()
            }
        }
        None => match arriving {
            Some(index) => subpath.segments[index].end(),
            None => subpath.start,
        },
    };
    // The two **handles**: a straight side has none, and its tangent
    // is the line itself.
    let out_handle = match leaving {
        Some(index) => match &subpath.segments[index] {
            Segment::Line { .. } => None,
            Segment::Cubic { c1, .. } => Some(*c1),
        },
        None => None,
    };
    let in_handle = match arriving {
        Some(index) => match &subpath.segments[index] {
            Segment::Line { .. } => None,
            Segment::Cubic { c2, .. } => Some(*c2),
        },
        None => None,
    };
    if leaving.is_none() || arriving.is_none() {
        return Err(format!("the point {point} has a curve on only one side"));
    }
    let out_offset = match out_handle {
        Some(handle) => Point::new(handle.x - at.x, handle.y - at.y),
        None => Point::new(0.0, 0.0),
    };
    let in_offset = match in_handle {
        Some(handle) => Point::new(handle.x - at.x, handle.y - at.y),
        None => Point::new(0.0, 0.0),
    };
    let out_is_zero = out_offset.x.abs() < f64::EPSILON && out_offset.y.abs() < f64::EPSILON;
    let in_is_zero = in_offset.x.abs() < f64::EPSILON && in_offset.y.abs() < f64::EPSILON;
    if out_is_zero && in_is_zero {
        return Err(format!("the point {point} has no handles to smooth"));
    }
    // The **new** handle: the average of the two, or the one that is
    // there when the other side is a straight line.
    let offset = if out_is_zero || in_is_zero {
        if out_is_zero { in_offset } else { out_offset }
    } else {
        Point::new((out_offset.x + in_offset.x) / 2.0, (out_offset.y + in_offset.y) / 2.0)
    };
    let out_at = Point::new(at.x + offset.x, at.y + offset.y);
    let in_at = Point::new(at.x - offset.x, at.y - offset.y);
    // The **leaving** side: the handle at the point, and — when the
    // side was a straight line — the other end keeps the line's
    // direction.
    if let Some(index) = leaving {
        let end = subpath.segments[index].end();
        let c2 = if out_is_zero {
            // A straight line that becomes a curve: the other end's
            // handle is a third of the way along it.
            Point::new(end.x - (end.x - at.x) / 3.0, end.y - (end.y - at.y) / 3.0)
        } else {
            match &subpath.segments[index] {
                Segment::Line { .. } => end,
                Segment::Cubic { c2, .. } => *c2,
            }
        };
        subpath.segments[index] = Segment::Cubic { c1: out_at, c2, to: end };
    }
    // The **arriving** side: the same, the other way round.
    if let Some(index) = arriving {
        let start = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
        let c1 = if in_is_zero {
            Point::new(start.x + (at.x - start.x) / 3.0, start.y + (at.y - start.y) / 3.0)
        } else {
            match &subpath.segments[index] {
                Segment::Line { .. } => start,
                Segment::Cubic { c1, .. } => *c1,
            }
        };
        subpath.segments[index] = Segment::Cubic { c1, c2: in_at, to: at };
    }
    Ok(())
}

/// Make a point of a path a **corner**.
///
/// The **handles** at the point are pulled back to it, so the two sides
/// leave it as straight lines: the tree calls that a *corner*, and the
/// point that has no handles at it is a corner already.
///
/// The tree's `path.convertAnchors` is one command with a `to` that
/// says `corner` or `smooth`. This tree has one command for each, and
/// this one is the corner; the smooth one is `vector.path.smooth`.
#[derive(Command)]
#[command(
    id = "vector.path.corner",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Points"),
    mutates,
    undoable
)]
pub struct CornerPoint {
    /// The path.
    pub path: NodeRef,
    /// Which contour of the path, counting from zero. The first one if
    /// it is left out.
    pub contour: Option<Number>,
    /// Which point of the contour, counting from zero.
    pub point: Number,
}

impl CornerPoint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let contour = self.contour.as_ref().map(|contour| contour.0).unwrap_or(0.0);
        let contour = usize::try_from(contour.max(0.0) as i64).unwrap_or(0);
        let point = usize::try_from(self.point.0.max(0.0) as i64).unwrap_or(0);
        corner_point(&mut vector.path, contour, point).map_err(|reason| EngineError::Failed {
            id: "vector.path.corner".to_string(),
            message: reason,
        })?;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the point is a corner")
            .with_data(serde_json::json!({ "id": self.path.0, "contour": contour, "point": point })))
    }
}

/// **Reshape** a path: grab it at a point and drag it.
///
/// The anchors around the grab follow with a **smooth falloff** over a
/// radius, and the radius is measured **along** the path — not in a
/// straight line from the grab, but along the curve the path is.
///
/// The grab may be on a point that is not an anchor: the path is
/// grabbed where it is nearest to the point, and an anchor is added
/// there — unless an anchor is already within the tolerance.
#[derive(Command)]
#[command(
    id = "vector.path.reshape",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Reshape"),
    mutates,
    undoable
)]
pub struct ReshapePath {
    /// The path.
    pub path: NodeRef,
    /// Where the path is grabbed: the x of the point.
    pub x: Number,
    /// Where the path is grabbed: the y of the point.
    pub y: Number,
    /// How far the grab is dragged: the x.
    pub dx: Number,
    /// How far the grab is dragged: the y.
    pub dy: Number,
    /// How near an anchor must be for it to be the one that is grabbed.
    /// The tree's own default is three.
    pub tol: Option<Number>,
    /// How far the anchors around follow, measured along the path. The
    /// default is half the size of the contour, and it is never less
    /// than one.
    pub radius: Option<Number>,
}

impl ReshapePath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let at = Point::new(self.x.0, self.y.0);
        let delta = Point::new(self.dx.0, self.dy.0);
        let tol = self.tol.as_ref().map(|tol| tol.0).unwrap_or(3.0);
        if !tol.is_finite() || tol < 0.0 {
            return Err(EngineError::Failed {
                id: "vector.path.reshape".to_string(),
                message: "the tolerance is a distance, and it is not a negative one".to_string(),
            });
        }
        let radius = self.radius.as_ref().map(|radius| radius.0);
        if let Some(radius) = radius
            && (!radius.is_finite() || radius < 0.0)
        {
            return Err(EngineError::Failed {
                id: "vector.path.reshape".to_string(),
                message: "the radius is a distance, and it is not a negative one".to_string(),
            });
        }
        // Where the path is **nearest** to the grab.
        let Some(nearest) = reshape::nearest(&vector.path, at) else {
            return Err(EngineError::Failed {
                id: "vector.path.reshape".to_string(),
                message: "the path is empty, and there is nothing to reshape".to_string(),
            });
        };
        // The **contour** the grab is on.
        let Some(subpath) = vector.path.subpaths.get(nearest.contour).cloned() else {
            return Err(EngineError::Failed {
                id: "vector.path.reshape".to_string(),
                message: format!("the path has no contour {}", nearest.contour),
            });
        };
        // The **anchor** that is grabbed: the nearest one, when it is
        // near enough, and otherwise an anchor that is added there.
        let anchors = reshape::anchors(&subpath);
        let nearest_anchor = anchors
            .iter()
            .enumerate()
            .map(|(index, anchor)| (index, distance(*anchor, at)))
            .min_by(|left, right| left.1.total_cmp(&right.1));
        let mut subpath = subpath;
        let grabbed = match nearest_anchor {
            Some((index, distance)) if distance <= tol => index,
            _ => {
                let split = geom::split_segment(&subpath, nearest.segment, nearest.t).map_err(|reason| {
                    EngineError::Failed {
                        id: "vector.path.reshape".to_string(),
                        message: format!("the path cannot be reshaped there: {reason}"),
                    }
                })?;
                subpath = split;
                nearest.segment + 1
            }
        };
        vector.path.subpaths[nearest.contour] = subpath.clone();
        // The **positions** of the anchors along the path, and the
        // **radius** the falloff is measured over.
        let (positions, total) = reshape::positions(&subpath);
        let radius = radius.unwrap_or_else(|| reshape::size(&subpath));
        let from = positions.get(grabbed).copied().unwrap_or(0.0);
        let anchors = reshape::anchors(&subpath);
        let closed = subpath.closed;
        // The anchors **move**: the grabbed one the whole way, and the
        // others by their weight.
        for (index, anchor) in anchors.iter().enumerate() {
            let weight = if index == grabbed {
                1.0
            } else {
                let away = (positions.get(index).copied().unwrap_or(f64::INFINITY) - from).abs();
                let away = if closed { away.min(total - away) } else { away };
                reshape::weight(away, radius)
            };
            if weight == 0.0 {
                continue;
            }
            let moved = Point::new(anchor.x + delta.x * weight, anchor.y + delta.y * weight);
            move_point(&mut vector.path, nearest.contour, index, moved).map_err(|reason| {
                EngineError::Failed { id: "vector.path.reshape".to_string(), message: reason }
            })?;
        }
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the path is reshaped").with_data(serde_json::json!({
            "id": self.path.0,
            "contour": nearest.contour,
            "point": grabbed,
            "radius": radius,
        })))
    }
}

/// How far two points are.
fn distance(from: Point, to: Point) -> f64 {
    ((from.x - to.x).powi(2) + (from.y - to.y).powi(2)).sqrt()
}

/// Pull the **handles** at a point back to it.
///
/// The handles at the point belong to the two sides that meet there:
/// the side that leaves the point holds the handle that leaves it, and
/// the side that arrives at the point holds the handle that arrives.
/// The handles at the **other** ends of those sides are not touched —
/// they belong to other points.
fn corner_point(path: &mut Path, contour: usize, point: usize) -> Result<(), String> {
    let Some(subpath) = path.subpaths.get_mut(contour) else {
        return Err(format!("the path has no contour {contour}"));
    };
    let segments = subpath.segments.len();
    let count = segments + 1;
    if point >= count {
        return Err(format!("the contour has {count} points and not a point {point}"));
    }
    // A closed contour may leave its **closing edge** out, and a handle
    // needs a segment to hold it: the edge is made explicit when the
    // point needs one.
    if subpath.closed && closing_segment(subpath).is_none() && (point == 0 || point == segments) {
        subpath.segments.push(Segment::Line { to: subpath.start });
    }
    let leaving = leaving_segment(subpath, point);
    let arriving = arriving_segment(subpath, point);
    // The **point** itself: where the two sides meet.
    let at = match leaving {
        Some(index) => {
            if index == 0 {
                subpath.start
            } else {
                subpath.segments[index - 1].end()
            }
        }
        None => match arriving {
            Some(index) => subpath.segments[index].end(),
            None => subpath.start,
        },
    };
    let mut pulled = 0;
    // The side that **leaves** the point: its handle at the point goes
    // back to the point.
    if let Some(index) = leaving
        && let Segment::Cubic { c1: _, c2, to } = subpath.segments[index]
    {
        subpath.segments[index] = Segment::Cubic { c1: at, c2, to };
        pulled += 1;
    }
    // And the side that **arrives** at it: the same, the other way
    // round.
    if let Some(index) = arriving
        && let Segment::Cubic { c1, c2: _, to } = subpath.segments[index]
    {
        subpath.segments[index] = Segment::Cubic { c1, c2: at, to };
        pulled += 1;
    }
    if pulled == 0 {
        return Err(format!("the point {point} has no handles to pull back"));
    }
    Ok(())
}

/// Say which parts of a path are **inside**.
///
/// The rule is `nonzero` — the default, and the rule that makes a
/// contour inside another one a hole when the two run in opposite
/// directions — or `evenodd`, which does not care which way round a
/// contour runs.
#[derive(Command)]
#[command(
    id = "vector.path.rule",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Rule"),
    mutates,
    undoable
)]
pub struct SetRule {
    /// The path.
    pub path: NodeRef,
    /// The rule: `nonzero` or `evenodd`.
    pub rule: Text,
}

impl SetRule {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut vector = vector_of(session, self.path.0)?;
        let rule = match self.rule.0.trim().to_ascii_lowercase().as_str() {
            "nonzero" | "non-zero" => FillRule::NonZero,
            "evenodd" | "even-odd" => FillRule::EvenOdd,
            other => {
                return Err(EngineError::Failed {
                    id: "vector.path.rule".to_string(),
                    message: format!("the rule is `nonzero` or `evenodd`, and not `{other}`"),
                });
            }
        };
        vector.path.rule = rule;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed("the rule is set").with_data(serde_json::json!({ "id": self.path.0 })))
    }
}

/// Keep what is in one path and not in the other.
///
/// The **exclusion**: what the two paths do not share. It is the
/// difference the other way round, and it is the operation that leaves
/// the overlap out.
#[derive(Command)]
#[command(
    id = "vector.path.exclude",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct ExcludePaths {
    /// The first path.
    ///
    /// Leave it out and the **selection** is used instead.
    pub first: Option<NodeRef>,
    /// The second path.
    ///
    /// Leave it out and the **selection** is used instead.
    pub second: Option<NodeRef>,
}

impl ExcludePaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let (first, second) = two_of(session, self.first, self.second)?;
        combine(session, "vector.path.exclude", first, second, geom::BoolOp::Xor)
    }
}

// ------------------------------------------------------------- the helpers --

/// A number that is an **index**: a whole number and not a negative
/// one.
///
/// A number that is not an index is **refused** and not rounded: an
/// index of one and a half is a question, and the answer is that the
/// kit has no such point.
fn index_of(value: Option<Number>, what: &str) -> Result<usize, String> {
    let number = value.map(|number| number.0).unwrap_or(0.0);
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return Err(format!("{what} is a whole number and not {number}"));
    }
    Ok(number as usize)
}

/// Which segment **leaves** a point, if there is one.
///
/// The point at the start of a contour is left by the first segment,
/// and every other point is left by the segment that starts there. The
/// last point of a contour is left by nothing: the curve that closes a
/// closed contour is not in the kit's path.
fn leaving_segment(subpath: &SubPath, point: usize) -> Option<usize> {
    let segments = subpath.segments.len();
    if point < segments {
        return Some(point);
    }
    // The last point of a **closed** contour is left by the closing
    // segment, when the contour has one.
    if point == segments {
        return closing_segment(subpath);
    }
    None
}

/// Which segment **arrives** at a point, if there is one.
///
/// Every point but the first is arrived at by the segment before it,
/// and the first point of a **closed** contour is arrived at by the
/// last segment.
fn arriving_segment(subpath: &SubPath, point: usize) -> Option<usize> {
    if point > 0 {
        return Some(point - 1);
    }
    // The first point of a **closed** contour is arrived at by the
    // closing segment, when the contour has one.
    closing_segment(subpath)
}

/// The **closing segment** of a closed contour, if it has one.
///
/// A closed contour may leave its closing edge out — the edge is then a
/// straight line, and there is nothing to hold a handle — or it may
/// have one, and then that segment ends where the contour starts.
fn closing_segment(subpath: &SubPath) -> Option<usize> {
    let segments = subpath.segments.len();
    if !subpath.closed || segments == 0 {
        return None;
    }
    let last = segments - 1;
    let end = subpath.segments[last].end();
    let start = subpath.start;
    if (end.x - start.x).abs() < f64::EPSILON && (end.y - start.y).abs() < f64::EPSILON {
        Some(last)
    } else {
        None
    }
}

/// Move a point of a path, and the handles that belong to it.
fn move_point(path: &mut Path, contour: usize, point: usize, to: Point) -> Result<(), String> {
    let Some(subpath) = path.subpaths.get_mut(contour) else {
        return Err(format!("the path has no contour {contour}"));
    };
    let count = subpath.segments.len() + 1;
    if point >= count {
        return Err(format!("the contour has {count} points and not a point {point}"));
    }
    let from = if point == 0 { subpath.start } else { subpath.segments[point - 1].end() };
    let delta = Point::new(to.x - from.x, to.y - from.y);
    // The **point** itself.
    if point == 0 {
        subpath.start = to;
    } else if let Segment::Line { to: end } = &mut subpath.segments[point - 1] {
        *end = to;
    } else if let Segment::Cubic { to: end, .. } = &mut subpath.segments[point - 1] {
        *end = to;
    }
    // The **handles** move with it: an anchor carries its handles.
    let leaving = leaving_segment(subpath, point);
    let arriving = arriving_segment(subpath, point);
    if let Some(index) = leaving
        && let Segment::Cubic { c1, .. } = &mut subpath.segments[index]
    {
        *c1 = Point::new(c1.x + delta.x, c1.y + delta.y);
    }
    if let Some(index) = arriving
        && let Segment::Cubic { c2, .. } = &mut subpath.segments[index]
    {
        *c2 = Point::new(c2.x + delta.x, c2.y + delta.y);
    }
    Ok(())
}

/// Set the handles of a point of a path.
///
/// A handle on a straight segment makes it a curve, and the handle it
/// did not have is the point itself: a curve whose two handles are its
/// end points is a straight line.
fn handle_point(
    path: &mut Path,
    contour: usize,
    point: usize,
    c1: Option<Point>,
    c2: Option<Point>,
) -> Result<(), String> {
    let Some(subpath) = path.subpaths.get_mut(contour) else {
        return Err(format!("the path has no contour {contour}"));
    };
    let count = subpath.segments.len() + 1;
    if point >= count {
        return Err(format!("the contour has {count} points and not a point {point}"));
    }
    if let Some(handle) = c1 {
        let Some(index) = leaving_segment(subpath, point) else {
            return Err(format!("the point {point} has no curve leaving it"));
        };
        let end = subpath.segments[index].end();
        let old_c2 = match &subpath.segments[index] {
            Segment::Line { .. } => end,
            Segment::Cubic { c2, .. } => *c2,
        };
        subpath.segments[index] = Segment::Cubic { c1: handle, c2: old_c2, to: end };
    }
    if let Some(handle) = c2 {
        let Some(index) = arriving_segment(subpath, point) else {
            return Err(format!("the point {point} has no curve arriving at it"));
        };
        let start = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
        let end = subpath.segments[index].end();
        let old_c1 = match &subpath.segments[index] {
            Segment::Line { .. } => start,
            Segment::Cubic { c1, .. } => *c1,
        };
        subpath.segments[index] = Segment::Cubic { c1: old_c1, c2: handle, to: end };
    }
    Ok(())
}

/// The path a node carries.
fn vector_of(session: &Session, id: u64) -> Result<VectorNode, EngineError> {
    let node = session.document().node(id).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no node `{id}`"),
    })?;
    VectorNode::from_json(&node.data).map_err(|reason| EngineError::Failed {
        id: id.to_string(),
        message: format!("the node `{id}` has no path: {reason}"),
    })
}

/// Put a path back into a node, and move the node's box to where it is.
fn write_back(session: &mut Session, id: u64, vector: &VectorNode) -> Result<(), EngineError> {
    let rect = geom::bounds(&vector.placed());
    let document = session.document_mut()?;
    let node = document.node_mut(id).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no node `{id}`"),
    })?;
    node.data = vector.to_json();
    node.rect = Some(rect);
    Ok(())
}

/// The two paths a command works on.
///
/// The ones it was **given**, or the ones that are **selected** — and the
/// selection is the session's own state, so a person who picked two
/// shapes and an agent that names two ids are saying the same thing.
fn two_of(
    session: &Session,
    first: Option<NodeRef>,
    second: Option<NodeRef>,
) -> Result<(u64, u64), EngineError> {
    // A path operation works on whole paths: a boolean of half a path is
    // not a thing this battery does.
    let selected: Vec<u64> = session.selected_whole_ids();
    let need = |what: &str| {
        Err(EngineError::Failed {
            id: "vector.path".to_string(),
            message: format!("there is nothing {what}: name two paths or select two"),
        })
    };
    match (first, second) {
        (Some(first), Some(second)) => Ok((first.0, second.0)),
        (Some(first), None) => match selected.first() {
            Some(second) => Ok((first.0, *second)),
            None => need("selected"),
        },
        (None, Some(second)) => match selected.first() {
            Some(first) => Ok((*first, second.0)),
            None => need("selected"),
        },
        (None, None) => {
            if selected.len() < 2 {
                return need("selected");
            }
            Ok((selected[0], selected[1]))
        }
    }
}

/// Combine two paths into the first one, and take the second away.
fn combine(
    session: &mut Session,
    id: &str,
    first: u64,
    second: u64,
    op: geom::BoolOp,
) -> Result<Outcome, EngineError> {
    let one = vector_of(session, first)?;
    let two = vector_of(session, second)?;
    if one.path.is_empty() || two.path.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "one of the paths is empty".to_string(),
        });
    }
    let path = geom::boolean(&one.path, &two.path, op)
        .map_err(|error| EngineError::Failed { id: id.to_string(), message: error.to_string() })?;
    let mut result = one.clone();
    result.path = path;
    write_back(session, first, &result)?;
    session.document_mut()?.remove_node(second);
    Ok(Outcome::changed("the paths are one now")
        .with_data(serde_json::json!({ "id": first, "removed": second })))
}

/// Put a path in the document, and give back its id.
fn add_path(
    session: &mut Session,
    name: String,
    path: Path,
    fill: Option<&Text>,
) -> Result<u64, EngineError> {
    let colour = match fill {
        Some(fill) => parse_colour(&fill.0)?,
        None => Colour::BLACK,
    };
    let vector = VectorNode {
        path: path.clone(),
        fill: Fill::Solid(colour),
        stroke: None,
        blend: BlendMode::Normal,
        transform: geom::Matrix::identity(),
    };
    let rect = bounds(&path);
    let id = session.document_mut()?.add_node_with(name, "vector.path", Some(rect), vector.to_json());
    Ok(id)
}

/// The colour a command was given, as `#rrggbb`.
///
/// The plugin reads its own colour, because a colour is the plugin's own
/// type: v2's parameters have no colour kind, and a plugin that needs one
/// reads it from a text and says so.
fn parse_colour(text: &str) -> Result<Colour, EngineError> {
    // The **kit's** spelling of a colour: every battery that takes one
    // takes it this way, and a second spelling of the same thing is a
    // second thing to get wrong.
    Colour::from_hex(text).map_err(|message| EngineError::Failed { id: "vector".to_string(), message })
}

/// An ellipse, as four cubic curves.
///
/// The constant is the one a circle has always been drawn with: the
/// control points sit at the corner of the square the circle is inscribed
/// in, pulled in by `4/3 (√2 − 1)`.
fn ellipse(x: f64, y: f64, width: f64, height: f64) -> Path {
    let rx = width / 2.0;
    let ry = height / 2.0;
    let kappa = 0.5522847498307936;
    let (cx, cy) = (x, y);
    let start = Point::new(cx + rx, cy);
    let segments = vec![
        Segment::Cubic {
            c1: Point::new(cx + rx, cy + kappa * ry),
            c2: Point::new(cx + kappa * rx, cy + ry),
            to: Point::new(cx, cy + ry),
        },
        Segment::Cubic {
            c1: Point::new(cx - kappa * rx, cy + ry),
            c2: Point::new(cx - rx, cy + kappa * ry),
            to: Point::new(cx - rx, cy),
        },
        Segment::Cubic {
            c1: Point::new(cx - rx, cy - kappa * ry),
            c2: Point::new(cx - kappa * rx, cy - ry),
            to: Point::new(cx, cy - ry),
        },
        Segment::Cubic {
            c1: Point::new(cx + kappa * rx, cy - ry),
            c2: Point::new(cx + rx, cy - kappa * ry),
            to: Point::new(cx + rx, cy),
        },
    ];
    Path::single(SubPath { start, segments, closed: true })
}

/// A regular polygon, as straight lines.
fn polygon(x: f64, y: f64, radius: f64, sides: usize) -> Path {
    let mut points = Vec::with_capacity(sides);
    for index in 0..sides {
        let angle = std::f64::consts::TAU * (index as f64) / (sides as f64);
        points.push(Point::new(x + radius * angle.cos(), y + radius * angle.sin()));
    }
    Path::polygon(&points)
}

/// Whether a point is inside a polygon.
///
/// The even-odd rule: a ray from the point crosses the outline an odd
/// number of times if the point is inside it.
fn polygon_contains(points: &[Point], point: Point) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut index = 0;
    let mut previous = points.len() - 1;
    while index < points.len() {
        let a = points[index];
        let b = points[previous];
        if (a.y > point.y) != (b.y > point.y) {
            let crossing = (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
            if point.x < crossing {
                inside = !inside;
            }
        }
        previous = index;
        index += 1;
    }
    inside
}

/// The bounding box of a path.
fn bounds(path: &Path) -> Rect {
    geom::bounds(path)
}

// The plugin's anchor.
//
// One name that keeps every item above in the program, on every target —
// including `wasm32-unknown-unknown`, whose linker drops a crate that
// nothing names. See the SDK's `anchor` module.
craftkit_sdk::plugin_anchor!(
    VectorLayer,
    VectorPathKind,
    VectorTool,
    AddRectangle,
    AddEllipse,
    AddPolygon,
    AddStar,
    AddLine,
    AddSpiral,
    UnitePaths,
    SubtractPaths,
    IntersectPaths,
    SimplifyPath,
    OffsetPath,
    JoinPath,
    CapPath,
    SmoothPoint,
    OutlinePath,
    TransformPath,
    ClosePath,
    ReversePath,
    AddAnchor,
    RemoveAnchor,
    AveragePoints,
    CornerPoint,
    VectorPanel,
    TotalAreaEvaluator
);

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// The battery's commands are in the ecosystem.
    #[test]
    fn the_batterys_commands_are_in_the_ecosystem() {
        let ecosystem = ecosystem();
        assert!(ecosystem.commands.get("vector.rectangle").is_some());
        assert!(ecosystem.commands.get("vector.ellipse").is_some());
        assert!(ecosystem.commands.get("vector.polygon").is_some());
        assert!(ecosystem.commands.get("vector.path.unite").is_some());
        assert!(ecosystem.commands.get("vector.path.subtract").is_some());
        assert!(ecosystem.commands.get("vector.path.intersect").is_some());
        assert!(ecosystem.commands.get("vector.path.simplify").is_some());
        assert!(ecosystem.commands.get("vector.path.offset").is_some());
        assert!(ecosystem.kinds.get("vector.path").is_some());
        assert!(ecosystem.evaluators.get("vector.totalarea").is_some());
    }

    /// A rectangle is a path, and the path is the node's payload.
    #[test]
    fn a_rectangle_is_a_path_in_the_payload() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 5.0 }))
            .expect("the rectangle is added");
        let node = session.document().nodes.first().expect("there is a node");
        assert_eq!(node.kind, "vector.path");
        let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
        assert_eq!(vector.path.subpaths.len(), 1, "a rectangle is one contour");
        assert_eq!(vector.path.subpaths[0].segments.len(), 3, "a rectangle is three sides and a closing one");
        assert!(vector.path.subpaths[0].closed);
        assert_eq!(node.rect, Some(Rect::new(0.0, 0.0, 10.0, 5.0)), "the node knows where it is");
    }

    /// An ellipse is four curves, not four sides.
    #[test]
    fn an_ellipse_is_four_curves() {
        let mut session = session();
        session
            .execute("vector.ellipse", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the ellipse is added");
        let node = session.document().nodes.first().expect("there is a node");
        let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
        assert_eq!(vector.path.subpaths[0].segments.len(), 4);
        assert!(
            vector.path.subpaths[0].segments.iter().all(|segment| segment.controls().is_some()),
            "an ellipse is made of curves"
        );
    }

    /// A colour the plugin does not know is refused.
    #[test]
    fn a_colour_that_is_not_a_colour_is_refused() {
        let mut session = session();
        let error = session
            .execute(
                "vector.rectangle",
                &json!({ "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0, "fill": "mauve" }),
            )
            .expect_err("the colour is refused");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
        assert!(session.document().nodes.is_empty(), "the document is as it was");
    }

    /// A polygon with two sides is refused.
    #[test]
    fn a_polygon_with_too_few_sides_is_refused() {
        let mut session = session();
        let error = session
            .execute("vector.polygon", &json!({ "x": 0.0, "y": 0.0, "radius": 5.0, "sides": 2.0 }))
            .expect_err("the polygon is refused");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }

    /// The plan says what the paths add up to.
    #[test]
    fn the_plan_says_what_the_paths_add_up_to() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 5.0 }))
            .expect("the rectangle is added");
        let plan = session.evaluate();
        let total = plan
            .items
            .iter()
            .find(|item| item.kind == "vector.totalarea")
            .expect("the total area is in the plan");
        let area = total.value.get("area").and_then(|area| area.as_f64()).expect("an area");
        assert!((area - 50.0).abs() < 0.001, "the area is {area}, not 50");
    }

    /// The payload survives a round trip through a file.
    ///
    /// The core carries the payload and does not read it, and the file
    /// carries it too — so a document written by a program that had the
    /// plugin is still a document.
    #[test]
    fn the_payload_survives_a_round_trip() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0 }))
            .expect("the rectangle is added");
        let document = session.document().clone();
        let written = document.to_json();
        let read = craftkit_sdk::Document::from_json(&written).expect("the document is read");
        assert_eq!(read, document, "a round trip is the same document");
        let vector = VectorNode::from_json(&read.nodes[0].data).expect("the payload is the plugin's own");
        assert_eq!(vector.path.subpaths[0].segments.len(), 3);
    }

    /// A point inside a path is inside it, and a point outside is outside.
    #[test]
    fn a_path_is_hit_by_its_own_points() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the rectangle is added");
        let document = session.document();
        let node = document.nodes.first().expect("there is a node");
        assert!(VectorPathKind::hit(node, document, Point::new(5.0, 5.0)), "the middle is inside");
        assert!(!VectorPathKind::hit(node, document, Point::new(50.0, 5.0)), "the far side is outside");
    }

    /// Two paths can be united, and the second one is gone.
    #[test]
    fn two_paths_can_be_united() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the first rectangle is added");
        session
            .execute("vector.rectangle", &json!({ "x": 5.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the second rectangle is added");
        session
            .execute("vector.path.unite", &json!({ "first": 1, "second": 2 }))
            .expect("the paths are united");
        assert_eq!(session.document().nodes.len(), 1, "the two paths are one now");
        let node = &session.document().nodes[0];
        let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
        let area = geom::area(&vector.path);
        assert!((area - 150.0).abs() < 0.1, "the area is {area}, not 150");
    }

    /// The selection can be united.
    ///
    /// A person picks two shapes and says "make them one"; an agent names
    /// two ids. The command takes either, and the **selection** is the
    /// session's own state.
    #[test]
    fn the_selection_can_be_united() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the first rectangle is added");
        session
            .execute("vector.rectangle", &json!({ "x": 5.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the second rectangle is added");
        session.execute("select.all", &json!({})).expect("everything is selected");
        session.execute("vector.path.unite", &json!({})).expect("the selected paths are united");
        assert_eq!(session.document().nodes.len(), 1, "the two paths are one now");
        let node = &session.document().nodes[0];
        let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
        let area = geom::area(&vector.path);
        assert!((area - 150.0).abs() < 0.1, "the area is {area}, not 150");
    }

    /// A hole is a contour of its own.
    ///
    /// This is the thing a flat path could not hold, and the reason the
    /// path grew contours and a rule.
    #[test]
    fn a_hole_is_a_contour_of_its_own() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the outer rectangle is added");
        session
            .execute("vector.rectangle", &json!({ "x": 3.0, "y": 3.0, "width": 4.0, "height": 4.0 }))
            .expect("the inner rectangle is added");
        session
            .execute("vector.path.subtract", &json!({ "first": 1, "second": 2 }))
            .expect("the inner rectangle is taken away");
        let node = &session.document().nodes[0];
        let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
        assert!(vector.path.subpaths.len() > 1, "the shape has {} contour(s)", vector.path.subpaths.len());
        let area = geom::area(&vector.path);
        assert!((area - 84.0).abs() < 0.1, "the area is {area}, not 84");
    }

    /// A path can be pushed out, and the curves stay curves.
    #[test]
    fn a_path_can_be_pushed_out() {
        let mut session = session();
        session
            .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
            .expect("the rectangle is added");
        session
            .execute("vector.path.offset", &json!({ "node": 1, "distance": 1.0 }))
            .expect("the path is offset");
        let node = &session.document().nodes[0];
        let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
        let area = geom::area(&vector.path);
        assert!((area - 144.0).abs() < 0.5, "the area is {area}, not 144");
        assert_eq!(node.rect, Some(Rect::new(-1.0, -1.0, 12.0, 12.0)), "the node knows where it is");
    }

    /// Making a path simpler does not change what it covers.
    ///
    /// A simplification may drop the points a flat line would serve and
    /// keep the rest: a polygon with sharp corners keeps its corners, and
    /// what it covers is the same either way.
    #[test]
    fn making_a_path_simpler_does_not_change_what_it_covers() {
        let mut session = session();
        session
            .execute("vector.ellipse", &json!({ "x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0 }))
            .expect("the ellipse is added");
        let before = {
            let node = &session.document().nodes[0];
            let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
            geom::area(&vector.path)
        };
        session
            .execute("vector.path.simplify", &json!({ "node": 1, "tolerance": 0.05 }))
            .expect("the path is simpler");
        let after = {
            let node = &session.document().nodes[0];
            let vector = VectorNode::from_json(&node.data).expect("the payload is the plugin's own");
            geom::area(&vector.path)
        };
        assert!((before - after).abs() / before < 0.01, "the area went from {before} to {after}");
    }

    /// A command on a node that is not there is refused.
    #[test]
    fn a_command_on_a_node_that_is_not_there_is_refused() {
        let mut session = session();
        let error = session
            .execute("vector.path.simplify", &json!({ "node": 99 }))
            .expect_err("the node is not there");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }
}
