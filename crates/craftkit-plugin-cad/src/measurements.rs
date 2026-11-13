//! The **measure** of a drawing: how long, how big, how wide, and how
//! far apart.
//!
//! A drawing is not only a picture of things: it is a set of things
//! whose **sizes** may be asked for. A line is so long, a circle so
//! big, a polyline so much area, and two lines so far apart at an
//! angle.
//!
//! Every number here is **derived**: it is worked out from the geometry
//! in the payload every time it is asked for, and it is never stored.
//! A measurement that was stored would be a second copy of the
//! geometry, and a second copy is a copy that can be wrong.
//!
//! The tree's rules, and they are the finding:
//!
//! - a **length** is a length of a path: a line is the distance
//!   between its ends, a polyline is the sum of its segments, a circle
//!   is its **circumference** and an arc is the length of the piece of
//!   it that the arc is;
//! - an **area** is the area **enclosed**: a circle is pi times the
//!   radius squared, an ellipse is pi times the two half axes, and a
//!   polyline is enclosed **only when it is closed**;
//! - a **radius** is the radius of a circle or an arc, and the
//!   **diameter** is twice it;
//! - an **angle** is the angle at a point between two directions, and
//!   it is in **degrees**, and it is never more than a straight one.
//!
//! # What the port found
//!
//! - **The tree's measures are commands of their own, and each one
//!   answers about a different kind of thing.** Here the same: a
//!   length, an area, a radius and an angle are four commands and not
//!   one command with a mode. A command that asks "what do you want
//!   measured?" has already asked the caller to do the work the
//!   command should do.
//! - **The tree's area of an open polyline is nothing and the tree
//!   says so.** Here the same: an open polyline encloses no area, and
//!   the measure is **refused** rather than answered with a zero. A
//!   zero is a number, and a number that means "this was not asked
//!   properly" is a lie.
//! - **The tree's angle is worked out from two lines and their
//!   intersection.** Here the **vertex** is given: the angle is at a
//!   point, between two directions. Working the vertex out means
//!   extending two lines until they meet, and a line that is parallel
//!   to another never meets it — a case the tree answers with an error
//!   and this one does not have.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// How many points a curve is sampled with.
///
/// Enough that the length of a curve is near the truth, and few enough
/// that the answer comes at once. The length of an ellipse is
/// **sampled**, and that is said here and in the README: the tree's is
/// exact.
const CURVE_SEGMENTS: usize = 256;

/// The **entity** a node is, if it is one.
fn the_entity<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no entity `{node}`"),
        });
    };
    if !found.kind.starts_with("cad.entity.") {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not an entity", found.kind),
        });
    }
    Ok(found)
}

/// The **geometry** of an entity.
fn the_geometry<'a>(
    node: &'a craftkit_sdk::Node,
    command: &str,
    entity: u64,
) -> Result<&'a Json, EngineError> {
    node.data.get("geometry").ok_or_else(|| EngineError::Failed {
        id: command.to_string(),
        message: format!("the entity `{entity}` has no geometry"),
    })
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

