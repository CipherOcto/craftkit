//! The **knife**: a closed shape cut along a run of points.
//!
//! The tree's Knife cuts closed shapes along the line the knife makes,
//! and leaves a **slit** between the pieces — a gap of a fixed width,
//! so that the pieces do not touch.
//!
//! The rules are the tree's own:
//!
//! - the knife is a **run of points**, and it needs two of them;
//! - the slit is the run **stroked** with a width of its own, so that
//!   the pieces are apart and not merely touching;
//! - only **closed shapes** are cut: an open path has no inside, and
//!   there is nothing to cut it into.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{
    Cap, EngineError, FormatValue, Join, NodeRef, Outcome, Path, Point, Session, SubPath, Text,
};

use craftkit_vector_geom as geom;

use crate::{vector_of, write_back};

/// The **width of the slit** the knife leaves between the pieces.
///
/// The tree's own: a fiftieth of a point, so that the pieces are apart
/// and the slit is not seen.
const KNIFE_GAP: f64 = 0.05;

/// The most **points** the knife's run may have.
const MAX_POINTS: usize = 10_000;

commands! {
    /// Cut a closed shape along a run of points.
    vector.path.knife {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Cut"),
        params: {
            /// The path.

            path: NodeRef,
            /// The knife's run: a list of points, `[[x, y], …]`.

            points: Json,
            /// What to call the pieces.

            name: Option<Text>,
        },
        run: {

            let id = "vector.path.knife".to_string();
            let run =
                read_run(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let vector = vector_of(session, self.path.0)?;
            if vector.path.subpaths.iter().any(|contour| !contour.closed) {
                return Err(EngineError::Failed {
                    id,
                    message: "the knife cuts closed shapes, and this path has an open contour".to_string(),
                });
            }
            if vector.path.subpaths.is_empty() {
                return Err(EngineError::Failed { id, message: "the path has no shape to cut".to_string() });
            }
            // The **slit**: the knife's run, stroked with a width of its
            // own.
            let slit = geom::outline_stroke(&run_path(&run), KNIFE_GAP, Cap::Butt, Join::Round, 4.0);
            let cut = geom::boolean(&vector.path, &slit, geom::BoolOp::Difference)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            if cut.subpaths.len() < 2 {
                return Ok(Outcome::new("the knife did not cut the shape in two")
                    .with_data(json!({ "id": self.path.0 })));
            }
            // One path per piece: the first keeps the node, and the others
            // are new nodes beside it.
            let name = match &self.name {
                Some(name) => name.0.clone(),
                None => session.document().node(self.path.0).map(|node| node.name.clone()).unwrap_or_default(),
            };
            // A **hole** travels with the shape it is in.
            let mut pieces = crate::components::components(&cut).into_iter();
            let Some(first) = pieces.next() else {
                return Err(EngineError::Failed { id, message: "the knife left no shape".to_string() });
            };
            let mut vector = vector;
            vector.path = first;
            write_back(session, self.path.0, &vector)?;
            let mut ids: Vec<u64> = vec![self.path.0];
            for piece in pieces {
                ids.push(add_piece(session, &name, &vector, piece)?);
            }
            Ok(Outcome::changed("the shape is cut").with_data(json!({ "ids": ids })))

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
    // The piece keeps the **rule** of the shape it was cut from: a path
    // with the even-odd rule does not become a path with the other one.
    piece.path = path;
    let rect = geom::bounds(&piece.placed());
    let id =
        session.document_mut()?.add_node_with(name.to_string(), "vector.path", Some(rect), piece.to_json());
    Ok(id)
}

/// The knife's **run**, as the caller gave it.
fn read_run(value: &Json) -> Result<Vec<Point>, String> {
    let Some(list) = value.as_array() else {
        return Err("the knife's run is not a list of points".to_string());
    };
    if list.len() < 2 {
        return Err("the knife needs at least two points".to_string());
    }
    if list.len() > MAX_POINTS {
        return Err(format!("the knife's run may have at most {MAX_POINTS} points"));
    }
    let mut run: Vec<Point> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        let (x, y) = match item {
            Json::Array(pair) => {
                (pair.first().and_then(|value| value.as_f64()), pair.get(1).and_then(|value| value.as_f64()))
            }
            Json::Object(table) => (
                table.get("x").and_then(|value| value.as_f64()),
                table.get("y").and_then(|value| value.as_f64()),
            ),
            _ => return Err(format!("the point {index} of the run is not a point")),
        };
        match (x, y) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {}
            _ => return Err(format!("the point {index} of the run has no numbers in it")),
        }
        run.push(Point::new(x.unwrap_or_default(), y.unwrap_or_default()));
    }
    Ok(run)
}

/// The knife's run as a **path**: a line through the points.
fn run_path(run: &[Point]) -> Path {
    let segments: Vec<craftkit_sdk::Segment> =
        run.windows(2).map(|pair| craftkit_sdk::Segment::Line { to: pair[1] }).collect();
    Path::single(SubPath { start: run[0], segments, closed: false })
}
