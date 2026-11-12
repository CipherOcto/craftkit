//! The **array** of an entity.
//!
//! An array is a set of the **items** of an entity: the entity itself
//! and its copies, put in a grid or about a centre. The copies are
//! entities of their own — they may be moved, turned and taken away
//! without the entity they are copies of feeling it.
//!
//! # The rules
//!
//! - **The entity is the first item.** The array holds the entity and
//!   the copies, and the entity stands where the first item stands.
//! - **The copies are in the same block and on the same layer.** A
//!   copy of a thing on a layer that is not on that layer is not a
//!   copy of it.
//! - **A count that is not a whole number, or that is less than one,
//!   is refused.** There is no such thing as half a row.
//! - **An array may not be endless.** A thousand copies is the most an
//!   array may hold.
//!
//! # What it does not do
//!
//! - **The array is not a thing of its own.** The tree's array is a
//!   block that holds the items, and it may be edited as one. Here the
//!   items are entities of the block and the array is not held by
//!   anything.
//! - **The array is not drawn.** The tree draws a preview of the array
//!   while it is made, and there is no drawing here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, Node, NodeRef, Number, Outcome, Relation, Session};

/// The most copies an array may hold.
const MOST_COPIES: usize = 1000;

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

/// A **count**: a whole number of one or more.
fn count_of(value: f64, id: &str, what: &str) -> Result<usize, EngineError> {
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is a whole number of one or more"),
        });
    }
    Ok(value as usize)
}

/// The **block** an entity is in.
fn block_of(document: &Document, entity: u64) -> Option<u64> {
    document.roots().iter().find_map(|root| parent_in(root, entity))
}

/// The node that **holds** a node, if it is held by one.
fn parent_in(node: &Node, entity: u64) -> Option<u64> {
    for child in node.children.iter() {
        if child.id == entity {
            return Some(node.id);
        }
        if let Some(found) = parent_in(child, entity) {
            return Some(found);
        }
    }
    None
}

/// The **centre** of a geometry: the middle of the box it covers.
fn centre_of(geometry: &Json) -> Option<(f64, f64)> {
    let mut lowest = (f64::INFINITY, f64::INFINITY);
    let mut highest = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut any = false;
    for (x, y) in crate::points_of(geometry) {
        any = true;
        lowest.0 = lowest.0.min(x);
        lowest.1 = lowest.1.min(y);
        highest.0 = highest.0.max(x);
        highest.1 = highest.1.max(y);
    }
    if !any {
        return None;
    }
    let (x, y) = ((lowest.0 + highest.0) / 2.0, (lowest.1 + highest.1) / 2.0);
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}

/// The **layer** of an entity, and the **block** an insert refers to:
/// the copies carry them, because a copy of a thing on a layer that is
/// not on that layer is not a copy of it.
fn what_is_carried(session: &Session, entity: u64) -> (Option<u64>, Option<(u64, Json)>) {
    let layer = session
        .document()
        .relations_from(entity, Some(craftkit_sdk::layer::ON_LAYER))
        .first()
        .map(|relation| relation.to);
    let inserts = session
        .document()
        .relations_from(entity, Some("cad.inserts"))
        .first()
        .map(|relation| (relation.to, relation.payload.clone()));
    (layer, inserts)
}

/// The **copy** of an entity, put in a block and on a layer.
fn copy_of(
    session: &mut Session,
    id: &str,
    block: u64,
    name: &str,
    kind: &str,
    geometry: Json,
    carried: &(Option<u64>, Option<(u64, Json)>),
) -> Result<u64, EngineError> {
    let copy = crate::add_entity(session, block, name, kind, geometry, id)?;
    let (layer, inserts) = carried;
    if let Some(layer) = layer {
        let name = session.document().node(*layer).map(|node| node.name.clone()).unwrap_or_default();
        session.document_mut()?.add_relation(Relation::with_payload(
            copy,
            *layer,
            craftkit_sdk::layer::ON_LAYER,
            json!({ "layer": name }),
        ));
    }
    if let Some((block_node, payload)) = inserts {
        session.document_mut()?.add_relation(Relation::with_payload(
            copy,
            *block_node,
            "cad.inserts",
            payload.clone(),
        ));
    }
    Ok(copy)
}

