//! The **erasing** of the parts of a path under a drag.
//!
//! The tree's Path Eraser takes away the parts of a path that are under
//! the drag: the path is sampled, the parts under the drag are found,
//! and what is left is the stretches that survive — each a stretch of
//! the path as it was, so that the shape does not change.
//!
//! The rules are the tree's own:
//!
//! - the drag is a **run of points**, and the parts of the path within
//!   **half the width** of it are taken away;
//! - the path is **sampled** at least every half width, so that a
//!   narrow stroke is not missed;
//! - a stretch that is left is the path **as it was**: the curve of the
//!   stretch is the curve of the path.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{
    EngineError, FormatValue, NodeRef, Number, Outcome, Path, Point, Segment, Session, SubPath, Text,
};

use crate::{closing_segment, vector_of, write_back};

/// The **width** of the eraser when the caller names none.
const DEFAULT_WIDTH: f64 = 8.0;

/// The **samples** a side is sampled at, when the width does not ask
/// for more.
const SAMPLES: usize = 24;

/// The most **samples** one side is sampled at.
const MAX_SAMPLES: usize = 4096;

commands! {
    /// Take away the parts of a path that are **under a drag**.
    vector.path.erase {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Erase"),
        params: {
            /// The path.
            path: NodeRef,
            /// The drag: a run of points, `[[x, y], …]`.
            points: Json,
            /// How wide the eraser is. 8 if it is not given.
            width: Option<Number>,
            /// What to call the paths that are left.
            name: Option<Text>,
        },
        run: {
            let id = "vector.path.erase".to_string();
            let drag =
                read_drag(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let width = self.width.as_ref().map(|width| width.0).unwrap_or(DEFAULT_WIDTH);
            if !width.is_finite() || width <= 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "the width of the eraser is a number and not a negative one".to_string(),
                });
            }
            let half = width / 2.0;
            let mut vector = vector_of(session, self.path.0)?;
            // The parts under the drag are the parts within half the width
            // of it.
            let under = |point: Point| distance_to_drag(&drag, point) <= half;
            let mut pieces: Vec<SubPath> = Vec::new();
            let mut erased = false;
            for contour in &vector.path.subpaths {
                // A closed contour may leave its **closing edge** out, and
                // the edge is part of the shape: it is a side of its own
                // here, and the eraser takes it like any other.
                let mut contour = contour.clone();
                if contour.closed && closing_segment(&contour).is_none() {
                    contour.segments.push(Segment::Line { to: contour.start });
                }
                match erase_contour(&contour, half, &under) {
                    Some(parts) => {
                        erased = true;
                        pieces.extend(parts);
                    }
                    None => pieces.push(contour),
                }
            }
            if !erased {
                return Ok(Outcome::new("nothing of the path was under the drag")
                    .with_data(json!({ "id": self.path.0 })));
            }
            let whole = vector.path.subpaths.len() == 1;
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
                Ok(Outcome::changed("the parts under the drag are gone").with_data(json!({ "ids": ids })))
            } else {
                // The pieces stay contours of the path.
                vector.path = Path { subpaths: pieces, rule: vector.path.rule };
                write_back(session, self.path.0, &vector)?;
                Ok(Outcome::changed("the parts under the drag are gone").with_data(json!({ "id": self.path.0 })))
            }

        }
    }
}

/// Put a **piece** of an erased path in the document, and give back its
/// id.
///
/// The piece is the path it was cut from, with the shape of the piece:
/// the same name, the same paint and the same place.
fn add_piece(
    session: &mut Session,
    name: &str,
    vector: &crate::VectorNode,
    path: Path,
) -> Result<u64, EngineError> {
    let mut piece = vector.clone();
    piece.path = path;
    let rect = craftkit_vector_geom::bounds(&piece.placed());
    let id =
        session.document_mut()?.add_node_with(name.to_string(), "vector.path", Some(rect), piece.to_json());
    Ok(id)
}

