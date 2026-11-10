//! The **curve through the points**.
//!
//! The tree's curvature tool makes a path that curves through the points
//! a person clicks, and the curve it makes is the tree's own: every
//! point of it is a **smooth point** whose handles follow the circle
//! through that point and its two neighbours, and the ends of an open
//! path take the tangent of the point next to them.
//!
//! The rules are the tree's, and they are here by name:
//!
//! - the handles of a smooth point are along the **circle** through the
//!   three points, and each as long as that circle says;
//! - a turn **tighter than thirty degrees** is cut back, by the square
//!   of how far it is under, so that a narrow V comes to a point
//!   instead of swinging round a near-whole circle;
//! - three points **in a line** give a straight tangent;
//! - a point with **no direction** — a chord of no length, or a fold
//!   straight back on itself — is a **corner**;
//! - the **ends** of an open path take the tangent of their neighbour,
//!   mirrored in the chord, so that the end leaves the chord at the
//!   angle the curve meets its neighbour at.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Path, Point, Segment, Session, SubPath, Text};

use crate::add_path;
use crate::{vector_of, write_back};

/// The most **points** one curve may go through.
///
/// The bound is this battery's: a curve through a hundred thousand
/// points is a document nobody can open, and a caller that wants one
/// says so and is refused by name.
const MAX_POINTS: usize = 10_000;

/// A handle's length as a fraction of its segment's chord where the
/// handle runs along the chord: a third of the chord.
const HANDLE_K: f64 = 1.0 / 3.0;

/// The sine of half the angle between a smooth point's chords (sin 15°)
/// below which its handles shorten: a turn tighter than 30° is cut back
/// toward a corner instead of swinging round the circle through the
/// three points, which for a narrow V is a near-whole circle.
const TIGHT_TURN: f64 = 0.258_819_045_102_520_8;

