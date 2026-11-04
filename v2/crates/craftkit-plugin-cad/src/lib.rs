//! `craftkit-plugin-cad` — the document model of a **CAD drawing**.
//!
//! # What is ported
//!
//! A drawing is not a tree, and it is not a document of one thing. It is
//! **tables and a flat store**: layers, blocks and styles are named
//! tables, and the entities are a flat list with typed handles.
//!
//! - a **drawing** — a node;
//! - a **layer** — a node, and a table row;
//! - a **block** — a node, and a table row that **contains** entities;
//! - an **entity** — a node of an **open kind**: `cad.entity.line`,
//!   `cad.entity.circle`, `cad.entity.arc`, `cad.entity.ellipse`,
//!   `cad.entity.polyline`, `cad.entity.text` and `cad.entity.insert`;
//! - an entity's **layer** — a **relation**: an edge that says something,
//!   not a parent;
//! - a **block insert** — a node that **references** a block: an edge
//!   from the instance to the definition;
//! - a **layer's settings** — whether it is on, frozen or locked, its
//!   colour, the colour it is drawn in, its line type and its line
//!   weight (the module `layersettings`);
//! - the **arc** — a piece of a circle, with its centre, its radius
//!   and its two ends (the module `arcs`);
//! - the **audit** of a drawing — what is wrong with it, reported and
//!   not repaired (the module `audits`);
//! - the **mirror** of an entity — the same entity turned over a line
//!   (the module `mirrors`);
//! - the **copy** of an entity — a second entity of the same kind,
//!   with the same geometry and the same layer (the module `copies`);
//! - the **curves** — the ellipse, with its centre, its major axis
//!   and its ratio, and the polyline, with its points (the module
//!   `curves`);
//! - the **modification** of an entity — it may be moved, turned,
//!   scaled and taken away, and a change of the geometry is a change
//!   of the geometry and of nothing else (the module `modify`);
//! - the **order** the entities are drawn in — the children of a block
//!   are drawn in the order they stand in, and an entity may be
//!   brought to the front or sent to the back (the module `order`);
//! - the **measure** of a drawing — how long, how big, how wide and
//!   how far apart, and every one of them **derived** (the module
//!   `measurements`);
//! - the **geometry** — the entity's payload, in the world's units;
//! - the **extents** — derived, every time, by the evaluator.
//!
//! # The three things the port found
//!
//! - **A drawing is tables and a flat store, and the tree is not
//!   there.** Structure in a drawing is *reference by name* (a layer, a
//!   block) and by *handle* (a group, a constraint). So a layer is a
//!   **table** — a node — and an entity's layer is an **edge that says
//!   something** rather than a parent. That is the second time a port
//!   has answered "what is this relationship?" with an edge: the PDF
//!   port said it about a reference, and this says it about a layer.
//! - **A closed union of thirty kinds is what the open kind is for.**
//!   In the application every entity kind is a variant with its own
//!   struct, its own file mapping, its own transform and its own
//!   decomposition, and a new kind of entity is a change in four places
//!   at once. Here a kind is an **open string**: a new entity is a new
//!   kind, and nothing else moves.
//! - **The numbers belong to a library, and the core has no unit.** The
//!   geometry is in the payload, in the world's own units, and the
//!   evaluation that needs to do arithmetic reaches for a **library** —
//!   not for a unit in the core. This is what §39 said a numeric core
//!   would look like when it arrives, and it arrives as a plugin.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod arcs;
mod audits;
mod copies;
mod curves;
mod layersettings;
mod measurements;
mod mirrors;
mod modify;
mod order;

pub use arcs::AddArc;
pub use audits::AuditDrawing;
pub use copies::CopyEntity;
use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{
    Appearance, BlendMode, Canvas, Colour, Document, EngineError, Fill, Node, NodeRef, Number, Outcome, Path,
    Plan, PlanItem, Point, Relation, Session, Stroke, Text,
};
pub use curves::{AddEllipse, AddPolyline};
pub use layersettings::{
    FreezeLayer, ListLayers, LockLayer, SetLayerColor, SetLayerFill, SetLayerLinetype, SetLayerLineweight,
    SetLayerOn,
};
pub use measurements::{MeasureAngle, MeasureArea, MeasureLength, MeasureRadius};
pub use mirrors::MirrorEntity;
pub use modify::{MoveEntity, RemoveEntity, RotateEntity, ScaleEntity};
pub use order::{BackwardEntity, BringToFront, ForwardEntity, ListEntities, SendToBack};

