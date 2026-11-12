//! The **mirror and cut**: a shape mirrored in an axis.
//!
//! The tree's Mirror & Cut cuts the shapes it is given at an axis,
//! keeps the **side** that is named, and joins what is kept to its own
//! **mirror** across the axis — so that the shape that is left is
//! **symmetric** about the axis.
//!
//! The rules are the tree's own:
//!
//! - the axis is **vertical**, **horizontal** or **free** — a line
//!   through two points — and a vertical or horizontal axis runs
//!   through the point it is given;
//! - the side that is kept is **left**, **right**, **top** or
//!   **bottom**, and the default is **left**, or **top** when the axis
//!   runs more across than down;
//! - a side that is **not** a side of the axis is refused;
//! - what is kept is the shape **cut** at the axis, and the result is
//!   the shape **joined** to its mirror.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Path, Point, Text};

use craftkit_vector_geom as geom;

use crate::axis::Axis;
use crate::{vector_of, write_back};

/// How far the **box** around the shape is grown, so that the axis
/// reaches the shape even when it grazes it.
///
/// The tree's own: a unit.
const REACH: f64 = 1.0;

commands! {
    /// Mirror a closed shape in an axis, and join it to its mirror.
    vector.path.cut.mirror {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Mirror and cut"),
        params: {
            /// The path.

            path: NodeRef,
            /// The axis: `vertical` (the default), `horizontal` or `free`.

            axis: Option<Text>,
            /// A point on the axis — where a vertical or horizontal axis runs.

            from: Option<Json>,
            /// The other point of the axis, when it is `free`.

            to: Option<Json>,
            /// The side that is kept: `left`, `right`, `top` or `bottom`.

            keep: Option<Text>,
        },
        methods: {
            /// The point a **vertical** or **horizontal** axis runs through: the
            /// one the caller names, or the **centre** of the shape.
            fn at(&self, vector: &crate::VectorNode) -> Result<Point, EngineError> {
                if let Some(from) = &self.from {
                    return read_point(from).map_err(|message| EngineError::Failed {
                        id: "vector.path.cut.mirror".to_string(),
                        message,
                    });
                }
                let bounds = geom::bounds(&vector.path);
                Ok(Point::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0))
            }

        },
        run: {

            let id = "vector.path.cut.mirror".to_string();
            let vector = vector_of(session, self.path.0)?;
            if vector.path.subpaths.iter().any(|contour| !contour.closed) {
                return Err(EngineError::Failed {
                    id,
                    message: "the mirror cuts closed shapes, and this path has an open contour".to_string(),
                });
            }
            if vector.path.is_empty() {
                return Err(EngineError::Failed { id, message: "the path has no shape to cut".to_string() });
            }
            let kind = self.axis.as_ref().map(|axis| axis.0.clone()).unwrap_or_else(|| "vertical".to_string());
            let axis = match kind.as_str() {
                "vertical" => Axis::vertical(self.at(&vector)?),
                "horizontal" => Axis::horizontal(self.at(&vector)?),
                "free" => {
                    let from = self.from.as_ref().ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: "a free axis needs the two points it runs through".to_string(),
                    })?;
                    let to = self.to.as_ref().ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: "a free axis needs the two points it runs through".to_string(),
                    })?;
                    let from =
                        read_point(from).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
                    let to = read_point(to).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
                    Axis::through(from, to).ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: "the axis's two points must differ".to_string(),
                    })?
                }
                other => {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("there is no axis called `{other}` (vertical|horizontal|free)"),
                    });
                }
            };
            let sign = keep_sign(&axis, self.keep.as_ref().map(|keep| keep.0.as_str()))
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            // The part of the shape on the side that is kept.
            let bounds = geom::bounds(&vector.path);
            let box_of =
                (bounds.x - REACH, bounds.y - REACH, bounds.x + bounds.w + REACH, bounds.y + bounds.h + REACH);
            // The half of the box on the side that is kept. The box is
            // wholly on the other side when the axis does not reach the
            // shape, and then there is nothing to keep.
            let kept = match axis.half_plane(sign, box_of) {
                Some(region) => geom::boolean(&vector.path, &region, geom::BoolOp::Intersect)
                    .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?,
                None => Path::default(),
            };
            if kept.is_empty() {
                // The shape is wholly on the **other** side: there is
                // nothing to keep, and nothing is left.
                session.document_mut()?.remove_node(self.path.0);
                return Ok(
                    Outcome::changed("the shape is taken away").with_data(json!({ "removed": self.path.0 }))
                );
            }
            // The shape's own **mirror** across the axis, and the two
            // joined.
            let reflection = reflection_of(&axis)?;
            let mirrored = reflection.transform_path(&kept);
            let joined = geom::boolean(&kept, &mirrored, geom::BoolOp::Union)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            if joined.is_empty() {
                return Err(EngineError::Failed { id, message: "the mirror left no shape".to_string() });
            }
            let mut vector = vector;
            vector.path = joined;
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the shape is mirrored").with_data(json!({ "id": self.path.0 })))

        }
    }
}

/// The **side** the sign names, as the tree's own rule: the side is
/// left, right, top or bottom, and the default is left — or top when
/// the axis runs more across than down.
fn keep_sign(axis: &Axis, keep: Option<&str>) -> Result<f64, String> {
    let keep = match keep {
        Some(keep) => keep.to_string(),
        None => {
            if axis.direction.x.abs() > axis.direction.y.abs() {
                "top".to_string()
            } else {
                "left".to_string()
            }
        }
    };
    let toward = match keep.as_str() {
        "left" => Point::new(-1.0, 0.0),
        "right" => Point::new(1.0, 0.0),
        "top" => Point::new(0.0, -1.0),
        "bottom" => Point::new(0.0, 1.0),
        other => return Err(format!("there is no side called `{other}` (left|right|top|bottom)")),
    };
    let turn = axis.direction.x * toward.y - axis.direction.y * toward.x;
    if turn.abs() < 1e-6 {
        return Err(format!("`{keep}` is not a side of this axis"));
    }
    Ok(turn.signum())
}

/// The **mirror** of the world across the axis.
fn reflection_of(axis: &Axis) -> Result<geom::Matrix, EngineError> {
    let (first, second) = match (axis.direction.x, axis.direction.y) {
        (0.0, _) => {
            (Point::new(axis.point.x, axis.point.y - 1000.0), Point::new(axis.point.x, axis.point.y + 1000.0))
        }
        (_, 0.0) => {
            (Point::new(axis.point.x - 1000.0, axis.point.y), Point::new(axis.point.x + 1000.0, axis.point.y))
        }
        _ => {
            let length = (axis.direction.x * axis.direction.x + axis.direction.y * axis.direction.y).sqrt();
            let step = Point::new(axis.direction.x / length, axis.direction.y / length);
            (
                Point::new(axis.point.x - step.x * 1000.0, axis.point.y - step.y * 1000.0),
                Point::new(axis.point.x + step.x * 1000.0, axis.point.y + step.y * 1000.0),
            )
        }
    };
    geom::Matrix::mirror(first, second).ok_or_else(|| EngineError::Failed {
        id: "vector.path.cut.mirror".to_string(),
        message: "the axis has no mirror".to_string(),
    })
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
