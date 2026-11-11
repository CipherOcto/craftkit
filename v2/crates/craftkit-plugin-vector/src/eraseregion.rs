//! The **eraser with a shaped tip**: the area under a tip taken away.
//!
//! The tree's Eraser takes a **tip** — a size, an angle and a
//! roundness — and the tip is an **ellipse**, not a circle. The tree's
//! other eraser takes the **area** of a closed shape away under the
//! tip, and the outline of an open path away under it.
//!
//! The rules are the tree's own:
//!
//! - the tip's **long axis** is the size, and its **short axis** is the
//!   roundness — a percentage of the long one;
//! - the tip's long axis runs at the **angle**, in degrees;
//! - the area the tip covers along the drag is the drag **squashed** by
//!   the tip's map, **stroked** round, and **stretched** back: the
//!   tree's own rule;
//! - a closed shape loses the **area** under the tip, and an open path
//!   loses the **outline** under it: the tree's own rule.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{
    Cap, EngineError, FormatValue, Join, NodeRef, Number, Outcome, Path, Point, Segment, Session, SubPath,
    Text,
};

use craftkit_vector_geom as geom;

use crate::erasepath::{distance_to_drag, erase_contour, read_drag};
use crate::{closing_segment, vector_of, write_back};

/// The tip's **long axis** when the caller names none.
const DEFAULT_SIZE: f64 = 10.0;

/// The tip's **angle** when the caller names none.
const DEFAULT_ANGLE: f64 = 0.0;

/// The tip's **roundness** when the caller names none, in percent.
const DEFAULT_ROUNDNESS: f64 = 100.0;

/// The **smallest** roundness a tip may have, in percent.
const MIN_ROUNDNESS: f64 = 1.0;

/// The **largest** roundness a tip may have, in percent.
const MAX_ROUNDNESS: f64 = 100.0;

/// Take the area under a shaped tip away.
#[derive(Command)]
#[command(
    id = "vector.path.erase.region",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Erase"),
    mutates,
    undoable
)]
pub struct EraseRegion {
    /// The path.
    pub path: NodeRef,
    /// The drag: a run of points, `[[x, y], …]`.
    pub points: Json,
    /// How long the tip's long axis is. 10 if it is not given.
    pub size: Option<Number>,
    /// The angle of the tip's long axis, in degrees. 0 if it is not
    /// given.
    pub angle: Option<Number>,
    /// How round the tip is, in percent of its long axis. 100 if it is
    /// not given.
    pub roundness: Option<Number>,
    /// What to call the paths that are left.
    pub name: Option<Text>,
}

impl EraseRegion {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.erase.region".to_string();
        let drag =
            read_drag(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let size = self.size.as_ref().map(|size| size.0).unwrap_or(DEFAULT_SIZE);
        let angle = self.angle.as_ref().map(|angle| angle.0).unwrap_or(DEFAULT_ANGLE);
        let roundness = self.roundness.as_ref().map(|roundness| roundness.0).unwrap_or(DEFAULT_ROUNDNESS);
        if !size.is_finite() || size <= 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "the size of the tip is a number and not a negative one".to_string(),
            });
        }
        if !angle.is_finite() {
            return Err(EngineError::Failed { id, message: "the angle of the tip is a number".to_string() });
        }
        if !roundness.is_finite() || !(MIN_ROUNDNESS..=MAX_ROUNDNESS).contains(&roundness) {
            return Err(EngineError::Failed {
                id,
                message: format!(
                    "the roundness of the tip is between {MIN_ROUNDNESS} and {MAX_ROUNDNESS} percent"
                ),
            });
        }
        // The tip's **map**: it takes a circle to the tip, and the
        // shape of the tip is the map's own.
        let tip = Tip::new(size, angle, roundness);
        let mut vector = vector_of(session, self.path.0)?;
        let closed = vector.path.subpaths.iter().all(|contour| contour.closed);
        let whole = vector.path.subpaths.len() == 1;
        if closed {
            // A closed shape loses the **area** under the tip.
            let region = tip.area_along(&drag);
            let left = geom::boolean(&vector.path, &region, geom::BoolOp::Difference)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            let before = geom::area(&vector.path).abs();
            let after = geom::area(&left).abs();
            if (before - after).abs() <= 1e-9 {
                return Ok(Outcome::new("none of the shape was under the tip")
                    .with_data(json!({ "id": self.path.0 })));
            }
            if left.is_empty() {
                // The whole shape was under the tip: it is gone.
                session.document_mut()?.remove_node(self.path.0);
                return Ok(
                    Outcome::changed("the shape is taken away").with_data(json!({ "removed": self.path.0 }))
                );
            }
            vector.path = left;
            write_back(session, self.path.0, &vector)?;
            return Ok(
                Outcome::changed("the area under the tip is gone").with_data(json!({ "id": self.path.0 }))
            );
        }
        // An open path loses the **outline** under the tip.
        let short = size * roundness / 100.0;
        let round: Vec<Point> = drag.iter().map(|point| tip.squash(*point)).collect();
        let under = |point: Point| distance_to_drag(&round, tip.squash(point)) <= size / 2.0;
        let mut pieces: Vec<SubPath> = Vec::new();
        let mut erased = false;
        for contour in &vector.path.subpaths {
            let mut contour = contour.clone();
            if contour.closed && closing_segment(&contour).is_none() {
                contour.segments.push(Segment::Line { to: contour.start });
            }
            match erase_contour(&contour, short / 2.0, &under) {
                Some(parts) => {
                    erased = true;
                    pieces.extend(parts);
                }
                None => pieces.push(contour),
            }
        }
        if !erased {
            return Ok(
                Outcome::new("none of the path was under the tip").with_data(json!({ "id": self.path.0 }))
            );
        }
        if whole {
            // One path per piece: the first keeps the node, and the
            // others are new nodes beside it.
            let mut pieces = pieces.into_iter();
            let Some(first) = pieces.next() else {
                return Err(EngineError::Failed {
                    id,
                    message: "the eraser took the whole path away".to_string(),
                });
            };
            let name = match &self.name {
                Some(name) => name.0.clone(),
                None => {
                    session.document().node(self.path.0).map(|node| node.name.clone()).unwrap_or_default()
                }
            };
            vector.path = Path { subpaths: vec![first], rule: vector.path.rule };
            write_back(session, self.path.0, &vector)?;
            let mut ids: Vec<u64> = vec![self.path.0];
            for piece in pieces {
                ids.push(add_piece(session, &name, &vector, Path::single(piece))?);
            }
            Ok(Outcome::changed("the parts under the tip are gone").with_data(json!({ "ids": ids })))
        } else {
            vector.path = Path { subpaths: pieces, rule: vector.path.rule };
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the parts under the tip are gone").with_data(json!({ "id": self.path.0 })))
        }
    }
}

