//! The **curves** of a drawing: the ellipse and the polyline.
//!
//! An **ellipse** is the tree's own: the **centre**, the **major
//! axis** — a vector from the centre to the end of the major axis —,
//! the **ratio** of the minor axis to the major one, and the two
//! **angles** it runs from and to. An ellipse with no angles is a
//! whole ellipse.
//!
//! A **polyline** is a list of **points** and whether it is
//! **closed**.
//!
//! # What the port found
//!
//! - **The tree's ellipse is DXF-shaped**: the major axis is a
//!   **vector** and not a length, so an ellipse may be turned. Here
//!   the same, and the minor axis is the major one turned a quarter
//!   and made shorter by the ratio.
//! - **The tree's ratio is above nothing and at most one**: the minor
//!   axis is not longer than the major one. Here the same, and a ratio
//!   outside it is refused.
//! - **The tree's bounds of an elliptical arc are exact** and are
//!   worked out from the derivative of the curve. Here the bounds of
//!   a whole ellipse are exact and the bounds of an **arc** of one are
//!   **sampled** — a difference, and it is written down here.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

/// How many points a curve is drawn with.
const CURVE_SEGMENTS: usize = 64;

/// The **point** a value is.
fn point_of(value: &Json, id: &str, what: &str) -> Result<(f64, f64), EngineError> {
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

/// The **angle** an arc runs from and to.
///
/// The sweep is the way round the curve the arc goes, and it is never
/// more than a whole turn.
fn sweep_of(start: f64, end: f64) -> f64 {
    let mut sweep = end - start;
    while sweep < 0.0 {
        sweep += std::f64::consts::TAU;
    }
    while sweep > std::f64::consts::TAU {
        sweep -= std::f64::consts::TAU;
    }
    sweep
}

/// Whether an ellipse is a **whole** one.
fn is_whole(start: f64, end: f64) -> bool {
    (sweep_of(start, end) - std::f64::consts::TAU).abs() < 1e-9
}

/// The **points** of an ellipse: the points of the curve, or — for a
/// whole ellipse — the two corners of the box it fills.
///
/// The corners are exact: the box of an ellipse is the box of its
/// centre and its two half axes, and that is the tree's bound too.
pub(crate) fn ellipse_points(geometry: &Json) -> Vec<(f64, f64)> {
    let Some((cx, cy)) = point_in(geometry, "center") else { return Vec::new() };
    let Some((mx, my)) = point_in(geometry, "major") else { return Vec::new() };
    let ratio = geometry.get("ratio").and_then(|ratio| ratio.as_f64()).unwrap_or(1.0);
    let start = geometry.get("start").and_then(|start| start.as_f64()).unwrap_or(0.0);
    let end = geometry.get("end").and_then(|end| end.as_f64()).unwrap_or(std::f64::consts::TAU);
    if is_whole(start, end) {
        // The half axes: the major one, and the minor one turned a
        // quarter and made shorter by the ratio.
        let half_x = (mx * mx + (my * ratio) * (my * ratio)).sqrt();
        let half_y = (my * my + (mx * ratio) * (mx * ratio)).sqrt();
        return vec![(cx - half_x, cy - half_y), (cx + half_x, cy + half_y)];
    }
    let sweep = sweep_of(start, end);
    let mut points = Vec::with_capacity(CURVE_SEGMENTS + 1);
    for step in 0..=CURVE_SEGMENTS {
        let angle = start + sweep * step as f64 / CURVE_SEGMENTS as f64;
        points.push((
            cx + mx * angle.cos() - my * ratio * angle.sin(),
            cy + mx * angle.sin() + my * ratio * angle.cos(),
        ));
    }
    points
}

/// The **points** of a polyline.
pub(crate) fn polyline_points(geometry: &Json) -> Vec<(f64, f64)> {
    let Some(list) = geometry.get("points").and_then(|points| points.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|value| {
            let x = value.get("x").and_then(|x| x.as_f64())?;
            let y = value.get("y").and_then(|y| y.as_f64())?;
            (x.is_finite() && y.is_finite()).then_some((x, y))
        })
        .collect()
}

/// The **point** of a geometry, if it has one there.
fn point_in(geometry: &Json, key: &str) -> Option<(f64, f64)> {
    let value = geometry.get(key)?;
    let x = value.get("x").and_then(|x| x.as_f64())?;
    let y = value.get("y").and_then(|y| y.as_f64())?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Put an **ellipse** in a block.
#[derive(Command)]
#[command(id = "cad.entity.ellipse.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddEllipse {
    /// The block it goes in.
    pub block: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Its centre.
    pub center: Json,
    /// Its **major axis**: a vector from the centre to the end of the
    /// major axis.
    pub major: Json,
    /// The **ratio** of the minor axis to the major one: above nothing
    /// and at most one.
    pub ratio: Number,
    /// The angle the arc runs from. The whole ellipse when it is not
    /// said.
    pub start: Option<Number>,
    /// The angle the arc runs to. The whole ellipse when it is not
    /// said.
    pub end: Option<Number>,
}

impl AddEllipse {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.entity.ellipse.add".to_string();
        let center = point_of(&self.center, &id, "the centre")?;
        let major = point_of(&self.major, &id, "the major axis")?;
        if major.0 == 0.0 && major.1 == 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "the major axis of an ellipse is not of no length".to_string(),
            });
        }
        if !self.ratio.0.is_finite() || self.ratio.0 <= 0.0 || self.ratio.0 > 1.0 {
            return Err(EngineError::Failed {
                id,
                message: "the ratio of an ellipse is above nothing and at most one".to_string(),
            });
        }
        let start = self.start.map(|start| start.0).unwrap_or(0.0);
        let end = self.end.map(|end| end.0).unwrap_or(std::f64::consts::TAU);
        if !start.is_finite() || !end.is_finite() {
            return Err(EngineError::Failed {
                id,
                message: "the angles of an ellipse are numbers".to_string(),
            });
        }
        let geometry = json!({
            "center": { "x": center.0, "y": center.1 },
            "major": { "x": major.0, "y": major.1 },
            "ratio": self.ratio.0,
            "start": start,
            "end": end,
        });
        let entity = crate::add_entity(
            session,
            self.block.0,
            &self.name.0,
            "cad.entity.ellipse",
            geometry,
            "cad.entity.ellipse.add",
        )?;
        Ok(Outcome::changed(format!("added the ellipse `{}`", self.name.0))
            .with_data(json!({ "id": entity, "name": self.name.0 })))
    }
}