/// The CAD layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "cad", name = "CAD", requires("core"))]
pub struct CadLayer;

/// A **drawing**.
#[derive(NodeKind)]
#[node_kind(kind = "cad.drawing", label = "Drawing", layer = "cad", evaluates)]
pub struct DrawingKind;

impl DrawingKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a drawing **means**: its layers and its blocks.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "layers": node.children.iter().filter(|child| child.kind == "cad.layer").count(),
            "blocks": node.children.iter().filter(|child| child.kind == "cad.block").count(),
        }))
    }
}

/// A **layer**: a named table row, and a thing entities are put on.
#[derive(NodeKind)]
#[node_kind(kind = "cad.layer", label = "Layer", layer = "cad", evaluates)]
pub struct LayerKind;

impl LayerKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a layer **means**: its name and its colour.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "name": node.name,
            "color": node.data.get("color").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// A **block**: a named table row that **contains** entities.
#[derive(NodeKind)]
#[node_kind(kind = "cad.block", label = "Block", layer = "cad", evaluates)]
pub struct BlockKind;

impl BlockKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a block **means**: its entities.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "entities": node.children.iter().filter(|child| child.kind.starts_with("cad.entity.")).count(),
        }))
    }
}

/// An **entity**: a line, a circle, an arc, a text, an insert.
macro_rules! entity_kind {
    ($struct:ident, $kind:literal, $label:literal) => {
        /// A kind of entity.
        #[derive(NodeKind)]
        #[node_kind(kind = $kind, label = $label, layer = "cad", evaluates)]
        pub struct $struct;

        impl $struct {
            /// How the entity is drawn.
            ///
            /// The **colour** is the layer's: an entity is *on* a layer,
            /// and the layer says what colour that is. An entity with no
            /// layer is drawn in black, because black is what a drawing
            /// is drawn in when it says nothing else.
            ///
            /// The document is here because the layer is **in** it: an
            /// entity that cannot see the document cannot know which
            /// layer it is on, and a colour it cannot know is a colour
            /// it has to carry itself.
            fn paint(node: &Node, document: &Document, canvas: &mut dyn craftkit_sdk::Canvas) {
                let Some(geometry) = node.data.get("geometry") else { return };
                let colour = colour_of(document, node);
                if $kind == "cad.entity.insert" {
                    paint_insert(node, document, geometry, colour, canvas);
                    return;
                }
                draw_geometry($kind, geometry, colour, canvas);
            }

            /// Whether a point is **on** the entity.
            ///
            /// A line and a circle have no inside: they are outlines,
            /// and a point is on one when it is **near** it — within the
            /// tolerance, which is this program's number and not the
            /// core's.
            fn hit(node: &Node, document: &Document, point: Point) -> bool {
                let Some(geometry) = node.data.get("geometry") else { return false };
                if $kind == "cad.entity.insert" {
                    return hit_insert(node, document, geometry, point);
                }
                hit_geometry($kind, geometry, point)
            }

            /// What an entity **means**: its geometry, and the extents it
            /// covers. The extents are **derived**: they are worked out
            /// from the geometry every time.
            fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
                Some(json!({
                    "geometry": node.data.get("geometry").cloned().unwrap_or(Json::Null),
                    "extents": extents_of(node),
                }))
            }
        }
    };
}

entity_kind!(LineKind, "cad.entity.line", "Line");
entity_kind!(CircleKind, "cad.entity.circle", "Circle");
entity_kind!(ArcKind, "cad.entity.arc", "Arc");
entity_kind!(TextKind, "cad.entity.text", "Text");
entity_kind!(InsertKind, "cad.entity.insert", "Insert");
entity_kind!(EllipseKind, "cad.entity.ellipse", "Ellipse");
entity_kind!(PolylineKind, "cad.entity.polyline", "Polyline");

/// The colour an entity is drawn in.
///
/// It is the **layer's** colour: the entity is on a layer, the layer
/// holds an appearance, and the appearance is the kit's own. An entity
/// with no layer — or a layer with no appearance — is drawn in black.
fn colour_of(document: &Document, node: &Node) -> Colour {
    // The rule is the **kit's**: a node with no appearance of its own
    // takes the appearance of the layer it is on (§103), and the
    // membership is the kit's edge.
    document.appearance_of(node).map(|appearance| appearance.fill.colour()).unwrap_or(Colour::BLACK)
}

