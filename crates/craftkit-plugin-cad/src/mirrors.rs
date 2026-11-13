//! The **mirror** of an entity: the same entity, turned over a line.
//!
//! A mirror is a **line** — two points that say where it runs — and
//! every point of the entity is turned over it. What is on the other
//! side of the line is the mirror image, and the mirror image is a
//! thing of its own: it may be moved, turned and taken away without
//! the entity it came from feeling it.
//!
//! # What the port found
//!
//! - **The tree's mirror is a line and a selection.** Here the line is
//!   **two points** and one entity is mirrored at a time: a line that
//!   runs through two points is a line, and a line that is a pair of
//!   directions is a line that has to be worked out first.
//! - **The tree's mirror turns the entity's own angle.** Here the same:
//!   an ellipse carries its **major axis** as a vector, and a vector
//!   turned over a line is not a point turned over a line. The centre
//!   is a point and the axis is a vector, and the mirror knows which
//!   is which.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

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

/// A point, written back.
fn written(x: f64, y: f64) -> Json {
    json!({ "x": x, "y": y })
}

/// The **line** a mirror runs over: two points and the direction it
/// runs in.
struct Line {
    /// A point on the line.
    at: (f64, f64),
    /// The direction it runs in, as a unit vector.
    along: (f64, f64),
}

impl Line {
    /// The line through two points.
    fn through(from: (f64, f64), to: (f64, f64)) -> Option<Line> {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length = (dx * dx + dy * dy).sqrt();
        if !length.is_finite() || length <= f64::EPSILON {
            return None;
        }
        Some(Line { at: from, along: (dx / length, dy / length) })
    }

    /// The **mirror** of a point: the point on the other side.
    fn mirror(&self, point: (f64, f64)) -> (f64, f64) {
        let (ux, uy) = self.along;
        let (vx, vy) = (point.0 - self.at.0, point.1 - self.at.1);
        let along = vx * ux + vy * uy;
        (self.at.0 + 2.0 * along * ux - vx, self.at.1 + 2.0 * along * uy - vy)
    }

    /// The **mirror** of a **vector**: the direction turned over the
    /// line, without moving its starting point.
    ///
    /// A vector has no place, so the line's own point does not matter:
    /// only the direction of the line does.
    fn mirror_vector(&self, vector: (f64, f64)) -> (f64, f64) {
        let (ux, uy) = self.along;
        let along = vector.0 * ux + vector.1 * uy;
        (2.0 * along * ux - vector.0, 2.0 * along * uy - vector.1)
    }
}

/// The **geometry** of an entity, turned over a line.
fn mirrored(kind: &str, geometry: &Json, line: &Line) -> Option<Json> {
    let mut out = geometry.clone();
    let mirror_point = |value: &mut Json| {
        let x = value.get("x").and_then(|x| x.as_f64())?;
        let y = value.get("y").and_then(|y| y.as_f64())?;
        let (mx, my) = line.mirror((x, y));
        *value = written(mx, my);
        Some(())
    };
    match kind {
        "cad.entity.line" => {
            for key in ["from", "to"] {
                mirror_point(out.get_mut(key)?)?;
            }
        }
        "cad.entity.circle" => {
            mirror_point(out.get_mut("center")?)?;
        }
        "cad.entity.arc" => {
            mirror_point(out.get_mut("center")?)?;
            for key in ["start", "end"] {
                mirror_point(out.get_mut(key)?)?;
            }
        }
        "cad.entity.ellipse" => {
            mirror_point(out.get_mut("center")?)?;
            // The major axis is a **vector**, and a vector turned over a
            // line is not a point turned over a line.
            let (mx, my) = point_in(&out, "major")?;
            let (tx, ty) = line.mirror_vector((mx, my));
            *out.get_mut("major")? = written(tx, ty);
        }
        "cad.entity.polyline" => {
            let points = out.get_mut("points")?.as_array_mut()?;
            for point in points.iter_mut() {
                mirror_point(point)?;
            }
        }
        "cad.entity.text" | "cad.entity.insert" => {
            mirror_point(out.get_mut("at")?)?;
        }
        _ => return None,
    }
    Some(out)
}

commands! {
    /// **Mirror** an entity over a line.
    cad.entity.mirror {
        area: "cad",
        layer: "cad",
        icon: "copy",
        mutates,
        undoable,
        params: {
            /// The entity.
            entity: NodeRef,
            /// A point on the line the entity is turned over.
            from: Json,
            /// Another point on that line.
            to: Json,
        },
        run: {
            let id = "cad.entity.mirror".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let kind = node.kind.clone();
            let from = point_of(&self.from, &id, "the first point of the line")?;
            let to = point_of(&self.to, &id, "the second point of the line")?;
            let Some(line) = Line::through(from, to) else {
                return Err(EngineError::Failed {
                    id,
                    message: "the two points of the line are the same point".to_string(),
                });
            };
            let Some(geometry) = node.data.get("geometry") else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the entity `{entity}` has no geometry"),
                });
            };
            let Some(new) = mirrored(&kind, geometry, &line) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("an entity of the kind `{kind}` may not be turned over a line"),
                });
            };
            let Some(node) = session.document_mut()?.node_mut(entity) else {
                return Err(EngineError::Failed {
                    id: "cad.entity.mirror".to_string(),
                    message: format!("there is no entity `{entity}`"),
                });
            };
            node.data["geometry"] = new;
            Ok(Outcome::changed(format!("turned the entity `{entity}` over the line"))
                .with_data(json!({ "entity": entity })))

        }
    }
}
