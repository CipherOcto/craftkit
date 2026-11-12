//! The **blending mode** of a layer.
//!
//! A layer says how it is laid over what is under it: the **normal**
//! mode is the plain one, and the others mix the layer's colour with
//! the backdrop's by a rule.
//!
//! The tree's list is **long** — thirty-eight modes — and the tree's
//! finding is that the **name** of a mode is not the name it is
//! written with: a mode is found by its name with everything that is
//! not a letter or a digit taken out, so `Color Burn`, `ColorBurn` and
//! `color-burn` are all the same mode. The tree's modes come in
//! **groups**, and the last of each group is marked, so that a menu
//! may draw a line after it.
//!
//! And the tree's modes **step**: a step of one goes to the next mode
//! in the list and a step of minus one goes to the one before, and the
//! list wraps around.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The **blending modes**, in the tree's order.
///
/// The tree's own list, all thirty-eight of them, and the tree's own
/// names.
pub const MODES: [&str; 38] = [
    "Normal",
    "Dissolve",
    "Dancing Dissolve",
    "Darken",
    "Multiply",
    "Color Burn",
    "Classic Color Burn",
    "Linear Burn",
    "Darker Color",
    "Add",
    "Lighten",
    "Screen",
    "Color Dodge",
    "Classic Color Dodge",
    "Linear Dodge",
    "Lighter Color",
    "Overlay",
    "Soft Light",
    "Hard Light",
    "Linear Light",
    "Vivid Light",
    "Pin Light",
    "Hard Mix",
    "Difference",
    "Classic Difference",
    "Exclusion",
    "Subtract",
    "Divide",
    "Hue",
    "Saturation",
    "Color",
    "Luminosity",
    "Stencil Alpha",
    "Stencil Luma",
    "Silhouette Alpha",
    "Silhouette Luma",
    "Alpha Add",
    "Luminescent Premul",
];

/// The modes that **act on everything below** the layer and not on the
/// layer alone: the stencils and the silhouettes.
///
/// The tree's rule, and the tree's own list.
const STENCIL_MODES: [&str; 4] = ["Stencil Alpha", "Stencil Luma", "Silhouette Alpha", "Silhouette Luma"];

/// The **last** of each group of modes, so that a menu may draw a line
/// after it.
///
/// The tree's rule, and the tree's own list.
const GROUP_ENDS: [&str; 7] = [
    "Dancing Dissolve",
    "Darker Color",
    "Lighter Color",
    "Hard Mix",
    "Divide",
    "Luminosity",
    "Silhouette Luma",
];

/// The name of a mode with everything that is not a letter or a digit
/// taken out, in lower case.
///
/// The tree's rule: a mode is found by this, so `Color Burn`,
/// `ColorBurn` and `color burn` are one mode and not three.
fn key(name: &str) -> String {
    name.chars().filter(|character| character.is_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// The mode a name says, if it is one.
pub fn mode_of(name: &str) -> Option<&'static str> {
    let wanted = key(name);
    MODES.iter().find(|mode| key(mode) == wanted).copied()
}

/// The mode **after** this one, and the mode **before** it.
///
/// The tree's rule: the list wraps around, so the mode after the last
/// one is the first one and the mode before the first one is the last.
pub fn next_of(mode: &str) -> &'static str {
    match MODES.iter().position(|written| key(written) == key(mode)) {
        Some(index) => MODES[(index + 1) % MODES.len()],
        None => "Normal",
    }
}

/// The mode **before** this one.
pub fn previous_of(mode: &str) -> &'static str {
    match MODES.iter().position(|written| key(written) == key(mode)) {
        Some(index) => MODES[(index + MODES.len() - 1) % MODES.len()],
        None => "Normal",
    }
}

/// Whether a mode **acts on everything below** the layer.
pub fn is_stencil(mode: &str) -> bool {
    STENCIL_MODES.iter().any(|stencil| key(stencil) == key(mode))
}

/// Whether a mode is the **last of its group**.
pub fn ends_group(mode: &str) -> bool {
    GROUP_ENDS.iter().any(|end| key(end) == key(mode))
}

/// Set the **blending mode** of a layer.
///
/// The mode is named, or it is **stepped** by one — up or down — and
/// one of the two is needed: a command with neither does not know what
/// the mode should be.
///
/// A name that is not a mode is **refused**: the tree's command says
/// "need `mode`" and takes what it is given, and a mode that does not
/// exist cannot be laid over anything.
#[derive(Command)]
#[command(id = "fx.layer.blend", area = "fx", layer = "fx", icon = "paint", mutates, undoable)]
pub struct SetBlend {
    /// The layer.
    pub layer: NodeRef,
    /// The mode: `Normal`, `Multiply`, `Screen`, …
    pub mode: Option<Text>,
    /// The step: plus one for the next mode, minus one for the one
    /// before.
    pub step: Option<craftkit_sdk::Number>,
}

impl SetBlend {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.layer.blend".to_string();
        let layer = self.layer.0;
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        if node.kind != "fx.layer" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
            });
        }
        let current = mode_of(node.data.get("blend_mode").and_then(|mode| mode.as_str()).unwrap_or("Normal"))
            .unwrap_or("Normal");
        let wanted = match (&self.mode, self.step) {
            (Some(name), None) => {
                let name = name.0.clone();
                mode_of(&name).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no blending mode `{name}`"),
                })?
            }
            (None, Some(step)) => {
                let step = step.0;
                if !step.is_finite() || step == 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a step is a number of modes, and not zero".to_string(),
                    });
                }
                if step > 0.0 { next_of(current) } else { previous_of(current) }
            }
            (Some(_), Some(_)) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the mode and the step are two ways of saying one thing".to_string(),
                });
            }
            (None, None) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the mode, or the step, is needed".to_string(),
                });
            }
        };
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        node.data["blend_mode"] = serde_json::Value::String(wanted.to_string());
        Ok(Outcome::changed(format!("the layer `{layer}` is blended `{wanted}`")).with_data(
            serde_json::json!({
                "layer": layer,
                "blend_mode": wanted,
                "stencil": is_stencil(wanted),
                "group_end": ends_group(wanted),
            }),
        ))
    }
}
