//! The **modification** of an entity: moving it, turning it, making
//! it bigger or smaller, and taking it away.
//!
//! A drawing is not a picture: it is a set of things that may be
//! **moved**, **turned** and **scaled**, and every one of those is a
//! change of the geometry and of nothing else. The name of the
//! entity stays, the layer it is on stays, and the geometry is worked
//! out again.
//!
//! The tree's rules, and they are the finding:
//!
//! - a **turn** is about a **point**, and it is in **degrees**;
//! - a **scale** is about a **point** too, and its factor is above
//!   nothing: a scale of nothing is not a scale, and a scale of less
//!   than nothing turns a thing inside out;
//! - a **circle** that is scaled gets a **bigger radius**, and a
//!   circle that is turned keeps its radius;
//! - an **ellipse** that is scaled keeps its **ratio** — the minor
//!   axis is still the same part of the major one — and an ellipse
//!   that is turned has its **major axis turned** with it.
//!
//! # What the port found
//!
//! - **The tree's entities are a closed union and their modification
//!   is a change of the variant's fields**: here an entity is a node
//!   of an **open kind** and its modification is a change of the
//!   **geometry in its payload** — so a new kind of entity is
//!   modified by writing one more case and not by touching four
//!   places at once.
//! - **The tree's turn of a text turns the text's own angle** and here
//!   the same: a text carries its angle and the turn adds to it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// A point of the world's units.
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