/// Draw an entity's **geometry**.
///
/// Each kind draws its own shape: a line is a line, a circle is a
/// circle, and a text is a text. The circle is a **polygon** of
/// straight segments — the kit has no curves in its drawing except the
/// ones a path carries — and the number of them is the circle's own.
/// The **block** an insert refers to, if it is there.
///
/// The reference is an **edge** — an instance and its definition are two
/// things — and it is followed once, here.
fn inserted_block<'a>(document: &'a Document, node: &Node) -> Option<&'a Node> {
    let relations = document.relations_from(node.id, Some("cad.inserts"));
    let relation = relations.first()?;
    document.node(relation.to)
}

/// The entities of the block an insert refers to.
///
/// They are the block's **children**: a block contains its entities and
/// an insert references one.
fn inserted_entities<'a>(document: &'a Document, node: &Node) -> Vec<&'a Node> {
    let Some(block) = inserted_block(document, node) else { return Vec::new() };
    block
        .children
        .iter()
        .map(|child| &**child)
        .filter(|child| child.kind.starts_with("cad.entity."))
        .collect()
}

/// Draw an **insert**: the block it inserts, drawn where it is put.
///
/// An insert is a **reference**, and a reference that is drawn as
/// nothing is a reference the person cannot see. So the block's own
/// entities are drawn, moved by the insertion point.
///
/// The colour is the **insert's**: the insert is on a layer, and the
/// layer says what colour that is. The block's entities keep their own
/// layers for the rest, and an insert of an insert is not followed a
/// second time: a reference is followed once.
fn paint_insert(node: &Node, document: &Document, geometry: &Json, colour: Colour, canvas: &mut dyn Canvas) {
    let Some(at) = point_in(geometry, "at") else { return };
    for entity in inserted_entities(document, node) {
        let Some(own) = entity.data.get("geometry") else { continue };
        if entity.kind == "cad.entity.insert" {
            continue;
        }
        draw_geometry(&entity.kind, &translated(own, at), colour, canvas);
    }
}

/// Whether a point is **on** an insert.
///
/// It is on one when it is on the block it inserts: the insert is drawn
/// as the block, so it is hit as the block.
///
/// This is the reason the hit test is given the **document** (§102): a
/// kind that follows a reference cannot follow it without the
/// document, and an insert that is drawn as a block and hit as a point
/// is a drawing and a picking that disagree.
fn hit_insert(node: &Node, document: &Document, geometry: &Json, point: Point) -> bool {
    let Some(at) = point_in(geometry, "at") else { return false };
    inserted_entities(document, node).into_iter().any(|entity| {
        if entity.kind == "cad.entity.insert" {
            return false;
        }
        let Some(own) = entity.data.get("geometry") else { return false };
        hit_geometry(&entity.kind, &translated(own, at), point)
    })
}

