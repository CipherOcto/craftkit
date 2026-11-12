//! The **SVG** of a drawing: a drawing made into a picture file.
//!
//! This is the tree's own way of showing a drawing outside the tree:
//! the entities become SVG elements, the world is turned over — the
//! tree's world has Y up and an SVG has Y down — and the picture is
//! given a box.
//!
//! # The rules
//!
//! - The **box** of the picture is the **extents** of the drawing,
//!   with a **margin** of two percent of the larger side: the tree's
//!   own margin, so that nothing stands on the edge.
//! - The **world is turned over**: the tree's Y is up and an SVG's Y
//!   is down, so the drawing is flipped and its bottom left corner
//!   stands at the origin.
//! - The **numbers** carry at least six decimals, and more when the
//!   stroke would lose its leading digits: the tree's own rule.
//! - A drawing that has **no entity** has a box of one unit, and the
//!   picture is still a picture.
//! - The **colour** of an entity is the **kit's** colour (§103): its
//!   own appearance if it has one, else the appearance of the layer it
//!   is on, else black. This module keeps no second rule for it.
//! - An entity this tree cannot draw is **left out**, and the answer
//!   says what it left out.
//!
//! # What it does not do
//!
//! - The **blocks** of a drawing are not drawn as blocks: one
//!   block's entities are the picture's entities.
//!
//! - The tree draws a **display list**, and an **arc** in it is a
//!   run of short segments. Here an arc is an arc: the picture says
//!   `A` and the viewer draws the curve. The tree's rule about the
//!   box, the flip, the stroke and the decimals is the tree's; the
//!   shape of an arc is this tree's own.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "cad.export.svg";

/// The **width** of the picture, in pixels: the tree's own.
const WIDTH: f64 = 1600.0;

/// The **margin** of the picture, as a fraction of the larger side:
/// the tree's own.
const MARGIN: f64 = 0.02;

/// The **number** a geometry holds.
fn number_of(geometry: &Json, name: &str) -> Option<f64> {
    geometry.get(name).and_then(|value| value.as_f64()).filter(|value| value.is_finite())
}

/// The **point** a geometry holds.
fn point_of(geometry: &Json, name: &str) -> Option<(f64, f64)> {
    let point = geometry.get(name)?;
    let x = point.get("x")?.as_f64()?;
    let y = point.get("y")?.as_f64()?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}

/// The **text** of a number, with the decimals the tree's rule asks
/// for.
fn number_text(number: f64, decimals: usize) -> String {
    format!("{number:.decimals$}")
}

/// One **primitive** of the picture: the points it draws.
struct Primitive {
    /// The element's name.
    kind: &'static str,
    /// The points, in the drawing's own world.
    points: Vec<(f64, f64)>,
    /// The colour.
    colour: String,
    /// The radius of a circle.
    radius: f64,
    /// Whether a polyline is closed.
    closed: bool,
    /// The sweep of an arc.
    sweep: bool,
    /// The span of an arc, in radians.
    span: f64,
}

/// The **colour** of an entity, written as `#rrggbb`.
///
/// The colour is the **kit's** (§103): the entity's own appearance if
/// it has one, else the layer's, else black. This module does not keep
/// a second rule for the colour of a thing.
fn colour_of(document: &craftkit_sdk::Document, entity: &Node) -> String {
    let colour = crate::colour_of(document, entity);
    let channel = |value: f32| {
        let byte = (value.clamp(0.0, 1.0) * 255.0).round() as u32;
        format!("{byte:02x}")
    };
    format!("#{}{}{}", channel(colour.0[0]), channel(colour.0[1]), channel(colour.0[2]))
}