/// The **drag**, as the caller gave it.
pub(crate) fn read_drag(value: &Json) -> Result<Vec<Point>, String> {
    let Some(list) = value.as_array() else {
        return Err("the drag is not a list of points".to_string());
    };
    if list.is_empty() {
        return Err("the drag has no points".to_string());
    }
    let mut drag: Vec<Point> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        let (x, y) = match item {
            Json::Array(pair) => {
                (pair.first().and_then(|value| value.as_f64()), pair.get(1).and_then(|value| value.as_f64()))
            }
            Json::Object(table) => (
                table.get("x").and_then(|value| value.as_f64()),
                table.get("y").and_then(|value| value.as_f64()),
            ),
            _ => return Err(format!("the point {index} of the drag is not a point")),
        };
        match (x, y) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {}
            _ => return Err(format!("the point {index} of the drag has no numbers in it")),
        }
        drag.push(Point::new(x.unwrap_or_default(), y.unwrap_or_default()));
    }
    Ok(drag)
}

/// How far a point is from the **drag**.
///
/// The drag is a run of points, and the distance is to the run itself
/// and not to its points.
pub(crate) fn distance_to_drag(drag: &[Point], at: Point) -> f64 {
    let mut best = f64::INFINITY;
    for window in drag.windows(2) {
        best = best.min(distance_to_line(window[0], window[1], at));
    }
    if drag.len() == 1 {
        best = distance_to_line(drag[0], drag[0], at);
    }
    best
}

/// How far a point is from the **segment** through two points.
///
/// The perpendicular from the point meets the segment where it falls on
/// it, and where it falls beyond it the nearest end of the segment is
/// the answer.
fn distance_to_line(one: Point, two: Point, at: Point) -> f64 {
    let (dx, dy) = (two.x - one.x, two.y - one.y);
    let length = dx.hypot(dy);
    if length <= 1e-12 {
        return (at.x - one.x).hypot(at.y - one.y);
    }
    let along = ((at.x - one.x) * dx + (at.y - one.y) * dy) / (length * length);
    let along = along.clamp(0.0, 1.0);
    let closest = Point::new(one.x + dx * along, one.y + dy * along);
    (at.x - closest.x).hypot(at.y - closest.y)
}