/// Move every **point** of a geometry by an offset.
///
/// A geometry is points and numbers — a radius is a number and a point
/// is a point — so the move is done by looking for the things that are
/// points: an object with an `x` and a `y` in it.
fn translated(geometry: &Json, offset: Point) -> Json {
    match geometry {
        Json::Object(map) => {
            let mut moved = craftkit_sdk::serde_json::Map::new();
            let is_point = map.get("x").and_then(|x| x.as_f64()).is_some()
                && map.get("y").and_then(|y| y.as_f64()).is_some();
            for (key, value) in map.iter() {
                if is_point {
                    moved.insert(key.clone(), value.clone());
                } else {
                    moved.insert(key.clone(), translated(value, offset));
                }
            }
            if is_point {
                let x = map.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0) + offset.x;
                let y = map.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0) + offset.y;
                moved.insert("x".to_string(), Json::from(x));
                moved.insert("y".to_string(), Json::from(y));
            }
            Json::Object(moved)
        }
        Json::Array(list) => Json::Array(list.iter().map(|value| translated(value, offset)).collect()),
        other => other.clone(),
    }
}
fn draw_geometry(kind: &str, geometry: &Json, colour: Colour, canvas: &mut dyn Canvas) {
    match kind {
        "cad.entity.line" => {
            let (Some(from), Some(to)) = (point_in(geometry, "from"), point_in(geometry, "to")) else {
                return;
            };
            let path = Path::polygon(&[from, to]);
            canvas.path(
                &path,
                Fill::Solid(Colour::NONE),
                BlendMode::Normal,
                Some(Stroke { colour, width: 1.0, ..Default::default() }),
            );
        }
        "cad.entity.circle" => {
            let Some(center) = point_in(geometry, "center") else { return };
            let Some(radius) = geometry
                .get("radius")
                .and_then(|radius| radius.as_f64())
                .filter(|r| r.is_finite() && *r > 0.0)
            else {
                return;
            };
            let mut points = Vec::with_capacity(CIRCLE_SEGMENTS);
            for step in 0..CIRCLE_SEGMENTS {
                let angle = step as f64 / CIRCLE_SEGMENTS as f64 * std::f64::consts::TAU;
                points.push(Point::new(center.x + radius * angle.cos(), center.y + radius * angle.sin()));
            }
            let path = Path::polygon(&points);
            canvas.path(
                &path,
                Fill::Solid(Colour::NONE),
                BlendMode::Normal,
                Some(Stroke { colour, width: 1.0, ..Default::default() }),
            );
        }
        "cad.entity.arc" => {
            let points = arcs::arc_points(geometry);
            if points.len() < 2 {
                return;
            }
            let path = Path::polygon(&points.iter().map(|(x, y)| Point::new(*x, *y)).collect::<Vec<_>>());
            canvas.path(
                &path,
                Fill::Solid(Colour::NONE),
                BlendMode::Normal,
                Some(Stroke { colour, width: 1.0, ..Default::default() }),
            );
        }
        "cad.entity.ellipse" => {
            let points = curves::ellipse_points(geometry);
            if points.len() < 2 {
                return;
            }
            let path = Path::polygon(&points.iter().map(|(x, y)| Point::new(*x, *y)).collect::<Vec<_>>());
            canvas.path(
                &path,
                Fill::Solid(Colour::NONE),
                BlendMode::Normal,
                Some(Stroke { colour, width: 1.0, ..Default::default() }),
            );
        }
        "cad.entity.polyline" => {
            let points = curves::polyline_points(geometry);
            if points.len() < 2 {
                return;
            }
            let path = Path::polygon(&points.iter().map(|(x, y)| Point::new(*x, *y)).collect::<Vec<_>>());
            canvas.path(
                &path,
                Fill::Solid(Colour::NONE),
                BlendMode::Normal,
                Some(Stroke { colour, width: 1.0, ..Default::default() }),
            );
        }
        "cad.entity.text" => {
            let Some(at) = point_in(geometry, "at") else { return };
            let Some(text) = geometry.get("text").and_then(|text| text.as_str()) else { return };
            // The text's **height** is its own: a text that says how
            // tall it is knows how big it is drawn.
            let height = geometry.get("height").and_then(|height| height.as_f64()).unwrap_or(TEXT_HEIGHT);
            canvas.text(at, text, colour, height);
        }
        _ => {}
    }
}

/// Whether a point is **on** an entity.
///
/// A line is near when the point is near the segment; a circle is near
/// when the point is near the **circumference** — not near the middle,
/// which is the inside of the circle and not the circle.
fn hit_geometry(kind: &str, geometry: &Json, point: Point) -> bool {
    match kind {
        "cad.entity.line" => {
            let (Some(from), Some(to)) = (point_in(geometry, "from"), point_in(geometry, "to")) else {
                return false;
            };
            distance_to_segment(point, from, to) <= TOLERANCE
        }
        "cad.entity.circle" => {
            let Some(center) = point_in(geometry, "center") else { return false };
            let Some(radius) = geometry
                .get("radius")
                .and_then(|radius| radius.as_f64())
                .filter(|r| r.is_finite() && *r > 0.0)
            else {
                return false;
            };
            let away = ((point.x - center.x).powi(2) + (point.y - center.y).powi(2)).sqrt();
            (away - radius).abs() <= TOLERANCE
        }
        "cad.entity.arc" => near_a_curve(&arcs::arc_points(geometry), point),
        "cad.entity.ellipse" => {
            // The curve is sampled and the point is near one of its
            // segments: the tree's bound is exact and this one is
            // sampled.
            near_a_curve(&curves::ellipse_points(geometry), point)
        }
        "cad.entity.polyline" => near_a_curve(&curves::polyline_points(geometry), point),
        _ => false,
    }
}

/// How near a point has to be to be **on** a line.
///
/// It is this program's number: a drawing is in millimetres and a
/// millimetre is a fine thing to hit.
const TOLERANCE: f64 = 1.0;

/// Whether a point is **on** a curve: near one of its segments.
fn near_a_curve(points: &[(f64, f64)], point: Point) -> bool {
    if points.len() < 2 {
        return false;
    }
    points.windows(2).any(|pair| {
        let from = Point::new(pair[0].0, pair[0].1);
        let to = Point::new(pair[1].0, pair[1].1);
        distance_to_segment(point, from, to) <= TOLERANCE
    })
}