/// The **primitives** of a block: what the picture draws.
fn primitives_of(document: &craftkit_sdk::Document, block: &Node) -> (Vec<Primitive>, Vec<String>) {
    let mut out: Vec<Primitive> = Vec::new();
    let mut left_out: Vec<String> = Vec::new();
    for entity in block.children.iter() {
        let geometry = entity.data.get("geometry").cloned().unwrap_or(Json::Null);
        let colour = colour_of(document, entity);
        match entity.kind.as_str() {
            "cad.entity.line" => {
                let (Some(from), Some(to)) = (point_of(&geometry, "from"), point_of(&geometry, "to")) else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                out.push(Primitive {
                    kind: "path",
                    points: vec![from, to],
                    colour,
                    radius: 0.0,
                    closed: false,
                    sweep: false,
                    span: 0.0,
                });
            }
            "cad.entity.circle" => {
                let (Some(center), Some(radius)) =
                    (point_of(&geometry, "center"), number_of(&geometry, "radius"))
                else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                out.push(Primitive {
                    kind: "circle",
                    points: vec![center],
                    colour,
                    radius,
                    closed: false,
                    sweep: false,
                    span: 0.0,
                });
            }
            "cad.entity.arc" => {
                let (Some(center), Some(radius), Some(start), Some(end)) = (
                    point_of(&geometry, "center"),
                    number_of(&geometry, "radius"),
                    number_of(&geometry, "start"),
                    number_of(&geometry, "end"),
                ) else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                let from = (center.0 + radius * start.cos(), center.1 + radius * start.sin());
                let to = (center.0 + radius * end.cos(), center.1 + radius * end.sin());
                let span = end - start;
                out.push(Primitive {
                    kind: "arc",
                    points: vec![from, to],
                    colour,
                    radius,
                    closed: false,
                    // The world is turned over, so an arc that goes
                    // counter-clockwise here goes clockwise there.
                    sweep: false,
                    span,
                });
            }
            "cad.entity.polyline" => {
                let Some(points) = geometry.get("points").and_then(|points| points.as_array()) else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                let mut vertices: Vec<(f64, f64)> = Vec::new();
                for point in points {
                    let Some(x) = point.get("x").and_then(|x| x.as_f64()) else { continue };
                    let Some(y) = point.get("y").and_then(|y| y.as_f64()) else { continue };
                    if !x.is_finite() || !y.is_finite() {
                        continue;
                    }
                    vertices.push((x, y));
                }
                if vertices.len() < 2 {
                    left_out.push(entity.name.clone());
                    continue;
                }
                let closed = geometry.get("closed").and_then(|closed| closed.as_bool()).unwrap_or(false);
                out.push(Primitive {
                    kind: "polyline",
                    points: vertices,
                    colour,
                    radius: 0.0,
                    closed,
                    sweep: false,
                    span: 0.0,
                });
            }
            _ => left_out.push(entity.name.clone()),
        }
    }
    (out, left_out)
}

