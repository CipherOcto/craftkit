//! The **settings** of a layer.
//!
//! A layer is more than a name: it may be **on** or off, it may be
//! **frozen** or not, it may be **locked** or not, it may have a
//! **colour**, the colour it is **drawn in**, a **line type** and a
//! **line weight**.
//!
//! The tree's rules, and they are the finding:
//!
//! - a layer that is **off** is not drawn, and a layer that is
//!   **frozen** is not drawn either and may not be edited;
//! - a layer that is **locked** is drawn and may not be edited;
//! - a **colour** is an index above nothing or a name, and the two
//!   words `ByLayer` and `ByBlock` are refused: a layer's colour is
//!   its own and not the colour of whatever is on it;
//! - a **line weight** is a number of millimetres and not a number of
//!   anything else.
//!
//! # What it does not do
//!
//! - **The tree's current layer is not here**: an entity names the
//!   layer it is on, and there is no layer that everything goes on.
//! - **The tree's plot flag is not here**: what a drawing is plotted
//!   with is the host's business.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{
    Appearance, BlendMode, Bool, Colour, EngineError, Fill, Node, NodeRef, Number, Outcome, Session, Text,
};

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
    if found.kind != "cad.layer" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a layer", found.kind),
        });
    }
    Ok(found)
}

/// Whether a layer is **on**: a layer that says nothing is on.
fn is_on(node: &Node) -> bool {
    node.data.get("on").and_then(|on| on.as_bool()).unwrap_or(true)
}

/// Whether a layer is **frozen**.
fn is_frozen(node: &Node) -> bool {
    node.data.get("frozen").and_then(|frozen| frozen.as_bool()).unwrap_or(false)
}

/// Whether a layer is **locked**.
fn is_locked(node: &Node) -> bool {
    node.data.get("locked").and_then(|locked| locked.as_bool()).unwrap_or(false)
}

/// Whether a layer is **drawn**: a layer that is off or frozen is not.
pub(crate) fn is_drawn(node: &Node) -> bool {
    is_on(node) && !is_frozen(node)
}

/// **Set** a **flag** of a layer.
fn set_flag(
    session: &mut Session,
    id: &str,
    layer: u64,
    key: &str,
    on: bool,
) -> Result<Outcome, EngineError> {
    the_layer(session, id, layer)?;
    let Some(node) = session.document_mut()?.node_mut(layer) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no layer `{layer}`"),
        });
    };
    node.data[key] = json!(on);
    Ok(Outcome::changed(format!("the layer `{layer}` is {} `{key}`", if on { "" } else { "not " }))
        .with_data(json!({ "layer": layer, key: on })))
}

/// **Turn** a layer **on**, or **off**.
#[derive(Command)]
#[command(id = "cad.layer.on", area = "cad", layer = "cad", icon = "check", mutates, undoable)]
pub struct SetLayerOn {
    /// The layer.
    pub layer: NodeRef,
    /// Whether the layer is on.
    pub on: Bool,
}

impl SetLayerOn {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set_flag(session, "cad.layer.on", self.layer.0, "on", self.on.0)
    }
}

/// **Freeze** a layer, or **unfreeze** it.
#[derive(Command)]
#[command(id = "cad.layer.freeze", area = "cad", layer = "cad", icon = "snow", mutates, undoable)]
pub struct FreezeLayer {
    /// The layer.
    pub layer: NodeRef,
    /// Whether the layer is frozen.
    pub on: Bool,
}

impl FreezeLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set_flag(session, "cad.layer.freeze", self.layer.0, "frozen", self.on.0)
    }
}

/// **Lock** a layer, or **unlock** it.
#[derive(Command)]
#[command(id = "cad.layer.lock", area = "cad", layer = "cad", icon = "lock", mutates, undoable)]
pub struct LockLayer {
    /// The layer.
    pub layer: NodeRef,
    /// Whether the layer is locked.
    pub on: Bool,
}

impl LockLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set_flag(session, "cad.layer.lock", self.layer.0, "locked", self.on.0)
    }
}

/// **Set** the **colour** of a layer.
///
/// The colour is an **index** above nothing or a **name**, and the
/// two words `ByLayer` and `ByBlock` are refused: a layer's colour is
/// its own and not the colour of whatever is on it.
#[derive(Command)]
#[command(id = "cad.layer.color", area = "cad", layer = "cad", icon = "paint", mutates, undoable)]
pub struct SetLayerColor {
    /// The layer.
    pub layer: NodeRef,
    /// The colour: an index or a name.
    pub color: Text,
}