/// The distance from a point to a segment.
fn distance_to_segment(point: Point, from: Point, to: Point) -> f64 {
    let across = to.x - from.x;
    let down = to.y - from.y;
    let length = across * across + down * down;
    if length <= f64::MIN_POSITIVE {
        return ((point.x - from.x).powi(2) + (point.y - from.y).powi(2)).sqrt();
    }
    let along = ((point.x - from.x) * across + (point.y - from.y) * down) / length;
    let along = along.clamp(0.0, 1.0);
    let nearest = Point::new(from.x + along * across, from.y + along * down);
    ((point.x - nearest.x).powi(2) + (point.y - nearest.y).powi(2)).sqrt()
}

/// How many straight segments a circle is drawn with.
///
/// Sixty-four is enough that a circle looks like a circle at the sizes
/// a picture is drawn at, and few enough that the picture is quick.
const CIRCLE_SEGMENTS: usize = 64;

/// How tall a text is drawn when it does not say: the **em**, in the
/// world's units.
const TEXT_HEIGHT: f64 = 4.0;

/// A point of a geometry, read from the caller's JSON.
fn point_in(geometry: &Json, name: &str) -> Option<Point> {
    let value = geometry.get(name)?;
    let x = value.get("x").and_then(|x| x.as_f64())?;
    let y = value.get("y").and_then(|y| y.as_f64())?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(Point::new(x, y))
}

/// The **extents** of an entity: the box its geometry covers.
///
/// They are **derived**, and they are computed from the geometry in the
/// payload: the numbers are the payload's and the library's, and the
/// core has no unit and no opinion.
fn extents_of(node: &Node) -> Json {
    let Some(geometry) = node.data.get("geometry") else { return Json::Null };
    let mut lowest = (f64::INFINITY, f64::INFINITY);
    let mut highest = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut any = false;
    for point in points_of(geometry) {
        any = true;
        lowest.0 = lowest.0.min(point.0);
        lowest.1 = lowest.1.min(point.1);
        highest.0 = highest.0.max(point.0);
        highest.1 = highest.1.max(point.1);
    }
    if !any {
        return Json::Null;
    }
    json!({
        "min": { "x": lowest.0, "y": lowest.1 },
        "max": { "x": highest.0, "y": highest.1 },
    })
}

/// The points of a geometry, in the order they are written.
///
/// A line has two, a circle has its centre and a point on it, an arc
/// has a centre and two ends, a text has its insertion point, and an
/// insert has its insertion point.
fn points_of(geometry: &Json) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let read = |value: &Json, points: &mut Vec<(f64, f64)>| {
        let Some(x) = value.get("x").and_then(|x| x.as_f64()) else { return };
        let Some(y) = value.get("y").and_then(|y| y.as_f64()) else { return };
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        points.push((x, y));
    };
    match geometry {
        Json::Array(list) => {
            for value in list {
                read(value, &mut points);
            }
        }
        Json::Object(map) => {
            // A polyline is a list of points.
            if let Some(list) = map.get("points").and_then(|points| points.as_array()) {
                for value in list {
                    read(value, &mut points);
                }
            }
            // An arc is a curve, and its box is the box of the curve
            // and not the box of its two ends: the bulge is in the box.
            if map.contains_key("radius") && map.contains_key("start") && map.contains_key("end") {
                for (x, y) in arcs::arc_points(geometry) {
                    points.push((x, y));
                }
                return points;
            }
            // An ellipse is a curve, and its box is worked out from its
            // two half axes.
            if map.contains_key("major") {
                for (x, y) in curves::ellipse_points(geometry) {
                    points.push((x, y));
                }
                return points;
            }
            for key in ["from", "to", "center", "start", "end", "at"] {
                if let Some(value) = map.get(key) {
                    read(value, &mut points);
                }
            }
            if let Some(value) = map.get("radius")
                && let Some(center) = map.get("center")
                && let Some(x) = center.get("x").and_then(|x| x.as_f64()).filter(|x| x.is_finite())
                && let Some(y) = center.get("y").and_then(|y| y.as_f64()).filter(|y| y.is_finite())
                && let Some(radius) = value.as_f64().filter(|radius| radius.is_finite() && *radius > 0.0)
            {
                points.push((x - radius, y - radius));
                points.push((x + radius, y + radius));
            }
        }
        _ => {}
    }
    points
}

