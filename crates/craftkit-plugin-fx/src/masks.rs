//! The **masks** of a layer.
//!
//! A mask is a **path** on a layer: a list of vertices in the layer's
//! own space, with the tangents of the curve between them, and the
//! curve may be **closed** or open. Around the path are the settings
//! of the mask:
//!
//! - its **mode** — how it joins the masks below it: `None`, `Add`,
//!   `Subtract`, `Intersect`, `Lighten`, `Darken` or `Difference`;
//! - whether it is **inverted**;
//! - whether it is **locked**;
//! - its **feather**, its **opacity** and its **expansion**.
//!
//! A mask is a **node** here and not a list inside the layer's
//! payload: a mask is a thing with a life of its own, it is named,
//! it is drawn, and it can carry things of its own.
//!
//! # The tree's rules, and they are the finding
//!
//! - a mask is made of **vertices**, and a mask with no vertices is
//!   **refused**: the tree's pen tool makes a mask of a path, and a
//!   path of nothing is nothing.
//! - the **mode** of a mask is one of the tree's seven and no other,
//!   and the case does not matter.
//! - an **inverted** mask and a **locked** mask are **toggled** when
//!   the value is not said and **set** when it is.
//! - the masks of a layer are the layer's **children**, and a layer
//!   with no masks has none.

use craftkit_derive::{NodeKind, commands};

use craftkit_sdk::serde_json::{self, Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **modes** a mask may have, in the tree's own order.
pub const MODES: [&str; 7] = ["None", "Add", "Subtract", "Intersect", "Lighten", "Darken", "Difference"];

/// The mode a name says, if it says one of the tree's.
///
/// The tree's rule: the name is the tree's own, and the case does not
/// matter.
pub fn mode_of(name: &str) -> Option<&'static str> {
    let wanted = name.trim();
    MODES.iter().find(|mode| mode.eq_ignore_ascii_case(wanted)).copied()
}

/// A **mask**: a node on a layer.
#[derive(NodeKind)]
#[node_kind(kind = "fx.mask", label = "Mask", layer = "fx", evaluates)]
pub struct MaskKind;

impl MaskKind {
    /// A mask is not drawn.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a mask: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a mask **means**: its path and its settings.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(serde_json::json!({
            "vertices": node.data.get("vertices").cloned().unwrap_or(Json::Null),
            "closed": node.data.get("closed").cloned().unwrap_or(Json::Bool(false)),
            "mode": node.data.get("mode").cloned().unwrap_or(Json::Null),
            "inverted": node.data.get("inverted").cloned().unwrap_or(Json::Bool(false)),
            "locked": node.data.get("locked").cloned().unwrap_or(Json::Bool(false)),
            "feather": node.data.get("feather").cloned().unwrap_or(Json::Null),
            "opacity": node.data.get("opacity").cloned().unwrap_or(Json::Null),
            "expansion": node.data.get("expansion").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// The **layer** a node is, if it is one.
fn the_layer<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no layer `{node}`"),
        });
    };
    if found.kind != "fx.layer" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a layer", found.kind),
        });
    }
    Ok(found)
}

