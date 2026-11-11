//! The **converting** of points: smooth, or corner.
//!
//! The tree's Convert Anchor Points turns the points that are chosen
//! into **smooth** points or into **corner** points, and it works on
//! many of them at once.
//!
//! The rules are the tree's own:
//!
//! - a point is made **smooth** by the **neighbours' rule**: the two
//!   handles at the point are in line with the direction from the
//!   point before it to the point after it, and each is a **third** of
//!   the way to that side's neighbour;
//! - a point that is already smooth **keeps** its handles;
//! - a point is made a **corner** by **retracting** its handles: the
//!   handles come back to the point itself;
//! - a point whose neighbours are **one point** has no direction, and
//!   it is left as it is;
//! - an end of an open contour has **one** neighbour, and it takes
//!   that neighbour's direction.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Point, Segment, Session, SubPath, Text};

use crate::{arriving_segment, closing_segment, leaving_segment, vector_of, write_back};

/// The most **points** one call may name.
const MAX_POINTS: usize = 10_000;

/// Turn points of a path into smooth points or into corner points.
#[derive(Command)]
#[command(
    id = "vector.path.anchors.convert",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Points"),
    mutates,
    undoable
)]
pub struct ConvertAnchors {
    /// The path.
    pub path: NodeRef,
    /// Which contour of the path, counting from zero. The first one if
    /// it is left out.
    pub contour: Option<Number>,
    /// The points, counting from zero: a list of whole numbers.
    pub points: Json,
    /// What the points become: `smooth` or `corner`.
    pub to: Text,
}

impl ConvertAnchors {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.anchors.convert".to_string();
        let smooth = match self.to.0.as_str() {
            "smooth" => true,
            "corner" => false,
            other => {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the points become `smooth` or `corner`, not `{other}`"),
                });
            }
        };
        let points =
            read_points(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let mut vector = vector_of(session, self.path.0)?;
        let contour = self.contour.as_ref().map(|contour| contour.0).unwrap_or(0.0);
        let contour = usize::try_from(contour.max(0.0) as i64).unwrap_or(0);
        let Some(subpath) = vector.path.subpaths.get_mut(contour) else {
            return Err(EngineError::Failed { id, message: format!("the path has no contour {contour}") });
        };
        let mut converted: Vec<usize> = Vec::new();
        let mut left: Vec<usize> = Vec::new();
        for point in points {
            if point >= count_of(subpath) {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the contour has {} points and not a point {point}", count_of(subpath)),
                });
            }
            let done =
                if smooth { smooth_by_neighbours(subpath, point) } else { make_corner(subpath, point) };
            if done {
                converted.push(point);
            } else {
                left.push(point);
            }
        }
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed(if smooth { "the points are smooth" } else { "the points are corners" })
            .with_data(json!({
                "id": self.path.0,
                "contour": contour,
                "converted": converted,
                "left": left
            })))
    }
}

/// How many **points** a contour has.
///
/// An open contour has one point more than it has sides, and a closed
/// contour has one point fewer: its last side comes back to its first
/// point, and that point is not counted twice.
fn count_of(subpath: &SubPath) -> usize {
    let sides = subpath.segments.len();
    if subpath.closed && closing_segment(subpath).is_some() { sides } else { sides + 1 }
}

/// The **point** at an index of a contour.
fn point_at(subpath: &SubPath, index: usize) -> Point {
    if index == 0 { subpath.start } else { subpath.segments[index - 1].end() }
}