commands! {
    /// **Write** a drawing as an **SVG file**.
    ///
    /// The file is written where the caller says, and the answer says how
    /// many entities it holds and what it left out.
    cad.export.svg {
        area: "cad",
        layer: "cad",
        icon: "file",
        params: {
            /// Where to write the file.

            path: Text,
            /// The drawing to write.

            drawing: NodeRef,
        },
        run: {

            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a file has a path".to_string(),
                });
            }
            let drawing_id = self.drawing.0;
            let Some(drawing) = session.document().node(drawing_id).cloned() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("there is no drawing `{drawing_id}`"),
                });
            };
            if drawing.kind != "cad.drawing" {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{drawing_id}` is a `{}` and not a drawing", drawing.kind),
                });
            }
            // The **block** the picture is of: the model space, or the
            // first block of a drawing that has no model space.
            let blocks: Vec<std::sync::Arc<Node>> =
                drawing.children.iter().filter(|child| child.kind == "cad.block").cloned().collect();
            let model = blocks.iter().find(|block| block.name == "*Model_Space");
            let chosen = model.or_else(|| blocks.first());
            let (primitives, left_out) = match chosen {
                Some(block) => primitives_of(session.document(), block),
                None => (Vec::new(), Vec::new()),
            };
            // The **box**: the extents of the drawing, with the margin.
            let mut min_x = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            for primitive in &primitives {
                for (x, y) in &primitive.points {
                    min_x = min_x.min(*x - primitive.radius);
                    max_x = max_x.max(*x + primitive.radius);
                    min_y = min_y.min(*y - primitive.radius);
                    max_y = max_y.max(*y + primitive.radius);
                }
            }
            let empty = !min_x.is_finite() || !max_x.is_finite();
            let (width, height) =
                if empty { (1.0, 1.0) } else { ((max_x - min_x).max(1e-9), (max_y - min_y).max(1e-9)) };
            let margin = width.max(height) * MARGIN;
            let (x0, y1) = if empty { (0.0, 1.0) } else { (min_x - margin, max_y + margin) };
            let view_width = width + 2.0 * margin;
            let view_height = height + 2.0 * margin;
            // The **stroke** and the **decimals**: the tree's own rules.
            let stroke = view_width.max(view_height) / 1500.0;
            let decimals = (3.0 - stroke.log10().floor()).clamp(6.0, 15.0) as usize;
            // The **world turned over**: the tree's Y is up and an SVG's Y
            // is down.
            let tx = |x: f64| x - x0;
            let ty = |y: f64| y1 - y;
            let mut out = String::new();
            out.push_str(&format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>",
                number_text(view_width, decimals),
                number_text(view_height, decimals),
                WIDTH,
                (WIDTH * view_height / view_width).round(),
            ));
            for primitive in &primitives {
                match primitive.kind {
                    "path" => {
                        let (from, to) = (primitive.points[0], primitive.points[1]);
                        out.push_str(&format!(
                            "<path fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" d=\"M{} {}L{} {}\"/>",
                            primitive.colour,
                            number_text(stroke, decimals),
                            number_text(tx(from.0), decimals),
                            number_text(ty(from.1), decimals),
                            number_text(tx(to.0), decimals),
                            number_text(ty(to.1), decimals),
                        ));
                    }
                    "circle" => {
                        let center = primitive.points[0];
                        out.push_str(&format!(
                            "<circle fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                            primitive.colour,
                            number_text(stroke, decimals),
                            number_text(tx(center.0), decimals),
                            number_text(ty(center.1), decimals),
                            number_text(primitive.radius, decimals),
                        ));
                    }
                    "arc" => {
                        let (from, to) = (primitive.points[0], primitive.points[1]);
                        let large = primitive.span.abs() > std::f64::consts::PI;
                        out.push_str(&format!(
                            "<path fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" d=\"M{} {}A{} {} 0 {} {} {} {}\"/>",
                            primitive.colour,
                            number_text(stroke, decimals),
                            number_text(tx(from.0), decimals),
                            number_text(ty(from.1), decimals),
                            number_text(primitive.radius, decimals),
                            number_text(primitive.radius, decimals),
                            if large { 1 } else { 0 },
                            if primitive.sweep { 1 } else { 0 },
                            number_text(tx(to.0), decimals),
                            number_text(ty(to.1), decimals),
                        ));
                    }
                    _ => {
                        let mut points = String::new();
                        for (x, y) in &primitive.points {
                            points.push_str(&format!(
                                "{},{} ",
                                number_text(tx(*x), decimals),
                                number_text(ty(*y), decimals)
                            ));
                        }
                        // A **closed** polyline is a path that comes back
                        // to where it began.
                        if primitive.closed {
                            let mut path = String::new();
                            for (at, (x, y)) in primitive.points.iter().enumerate() {
                                let command = if at == 0 { "M" } else { "L" };
                                path.push_str(&format!(
                                    "{command}{} {} ",
                                    number_text(tx(*x), decimals),
                                    number_text(ty(*y), decimals)
                                ));
                            }
                            path.push('Z');
                            out.push_str(&format!(
                                "<path fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"{}\"/>",
                                primitive.colour,
                                number_text(stroke, decimals),
                                path.trim_end(),
                            ));
                        } else {
                            out.push_str(&format!(
                                "<polyline fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" points=\"{}\"/>",
                                primitive.colour,
                                number_text(stroke, decimals),
                                points.trim_end(),
                            ));
                        }
                    }
                }
            }
            out.push_str("</svg>\n");
            std::fs::write(&path, out.as_bytes()).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the file cannot be written to `{path}`: {error}"),
            })?;
            Ok(Outcome::new(format!("wrote {path}")).with_data(json!({
                "path": path,
                "entities": primitives.len(),
                "left_out": left_out,
                "bytes": out.len(),
            })))

        }
    }
}