/// A point, read from the caller's JSON.
fn point(value: &Json, id: &str, what: &str) -> Result<(f64, f64), EngineError> {
    let x = value.get("x").and_then(|x| x.as_f64()).unwrap_or(f64::NAN);
    let y = value.get("y").and_then(|y| y.as_f64()).unwrap_or(f64::NAN);
    if !x.is_finite() || !y.is_finite() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is a point in the world's units"),
        });
    }
    Ok((x, y))
}

/// Put a **drawing** in the document.
#[derive(Command)]
#[command(id = "cad.drawing.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddDrawing {
    /// What to call it.
    pub name: Text,
}

impl AddDrawing {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "cad.drawing.add".to_string(),
                message: "a drawing has a name".to_string(),
            });
        }
        let id =
            session.document_mut()?.add_node_with_under(None, name.clone(), "cad.drawing", None, json!({}));
        Ok(Outcome::changed(format!("added the drawing `{name}`"))
            .with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **layer** in a drawing.
#[derive(Command)]
#[command(id = "cad.layer.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddLayer {
    /// The drawing it goes in.
    pub drawing: NodeRef,
    /// The layer's name.
    pub name: Text,
    /// Its colour, as an index or a name.
    pub color: Option<Text>,
    /// The colour it is drawn in, as `#rrggbb`.
    ///
    /// It is the kit's own **appearance**, and the entities on the
    /// layer are drawn in it. A layer that says nothing is drawn in
    /// black.
    pub fill: Option<Text>,
}

impl AddLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "cad.layer.add".to_string(),
                message: "a layer has a name".to_string(),
            });
        }
        let drawing = self.drawing.0;
        if session.document().node(drawing).is_none() {
            return Err(EngineError::Failed {
                id: "cad.layer.add".to_string(),
                message: format!("there is no drawing `{drawing}`"),
            });
        }
        let color = self.color.as_ref().map(|color| color.0.clone());
        let appearance =
            match &self.fill {
                Some(fill) => Some(Appearance {
                    fill: Fill::Solid(Colour::from_hex(&fill.0).map_err(|message| EngineError::Failed {
                        id: "cad.layer.add".to_string(),
                        message,
                    })?),
                    stroke: None,
                    blend: BlendMode::Normal,
                }),
                None => None,
            };
        let id = session.document_mut()?.add_node_with_under(
            Some(drawing),
            name.clone(),
            "cad.layer",
            None,
            json!({ "color": color }),
        );
        if let Some(node) = session.document_mut()?.node_mut(id) {
            node.appearance = appearance;
        }
        Ok(Outcome::changed(format!("added the layer `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **block** in a drawing.
#[derive(Command)]
#[command(id = "cad.block.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddBlock {
    /// The drawing it goes in.
    pub drawing: NodeRef,
    /// The block's name.
    pub name: Text,
}

impl AddBlock {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "cad.block.add".to_string(),
                message: "a block has a name".to_string(),
            });
        }
        let drawing = self.drawing.0;
        if session.document().node(drawing).is_none() {
            return Err(EngineError::Failed {
                id: "cad.block.add".to_string(),
                message: format!("there is no drawing `{drawing}`"),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            Some(drawing),
            name.clone(),
            "cad.block",
            None,
            json!({}),
        );
        Ok(Outcome::changed(format!("added the block `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **line** in a block.
#[derive(Command)]
#[command(id = "cad.entity.line.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddLine {
    /// The block it goes in.
    pub block: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Where it starts.
    pub from: Json,
    /// Where it ends.
    pub to: Json,
}

impl AddLine {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let geometry = json!({
            "from": { "x": point(&self.from, "cad.entity.line.add", "the start")?.0, "y": point(&self.from, "cad.entity.line.add", "the start")?.1 },
            "to": { "x": point(&self.to, "cad.entity.line.add", "the end")?.0, "y": point(&self.to, "cad.entity.line.add", "the end")?.1 },
        });
        let entity = add_entity(
            session,
            self.block.0,
            &self.name.0,
            "cad.entity.line",
            geometry,
            "cad.entity.line.add",
        )?;
        Ok(Outcome::changed(format!("added the line `{}`", self.name.0))
            .with_data(json!({ "id": entity, "name": self.name.0 })))
    }
}

/// Put a **circle** in a block.
#[derive(Command)]
#[command(id = "cad.entity.circle.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddCircle {
    /// The block it goes in.
    pub block: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Its centre.
    pub center: Json,
    /// Its radius.
    pub radius: Number,
}

impl AddCircle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let (x, y) = point(&self.center, "cad.entity.circle.add", "the centre")?;
        let radius = self.radius.0;
        if !radius.is_finite() || radius <= 0.0 {
            return Err(EngineError::Failed {
                id: "cad.entity.circle.add".to_string(),
                message: "a radius is a number above zero".to_string(),
            });
        }
        let geometry = json!({ "center": { "x": x, "y": y }, "radius": radius });
        let entity = add_entity(
            session,
            self.block.0,
            &self.name.0,
            "cad.entity.circle",
            geometry,
            "cad.entity.circle.add",
        )?;
        Ok(Outcome::changed(format!("added the circle `{}`", self.name.0))
            .with_data(json!({ "id": entity, "name": self.name.0 })))
    }
}

