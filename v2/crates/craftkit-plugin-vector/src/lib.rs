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
//! - three **commands**, `vector.rectangle`, `vector.ellipse` and
//!   `vector.polygon`;
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

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel, Tool};
use craftkit_sdk::{
    Canvas, Colour, Document, EngineError, FormatValue, Node, NodeRef, Number, Outcome, Path, Plan, PlanItem,
    Point, Rect, Segment, Session, Stroke, SubPath, Text,
};

use craftkit_vector_geom as geom;

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
/// filled with, and the stroke on its outline.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorNode {
    /// The outline.
    pub path: Path,
    /// What it is filled with.
    pub fill: Colour,
    /// The stroke on its outline, if it has one.
    pub stroke: Option<Stroke>,
}

impl FormatValue for VectorNode {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "path": self.path.to_json(),
            "fill": self.fill.to_json(),
            "stroke": self.stroke.as_ref().map(|stroke| stroke.to_json()),
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let path = value.get("path").ok_or_else(|| "a vector node has no path".to_string())?;
        let fill = match value.get("fill") {
            Some(fill) => Colour::from_json(fill)?,
            None => Colour::BLACK,
        };
        let stroke = match value.get("stroke") {
            Some(stroke) if !stroke.is_null() => Some(Stroke::from_json(stroke)?),
            _ => None,
        };
        Ok(VectorNode { path: Path::from_json(path)?, fill, stroke })
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
        let (fill, stroke) = match document.appearance_of(node) {
            Some(appearance) => (appearance.fill, appearance.stroke),
            None => (vector.fill, vector.stroke),
        };
        let fill = if node.muted { Colour::NONE } else { fill };
        canvas.path(&vector.path, fill, stroke);
    }

    /// Whether a point is inside a path.
    ///
    /// The path is flattened to a polygon and the point is tested against
    /// it. A curve is tested by its samples, which is an approximation, and
    /// it is written down rather than hidden.
    fn hit(node: &Node, _document: &Document, point: Point) -> bool {
        let Ok(vector) = VectorNode::from_json(&node.data) else { return false };
        vector.path.subpaths.iter().any(|subpath| polygon_contains(&geom::flatten(subpath, 0.05), point))
    }

    /// What a path **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let vector = VectorNode::from_json(&node.data).ok()?;
        let segments: usize = vector.path.subpaths.iter().map(|subpath| subpath.segments.len()).sum();
        Some(serde_json::json!({
            "area": geom::area(&vector.path),
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
        "vector.path.offset"
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

// ------------------------------------------------------------- the helpers --

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
    let rect = geom::bounds(&vector.path);
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
    let vector = VectorNode { path: path.clone(), fill: colour, stroke: None };
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
    UnitePaths,
    SubtractPaths,
    IntersectPaths,
    SimplifyPath,
    OffsetPath,
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