/// The **point** of a geometry, if it has one there.
fn point_in(geometry: &Json, key: &str) -> Option<(f64, f64)> {
    let value = geometry.get(key)?;
    let x = value.get("x").and_then(|x| x.as_f64())?;
    let y = value.get("y").and_then(|y| y.as_f64())?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The **radius** of a geometry, if it has one.
fn radius_of(geometry: &Json) -> Option<f64> {
    geometry
        .get("radius")
        .and_then(|radius| radius.as_f64())
        .filter(|radius| radius.is_finite() && *radius > 0.0)
}

/// The distance between two **points**.
fn distance(from: (f64, f64), to: (f64, f64)) -> f64 {
    ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt()
}

/// The **points** of a polyline.
fn polyline_points(geometry: &Json) -> Vec<(f64, f64)> {
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

/// Whether a polyline is **closed**.
fn is_closed(geometry: &Json) -> bool {
    geometry.get("closed").and_then(|closed| closed.as_bool()).unwrap_or(false)
}

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

/// The **points** of an ellipse: the curve itself, sampled.
///
/// A whole ellipse is a whole turn of the curve, and a part of one is
/// the sweep it runs through.
fn ellipse_curve_points(geometry: &Json) -> Vec<(f64, f64)> {
    let Some((cx, cy)) = point_in(geometry, "center") else { return Vec::new() };
    let Some((mx, my)) = point_in(geometry, "major") else { return Vec::new() };
    let ratio = geometry.get("ratio").and_then(|ratio| ratio.as_f64()).unwrap_or(1.0);
    let start = geometry.get("start").and_then(|start| start.as_f64()).unwrap_or(0.0);
    let end = geometry.get("end").and_then(|end| end.as_f64()).unwrap_or(std::f64::consts::TAU);
    let sweep = sweep_between(start, end);
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

/// The **length** of a geometry: the length of the path it draws.
pub(crate) fn length_of(kind: &str, geometry: &Json) -> Option<f64> {
    match kind {
        "cad.entity.line" => {
            let from = point_in(geometry, "from")?;
            let to = point_in(geometry, "to")?;
            Some(distance(from, to))
        }
        "cad.entity.polyline" => {
            let points = polyline_points(geometry);
            if points.len() < 2 {
                return None;
            }
            let mut total: f64 = points.windows(2).map(|pair| distance(pair[0], pair[1])).sum();
            if is_closed(geometry) {
                total += distance(points[points.len() - 1], points[0]);
            }
            Some(total)
        }
        "cad.entity.circle" => Some(std::f64::consts::TAU * radius_of(geometry)?),
        "cad.entity.arc" => Some(crate::arcs::arc_radius(geometry)? * crate::arcs::arc_sweep(geometry)?),
        "cad.entity.ellipse" => {
            let points = ellipse_curve_points(geometry);
            if points.len() < 2 {
                return None;
            }
            Some(points.windows(2).map(|pair| distance(pair[0], pair[1])).sum())
        }
        _ => None,
    }
}

/// The **area** a geometry encloses.
fn area_of(kind: &str, geometry: &Json) -> Option<f64> {
    match kind {
        "cad.entity.circle" => Some(std::f64::consts::PI * radius_of(geometry)? * radius_of(geometry)?),
        "cad.entity.ellipse" => {
            let (mx, my) = point_in(geometry, "major")?;
            let ratio = geometry.get("ratio").and_then(|ratio| ratio.as_f64()).unwrap_or(1.0);
            if !ratio.is_finite() || ratio <= 0.0 || ratio > 1.0 {
                return None;
            }
            let major = (mx * mx + my * my).sqrt();
            Some(std::f64::consts::PI * major * major * ratio)
        }
        "cad.entity.polyline" => {
            if !is_closed(geometry) {
                // An open polyline encloses nothing, and that is not a
                // zero: it is a question that was not asked properly.
                return None;
            }
            let points = polyline_points(geometry);
            if points.len() < 3 {
                return None;
            }
            // The **shoelace**: the sum of the triangles from the first
            // point to every side of the polygon.
            let mut twice = 0.0;
            for index in 0..points.len() {
                let (x1, y1) = points[index];
                let (x2, y2) = points[(index + 1) % points.len()];
                twice += x1 * y2 - x2 * y1;
            }
            Some(twice.abs() / 2.0)
        }
        _ => None,
    }
}

/// The **radius** of a geometry.
fn radius_of_kind(kind: &str, geometry: &Json) -> Option<f64> {
    match kind {
        "cad.entity.circle" => radius_of(geometry),
        "cad.entity.arc" => crate::arcs::arc_radius(geometry),
        _ => None,
    }
}

commands! {
    /// The **length** of an entity's path.
    cad.measure.length {
        area: "cad",
        layer: "cad",
        icon: "ruler",
        params: {
            /// The entity.
            entity: NodeRef,
        },
        run: {
            let id = "cad.measure.length".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            let kind = node.kind.clone();
            let geometry = the_geometry(node, &id, entity)?;
            let Some(length) = length_of(&kind, geometry) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("not supported yet: the length of a `{kind}`"),
                });
            };
            Ok(Outcome::new(format!("the `{kind}` `{entity}` is {length} long"))
                .with_data(json!({ "entity": entity, "length": length })))

        }
    }

    /// The **area** an entity encloses.
    cad.measure.area {
        area: "cad",
        layer: "cad",
        icon: "rect",
        params: {
            /// The entity.
            entity: NodeRef,
        },
        run: {
            let id = "cad.measure.area".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            let kind = node.kind.clone();
            let geometry = the_geometry(node, &id, entity)?;
            let Some(area) = area_of(&kind, geometry) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("not supported yet: the area of a `{kind}`"),
                });
            };
            Ok(Outcome::new(format!("the `{kind}` `{entity}` encloses {area}"))
                .with_data(json!({ "entity": entity, "area": area })))

        }
    }

    /// The **radius** of an entity.
    cad.measure.radius {
        area: "cad",
        layer: "cad",
        icon: "circle",
        params: {
            /// The entity.
            entity: NodeRef,
        },
        run: {
            let id = "cad.measure.radius".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            let kind = node.kind.clone();
            let geometry = the_geometry(node, &id, entity)?;
            let Some(radius) = radius_of_kind(&kind, geometry) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("not supported yet: the radius of a `{kind}`"),
                });
            };
            Ok(Outcome::new(format!("the `{kind}` `{entity}` has the radius {radius}"))
                .with_data(json!({ "entity": entity, "radius": radius, "diameter": radius * 2.0 })))

        }
    }

    /// The **angle** at a point, between two directions.
    cad.measure.angle {
        area: "cad",
        layer: "cad",
        icon: "rotate",
        params: {
            /// The point the angle is at.
            vertex: Json,
            /// The first direction: the point it points at.
            first: Json,
            /// The second direction: the point it points at.
            second: Json,
        },
        run: {
            let id = "cad.measure.angle".to_string();
            let vertex = point_of(&self.vertex, &id, "the vertex")?;
            let first = point_of(&self.first, &id, "the first direction")?;
            let second = point_of(&self.second, &id, "the second direction")?;
            if distance(vertex, first) <= f64::EPSILON || distance(vertex, second) <= f64::EPSILON {
                return Err(EngineError::Failed {
                    id,
                    message: "a direction is a point that is not the vertex".to_string(),
                });
            }
            let mut degrees = (angle_of(vertex, second) - angle_of(vertex, first)).to_degrees();
            while degrees < 0.0 {
                degrees += 360.0;
            }
            while degrees > 360.0 {
                degrees -= 360.0;
            }
            // An angle is never more than a straight one: the two
            // directions are the same either way round.
            if degrees > 180.0 {
                degrees = 360.0 - degrees;
            }
            Ok(Outcome::new(format!("the angle is {degrees} degrees")).with_data(json!({ "angle": degrees })))

        }
    }
}