/// Put a **piece** of an erased path in the document, and give back its
/// id.
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

/// The tip of the eraser: an **ellipse**, and the map that makes it.
///
/// The map takes a circle to the tip: its long axis runs at the angle
/// and is the size across, and its short axis is the roundness — a
/// percentage of the long one.
struct Tip {
    /// The map: `a`, `b`, `d` — the two halves of the symmetric matrix
    /// of the tip.
    a: f64,
    b: f64,
    d: f64,
    /// The map that undoes it.
    inverse: (f64, f64, f64),
    /// The size of the tip's long axis.
    size: f64,
}

impl Tip {
    /// The tip of the given size, angle and roundness.
    fn new(size: f64, angle: f64, roundness: f64) -> Tip {
        let radians = angle.to_radians();
        let u = (radians.cos(), -radians.sin());
        let v = (-u.1, u.0);
        let ratio = roundness / 100.0;
        let a = u.0 * u.0 + ratio * v.0 * v.0;
        let b = u.0 * u.1 + ratio * v.0 * v.1;
        let d = u.1 * u.1 + ratio * v.1 * v.1;
        let determinant = a * d - b * b;
        let inverse = if determinant.abs() > 1e-12 {
            (d / determinant, -b / determinant, a / determinant)
        } else {
            (1.0, 0.0, 1.0)
        };
        Tip { a, b, d, inverse, size }
    }

    /// The point **squashed** by the tip's map: the world with the tip
    /// turned back into a circle.
    fn squash(&self, point: Point) -> Point {
        let (i, j, k) = self.inverse;
        Point::new(i * point.x + j * point.y, j * point.x + k * point.y)
    }

    /// The point **stretched** by the tip's map.
    fn stretch(&self, point: Point) -> Point {
        Point::new(self.a * point.x + self.b * point.y, self.b * point.x + self.d * point.y)
    }

    /// The **area** the tip covers along the drag: the drag squashed,
    /// stroked round, and stretched back.
    ///
    /// A drag of **one point** is no line to stroke, and the tip there
    /// is the ellipse itself.
    fn area_along(&self, drag: &[Point]) -> Path {
        if drag.len() == 1 {
            return self.ellipse_at(drag[0]);
        }
        let round: Vec<Point> = drag.iter().map(|point| self.squash(*point)).collect();
        let segments: Vec<Segment> = round.windows(2).map(|pair| Segment::Line { to: pair[1] }).collect();
        let squashed = Path::single(SubPath { start: round[0], segments, closed: false });
        let stroked = geom::without_degenerate(&geom::outline_stroke(
            &squashed,
            self.size,
            Cap::Round,
            Join::Round,
            4.0,
        ));
        // The stroke is stretched back into the tip's own shape.
        Path {
            subpaths: stroked
                .subpaths
                .iter()
                .map(|subpath| SubPath {
                    start: self.stretch(subpath.start),
                    segments: subpath
                        .segments
                        .iter()
                        .map(|segment| match segment {
                            Segment::Line { to } => Segment::Line { to: self.stretch(*to) },
                            Segment::Cubic { c1, c2, to } => Segment::Cubic {
                                c1: self.stretch(*c1),
                                c2: self.stretch(*c2),
                                to: self.stretch(*to),
                            },
                        })
                        .collect(),
                    closed: subpath.closed,
                })
                .collect(),
            rule: stroked.rule,
        }
    }

    /// The tip itself: the **ellipse** at a place, as a polygon.
    fn ellipse_at(&self, centre: Point) -> Path {
        let steps = 64;
        let mut points: Vec<Point> = Vec::with_capacity(steps);
        for step in 0..steps {
            let angle = 2.0 * std::f64::consts::PI * step as f64 / steps as f64;
            let circle = Point::new(angle.cos() * self.size / 2.0, angle.sin() * self.size / 2.0);
            points.push(self.stretch(Point::new(centre.x + circle.x, centre.y + circle.y)));
        }
        let segments: Vec<Segment> = points
            .windows(2)
            .map(|pair| Segment::Line { to: pair[1] })
            .chain(std::iter::once(Segment::Line { to: points[0] }))
            .collect();
        Path::single(SubPath { start: points[0], segments, closed: true })
    }
}
