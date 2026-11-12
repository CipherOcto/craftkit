//! The **shapes** the tree makes that this tree did not: the star,
//! the line and the spiral.
//!
//! The tree's editor makes a star, a line and a spiral out of nothing
//! but a few numbers, and the geometry is the tree's own: the points
//! of a star alternate between two radii, a line is a contour of two
//! points, and a spiral is a run of quarter arcs that get smaller and
//! smaller.

use craftkit_derive::Command;

use craftkit_sdk::document::{Path, Point, Segment, SubPath};
use craftkit_sdk::format::FormatValue;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Number, Outcome, Session, Text};

use crate::VectorNode;
use crate::add_path;

/// The most points a **star** may have.
///
/// The tree's own bound is a thousand, and a star with more points
/// than that is a shape nobody drew.
const MAX_STAR_POINTS: u32 = 1000;

/// The least **decay** a spiral may have, in percent.
///
/// The tree's own bound is five percent: a spiral that decays faster
/// than that is a point and not a spiral.
const MIN_DECAY: f64 = 5.0;

/// The most **decay** a spiral may have, in percent.
const MAX_DECAY: f64 = 99.99;

/// The **kappa**: the constant that turns a quarter circle into a
/// cubic curve.
///
/// It is the tree's constant and it is the same in every place that
/// draws a quarter of a circle.
const KAPPA: f64 = 0.5522847498307936;

/// A **star**: points around a centre, at two radii.
///
/// The points alternate: the first is at the **outer** radius, the
/// second at the **inner** one, and so on. The star starts at the top
/// and goes round, and the **rotation** turns it.
#[derive(Command)]
#[command(
    id = "vector.star",
    area = "vector",
    layer = "vector",
    icon = "star",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddStar {
    /// Its centre, across.
    pub x: Number,
    /// Its centre, down.
    pub y: Number,
    /// How far its points are from the centre.
    pub radius: Number,
    /// How far its inner corners are from the centre.
    ///
    /// Without it, the inner radius is half the outer one.
    pub inner_radius: Option<Number>,
    /// How many points it has.
    pub points: Option<Number>,
    /// How far it is turned, in degrees.
    pub rotation: Option<Number>,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl AddStar {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.star".to_string();
        let radius = self.radius.0;
        if !radius.is_finite() || radius < 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a radius is a number and not a negative one".to_string(),
            });
        }
        let inner = match &self.inner_radius {
            Some(inner) => inner.0,
            None => radius / 2.0,
        };
        if !inner.is_finite() || inner < 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a radius is a number and not a negative one".to_string(),
            });
        }
        let points = match &self.points {
            Some(points) => {
                let count = points.0;
                if !count.is_finite() || count < 2.0 || count > MAX_STAR_POINTS as f64 {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("a star has between two and {MAX_STAR_POINTS} points"),
                    });
                }
                count as u32
            }
            None => 5,
        };
        let rotation = self.rotation.map(|rotation| rotation.0).unwrap_or(0.0);
        if !rotation.is_finite() {
            return Err(EngineError::Failed { id, message: "a rotation is a number of degrees".to_string() });
        }
        let path = star(self.x.0, self.y.0, radius, inner, points, rotation);
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("star {points}"),
        };
        let node = add_path(session, name, path, self.fill.as_ref())?;
        Ok(Outcome::changed(format!("added a star `{node}`")).with_data(json!({ "id": node })))
    }
}

/// A **line**: a contour of two points.
///
/// A line is stroked and not filled: there is nothing inside it. The
/// colour it is stroked with is the path's own stroke, and a path
/// without a stroke is not drawn.
#[derive(Command)]
#[command(
    id = "vector.line",
    area = "vector",
    layer = "vector",
    icon = "line",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddLine {
    /// Where it starts, across.
    pub x1: Number,
    /// Where it starts, down.
    pub y1: Number,
    /// Where it ends, across.
    pub x2: Number,
    /// Where it ends, down.
    pub y2: Number,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is stroked with, as `#rrggbb`.
    pub stroke: Option<Text>,
}

impl AddLine {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.line".to_string();
        for number in [self.x1.0, self.y1.0, self.x2.0, self.y2.0] {
            if !number.is_finite() {
                return Err(EngineError::Failed { id, message: "a point is a number".to_string() });
            }
        }
        let start = Point::new(self.x1.0, self.y1.0);
        let end = Point::new(self.x2.0, self.y2.0);
        let path = Path::single(SubPath { start, segments: vec![Segment::Line { to: end }], closed: false });
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => "line".to_string(),
        };
        let node = add_path(session, name, path, None)?;
        // The **stroke**: a line is stroked, and a path with no stroke
        // is not drawn at all.
        if let Some(stroke) = &self.stroke {
            let colour = crate::parse_colour(&stroke.0)?;
            if let Some(node) = session.document_mut()?.node_mut(node)
                && let Ok(mut vector) = VectorNode::from_json(&node.data)
            {
                vector.stroke = Some(craftkit_sdk::document::Stroke { colour, ..Default::default() });
                node.data = vector.to_json();
            }
        }
        Ok(Outcome::changed(format!("added a line `{node}`")).with_data(json!({ "id": node })))
    }
}

