//! The **keyframes** of a property.
//!
//! A property may be **animated**: its value is then a list of
//! keyframes, and the value of the property at a time is the
//! **interpolation** of the keyframes around it. So the value at a time
//! is **derived** — it changes when a keyframe moves, and nobody
//! writes it down.
//!
//! A keyframe is a **frame** and a **value**, and the keyframes of a
//! property are kept **sorted by frame**: the tree's `set_key` inserts
//! a key and keeps the list sorted, and so does this one.
//!
//! The **interpolation** is the tree's: a number is interpolated
//! between two numbers, a list of numbers between two lists of the same
//! length, and anything else is **held** — the value of the earlier
//! keyframe — because a value that does not interpolate does not
//! interpolate.
//!
//! # What it does not do
//!
//! - **The shape of the curve is not here.** The tree's keyframes have
//!   an interpolation on each side — linear, bezier or hold — and
//!   eases with a speed and an influence. Here the interpolation is
//!   linear, or a hold, and the shape is the tree's business.
//! - **The expression is not here.** The tree may put an expression on
//!   a property, and the expression is the plugin's own language.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::{get_at, parse_path, set_at};

/// The **keys** of a property: the keyframes, sorted by frame.
fn keys_of(value: &serde_json::Value) -> Option<Vec<(u64, serde_json::Value)>> {
    let list = value.get("keys")?.as_array()?;
    let mut keys: Vec<(u64, serde_json::Value)> = Vec::with_capacity(list.len());
    for key in list {
        let frame = key.get("frame")?.as_u64()?;
        let value = key.get("value")?.clone();
        keys.push((frame, value));
    }
    Some(keys)
}

/// A property's value with a **new list of keyframes**.
///
/// The rest of the value is kept: a property may carry more than its
/// keyframes, and taking it away would be taking away something the
/// caller did not ask about.
fn with_keys(value: &serde_json::Value, keys: &[(u64, serde_json::Value)]) -> serde_json::Value {
    let list: Vec<serde_json::Value> =
        keys.iter().map(|(frame, value)| serde_json::json!({ "frame": frame, "value": value })).collect();
    let mut new = value.clone();
    if !new.is_object() {
        new = serde_json::json!({});
    }
    new["keys"] = serde_json::Value::Array(list);
    new
}

/// **Set** a keyframe on a property.
///
/// The property must be **animated** — its value is a list of
/// keyframes — and a keyframe that is already there is **changed**,
/// and one that is not there is **added**, and the list stays sorted
/// by frame.
#[derive(Command)]
#[command(id = "fx.keyframe.set", area = "fx", layer = "fx", icon = "plus", mutates, undoable)]
pub struct SetKeyframe {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
    /// What the property is at that frame.
    pub value: serde_json::Value,
}

impl SetKeyframe {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.set".to_string();
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let segments = parse_path(&path);
        if segments.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a path has something in it".to_string(),
            });
        }
        let frame = self.frame.0;
        if !frame.is_finite() || frame < 0.0 || frame.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a frame is a whole number".to_string(),
            });
        }
        let frame = frame as u64;
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
        let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        let Some(current) = get_at(&props, &segments).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no `{path}` here"),
            });
        };
        let Some(mut keys) = keys_of(&current) else {
            return Err(EngineError::Failed { id: id.clone(), message: format!("`{path}` is not animated") });
        };
        // The keyframe that is there is changed, and the one that is
        // not there is added, and the list stays sorted.
        let value = self.value.clone();
        match keys.iter_mut().find(|(at, _)| *at == frame) {
            Some(key) => key.1 = value,
            None => {
                keys.push((frame, value));
                keys.sort_by_key(|(at, _)| *at);
            }
        }
        let new = with_keys(&current, &keys);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            node.data["props"] = props;
        }
        Ok(Outcome::changed(format!("`{path}` has a keyframe at frame {frame}"))
            .with_data(serde_json::json!({ "layer": layer, "path": path, "frame": frame })))
    }
}

/// **Take away** a keyframe of a property.
///
/// A frame that has **no keyframe** is refused: a keyframe that was
/// never there is not a keyframe that was taken away.
#[derive(Command)]
#[command(id = "fx.keyframe.remove", area = "fx", layer = "fx", icon = "minus", mutates, undoable)]
pub struct RemoveKeyframe {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
}