/// Put a **text** in a block.
#[derive(Command)]
#[command(id = "cad.entity.text.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddText {
    /// The block it goes in.
    pub block: NodeRef,
    /// What to call it.
    pub name: Text,
    /// What it says.
    pub text: Text,
    /// Where it is written.
    pub at: Json,
    /// How big it is.
    pub height: Option<Number>,
}

impl AddText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let (x, y) = point(&self.at, "cad.entity.text.add", "the insertion point")?;
        let height = match &self.height {
            Some(height) if height.0.is_finite() && height.0 > 0.0 => height.0,
            Some(_) => {
                return Err(EngineError::Failed {
                    id: "cad.entity.text.add".to_string(),
                    message: "a height is a number above zero".to_string(),
                });
            }
            None => 1.0,
        };
        let geometry = json!({ "at": { "x": x, "y": y }, "height": height, "text": self.text.0 });
        let entity = add_entity(
            session,
            self.block.0,
            &self.name.0,
            "cad.entity.text",
            geometry,
            "cad.entity.text.add",
        )?;
        Ok(Outcome::changed(format!("added the text `{}`", self.name.0))
            .with_data(json!({ "id": entity, "name": self.name.0 })))
    }
}

/// Put an **insert** in a block: an instance of another block.
///
/// The insert **references** the block it inserts, and the reference is
/// an **edge**: an instance and its definition are two things, and the
/// edge between them says which.
#[derive(Command)]
#[command(id = "cad.entity.insert.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddInsert {
    /// The block the instance goes in.
    pub block: NodeRef,
    /// What to call it.
    pub name: Text,
    /// The block it inserts.
    pub inserts: NodeRef,
    /// Where it is put.
    pub at: Json,
}

impl AddInsert {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let inserts = self.inserts.0;
        let block_name = {
            let Some(block) = session.document().node(inserts) else {
                return Err(EngineError::Failed {
                    id: "cad.entity.insert.add".to_string(),
                    message: format!("there is no block `{inserts}`"),
                });
            };
            if block.kind != "cad.block" {
                return Err(EngineError::Failed {
                    id: "cad.entity.insert.add".to_string(),
                    message: format!("the node `{inserts}` is not a block"),
                });
            }
            block.name.clone()
        };
        let (x, y) = point(&self.at, "cad.entity.insert.add", "the insertion point")?;
        let geometry = json!({ "at": { "x": x, "y": y } });
        let entity = add_entity(
            session,
            self.block.0,
            &self.name.0,
            "cad.entity.insert",
            geometry,
            "cad.entity.insert.add",
        )?;
        session.document_mut()?.add_relation(Relation::with_payload(
            entity,
            inserts,
            "cad.inserts",
            json!({ "block": block_name }),
        ));
        Ok(Outcome::changed(format!("added the insert `{}`", self.name.0))
            .with_data(json!({ "id": entity, "inserts": inserts })))
    }
}

/// Put an entity on a **layer**.
///
/// The layer is **not** the entity's parent: the entity is in a block,
/// and it is *on* a layer. So this is an **edge**, and it says what it
/// is.
#[derive(Command)]
#[command(id = "cad.entity.layer", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct PutOnLayer {
    /// The entity.
    pub entity: NodeRef,
    /// The layer.
    pub layer: NodeRef,
}

