//! The **line cut**: a shape split along a line.
//!
//! The tree's Line Cut splits the shapes it is given along a **line** —
//! not a run of points, but a line that goes on for ever, through two
//! points the caller names — and each side of the line keeps the part
//! of the shape that is on it.
//!
//! The rules are the tree's own:
//!
//! - the line is through **two points**, and they must **differ**;
//! - the shape is cut by **intersection** with each **half-plane** of
//!   the line, and the pieces are the parts of what is left;
//! - a line that **misses** the shape leaves it as it was;
//! - a **hole** travels with the shape it is in.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, FormatValue, NodeRef, Outcome, Path, Point, Session, Text};

use craftkit_vector_geom as geom;

use crate::axis::Axis;
use crate::components::components;
use crate::{vector_of, write_back};

/// How far the **box** around the shape is grown, so that a line that
/// grazes the shape is still a line through it.
///
/// The tree's own: a unit.
const REACH: f64 = 1.0;

commands! {
    /// Split a closed shape along a line.
    vector.path.cut.line {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Cut by line"),
        params: {
            /// The path.
            path: NodeRef,
            /// The line's **first** point, `{"x": …, "y": …}` or `[x, y]`.
            from: Json,
            /// The line's **second** point.
            to: Json,
            /// What to call the pieces.
            name: Option<Text>,
        },
        run: {
            let id = "vector.path.cut.line".to_string();
            let from =
                read_point(&self.from).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let to = read_point(&self.to).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let vector = vector_of(session, self.path.0)?;
            if vector.path.subpaths.iter().any(|contour| !contour.closed) {
                return Err(EngineError::Failed {
                    id,
                    message: "the line cuts closed shapes, and this path has an open contour".to_string(),
                });
            }
            if vector.path.is_empty() {
                return Err(EngineError::Failed { id, message: "the path has no shape to cut".to_string() });
            }
            let Some(axis) = Axis::through(from, to) else {
                return Err(EngineError::Failed { id, message: "the line's two points must differ".to_string() });
            };
            // The two **sides** of the line: each a half of the box around
            // the shape, cut by the line.
            let box_of = box_around(&vector.path);
            let mut pieces: Vec<Path> = Vec::new();
            for side in [1.0, -1.0] {
                let Some(region) = axis.half_plane(side, box_of) else {
                    return Ok(
                        Outcome::new("the line did not reach the shape").with_data(json!({ "id": self.path.0 }))
                    );
                };
                let half = geom::boolean(&vector.path, &region, geom::BoolOp::Intersect)
                    .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
                if half.is_empty() {
                    return Ok(Outcome::new("the line did not cut the shape in two")
                        .with_data(json!({ "id": self.path.0 })));
                }
                pieces.extend(components(&half));
            }
            if pieces.len() < 2 {
                return Ok(
                    Outcome::new("the line did not cut the shape in two").with_data(json!({ "id": self.path.0 }))
                );
            }
            // One path per piece: the first keeps the node, and the others
            // are new nodes beside it.
            let name = match &self.name {
                Some(name) => name.0.clone(),
                None => session.document().node(self.path.0).map(|node| node.name.clone()).unwrap_or_default(),
            };
            let mut pieces = pieces.into_iter();
            let Some(first) = pieces.next() else {
                return Err(EngineError::Failed { id, message: "the line left no shape".to_string() });
            };
            let mut vector = vector;
            vector.path = first;
            write_back(session, self.path.0, &vector)?;
            let mut ids: Vec<u64> = vec![self.path.0];
            for piece in pieces {
                ids.push(add_piece(session, &name, &vector, piece)?);
            }
            Ok(Outcome::changed("the shape is cut in two").with_data(json!({ "ids": ids })))

        }
    }
}

/// Put a **piece** of a cut shape in the document, and give back its id.
///
/// The piece is the shape it was cut from, with the shape of the piece:
/// the same name, the same paint and the same place.
fn add_piece(
    session: &mut Session,
    name: &str,
    vector: &crate::VectorNode,
    path: Path,
) -> Result<u64, EngineError> {
    let mut piece = vector.clone();
    piece.path = path;
    let rect = geom::bounds(&piece.placed());
    let id =
        session.document_mut()?.add_node_with(name.to_string(), "vector.path", Some(rect), piece.to_json());
    Ok(id)
}

/// The **box** around a shape, grown by a unit.
fn box_around(path: &Path) -> (f64, f64, f64, f64) {
    let bounds = geom::bounds(path);
    (bounds.x - REACH, bounds.y - REACH, bounds.x + bounds.w + REACH, bounds.y + bounds.h + REACH)
}

/// A **point**, as the caller gave it.
fn read_point(value: &Json) -> Result<Point, String> {
    let (x, y) = match value {
        Json::Array(pair) => {
            (pair.first().and_then(|value| value.as_f64()), pair.get(1).and_then(|value| value.as_f64()))
        }
        Json::Object(table) => {
            (table.get("x").and_then(|value| value.as_f64()), table.get("y").and_then(|value| value.as_f64()))
        }
        _ => return Err("the point is not a point".to_string()),
    };
    match (x, y) {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Ok(Point::new(x, y)),
        _ => Err("the point has no numbers in it".to_string()),
    }
}