/// Make a point **smooth** by the tree's neighbours' rule.
///
/// The handles at the point are in line with the direction from the
/// point before it to the point after it, and each is a third of the
/// way to that side's neighbour. A point that is already smooth keeps
/// its handles, and a point whose neighbours are one point has no
/// direction and is left as it is.
pub(crate) fn smooth_by_neighbours(subpath: &mut SubPath, point: usize) -> bool {
    // The **closing side** of a closed contour is made explicit: the
    // neighbours of a point are its sides, and a side that is not
    // there holds no handle.
    if subpath.closed && closing_segment(subpath).is_none() {
        subpath.segments.push(Segment::Line { to: subpath.start });
    }
    let at = point_at(subpath, point);
    let sides = subpath.segments.len();
    let previous = if point > 0 {
        Some(point_at(subpath, point - 1))
    } else if subpath.closed {
        Some(point_at(subpath, sides - 1))
    } else {
        None
    };
    let next = if subpath.closed {
        if point + 1 < sides { Some(point_at(subpath, point + 1)) } else { Some(subpath.start) }
    } else if point < sides {
        Some(point_at(subpath, point + 1))
    } else {
        None
    };
    // A point that is **already smooth** keeps its handles.
    if is_smooth(subpath, point) {
        return false;
    }
    let (before, after) = (previous.unwrap_or(at), next.unwrap_or(at));
    let (dx, dy) = (after.x - before.x, after.y - before.y);
    let length = (dx * dx + dy * dy).sqrt();
    if length <= 1e-9 {
        // The neighbours are **one point**: there is no direction.
        return false;
    }
    let (ux, uy) = (dx / length, dy / length);
    let to_before = distance(at, before) / 3.0;
    let to_after = distance(at, after) / 3.0;
    let h_in = Point::new(at.x - ux * to_before, at.y - uy * to_before);
    let h_out = Point::new(at.x + ux * to_after, at.y + uy * to_after);
    // The side that **leaves** the point: the handle at the point, and
    // the far end keeps what it had.
    if let Some(index) = leaving_segment(subpath, point) {
        let end = subpath.segments[index].end();
        let c2 = match &subpath.segments[index] {
            Segment::Line { .. } => end,
            Segment::Cubic { c2, .. } => *c2,
        };
        subpath.segments[index] = Segment::Cubic { c1: h_out, c2, to: end };
    }
    // The side that **arrives** at the point: the same, the other way
    // round.
    if let Some(index) = arriving_segment(subpath, point) {
        let start = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
        let c1 = match &subpath.segments[index] {
            Segment::Line { .. } => start,
            Segment::Cubic { c1, .. } => *c1,
        };
        subpath.segments[index] = Segment::Cubic { c1, c2: h_in, to: at };
    }
    true
}

/// Make a point a **corner**: its handles come back to the point.
fn make_corner(subpath: &mut SubPath, point: usize) -> bool {
    let at = point_at(subpath, point);
    // The side that **leaves** the point: its handle comes back.
    if let Some(index) = leaving_segment(subpath, point) {
        let end = subpath.segments[index].end();
        let c2 = match &subpath.segments[index] {
            Segment::Line { .. } => end,
            Segment::Cubic { c2, .. } => *c2,
        };
        subpath.segments[index] = Segment::Cubic { c1: at, c2, to: end };
    }
    // The side that **arrives** at the point: the same, the other way
    // round.
    if let Some(index) = arriving_segment(subpath, point) {
        let start = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
        let c1 = match &subpath.segments[index] {
            Segment::Line { .. } => start,
            Segment::Cubic { c1, .. } => *c1,
        };
        subpath.segments[index] = Segment::Cubic { c1, c2: at, to: at };
    }
    true
}

/// Whether a point is **already smooth**: the handles at the point are
/// in line, and there is a handle on each side.
fn is_smooth(subpath: &SubPath, point: usize) -> bool {
    let at = point_at(subpath, point);
    let Some(arriving) = arriving_segment(subpath, point) else {
        return false;
    };
    let Some(leaving) = leaving_segment(subpath, point) else {
        return false;
    };
    let h_in = match &subpath.segments[arriving] {
        Segment::Line { .. } => at,
        Segment::Cubic { c2, .. } => *c2,
    };
    let h_out = match &subpath.segments[leaving] {
        Segment::Line { .. } => at,
        Segment::Cubic { c1, .. } => *c1,
    };
    is_collinear(at, h_in, h_out)
}

/// Whether the two handles at a point are **in line**: they point away
/// from the point and they are in the same line.
fn is_collinear(point: Point, a: Point, b: Point) -> bool {
    let (ax, ay) = (a.x - point.x, a.y - point.y);
    let (bx, by) = (b.x - point.x, b.y - point.y);
    let cross = ax * by - ay * bx;
    let scale = (ax * ax + ay * ay).sqrt().max((bx * bx + by * by).sqrt()).max(1.0);
    cross.abs() <= 1e-6 * scale && ax * bx + ay * by < 0.0
}

/// How far two points are **apart**.
fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// The **points** the caller named: whole numbers, counting from zero.
pub(crate) fn read_points(value: &Json) -> Result<Vec<usize>, String> {
    let Some(list) = value.as_array() else {
        return Err("the points are a list of whole numbers".to_string());
    };
    if list.len() > MAX_POINTS {
        return Err(format!("the points are at most {MAX_POINTS} of them"));
    }
    let mut points: Vec<usize> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        let Some(number) = item.as_f64() else {
            return Err(format!("the point {index} is not a number"));
        };
        if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
            return Err(format!("the point {index} is a whole number and not {number}"));
        }
        points.push(number as usize);
    }
    Ok(points)
}