impl SetLayerColor {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.layer.color".to_string();
        let layer = self.layer.0;
        the_layer(session, &id, layer)?;
        let color = self.color.0.trim().to_string();
        if color.is_empty() {
            return Err(EngineError::Failed { id, message: "a layer has a colour".to_string() });
        }
        if color.eq_ignore_ascii_case("bylayer") || color.eq_ignore_ascii_case("byblock") {
            return Err(EngineError::Failed {
                id,
                message: format!("a layer's colour must be an index or a colour, and `{color}` is neither"),
            });
        }
        if color.parse::<u64>().is_ok_and(|index| index == 0) {
            return Err(EngineError::Failed { id, message: "a colour index is above nothing".to_string() });
        }
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: "cad.layer.color".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        node.data["color"] = json!(color);
        Ok(Outcome::changed(format!("the layer `{layer}` is `{color}`"))
            .with_data(json!({ "layer": layer, "color": color })))
    }
}

/// **Set** the colour a layer is **drawn in**, as `#rrggbb`.
#[derive(Command)]
#[command(id = "cad.layer.fill", area = "cad", layer = "cad", icon = "paint", mutates, undoable)]
pub struct SetLayerFill {
    /// The layer.
    pub layer: NodeRef,
    /// The colour it is drawn in, as `#rrggbb`.
    pub fill: Text,
}

impl SetLayerFill {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.layer.fill".to_string();
        let layer = self.layer.0;
        the_layer(session, &id, layer)?;
        let fill = self.fill.0.trim().to_string();
        let colour =
            Colour::from_hex(&fill).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let appearance = Appearance { fill: Fill::Solid(colour), stroke: None, blend: BlendMode::Normal };
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: "cad.layer.fill".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        node.appearance = Some(appearance);
        Ok(Outcome::changed(format!("the layer `{layer}` is drawn in `{fill}`"))
            .with_data(json!({ "layer": layer, "fill": fill })))
    }
}

/// **Set** the **line type** of a layer.
#[derive(Command)]
#[command(id = "cad.layer.linetype", area = "cad", layer = "cad", icon = "line", mutates, undoable)]
pub struct SetLayerLinetype {
    /// The layer.
    pub layer: NodeRef,
    /// The line type: `Continuous`, `Dashed`, `Center` and so on.
    pub linetype: Text,
}

impl SetLayerLinetype {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.layer.linetype".to_string();
        let layer = self.layer.0;
        the_layer(session, &id, layer)?;
        let linetype = self.linetype.0.trim().to_string();
        if linetype.is_empty() {
            return Err(EngineError::Failed { id, message: "a layer has a line type".to_string() });
        }
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: "cad.layer.linetype".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        node.data["linetype"] = json!(linetype);
        Ok(Outcome::changed(format!("the layer `{layer}` is `{linetype}`"))
            .with_data(json!({ "layer": layer, "linetype": linetype })))
    }
}

/// **Set** the **line weight** of a layer, in millimetres.
#[derive(Command)]
#[command(id = "cad.layer.lineweight", area = "cad", layer = "cad", icon = "weight", mutates, undoable)]
pub struct SetLayerLineweight {
    /// The layer.
    pub layer: NodeRef,
    /// The line weight, in millimetres.
    pub mm: Number,
}

impl SetLayerLineweight {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.layer.lineweight".to_string();
        let layer = self.layer.0;
        the_layer(session, &id, layer)?;
        if !self.mm.0.is_finite() || self.mm.0 <= 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a line weight is a number of millimetres".to_string(),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: "cad.layer.lineweight".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        node.data["lineweight_mm"] = json!(self.mm.0);
        Ok(Outcome::changed(format!("the layer `{layer}` weighs {} millimetres", self.mm.0))
            .with_data(json!({ "layer": layer, "mm": self.mm.0 })))
    }
}

/// **Read** the layers of a drawing.
#[derive(Command)]
#[command(id = "cad.layer.list", area = "cad", layer = "cad", icon = "list")]
pub struct ListLayers {
    /// The drawing.
    pub drawing: NodeRef,
}

impl ListLayers {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.layer.list".to_string();
        let drawing = self.drawing.0;
        let Some(node) = session.document().node(drawing) else {
            return Err(EngineError::Failed { id, message: format!("there is no drawing `{drawing}`") });
        };
        if node.kind != "cad.drawing" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{drawing}` is a `{}` and not a drawing", node.kind),
            });
        }
        let layers: Vec<craftkit_sdk::serde_json::Value> = node
            .children
            .iter()
            .filter(|child| child.kind == "cad.layer")
            .map(|child| {
                json!({
                    "id": child.id,
                    "name": child.name,
                    "color": child.data.get("color").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
                    "on": is_on(child),
                    "frozen": is_frozen(child),
                    "locked": is_locked(child),
                    "linetype": child.data.get("linetype").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
                    "lineweight_mm": child.data.get("lineweight_mm").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
                })
            })
            .collect();
        Ok(Outcome::new(format!("the drawing `{drawing}` has {} layer(s)", layers.len()))
            .with_data(json!({ "drawing": drawing, "layers": layers })))
    }
}