/// A value that **is** a point.
fn point_value(value: &Json) -> Option<(f64, f64)> {
    let x = value.get("x").and_then(|x| x.as_f64())?;
    let y = value.get("y").and_then(|y| y.as_f64())?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The **point** of a geometry, if it has one there.
fn point_in(geometry: &Json, key: &str) -> Option<(f64, f64)> {
    point_value(geometry.get(key)?)
}

/// A point, written back.
fn written(x: f64, y: f64) -> Json {
    json!({ "x": x, "y": y })
}

/// The **geometry** of an entity, moved by a vector.
pub(crate) fn moved(kind: &str, geometry: &Json, dx: f64, dy: f64) -> Option<Json> {
    let mut out = geometry.clone();
    let shift = |value: &mut Json| {
        let x = value.get("x").and_then(|x| x.as_f64())? + dx;
        let y = value.get("y").and_then(|y| y.as_f64())? + dy;
        *value = written(x, y);
        Some(())
    };
    match kind {
        "cad.entity.line" => {
            shift(out.get_mut("from")?)?;
            shift(out.get_mut("to")?)?;
        }
        "cad.entity.circle" | "cad.entity.arc" => {
            shift(out.get_mut("center")?)?;
            if kind == "cad.entity.arc" {
                shift(out.get_mut("start")?)?;
                shift(out.get_mut("end")?)?;
            }
        }
        "cad.entity.ellipse" => {
            shift(out.get_mut("center")?)?;
        }
        "cad.entity.polyline" => {
            let points = out.get_mut("points")?.as_array_mut()?;
            for point in points.iter_mut() {
                shift(point)?;
            }
        }
        "cad.entity.text" | "cad.entity.insert" => {
            shift(out.get_mut("at")?)?;
        }
        _ => return None,
    }
    Some(out)
}

/// The **geometry** of an entity, turned about a point.
pub(crate) fn turned(kind: &str, geometry: &Json, about: (f64, f64), degrees: f64) -> Option<Json> {
    let mut out = geometry.clone();
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let turn = |x: f64, y: f64| -> (f64, f64) {
        let dx = x - about.0;
        let dy = y - about.1;
        (about.0 + dx * cos - dy * sin, about.1 + dx * sin + dy * cos)
    };
    match kind {
        "cad.entity.line" => {
            for key in ["from", "to"] {
                let point = point_in(&out, key)?;
                let (x, y) = turn(point.0, point.1);
                *out.get_mut(key)? = written(x, y);
            }
        }
        "cad.entity.circle" | "cad.entity.arc" => {
            let center = point_in(&out, "center")?;
            let (x, y) = turn(center.0, center.1);
            out["center"] = written(x, y);
            if kind == "cad.entity.arc" {
                for key in ["start", "end"] {
                    let point = point_in(&out, key)?;
                    let (x, y) = turn(point.0, point.1);
                    *out.get_mut(key)? = written(x, y);
                }
            }
        }
        "cad.entity.ellipse" => {
            let center = point_in(&out, "center")?;
            let (x, y) = turn(center.0, center.1);
            out["center"] = written(x, y);
            // The major axis is a **vector**, and it turns with the
            // ellipse.
            let major = point_in(&out, "major")?;
            let (mx, my) = turn(major.0, major.1);
            out["major"] = written(mx - x, my - y);
        }
        "cad.entity.polyline" => {
            let points = out.get_mut("points")?.as_array_mut()?;
            for point in points.iter_mut() {
                let (x, y) = point_value(point)?;
                let (x, y) = turn(x, y);
                *point = written(x, y);
            }
        }
        "cad.entity.text" | "cad.entity.insert" => {
            let at = point_in(&out, "at")?;
            let (x, y) = turn(at.0, at.1);
            out["at"] = written(x, y);
            // A text carries its own angle, and the turn adds to it.
            if let Some(angle) = out.get("angle").and_then(|angle| angle.as_f64()) {
                out["angle"] = json!(angle + degrees);
            }
        }
        _ => return None,
    }
    Some(out)
}

/// The **geometry** of an entity, scaled about a point.
fn scaled(kind: &str, geometry: &Json, about: (f64, f64), factor: f64) -> Option<Json> {
    let mut out = geometry.clone();
    let scale = |x: f64, y: f64| -> (f64, f64) {
        (about.0 + (x - about.0) * factor, about.1 + (y - about.1) * factor)
    };
    match kind {
        "cad.entity.line" => {
            for key in ["from", "to"] {
                let point = point_in(&out, key)?;
                let (x, y) = scale(point.0, point.1);
                *out.get_mut(key)? = written(x, y);
            }
        }
        "cad.entity.circle" | "cad.entity.arc" => {
            let center = point_in(&out, "center")?;
            let (x, y) = scale(center.0, center.1);
            out["center"] = written(x, y);
            // A circle that is scaled gets a bigger radius.
            if let Some(radius) = out.get("radius").and_then(|radius| radius.as_f64()) {
                out["radius"] = json!(radius * factor);
            }
            if kind == "cad.entity.arc" {
                for key in ["start", "end"] {
                    let point = point_in(&out, key)?;
                    let (x, y) = scale(point.0, point.1);
                    *out.get_mut(key)? = written(x, y);
                }
            }
        }
        "cad.entity.ellipse" => {
            let center = point_in(&out, "center")?;
            let (x, y) = scale(center.0, center.1);
            out["center"] = written(x, y);
            // The major axis is scaled and the **ratio** is not: the
            // minor axis is still the same part of the major one.
            let major = point_in(&out, "major")?;
            let (mx, my) = scale(major.0, major.1);
            out["major"] = written(mx - x, my - y);
        }
        "cad.entity.polyline" => {
            let points = out.get_mut("points")?.as_array_mut()?;
            for point in points.iter_mut() {
                let (x, y) = point_value(point)?;
                let (x, y) = scale(x, y);
                *point = written(x, y);
            }
        }
        "cad.entity.text" | "cad.entity.insert" => {
            let at = point_in(&out, "at")?;
            let (x, y) = scale(at.0, at.1);
            out["at"] = written(x, y);
            // A text that is scaled is a bigger text.
            if let Some(height) = out.get("height").and_then(|height| height.as_f64()) {
                out["height"] = json!(height * factor);
            }
        }
        _ => return None,
    }
    Some(out)
}

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

commands! {
    /// **Move** an entity by a vector.
    cad.entity.move {
        area: "cad",
        layer: "cad",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
            /// The vector it moves by.

            by: Json,
        },
        run: {

            let id = "cad.entity.move".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let (dx, dy) = point_of(&self.by, &id, "the vector")?;
            let Some(geometry) = node.data.get("geometry") else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the entity `{entity}` has no geometry"),
                });
            };
            let Some(new) = moved(&node.kind, geometry, dx, dy) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("an entity of the kind `{}` may not be moved", node.kind),
                });
            };
            let Some(node) = session.document_mut()?.node_mut(entity) else {
                return Err(EngineError::Failed {
                    id: "cad.entity.move".to_string(),
                    message: format!("there is no entity `{entity}`"),
                });
            };
            node.data["geometry"] = new;
            Ok(Outcome::changed(format!("moved the entity `{entity}`"))
                .with_data(json!({ "entity": entity, "by": { "x": dx, "y": dy } })))

        }
    }
    /// **Turn** an entity about a point.
    cad.entity.rotate {
        area: "cad",
        layer: "cad",
        icon: "rotate",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
            /// The point it turns about.

            about: Json,
            /// The angle, in degrees.

            degrees: Number,
        },
        run: {

            let id = "cad.entity.rotate".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let about = point_of(&self.about, &id, "the point")?;
            if !self.degrees.0.is_finite() {
                return Err(EngineError::Failed { id, message: "an angle is a number of degrees".to_string() });
            }
            let Some(geometry) = node.data.get("geometry") else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the entity `{entity}` has no geometry"),
                });
            };
            let Some(new) = turned(&node.kind, geometry, about, self.degrees.0) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("an entity of the kind `{}` may not be turned", node.kind),
                });
            };
            let Some(node) = session.document_mut()?.node_mut(entity) else {
                return Err(EngineError::Failed {
                    id: "cad.entity.rotate".to_string(),
                    message: format!("there is no entity `{entity}`"),
                });
            };
            node.data["geometry"] = new;
            Ok(Outcome::changed(format!("turned the entity `{entity}`"))
                .with_data(json!({ "entity": entity, "degrees": self.degrees.0 })))

        }
    }
    /// **Scale** an entity about a point.
    cad.entity.scale {
        area: "cad",
        layer: "cad",
        icon: "scale",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
            /// The point it scales about.

            about: Json,
            /// The factor: above nothing.

            factor: Number,
        },
        run: {

            let id = "cad.entity.scale".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let about = point_of(&self.about, &id, "the point")?;
            if !self.factor.0.is_finite() || self.factor.0 <= 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "a factor is a number above nothing".to_string(),
                });
            }
            let Some(geometry) = node.data.get("geometry") else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the entity `{entity}` has no geometry"),
                });
            };
            let Some(new) = scaled(&node.kind, geometry, about, self.factor.0) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("an entity of the kind `{}` may not be scaled", node.kind),
                });
            };
            let Some(node) = session.document_mut()?.node_mut(entity) else {
                return Err(EngineError::Failed {
                    id: "cad.entity.scale".to_string(),
                    message: format!("there is no entity `{entity}`"),
                });
            };
            node.data["geometry"] = new;
            Ok(Outcome::changed(format!("scaled the entity `{entity}`"))
                .with_data(json!({ "entity": entity, "factor": self.factor.0 })))

        }
    }
    /// **Take** an entity **away**.
    cad.entity.remove {
        area: "cad",
        layer: "cad",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
        },
        run: {

            let id = "cad.entity.remove".to_string();
            let entity = self.entity.0;
            the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            if !session.document_mut()?.remove_node(entity) {
                return Err(EngineError::Failed { id, message: format!("there is no entity `{entity}`") });
            }
            Ok(Outcome::changed(format!("took the entity `{entity}` away"))
                .with_data(json!({ "entity": entity })))

        }
    }
}