/// The **pieces** of a contour with the parts under the drag taken away.
///
/// The contour is sampled, the parts under the drag are found by
/// halving the way at every crossing, and what is left is the stretches
/// that survive — each a stretch of the contour as it was.
pub(crate) fn erase_contour(
    contour: &SubPath,
    step: f64,
    under: &dyn Fn(Point) -> bool,
) -> Option<Vec<SubPath>> {
    let sides = contour.segments.len();
    if sides == 0 {
        return None;
    }
    // A **run** of the contour that is kept: the side it is on, and the
    // two places on it.
    let mut runs: Vec<Vec<(usize, f64, f64)>> = Vec::new();
    let mut current: Vec<(usize, f64, f64)> = Vec::new();
    let mut any = false;
    for index in 0..sides {
        let side = contour.segments[index];
        let from = if index == 0 { contour.start } else { contour.segments[index - 1].end() };
        // The curve moves at most three times its longest control edge
        // per unit of t, so with n samples the neighbours are at most
        // 3 · edge / n apart.
        let edge = match side {
            Segment::Line { to } => (to.x - from.x).hypot(to.y - from.y),
            Segment::Cubic { c1, c2, to } => (c1.x - from.x)
                .hypot(c1.y - from.y)
                .max((c2.x - c1.x).hypot(c2.y - c1.y))
                .max((to.x - c2.x).hypot(to.y - c2.y)),
        };
        let wanted = (3.0 * edge / step).ceil();
        let samples = if wanted.is_finite() && wanted > SAMPLES as f64 {
            (wanted as usize).min(MAX_SAMPLES)
        } else {
            SAMPLES
        };
        let mut previous_t = 0.0f64;
        let mut previous_under = under(point_at(side, from, 0.0));
        let mut start = if previous_under { None } else { Some(0.0f64) };
        if previous_under {
            any = true;
            if !current.is_empty() {
                runs.push(std::mem::take(&mut current));
            }
        }
        for sample in 1..=samples {
            let t = sample as f64 / samples as f64;
            let now_under = under(point_at(side, from, t));
            if now_under != previous_under {
                // Where the side crosses the edge of the region: by
                // halving the way, over and over.
                let (mut low, mut high) = (previous_t, t);
                for _ in 0..30 {
                    let middle = 0.5 * (low + high);
                    if under(point_at(side, from, middle)) == previous_under {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                let crossing = 0.5 * (low + high);
                if previous_under {
                    start = Some(crossing);
                } else {
                    any = true;
                    if let Some(begin) = start.take() {
                        current.push((index, begin, crossing));
                    }
                    runs.push(std::mem::take(&mut current));
                }
            }
            previous_t = t;
            previous_under = now_under;
        }
        if let Some(begin) = start {
            current.push((index, begin, 1.0));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs.retain(|run| !run.is_empty());
    if !any {
        return None;
    }
    // A closed contour's last and first runs meet at its start.
    if contour.closed && runs.len() >= 2 {
        let first_starts =
            runs.first().and_then(|run| run.first()).is_some_and(|span| span.0 == 0 && span.1 == 0.0);
        let last_ends =
            runs.last().and_then(|run| run.last()).is_some_and(|span| span.0 + 1 == sides && span.2 == 1.0);
        if first_starts && last_ends {
            let first = runs.remove(0);
            if let Some(last) = runs.last_mut() {
                last.extend(first);
            }
        }
    }
    let pieces: Vec<SubPath> = runs
        .iter()
        .filter(|run| run.len() > 1 || run[0].2 - run[0].1 > 1e-4)
        .filter_map(|run| piece_of(contour, run))
        .collect();
    Some(pieces)
}

/// The **piece** a run of the contour makes.
///
/// The stretch is the path as it was: the curve of the stretch is the
/// curve of the side it is on.
fn piece_of(contour: &SubPath, run: &[(usize, f64, f64)]) -> Option<SubPath> {
    let mut segments: Vec<Segment> = Vec::with_capacity(run.len());
    let mut start = Point::default();
    for (number, (index, t0, t1)) in run.iter().enumerate() {
        let side = contour.segments[*index];
        let from = if *index == 0 { contour.start } else { contour.segments[*index - 1].end() };
        match side {
            Segment::Line { to } => {
                if number == 0 {
                    start = lerp(from, to, *t0);
                }
                segments.push(Segment::Line { to: lerp(from, to, *t1) });
            }
            Segment::Cubic { c1, c2, to } => {
                if number == 0 {
                    start = point_at(side, from, *t0);
                }
                let (first, second, end) = part_of(from, c1, c2, to, *t0, *t1);
                segments.push(Segment::Cubic { c1: first, c2: second, to: end });
            }
        }
    }
    if segments.is_empty() {
        return None;
    }
    Some(SubPath { start, segments, closed: false })
}

/// The **point** of a side at a place on it.
fn point_at(side: Segment, from: Point, t: f64) -> Point {
    match side {
        Segment::Line { to } => lerp(from, to, t),
        Segment::Cubic { c1, c2, to } => {
            let q0 = lerp(from, c1, t);
            let q1 = lerp(c1, c2, t);
            let q2 = lerp(c2, to, t);
            let r0 = lerp(q0, q1, t);
            let r1 = lerp(q1, q2, t);
            lerp(r0, r1, t)
        }
    }
}

/// The **part** of a curve between two places on it.
///
/// The curve is cut at the first place and the piece is cut again at
/// the second: the two halves are the curve, and the part between them
/// is the part of the curve that was asked for.
fn part_of(from: Point, c1: Point, c2: Point, to: Point, t0: f64, t1: f64) -> (Point, Point, Point) {
    let q0 = lerp(from, c1, t0);
    let q1 = lerp(c1, c2, t0);
    let q2 = lerp(c2, to, t0);
    let r0 = lerp(q0, q1, t0);
    let r1 = lerp(q1, q2, t0);
    let middle = lerp(r0, r1, t0);
    // The rest of the curve, from the cut.
    let span = 1.0 - t0;
    let u = if span > 1e-12 { ((t1 - t0) / span).clamp(0.0, 1.0) } else { 1.0 };
    let s0 = lerp(middle, r1, u);
    let s1 = lerp(r1, q2, u);
    let s2 = lerp(q2, to, u);
    let t0 = lerp(s0, s1, u);
    let t1 = lerp(s1, s2, u);
    let end = lerp(t0, t1, u);
    (t0, t1, end)
}

/// A point **between** two points.
fn lerp(one: Point, two: Point, t: f64) -> Point {
    Point::new(one.x + (two.x - one.x) * t, one.y + (two.y - one.y) * t)
}
