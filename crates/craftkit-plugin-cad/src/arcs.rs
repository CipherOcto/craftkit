//! The **arc** of a drawing: a piece of a circle.
//!
//! An arc is a **centre**, a **radius** and two **ends**, and it runs
//! from one end to the other the way a circle does: **counter-
//! clockwise**, and the way round it goes is not more than a whole
//! turn.
//!
//! # What the port found
//!
//! - **The tree's arc is two angles and a radius.** Here the arc is
//!   made with two angles too, and it **carries its ends**: the two
//!   points where the arc begins and ends. The angles are worked out
//!   from them, and they are worked out every time — the sweep of an
//!   arc is a fact of its ends, not a number that is written down and
//!   may go stale.
//! - **The tree's arc is a variant of its own**, with its own fields
//!   and its own file mapping. Here an arc is a node of an **open
//!   kind**, and a new kind of curve is a new kind and nothing else.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// How many points an arc is drawn with.
const ARC_SEGMENTS: usize = 64;

/// The **sweep** an angle runs through, from nothing to a whole turn.
fn sweep_between(start: f64, end: f64) -> f64 {
    let mut sweep = end - start;
    while sweep < 0.0 {
        sweep += std::f64::consts::TAU;
    }
    while sweep > std::f64::consts::TAU {
        sweep -= std::f64::consts::TAU;
    }
    sweep
}

/// The **angle** of a direction, in radians.
fn angle_of(from: (f64, f64), to: (f64, f64)) -> f64 {
    (to.1 - from.1).atan2(to.0 - from.0)
}

/// The **point** of a geometry, if it has one there.
fn point_in(geometry: &Json, key: &str) -> Option<(f64, f64)> {
    let value = geometry.get(key)?;
    let x = value.get("x").and_then(|x| x.as_f64())?;
    let y = value.get("y").and_then(|y| y.as_f64())?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The **radius** of an arc.
///
/// It is the radius in the payload when it is there, and the distance
/// from the centre to an end when it is not: an arc that says how far
/// it is from its centre has a radius, and one that does not has one
/// anyway.
pub(crate) fn arc_radius(geometry: &Json) -> Option<f64> {
    if let Some(radius) = geometry
        .get("radius")
        .and_then(|radius| radius.as_f64())
        .filter(|radius| radius.is_finite() && *radius > 0.0)
    {
        return Some(radius);
    }
    let center = point_in(geometry, "center")?;
    let start = point_in(geometry, "start")?;
    let radius = ((start.0 - center.0).powi(2) + (start.1 - center.1).powi(2)).sqrt();
    (radius.is_finite() && radius > 0.0).then_some(radius)
}

/// The **sweep** of an arc, in radians: how far it goes the way round
/// it goes.
pub(crate) fn arc_sweep(geometry: &Json) -> Option<f64> {
    let center = point_in(geometry, "center")?;
    let start = point_in(geometry, "start")?;
    let end = point_in(geometry, "end")?;
    Some(sweep_between(angle_of(center, start), angle_of(center, end)))
}

/// The **points** of an arc: the curve, sampled.
pub(crate) fn arc_points(geometry: &Json) -> Vec<(f64, f64)> {
    let Some(center) = point_in(geometry, "center") else { return Vec::new() };
    let Some(radius) = arc_radius(geometry) else { return Vec::new() };
    let Some(sweep) = arc_sweep(geometry) else { return Vec::new() };
    let Some(start) = point_in(geometry, "start") else { return Vec::new() };
    let first = angle_of(center, start);
    let mut points = Vec::with_capacity(ARC_SEGMENTS + 1);
    for step in 0..=ARC_SEGMENTS {
        let angle = first + sweep * step as f64 / ARC_SEGMENTS as f64;
        points.push((center.0 + radius * angle.cos(), center.1 + radius * angle.sin()));
    }
    points
}

commands! {
    /// Put an **arc** in a block.
    cad.entity.arc.add {
        area: "cad",
        layer: "cad",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The block it goes in.
            block: NodeRef,
            /// What to call it.
            name: Text,
            /// Its centre.
            center: Json,
            /// Its radius: above nothing.
            radius: Number,
            /// The angle it starts at, in radians.
            start: Number,
            /// The angle it ends at, in radians.
            end: Number,
        },
        run: {
            let id = "cad.entity.arc.add".to_string();
            let (cx, cy) = point_of(&self.center, &id, "the centre")?;
            let radius = self.radius.0;
            if !radius.is_finite() || radius <= 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "the radius of an arc is a number above nothing".to_string(),
                });
            }
            let start = self.start.0;
            let end = self.end.0;
            if !start.is_finite() || !end.is_finite() {
                return Err(EngineError::Failed {
                    id,
                    message: "the angles of an arc are numbers of radians".to_string(),
                });
            }
            if sweep_between(start, end) <= 0.0 {
                return Err(EngineError::Failed { id, message: "an arc of no sweep is not an arc".to_string() });
            }
            let geometry = json!({
                "center": { "x": cx, "y": cy },
                "radius": radius,
                "start": {
                    "x": cx + radius * start.cos(),
                    "y": cy + radius * start.sin(),
                },
                "end": {
                    "x": cx + radius * end.cos(),
                    "y": cy + radius * end.sin(),
                },
            });
            let entity = crate::add_entity(
                session,
                self.block.0,
                &self.name.0,
                "cad.entity.arc",
                geometry,
                "cad.entity.arc.add",
            )?;
            Ok(Outcome::changed(format!("added the arc `{}`", self.name.0))
                .with_data(json!({ "id": entity, "name": self.name.0 })))

        }
    }
}

/// A **point**, read from the caller's JSON.
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