/// Put a **curve through the points** in the document.
///
/// The curve is the tree's own: the curve tool's curve, worked out from
/// the points and the points next to them.
#[derive(Command)]
#[command(
    id = "vector.path.spline",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddSpline {
    /// The points the curve goes through: `{x, y}`, each with an
    /// optional `corner`, or a pair of numbers `[x, y]`.
    pub points: Json,
    /// Whether the curve is closed.
    pub closed: Option<Bool>,
    /// The path to curve, if there is one already.
    ///
    /// Without it, a new path is put in the document.
    pub path: Option<NodeRef>,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl AddSpline {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.spline".to_string();
        let points =
            read_points(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let closed = self.closed.map(|closed| closed.0).unwrap_or(false);
        let path = spline(&points, closed);
        if path.is_empty() {
            return Err(EngineError::Failed { id, message: "a curve needs at least two points".to_string() });
        }
        match self.path {
            // The path that is named is the path that is made: the tree's
            // own command replaces a path when it is given one.
            Some(node) => {
                let mut vector = vector_of(session, node.0)?;
                vector.path = path;
                write_back(session, node.0, &vector)?;
                Ok(Outcome::changed(format!("the path `{}` curves through the points", node.0))
                    .with_data(json!({ "id": node.0 })))
            }
            None => {
                // The tree's own name for a curve made this way.
                let name = match &self.name {
                    Some(name) => name.0.clone(),
                    None => "Curvature".to_string(),
                };
                let node = add_path(session, name, path, self.fill.as_ref())?;
                Ok(Outcome::changed(format!("added a curve `{node}`")).with_data(json!({ "id": node })))
            }
        }
    }
}

/// The points a curve goes through, as the caller gave them.
///
/// A point is `{x, y}` or a pair `[x, y]`, and a point may say it is a
/// **corner**. The tree's own two spellings, and the tree's own rule
/// that a point which is not a number is refused.
fn read_points(value: &Json) -> Result<Vec<(Point, bool)>, String> {
    let Some(list) = value.as_array() else {
        return Err("the points are not a list".to_string());
    };
    if list.len() > MAX_POINTS {
        return Err(format!("a curve may go through at most {MAX_POINTS} points"));
    }
    let mut points: Vec<(Point, bool)> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        let point = match item {
            Json::Array(pair) => {
                let x = pair.first().and_then(|value| value.as_f64());
                let y = pair.get(1).and_then(|value| value.as_f64());
                match (x, y) {
                    (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Point::new(x, y),
                    _ => return Err(format!("the point {index} has no numbers in it")),
                }
            }
            Json::Object(table) => {
                let x = table.get("x").and_then(|value| value.as_f64());
                let y = table.get("y").and_then(|value| value.as_f64());
                match (x, y) {
                    (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Point::new(x, y),
                    _ => return Err(format!("the point {index} has no numbers in it")),
                }
            }
            _ => return Err(format!("the point {index} is not a point")),
        };
        let corner = match item.get("corner") {
            None | Some(Json::Null) => false,
            Some(Json::Bool(flag)) => *flag,
            Some(_) => return Err(format!("the point {index} has a `corner` that is not true or false")),
        };
        points.push((point, corner));
    }
    Ok(points)
}

/// An **anchor** of the curve: a point and its two handles.
struct Anchor {
    /// Where the anchor is.
    p: Point,
    /// The handle the curve that arrives at the anchor comes by.
    h_in: Point,
    /// The handle the curve that leaves the anchor goes by.
    h_out: Point,
}

impl Anchor {
    /// A **corner**: a point with no handles.
    fn corner(p: Point) -> Anchor {
        Anchor { p, h_in: p, h_out: p }
    }

    /// Whether the curve that arrives at the anchor has a handle.
    fn has_in(&self) -> bool {
        self.h_in != self.p
    }

    /// Whether the curve that leaves the anchor has a handle.
    fn has_out(&self) -> bool {
        self.h_out != self.p
    }
}

/// A **vector**: the arithmetic the curve is worked out with.
#[derive(Clone, Copy)]
struct Vec2 {
    /// Across.
    x: f64,
    /// Down.
    y: f64,
}

impl Vec2 {
    /// A vector.
    fn new(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    /// This vector, `k` times as long.
    fn scale(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }

    /// How much of this vector is in the direction of `other`.
    fn dot(self, other: Vec2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// How much this vector turns away from `other`.
    fn cross(self, other: Vec2) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// How long this vector is.
    fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    /// How long this vector is, squared.
    fn squared_length(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Whether both its numbers are numbers.
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;

    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl From<Point> for Vec2 {
    fn from(point: Point) -> Vec2 {
        Vec2::new(point.x, point.y)
    }
}

impl From<Vec2> for Point {
    fn from(vector: Vec2) -> Point {
        Point::new(vector.x, vector.y)
    }
}

/// How long a handle leaving along `dir` should be for its segment,
/// `chord`, to follow a circular arc: `|chord| / (3 cos²(α/2))`, `α` the
/// angle between the handle and the chord. A third of the chord along
/// it, up to two thirds at 90° — the cap: a handle turned further is
/// treated as at 90°.
fn arc_handle(chord: Vec2, dir: Vec2) -> f64 {
    let (l, ld) = (chord.length(), dir.length());
    if !(l > 1e-9 && ld > 1e-12) {
        return 0.0;
    }
    // cos²(α/2) = (1 + cos α) / 2.
    let cos = (dir.dot(chord) / (ld * l)).clamp(0.0, 1.0);
    2.0 * HANDLE_K * l / (1.0 + cos)
}

/// The handles `(in, out)` of a smooth point `p` between `prev` and
/// `next`, so the curve through the three follows the circle through
/// them: both along that circle's tangent at `p`, the direction of
/// `(next − p)/|next − p|² − (prev − p)/|prev − p|²` (the shorter side
/// weighs more), each as long as [`arc_handle`] says for its own
/// segment; in a turn tighter than 30° ([`TIGHT_TURN`]) shorter, by the
/// square of how far it is under, so a narrow V comes to a point.
///
/// None when there is no direction: a chord of no length, or a fold
/// straight back on itself. The point is then a corner.
fn smooth_handles(prev: Point, p: Point, next: Point) -> Option<(Point, Point)> {
    let (a, b) = (Vec2::from(prev) - Vec2::from(p), Vec2::from(next) - Vec2::from(p));
    let (la2, lb2) = (a.squared_length(), b.squared_length());
    if !(la2.is_finite() && lb2.is_finite()) || la2 <= 1e-18 || lb2 <= 1e-18 {
        return None;
    }
    // In a line with `p` not between its neighbours, the path folds
    // straight back.
    if a.cross(b).abs() <= 1e-9 * la2.sqrt() * lb2.sqrt() && a.dot(b) > 0.0 {
        return None;
    }
    let t = b.scale(1.0 / lb2) - a.scale(1.0 / la2);
    let lt = t.length();
    if !lt.is_finite() || lt * la2.sqrt().min(lb2.sqrt()) <= 1e-9 {
        return None;
    }
    let u = t.scale(1.0 / lt);
    // sin(φ/2) = √((1 − cos φ) / 2), φ the angle at `p` between the
    // chords.
    let cos = (a.dot(b) / (la2.sqrt() * lb2.sqrt())).clamp(-1.0, 1.0);
    let k = (((1.0 - cos) / 2.0).sqrt() / TIGHT_TURN).min(1.0).powi(2);
    let (h_in, h_out) = (
        Vec2::from(p) - u.scale(k * arc_handle(a, u.scale(-1.0))),
        Vec2::from(p) + u.scale(k * arc_handle(b, u)),
    );
    if !h_in.is_finite() || !h_out.is_finite() {
        return None;
    }
    Some((h_in.into(), h_out.into()))
}

/// The one handle an end `p` of an open path gets so its segment, to the
/// neighbour `q`, follows the same circle as the curve at `q`: `q`'s
/// tangent mirrored in the chord, so the end leaves the chord at the
/// angle the curve meets `q` at, and as long as [`arc_handle`] says.
/// Where `q`'s own handle on their segment falls short of the circle's
/// (a tight turn at `q`), the end handle turns toward the chord and
/// shortens in that share, so the end segment straightens too.
///
/// None when `q` has no handle to take a tangent from (a corner): the
/// segment stays straight.
fn end_handle(p: Point, q: &Anchor, forward: bool) -> Option<Point> {
    let t = if q.has_out() {
        Vec2::from(q.h_out) - Vec2::from(q.p)
    } else if q.has_in() {
        Vec2::from(q.p) - Vec2::from(q.h_in)
    } else {
        return None;
    };
    let (t, chord) = (if forward { t } else { t.scale(-1.0) }, Vec2::from(q.p) - Vec2::from(p));
    let (lt, lc) = (t.length(), chord.length());
    if !(lt > 1e-12 && lc > 1e-9) {
        return None;
    }
    let (t, c) = (t.scale(1.0 / lt), chord.scale(1.0 / lc));
    let d = c.scale(2.0 * t.dot(c)) - t;
    let back =
        if forward { Vec2::from(q.h_in) - Vec2::from(q.p) } else { Vec2::from(q.h_out) - Vec2::from(q.p) };
    let full = arc_handle(Vec2::from(p) - Vec2::from(q.p), back);
    let share = if back.length() > 1e-9 && full > 1e-9 { (back.length() / full).min(1.0) } else { 1.0 };
    // Short of the circle, the end also turns toward its chord in that
    // share.
    let d = c.scale(1.0 - share) + d.scale(share);
    let d = d.scale(1.0 / d.length().max(1e-12));
    let h = Vec2::from(p) + d.scale(share * arc_handle(chord, d));
    if !h.is_finite() {
        return None;
    }
    Some(h.into())
}

/// The curve through the points: the tree's own spline.
///
/// The points are the tree's own: a point, and whether it is a corner.
/// A point that is not a corner and has a neighbour on each side is a
/// **smooth** point, and its handles follow the circle through the
/// three; a point that is a corner, or an end of an open path, or a
/// point with no direction, has no handles at all.
///
/// The ends of an open path are the tree's own too: the end takes the
/// tangent of its neighbour, mirrored in the chord, so that the curve
/// leaves the chord at the angle it meets the neighbour at.
fn spline(points: &[(Point, bool)], closed: bool) -> Path {
    let count = points.len();
    let mut anchors: Vec<Anchor> = Vec::with_capacity(count);
    for (index, &(p, corner)) in points.iter().enumerate() {
        let open_end = !closed && (index == 0 || index + 1 == count);
        let neighbours = (count >= 3 && !corner && !open_end)
            .then(|| points.get((index + count - 1) % count).zip(points.get((index + 1) % count)))
            .flatten()
            .map(|(prev, next)| (prev.0, next.0));
        anchors.push(match neighbours.and_then(|(prev, next)| smooth_handles(prev, p, next)) {
            Some((h_in, h_out)) => Anchor { p, h_in, h_out },
            None => Anchor::corner(p),
        });
    }
    if count >= 3 && !closed {
        // The end handles mirror the neighbours' tangents, which are in
        // place by now.
        let first = points.first().is_some_and(|point| !point.1);
        let last = points.last().is_some_and(|point| !point.1);
        if first && let Some(head) = end_handle(anchors[0].p, &anchors[1], true) {
            anchors[0].h_out = head;
        }
        if last && let Some(tail) = end_handle(anchors[count - 1].p, &anchors[count - 2], false) {
            anchors[count - 1].h_in = tail;
        }
    }
    // A closed curve needs more than two points: the tree's own rule.
    let closed = closed && count > 2;
    let sides = if closed { count } else { count.saturating_sub(1) };
    let mut segments: Vec<Segment> = Vec::with_capacity(sides);
    for index in 0..sides {
        let from = &anchors[index % count];
        let to = &anchors[(index + 1) % count];
        // A side with no handle on either end is a straight line.
        if !from.has_out() && !to.has_in() {
            segments.push(Segment::Line { to: to.p });
        } else {
            segments.push(Segment::Cubic { c1: from.h_out, c2: to.h_in, to: to.p });
        }
    }
    Path::single(SubPath {
        start: anchors.first().map(|anchor| anchor.p).unwrap_or_default(),
        segments,
        closed,
    })
}