impl PutOnLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let entity = self.entity.0;
        let layer = self.layer.0;
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: "cad.entity.layer".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        if node.kind != "cad.layer" {
            return Err(EngineError::Failed {
                id: "cad.entity.layer".to_string(),
                message: format!("the node `{layer}` is not a layer"),
            });
        }
        let name = node.name.clone();
        let added = session.document_mut()?.add_relation(Relation::with_payload(
            entity,
            layer,
            craftkit_sdk::layer::ON_LAYER,
            json!({ "layer": name.clone() }),
        ));
        if !added {
            return Err(EngineError::Failed {
                id: "cad.entity.layer".to_string(),
                message: format!("the entity `{entity}` is already on the layer `{name}`"),
            });
        }
        Ok(Outcome::changed(format!("put the entity {entity} on the layer `{name}`"))
            .with_data(json!({ "entity": entity, "layer": layer, "name": name })))
    }
}

/// Put an entity in a block, with its geometry, and give back its id.
pub(crate) fn add_entity(
    session: &mut Session,
    block: u64,
    name: &str,
    kind: &str,
    geometry: Json,
    id: &str,
) -> Result<u64, EngineError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(EngineError::Failed { id: id.to_string(), message: "an entity has a name".to_string() });
    }
    let Some(node) = session.document().node(block) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no block `{block}`"),
        });
    };
    if node.kind != "cad.block" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{block}` is not a block"),
        });
    }
    let entity = session.document_mut()?.add_node_with_under(
        Some(block),
        name.clone(),
        kind,
        None,
        json!({ "geometry": geometry }),
    );
    Ok(entity)
}

/// The evaluator: the **extents** of a drawing.
///
/// They are derived: every entity's box is worked out from its
/// geometry, and the drawing's box is the box that holds them all.
#[derive(Evaluator)]
#[evaluator(id = "cad.extents", label = "Extents", layer = "cad")]
pub struct ExtentsEvaluator;

impl ExtentsEvaluator {
    /// The extents of every drawing.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for drawing in document.walk().filter(|node| node.kind == "cad.drawing") {
            let mut lowest = (f64::INFINITY, f64::INFINITY);
            let mut highest = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            let mut any = false;
            for block in drawing.children.iter().filter(|child| child.kind == "cad.block") {
                for entity in block.children.iter().filter(|child| child.kind.starts_with("cad.entity.")) {
                    // A layer that is off or frozen is not drawn, and
                    // what is not drawn is not in the extents.
                    let hidden = document
                        .relations_from(entity.id, Some(craftkit_sdk::layer::ON_LAYER))
                        .iter()
                        .filter_map(|relation| document.node(relation.to))
                        .any(|layer| !layersettings::is_drawn(layer));
                    if hidden {
                        continue;
                    }
                    let Some(geometry) = entity.data.get("geometry") else { continue };
                    for (x, y) in points_of(geometry) {
                        any = true;
                        lowest.0 = lowest.0.min(x);
                        lowest.1 = lowest.1.min(y);
                        highest.0 = highest.0.max(x);
                        highest.1 = highest.1.max(y);
                    }
                }
            }
            let value = if any {
                json!({
                    "min": { "x": lowest.0, "y": lowest.1 },
                    "max": { "x": highest.0, "y": highest.1 },
                })
            } else {
                Json::Null
            };
            plan.items.push(PlanItem { node: drawing.id, kind: "cad.extents".to_string(), value });
        }
        plan
    }
}

/// The panel that shows the drawing.
#[derive(Panel)]
#[panel(
    id = "panel.cad",
    label = "CAD",
    layer = "cad",
    commands("cad.drawing.add", "cad.layer.add", "cad.block.add", "cad.entity.line.add", "cad.entity.layer")
)]
pub struct CadPanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    CadLayer,
    DrawingKind,
    LayerKind,
    BlockKind,
    LineKind,
    CircleKind,
    ArcKind,
    TextKind,
    InsertKind,
    EllipseKind,
    PolylineKind,
    AddDrawing,
    AddLayer,
    AddBlock,
    AddLine,
    AddCircle,
    AddText,
    AddInsert,
    AddEllipse,
    AddPolyline,
    AddArc,
    CopyEntity,
    MirrorEntity,
    AuditDrawing,
    ListEntities,
    MoveEntity,
    RotateEntity,
    ScaleEntity,
    RemoveEntity,
    BringToFront,
    SendToBack,
    ForwardEntity,
    BackwardEntity,
    PutOnLayer,
    SetLayerOn,
    FreezeLayer,
    LockLayer,
    SetLayerColor,
    SetLayerFill,
    SetLayerLinetype,
    SetLayerLineweight,
    ListLayers,
    MeasureLength,
    MeasureArea,
    MeasureRadius,
    MeasureAngle,
    ExtentsEvaluator,
    CadPanel
);
