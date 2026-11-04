//! The **ease** of a keyframe: the easy ease, and the hold.
//!
//! An ease is a **speed** and an **influence**, and the influence is
//! the fraction of the segment the handle takes. Two eases are worth
//! naming, because they are the ones a person asks for by name:
//!
//! - the **easy ease**: the influence is a **third** of the segment
//!   and the speed is nothing, and the keyframe stops being an auto
//!   bezier — the ease is written down instead of coming from the
//!   neighbours;
//! - the **hold**: the value does not move at all until the next
//!   keyframe, and turning it off gives back the bezier the keyframe
//!   had, or a straight line if it had none.
//!
//! # What the port found
//!
//! - **The tree's easy ease is a number, not a calculation**: the
//!   influence is a third and the speed is nothing. Here the same —
//!   and the tree's survey said it was worked out from the
//!   neighbours, and it is not: it is a third, and a third is a
//!   third.
//! - **The tree's hold is turned off again by a rule**: the keyframe
//!   gets its **bezier** back when it eases — when it is an auto
//!   bezier, when it is continuous, when its in side is a bezier, or
//!   when it has an ease of its own — and a **straight line** when it
//!   does not. Here the same, and "it has an ease of its own" means
//!   "its ease is not the default one": the tree's ease is a list and
//!   an empty list is how it says "no ease", and here the ease is
//!   always there and the default one is how it says the same thing.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::keys::{Ease, Interp, Key, keys_of, with_keys};
use crate::{get_at, parse_path, set_at};

/// The **influence** of an easy ease: a third of the segment.
const EASY_INFLUENCE: f64 = 1.0 / 3.0;

/// Which **side** of a keyframe a command is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// Both sides.
    Both,
    /// The side the keyframe is entered on.
    In,
    /// The side the keyframe leaves on.
    Out,
}

impl Side {
    /// The side a name says, if it says one this tree knows.
    fn parse(name: &str) -> Option<Side> {
        match name.trim().to_lowercase().as_str() {
            "both" => Some(Side::Both),
            "in" => Some(Side::In),
            "out" => Some(Side::Out),
            _ => None,
        }
    }
}

/// The **keys** of a property, and the property itself.
///
/// The property is the value at the path, and the keys are the
/// keyframes it holds. A property that is not animated has no keys,
/// and a property that is not there is refused.
fn keys_at(session: &Session, id: &str, layer: u64, path: &str) -> Result<(Json, Vec<Key>), EngineError> {
    let segments = parse_path(path);
    if segments.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a path has something in it".to_string(),
        });
    }
    let Some(node) = session.document().node(layer) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no layer `{layer}`"),
        });
    };
    if node.kind != "fx.layer" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
        });
    }
    let props = node.data.get("props").cloned().unwrap_or(json!({}));
    let Some(current) = get_at(&props, &segments).cloned() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no `{path}` here"),
        });
    };
    let Some(keys) = keys_of(&current) else {
        return Err(EngineError::Failed { id: id.to_string(), message: format!("`{path}` is not animated") });
    };
    Ok((current, keys))
}

/// Put a property's **keys** back into a layer.
fn write_keys(
    session: &mut Session,
    id: &str,
    layer: u64,
    path: &str,
    current: &Json,
    keys: &[Key],
) -> Result<(), EngineError> {
    let segments = parse_path(path);
    let new = with_keys(current, keys);
    if let Some(node) = session.document_mut()?.node_mut(layer) {
        let mut props = node.data.get("props").cloned().unwrap_or(json!({}));
        set_at(&mut props, &segments, new)
            .map_err(|message| EngineError::Failed { id: id.to_string(), message })?;
        node.data["props"] = props;
    }
    Ok(())
}

/// The **frame** a command is about.
fn frame_of(id: &str, frame: Number) -> Result<u64, EngineError> {
    if !frame.0.is_finite() || frame.0 < 0.0 || frame.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a frame is a whole number".to_string(),
        });
    }
    Ok(frame.0 as u64)
}

/// Give a keyframe the **easy ease**.
#[derive(Command)]
#[command(id = "fx.keyframe.easy", area = "fx", layer = "fx", icon = "pencil", mutates, undoable)]
pub struct EasyEase {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
    /// Which **side** of the keyframe: `both`, `in` or `out`. The
    /// default is `both`.
    pub side: Option<Text>,
}

impl EasyEase {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.easy".to_string();
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let frame = frame_of(&id, self.frame)?;
        let side = match &self.side {
            Some(side) => Side::parse(&side.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a side: a side is `both`, `in` or `out`", side.0),
            })?,
            None => Side::Both,
        };
        let (current, mut keys) = keys_at(session, &id, layer, &path)?;
        let Some(index) = keys.iter().position(|key| key.frame == frame) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{path}` has no keyframe at frame {frame}"),
            });
        };
        let easy = Ease { speed: 0.0, influence: EASY_INFLUENCE };
        let key = &mut keys[index];
        match side {
            Side::Both => {
                key.interp = Interp::Bezier;
                key.ease = easy;
                key.in_interp = Interp::Bezier;
                key.in_ease = easy;
            }
            Side::In => {
                key.in_interp = Interp::Bezier;
                key.in_ease = easy;
            }
            Side::Out => {
                key.interp = Interp::Bezier;
                key.ease = easy;
            }
        }
        // The ease is written down now, so the keyframe is not an auto
        // bezier any more: an auto bezier takes its ease from its
        // neighbours.
        key.auto = false;
        write_keys(session, &id, layer, &path, &current, &keys)?;
        Ok(Outcome::changed(format!("the keyframe at frame {frame} has an easy ease"))
            .with_data(json!({ "layer": layer, "path": path, "frame": frame })))
    }
}

/// Turn a keyframe's **hold** on, or off again.
#[derive(Command)]
#[command(id = "fx.keyframe.hold", area = "fx", layer = "fx", icon = "pencil", mutates, undoable)]
pub struct ToggleHold {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
}

impl ToggleHold {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.hold".to_string();
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let frame = frame_of(&id, self.frame)?;
        let (current, mut keys) = keys_at(session, &id, layer, &path)?;
        let Some(index) = keys.iter().position(|key| key.frame == frame) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{path}` has no keyframe at frame {frame}"),
            });
        };
        // A keyframe that **eases** gets its bezier back; one that does
        // not gets a straight line.
        let default = Ease::default();
        let key = &mut keys[index];
        let eases = key.auto
            || key.continuous
            || key.in_interp == Interp::Bezier
            || key.ease.speed != default.speed
            || key.ease.influence != default.influence;
        let holding = key.interp == Interp::Hold;
        key.interp = match key.interp {
            Interp::Hold if eases => Interp::Bezier,
            Interp::Hold => Interp::Linear,
            _ => Interp::Hold,
        };
        let interp = key.interp;
        let state = if holding { "does not hold" } else { "holds" };
        write_keys(session, &id, layer, &path, &current, &keys)?;
        Ok(Outcome::changed(format!("the keyframe at frame {frame} {state} the value"))
            .with_data(json!({ "layer": layer, "path": path, "frame": frame, "interp": interp.name() })))
    }
}