/// The **mask** of a layer, if it is one.
fn the_mask(
    session: &Session,
    command: &str,
    layer: u64,
    mask: u64,
) -> Result<std::sync::Arc<Node>, EngineError> {
    let node = the_layer(session, command, layer)?;
    let Some(found) = node.children.iter().find(|child| child.id == mask) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no mask `{mask}` on the layer `{layer}`"),
        });
    };
    if found.kind != "fx.mask" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{mask}` is a `{}` and not a mask", found.kind),
        });
    }
    Ok(found.clone())
}

/// The **vertices** a value holds: a list of points.
fn vertices_of(value: &Json) -> Result<Vec<[f64; 2]>, String> {
    let Some(list) = value.as_array() else {
        return Err("the vertices of a mask are a list of points".to_string());
    };
    let mut vertices = Vec::with_capacity(list.len());
    for point in list {
        let Some(pair) = point.as_array() else {
            return Err("the vertices of a mask are a list of points".to_string());
        };
        let Some(x) = pair.first().and_then(|x| x.as_f64()) else {
            return Err("a point of a mask is two numbers".to_string());
        };
        let Some(y) = pair.get(1).and_then(|y| y.as_f64()) else {
            return Err("a point of a mask is two numbers".to_string());
        };
        if !x.is_finite() || !y.is_finite() {
            return Err("a point of a mask is two numbers".to_string());
        }
        vertices.push([x, y]);
    }
    Ok(vertices)
}

commands! {
    /// **Make** a **mask** on a layer.
    fx.mask.add {
        area: "fx",
        layer: "fx",
        icon: "mask",
        mutates,
        undoable,
        params: {
            /// The layer the mask is on.
            layer: NodeRef,
            /// The vertices of the mask: `[[x, y], …]` in the layer's space.
            vertices: Json,
            /// Whether the path is closed.
            closed: Option<Bool>,
            /// The mode: `None`, `Add`, `Subtract`, `Intersect`, `Lighten`,
            /// `Darken` or `Difference`.
            mode: Option<Text>,
        },
        run: {
            let id = "fx.mask.add".to_string();
            let layer = self.layer.0;
            the_layer(session, &id, layer)?;
            let vertices =
                vertices_of(&self.vertices).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            if vertices.is_empty() {
                return Err(EngineError::Failed {
                    id,
                    message: "a mask is made of vertices, and this one is made of none".to_string(),
                });
            }
            let mode = match &self.mode {
                Some(mode) => mode_of(&mode.0).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no mode `{}`: the modes are {}", mode.0, MODES.join(", ")),
                })?,
                None => "Add",
            };
            let data = json!({
                "vertices": vertices,
                "closed": self.closed.map(|closed| closed.0).unwrap_or(false),
                "mode": mode,
                "inverted": false,
                "locked": false,
                "feather": 0.0,
                "opacity": 100.0,
                "expansion": 0.0,
            });
            let count = session
                .document()
                .node(layer)
                .map(|node| node.children.iter().filter(|child| child.kind == "fx.mask").count())
                .unwrap_or(0);
            let name = format!("Mask {}", count + 1);
            let mask =
                session.document_mut()?.add_node_with_under(Some(layer), name.clone(), "fx.mask", None, data);
            Ok(Outcome::changed(format!("added the mask `{name}` to the layer `{layer}`"))
                .with_data(json!({ "id": mask, "layer": layer, "name": name })))

        }
    }
    /// **Take away** a mask of a layer.
    fx.mask.remove {
        area: "fx",
        layer: "fx",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The layer the mask is on.
            layer: NodeRef,
            /// The mask.
            mask: NodeRef,
        },
        run: {
            let id = "fx.mask.remove".to_string();
            let layer = self.layer.0;
            let mask = self.mask.0;
            the_mask(session, &id, layer, mask)?;
            if !session.document_mut()?.remove_node(mask) {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no mask `{mask}` on the layer `{layer}`"),
                });
            }
            Ok(Outcome::changed(format!("took the mask `{mask}` away"))
                .with_data(json!({ "layer": layer, "mask": mask })))

        }
    }
    /// **Set** the **mode** of a mask.
    fx.mask.mode {
        area: "fx",
        layer: "fx",
        icon: "mask",
        mutates,
        undoable,
        params: {
            /// The layer the mask is on.
            layer: NodeRef,
            /// The mask.
            mask: NodeRef,
            /// The mode: `None`, `Add`, `Subtract`, `Intersect`, `Lighten`,
            /// `Darken` or `Difference`.
            mode: Text,
        },
        run: {
            let id = "fx.mask.mode".to_string();
            let layer = self.layer.0;
            let mask = self.mask.0;
            the_mask(session, &id, layer, mask)?;
            let wanted = self.mode.0.clone();
            let mode = mode_of(&wanted).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no mode `{wanted}`: the modes are {}", MODES.join(", ")),
            })?;
            let Some(node) = session.document_mut()?.node_mut(mask) else {
                return Err(EngineError::Failed {
                    id: "fx.mask.mode".to_string(),
                    message: format!("there is no mask `{mask}` on the layer `{layer}`"),
                });
            };
            node.data["mode"] = json!(mode);
            Ok(Outcome::changed(format!("the mask `{mask}` is `{mode}`"))
                .with_data(json!({ "layer": layer, "mask": mask, "mode": mode })))

        }
    }
    /// **Invert** a mask, or **uninvert** it.
    ///
    /// The value says what to do; when it is not said the mask is
    /// **toggled**.
    fx.mask.invert {
        area: "fx",
        layer: "fx",
        icon: "mask",
        mutates,
        undoable,
        params: {
            /// The layer the mask is on.
            layer: NodeRef,
            /// The mask.
            mask: NodeRef,
            /// Whether the mask is inverted.
            value: Option<Bool>,
        },
        run: {
            set_flag(session, "fx.mask.invert", self.layer.0, self.mask.0, "inverted", self.value)

        }
    }
    /// **Lock** a mask, or **unlock** it.
    ///
    /// The value says what to do; when it is not said the mask is
    /// **toggled**.
    fx.mask.lock {
        area: "fx",
        layer: "fx",
        icon: "lock",
        mutates,
        undoable,
        params: {
            /// The layer the mask is on.
            layer: NodeRef,
            /// The mask.
            mask: NodeRef,
            /// Whether the mask is locked.
            value: Option<Bool>,
        },
        run: {
            set_flag(session, "fx.mask.lock", self.layer.0, self.mask.0, "locked", self.value)

        }
    }
    /// **Read** the masks of a layer.
    fx.mask.list {
        area: "fx",
        layer: "fx",
        icon: "list",
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            let id = "fx.mask.list".to_string();
            let layer = self.layer.0;
            let node = the_layer(session, &id, layer)?;
            let masks: Vec<Json> = node
                .children
                .iter()
                .filter(|child| child.kind == "fx.mask")
                .map(|child| {
                    json!({
                        "id": child.id,
                        "name": child.name,
                        "vertices": child.data.get("vertices").cloned().unwrap_or(Json::Null),
                        "closed": child.data.get("closed").cloned().unwrap_or(Json::Bool(false)),
                        "mode": child.data.get("mode").cloned().unwrap_or(Json::Null),
                        "inverted": child.data.get("inverted").cloned().unwrap_or(Json::Bool(false)),
                        "locked": child.data.get("locked").cloned().unwrap_or(Json::Bool(false)),
                        "feather": child.data.get("feather").cloned().unwrap_or(Json::Null),
                        "opacity": child.data.get("opacity").cloned().unwrap_or(Json::Null),
                        "expansion": child.data.get("expansion").cloned().unwrap_or(Json::Null),
                    })
                })
                .collect();
            Ok(Outcome::new(format!("the layer `{layer}` has {} mask(s)", masks.len()))
                .with_data(json!({ "layer": layer, "masks": masks })))

        }
    }
}

/// **Set** or **unset** a flag of a mask.
fn set_flag(
    session: &mut Session,
    id: &str,
    layer: u64,
    mask: u64,
    key: &str,
    value: Option<Bool>,
) -> Result<Outcome, EngineError> {
    let node = the_mask(session, id, layer, mask)?;
    let was = node.data.get(key).and_then(|flag| flag.as_bool()).unwrap_or(false);
    let now = value.map(|value| value.0).unwrap_or(!was);
    let Some(node) = session.document_mut()?.node_mut(mask) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no mask `{mask}` on the layer `{layer}`"),
        });
    };
    node.data[key] = json!(now);
    Ok(Outcome::changed(format!("the mask `{mask}` is {} `{key}`", if now { "" } else { "not " }))
        .with_data(json!({ "layer": layer, "mask": mask, key: now })))
}