/// A **spiral**: quarter arcs that get smaller and smaller.
///
/// The spiral winds **inward**: it starts at the outside and ends at
/// the centre, and the **decay** is how much of the radius is left
/// after each quarter turn.
#[derive(Command)]
#[command(
    id = "vector.spiral",
    area = "vector",
    layer = "vector",
    icon = "spiral",
    menu("Create"),
    mutates,
    undoable
)]
pub struct AddSpiral {
    /// Its centre, across.
    pub x: Number,
    /// Its centre, down.
    pub y: Number,
    /// How far out it reaches.
    pub radius: Number,
    /// How much of the radius is left after each quarter turn, in
    /// percent.
    pub decay: Option<Number>,
    /// How many quarter turns it makes.
    pub segments: Option<Number>,
    /// Whether it winds clockwise.
    pub clockwise: Option<Bool>,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl AddSpiral {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.spiral".to_string();
        let radius = self.radius.0;
        if !radius.is_finite() || radius <= 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a radius is a number and not a negative one".to_string(),
            });
        }
        let decay = self.decay.map(|decay| decay.0).unwrap_or(80.0);
        if !decay.is_finite() || !(MIN_DECAY..=MAX_DECAY).contains(&decay) {
            return Err(EngineError::Failed {
                id,
                message: format!("a decay is a number of percent from {MIN_DECAY} to {MAX_DECAY}"),
            });
        }
        let segments = match &self.segments {
            Some(segments) => {
                let count = segments.0;
                if !count.is_finite() || count < 2.0 || count > MAX_STAR_POINTS as f64 {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("a spiral has between two and {MAX_STAR_POINTS} turns"),
                    });
                }
                count as u32
            }
            None => 10,
        };
        let clockwise = self.clockwise.map(|clockwise| clockwise.0).unwrap_or(true);
        let path = spiral(self.x.0, self.y.0, radius, decay / 100.0, segments, clockwise);
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => "spiral".to_string(),
        };
        let node = add_path(session, name, path, self.fill.as_ref())?;
        Ok(Outcome::changed(format!("added a spiral `{node}`")).with_data(json!({ "id": node })))
    }
}

/// The **points** of a star: they alternate between the two radii.
///
/// The star starts at the **top** and turns the way the clock does,
/// and the rotation is added to that.
fn star(x: f64, y: f64, radius: f64, inner: f64, points: u32, rotation: f64) -> Path {
    let count = points.max(2) * 2;
    let turn = rotation.to_radians();
    let mut places = Vec::with_capacity(count as usize);
    for index in 0..count {
        let r = if index % 2 == 0 { radius } else { inner };
        let angle =
            -std::f64::consts::FRAC_PI_2 + turn + std::f64::consts::TAU * (index as f64) / (count as f64);
        places.push(Point::new(x + r * angle.cos(), y + r * angle.sin()));
    }
    Path::polygon(&places)
}

/// The **points** of a spiral: a run of quarter arcs that shrink.
///
/// The spiral winds inward: the anchors are worked out from the
/// outside in and then the contour is **reversed**, so that the path
/// starts at the centre and ends at the outside — the way a person
/// draws one.
fn spiral(x: f64, y: f64, radius: f64, decay: f64, segments: u32, clockwise: bool) -> Path {
    let direction = if clockwise { 1.0 } else { -1.0 };
    let mut anchors: Vec<(Point, Point, Point)> = Vec::with_capacity(segments as usize + 1);
    let mut r = radius;
    for _ in 0..=segments {
        let angle = direction * (anchors.len() as f64) * std::f64::consts::FRAC_PI_2;
        let point = Point::new(x + r * angle.cos(), y + r * angle.sin());
        // The **tangent** of a quarter arc of radius `r`: the
        // direction the curve leaves the point in.
        let tangent = Point::new(-angle.sin(), angle.cos());
        let before = r / decay;
        let after = r * decay;
        let in_handle = Point::new(
            point.x - tangent.x * direction * (KAPPA * (r + before) / 2.0),
            point.y - tangent.y * direction * (KAPPA * (r + before) / 2.0),
        );
        let out_handle = Point::new(
            point.x + tangent.x * direction * (KAPPA * (r + after) / 2.0),
            point.y + tangent.y * direction * (KAPPA * (r + after) / 2.0),
        );
        anchors.push((point, in_handle, out_handle));
        r *= decay;
    }
    // The spiral is **reversed**: it is worked out from the outside in
    // and it is drawn from the centre out, and a contour that is
    // turned around has its handles the other way round.
    anchors.reverse();
    let anchors: Vec<(Point, Point, Point)> =
        anchors.into_iter().map(|(point, in_handle, out_handle)| (point, out_handle, in_handle)).collect();
    let Some((first, _, first_out)) = anchors.first().copied() else { return Path::polygon(&[]) };
    let mut path = SubPath { start: first, segments: Vec::new(), closed: false };
    // The curve from one anchor to the next leaves by the first one's
    // **out** handle and arrives by the next one's **in** handle.
    let mut out = first_out;
    for (point, in_handle, _) in anchors.into_iter().skip(1) {
        path.segments.push(Segment::Cubic { c1: out, c2: in_handle, to: point });
        out = in_handle;
    }
    Path::single(path)
}