/// Put a **polyline** in a block.
#[derive(Command)]
#[command(id = "cad.entity.polyline.add", area = "cad", layer = "cad", icon = "plus", mutates, undoable)]
pub struct AddPolyline {
    /// The block it goes in.
    pub block: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Its **points**: `[{ "x": …, "y": … }, …]`.
    pub points: Json,
    /// Whether the polyline is **closed**.
    pub closed: Option<Bool>,
}

impl AddPolyline {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.entity.polyline.add".to_string();
        let Some(list) = self.points.as_array() else {
            return Err(EngineError::Failed {
                id,
                message: "the points of a polyline are a list of points".to_string(),
            });
        };
        let mut points = Vec::with_capacity(list.len());
        for value in list {
            let (x, y) = point_of(value, &id, "a point of a polyline")?;
            points.push(json!({ "x": x, "y": y }));
        }
        if points.len() < 2 {
            return Err(EngineError::Failed {
                id,
                message: "a polyline is made of points, and this one is made of none".to_string(),
            });
        }
        let geometry = json!({
            "points": points,
            "closed": self.closed.map(|closed| closed.0).unwrap_or(false),
        });
        let entity = crate::add_entity(
            session,
            self.block.0,
            &self.name.0,
            "cad.entity.polyline",
            geometry,
            "cad.entity.polyline.add",
        )?;
        Ok(Outcome::changed(format!("added the polyline `{}`", self.name.0))
            .with_data(json!({ "id": entity, "name": self.name.0 })))
    }
}