commands! {
    /// **Array** an entity in a **grid**: the rows and the columns.
    cad.entity.array.rectangular {
        area: "cad",
        layer: "cad",
        icon: "array",
        mutates,
        undoable,
        params: {
            /// The entity to array.

            entity: NodeRef,
            /// How many rows.

            rows: Number,
            /// How many columns.

            columns: Number,
            /// The distance between the rows.

            row_spacing: Number,
            /// The distance between the columns.

            column_spacing: Number,
            /// The block the copies go in. The block of the entity itself when
            /// it is not said.

            block: Option<NodeRef>,
        },
        run: {

            let id = "cad.entity.array.rectangular".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let rows = count_of(self.rows.0, &id, "the number of rows")?;
            let columns = count_of(self.columns.0, &id, "the number of columns")?;
            let items = rows * columns;
            if items > MOST_COPIES {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("an array holds at most {MOST_COPIES} items"),
                });
            }
            if !self.row_spacing.0.is_finite() || !self.column_spacing.0.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a distance is a number of the world's units".to_string(),
                });
            }
            let kind = node.kind.clone();
            let geometry = node.data.get("geometry").cloned().ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the entity `{entity}` has no geometry"),
            })?;
            let block = match self.block {
                Some(block) => block.0,
                None => block_of(session.document(), entity).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the entity `{entity}` is in no block"),
                })?,
            };
            let name = node.name.clone();
            let carried = what_is_carried(session, entity);
            // The **copies**: the entity is the first item, and the others
            // are copies of it.
            let mut made: Vec<u64> = Vec::new();
            for row in 0..rows {
                for column in 0..columns {
                    if row == 0 && column == 0 {
                        continue;
                    }
                    let dx = column as f64 * self.column_spacing.0;
                    let dy = row as f64 * self.row_spacing.0;
                    let Some(new) = crate::modify::moved(&kind, &geometry, dx, dy) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("an entity of the kind `{kind}` may not be arrayed"),
                        });
                    };
                    let copy = copy_of(
                        session,
                        &id,
                        block,
                        &format!("{name} {}", made.len() + 2),
                        &kind,
                        new,
                        &carried,
                    )?;
                    made.push(copy);
                }
            }
            let count = made.len();
            Ok(Outcome::changed(format!("arrayed the entity `{entity}` in {items} item(s)")).with_data(json!({
                "entity": entity,
                "copies": made,
                "count": count,
                "items": items,
                "rows": rows,
                "columns": columns,
            })))

        }
    }
    /// **Array** an entity **about a centre**: the items go round it.
    cad.entity.array.polar {
        area: "cad",
        layer: "cad",
        icon: "array",
        mutates,
        undoable,
        params: {
            /// The entity to array.

            entity: NodeRef,
            /// The centre of the array.

            centre: Json,
            /// How many items the array holds.

            count: Number,
            /// The angle the array goes through, in degrees. A whole turn when
            /// it is not said.

            degrees: Option<Number>,
            /// Whether the copies are turned with the array.

            rotate: Option<Bool>,
            /// The block the copies go in. The block of the entity itself when
            /// it is not said.

            block: Option<NodeRef>,
        },
        run: {

            let id = "cad.entity.array.polar".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let count = count_of(self.count.0, &id, "the number of items")?;
            if count > MOST_COPIES {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("an array holds at most {MOST_COPIES} items"),
                });
            }
            let degrees = match &self.degrees {
                Some(degrees) => degrees.0,
                None => 360.0,
            };
            if !degrees.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "an angle is a number of degrees".to_string(),
                });
            }
            let centre = point_of(&self.centre, &id, "the centre")?;
            let rotate = self.rotate.map(|rotate| rotate.0).unwrap_or(false);
            let kind = node.kind.clone();
            let geometry = node.data.get("geometry").cloned().ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the entity `{entity}` has no geometry"),
            })?;
            let Some(home) = centre_of(&geometry) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the entity `{entity}` has no centre to put about the centre"),
                });
            };
            let block = match self.block {
                Some(block) => block.0,
                None => block_of(session.document(), entity).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the entity `{entity}` is in no block"),
                })?,
            };
            let name = node.name.clone();
            let carried = what_is_carried(session, entity);
            // The **copies**: the entity is the first item, and the others
            // go round the centre.
            let step = degrees / count as f64;
            let mut made: Vec<u64> = Vec::new();
            for item in 1..count {
                let angle = item as f64 * step;
                let radians = angle.to_radians();
                let (sin, cos) = radians.sin_cos();
                // The **centre** of the item: the centre of the entity,
                // turned about the centre of the array.
                let (dx, dy) = (home.0 - centre.0, home.1 - centre.1);
                let (turned_x, turned_y) = (dx * cos - dy * sin, dx * sin + dy * cos);
                // The **place** of the centre of the item: the centre of
                // the entity, turned about the centre of the array.
                let place = (centre.0 + turned_x, centre.1 + turned_y);
                let turned = if rotate {
                    crate::modify::turned(&kind, &geometry, centre, angle)
                } else {
                    Some(geometry.clone())
                };
                let Some(turned) = turned else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("an entity of the kind `{kind}` may not be turned"),
                    });
                };
                // The **shift**: the item is put where its centre belongs.
                let Some(item) = centre_of(&turned) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the entity `{entity}` has no centre"),
                    });
                };
                let shift = (place.0 - item.0, place.1 - item.1);
                let Some(new) = crate::modify::moved(&kind, &turned, shift.0, shift.1) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("an entity of the kind `{kind}` may not be arrayed"),
                    });
                };
                let copy =
                    copy_of(session, &id, block, &format!("{name} {}", made.len() + 2), &kind, new, &carried)?;
                made.push(copy);
            }
            let items = made.len() + 1;
            Ok(Outcome::changed(format!("arrayed the entity `{entity}` in {items} item(s)")).with_data(json!({
                "entity": entity,
                "copies": made,
                "count": made.len(),
                "items": items,
                "centre": { "x": centre.0, "y": centre.1 },
                "degrees": degrees,
            })))

        }
    }
}