impl RemoveKeyframe {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.remove".to_string();
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let segments = parse_path(&path);
        if segments.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a path has something in it".to_string(),
            });
        }
        let frame = self.frame.0;
        if !frame.is_finite() || frame < 0.0 || frame.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a frame is a whole number".to_string(),
            });
        }
        let frame = frame as u64;
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
        let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        let Some(current) = get_at(&props, &segments).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no `{path}` here"),
            });
        };
        let Some(mut keys) = keys_of(&current) else {
            return Err(EngineError::Failed { id: id.clone(), message: format!("`{path}` is not animated") });
        };
        let before = keys.len();
        keys.retain(|(at, _)| *at != frame);
        if keys.len() == before {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{path}` has no keyframe at frame {frame}"),
            });
        }
        let new = with_keys(&current, &keys);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            node.data["props"] = props;
        }
        Ok(Outcome::changed(format!("`{path}` has no keyframe at frame {frame}"))
            .with_data(serde_json::json!({ "layer": layer, "path": path, "frame": frame })))
    }
}

/// The **value** of a property at a frame.
///
/// A property that is not animated has the same value at every frame,
/// and an animated one has the **interpolation** of its keyframes: the
/// first key's value before the first key, the last key's value after
/// the last key, and the line between them in between.
#[derive(Command)]
#[command(id = "fx.prop.at", area = "fx", layer = "fx", icon = "list")]
pub struct ValueAt {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame.
    pub frame: Number,
}

impl ValueAt {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.prop.at".to_string();
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let segments = parse_path(&path);
        if segments.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a path has something in it".to_string(),
            });
        }
        let frame = self.frame.0;
        if !frame.is_finite() || frame < 0.0 || frame.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a frame is a whole number".to_string(),
            });
        }
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
        let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        let Some(current) = get_at(&props, &segments).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no `{path}` here"),
            });
        };
        let value = match keys_of(&current) {
            Some(keys) => value_at(&keys, frame).unwrap_or(serde_json::Value::Null),
            None => current,
        };
        let message = format!("`{path}` is {value} at frame {frame}");
        Ok(Outcome::new(message)
            .with_data(serde_json::json!({ "layer": layer, "path": path, "frame": frame, "value": value })))
    }
}

/// The value of a **keyframed** property at a frame.
///
/// Before the first keyframe it is the first keyframe's value, after
/// the last one it is the last keyframe's value, and in between it is
/// the line between the two keyframes around it — when the values
/// interpolate, and the earlier value when they do not.
fn value_at(keys: &[(u64, serde_json::Value)], frame: f64) -> Option<serde_json::Value> {
    let first = keys.first()?;
    let last = keys.last()?;
    if keys.len() == 1 || frame <= first.0 as f64 {
        return Some(first.1.clone());
    }
    if frame >= last.0 as f64 {
        return Some(last.1.clone());
    }
    let index = keys.partition_point(|(at, _)| *at as f64 <= frame).saturating_sub(1);
    let (start, from) = keys.get(index)?;
    let (end, to) = keys.get(index + 1)?;
    let span = (end - start) as f64;
    if span <= 0.0 {
        return Some(from.clone());
    }
    let x = (frame - *start as f64) / span;
    Some(interpolate(from, to, x))
}

/// The line between two values.
///
/// Two numbers, and two lists of numbers of the same length, are
/// interpolated; anything else is **held**: the earlier value.
fn interpolate(a: &serde_json::Value, b: &serde_json::Value, x: f64) -> serde_json::Value {
    if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
        return serde_json::json!(a + (b - a) * x);
    }
    let (Some(list_a), Some(list_b)) = (a.as_array(), b.as_array()) else {
        return a.clone();
    };
    if list_a.len() != list_b.len() {
        return a.clone();
    }
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(list_a.len());
    for (from, to) in list_a.iter().zip(list_b.iter()) {
        let (Some(from), Some(to)) = (from.as_f64(), to.as_f64()) else {
            return a.clone();
        };
        out.push(serde_json::json!(from + (to - from) * x));
    }
    serde_json::Value::Array(out)
}
