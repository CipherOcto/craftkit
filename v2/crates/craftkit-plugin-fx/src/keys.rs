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
//! # The shape of the curve
//!
//! The tree's keyframe has an interpolation on the side it **leaves**
//! on, and there are **three** of them: `linear`, `bezier` and `hold`.
//!
//! - a **linear** keyframe moves the value in a straight line;
//! - a **hold** keyframe does not move the value at all until the next
//!   keyframe;
//! - a **bezier** keyframe moves the value along a curve, and the
//!   shape of the curve is the **ease**: a **speed** and an
//!   **influence**.
//!
//! The ease is the tree's, and it is the finding: the tree does not
//! store a curve, it stores **how fast the value is moving** and **how
//! much of the segment the handle takes**. The speed here is in value
//! units per **frame** — the tree's is in units per second, and a
//! frame is the unit of this tree.
//!
//! The tree's keyframe has an interpolation on **each** side — in and
//! out — and the segment between two keyframes is shaped by the out
//! side of the first and the in side of the second. So a keyframe here
//! has an interpolation on each side as well, and the two are set by
//! the same command with the **side** it names: `out` for the side the
//! keyframe leaves on and `in` for the side it is entered on.
//!
//! The tree's rule for the pair: the side the keyframe **leaves** on
//! decides the shape of the segment — a hold out side holds the value
//! — and the side it is **entered** on shapes the handle that comes in.
//! An in side that is not a bezier is a linear side, so a hold in side
//! shapes the segment it ends like a line.
//!
//! And a keyframe may be an **auto** bezier: its ease is not written
//! down but comes from its **neighbours** — the slope from the
//! keyframe before to the keyframe after, and an influence of a sixth.
//! So an auto keyframe follows the keys around it, and a keyframe with
//! no neighbour on one side is not an auto one.
//!
//! And a keyframe may be **continuous**: its two sides are locked to
//! each other, so the outgoing side takes the ease of the incoming one
//! and the curve is smooth at the keyframe. The **flag** does not
//! change the curve — the curve follows the eases that were written —
//! so the flag is a memory of the writing.
//!
//! # What it does not do
//!
//! - **The spatial handles are not here.** The tree's spatial
//!   properties move along a path with tangents of their own, and the
//!   path is sampled through an arc-length table; here a property is a
//!   number or a list of numbers and it moves in the value.
//! - **The expression is not here.** The tree may put an expression on
//!   a property, and the expression is the plugin's own language.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::{get_at, parse_path, set_at};

/// The **side** of a keyframe: the side it leaves on, or the side it
/// is entered on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// The side the keyframe leaves on.
    Out,
    /// The side the keyframe is entered on.
    In,
}

/// The **interpolation** of a keyframe on the side it leaves on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interp {
    /// The value moves in a straight line.
    Linear,
    /// The value moves along a **Bezier** curve, and the shape of it
    /// is the ease.
    Bezier,
    /// The value does not move at all until the next keyframe.
    Hold,
}

impl Interp {
    /// The interpolation a name says, if it says one this tree knows.
    pub fn parse(name: &str) -> Option<Interp> {
        match name.trim().to_lowercase().as_str() {
            "linear" => Some(Interp::Linear),
            "bezier" => Some(Interp::Bezier),
            "hold" => Some(Interp::Hold),
            _ => None,
        }
    }

    /// The name of the interpolation.
    pub fn name(self) -> &'static str {
        match self {
            Interp::Linear => "linear",
            Interp::Bezier => "bezier",
            Interp::Hold => "hold",
        }
    }
}

/// The **ease** of a keyframe: a speed and an influence.
///
/// The tree's `Ease`: the speed is in value units per second and the
/// influence is the fraction of the segment the handle takes. Here the
/// speed is in value units per **frame**, because a frame is the unit
/// of this tree and there is no clock in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ease {
    /// The speed, in value units per frame.
    pub speed: f64,
    /// The influence: the fraction of the segment the handle takes, from
    /// nothing at all to the whole of it.
    pub influence: f64,
}

impl Default for Ease {
    /// The tree's own default: no speed, and a sixth of the segment.
    fn default() -> Ease {
        Ease { speed: 0.0, influence: 1.0 / 6.0 }
    }
}

/// A **keyframe**: a frame, a value, and how it leaves.
#[derive(Debug, Clone)]
struct Key {
    /// The frame the keyframe is on.
    frame: u64,
    /// The value of the property at that frame.
    value: serde_json::Value,
    /// The interpolation of the side the keyframe leaves on.
    interp: Interp,
    /// The ease of a bezier.
    ease: Ease,
    /// The interpolation of the side the keyframe is **entered** on.
    in_interp: Interp,
    /// The ease of the side the keyframe is entered on.
    in_ease: Ease,
    /// Whether the keyframe is an **auto** bezier: its ease is not
    /// written down but comes from its neighbours.
    auto: bool,
    /// Whether the keyframe is **continuous**: its two sides are
    /// locked to each other.
    continuous: bool,
}

impl Key {
    /// The keyframe a piece of the payload holds.
    fn from_json(key: &serde_json::Value) -> Option<Key> {
        let frame = key.get("frame")?.as_u64()?;
        let value = key.get("value")?.clone();
        let interp = match key.get("interp").and_then(|interp| interp.as_str()) {
            Some(name) => Interp::parse(name)?,
            None => Interp::Linear,
        };
        let ease = match key.get("ease") {
            Some(ease) => Ease {
                speed: ease.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(0.0),
                influence: ease
                    .get("influence")
                    .and_then(|influence| influence.as_f64())
                    .unwrap_or(Ease::default().influence),
            },
            None => Ease::default(),
        };
        let in_interp = match key.get("in_interp").and_then(|interp| interp.as_str()) {
            Some(name) => Interp::parse(name)?,
            None => Interp::Linear,
        };
        let in_ease = match key.get("in_ease") {
            Some(ease) => Ease {
                speed: ease.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(0.0),
                influence: ease
                    .get("influence")
                    .and_then(|influence| influence.as_f64())
                    .unwrap_or(Ease::default().influence),
            },
            None => Ease::default(),
        };
        let auto = key.get("auto").and_then(|auto| auto.as_bool()).unwrap_or(false);
        let continuous = key.get("continuous").and_then(|continuous| continuous.as_bool()).unwrap_or(false);
        Some(Key { frame, value, interp, ease, in_interp, in_ease, auto, continuous })
    }

    /// The keyframe, as the payload holds it.
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "frame": self.frame,
            "value": self.value,
            "interp": self.interp.name(),
            "ease": { "speed": self.ease.speed, "influence": self.ease.influence },
            "in_interp": self.in_interp.name(),
            "in_ease": { "speed": self.in_ease.speed, "influence": self.in_ease.influence },
            "auto": self.auto,
            "continuous": self.continuous,
        })
    }
}

/// The **keys** of a property: the keyframes, sorted by frame.
fn keys_of(value: &serde_json::Value) -> Option<Vec<Key>> {
    let list = value.get("keys")?.as_array()?;
    let mut keys: Vec<Key> = Vec::with_capacity(list.len());
    for key in list {
        keys.push(Key::from_json(key)?);
    }
    Some(keys)
}

/// A property's value with a **new list of keyframes**.
///
/// The rest of the value is kept: a property may carry more than its
/// keyframes, and taking it away would be taking away something the
/// caller did not ask about.
fn with_keys(value: &serde_json::Value, keys: &[Key]) -> serde_json::Value {
    let list: Vec<serde_json::Value> = keys.iter().map(|key| key.to_json()).collect();
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
///
/// The **interpolation** of a keyframe is kept when its value is
/// changed, as the tree's `set_key` keeps it.
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
        match keys.iter_mut().find(|key| key.frame == frame) {
            Some(key) => key.value = value,
            None => {
                keys.push(Key {
                    frame,
                    value,
                    interp: Interp::Linear,
                    ease: Ease::default(),
                    in_interp: Interp::Linear,
                    in_ease: Ease::default(),
                    auto: false,
                    continuous: false,
                });
                keys.sort_by_key(|key| key.frame);
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
        keys.retain(|key| key.frame != frame);
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

/// **Lock** the two sides of a keyframe together: a **continuous**
/// keyframe.
///
/// The tree's rule: the outgoing side follows the incoming one — the
/// side that is not named takes the ease of the side that is — and
/// the keyframe is marked continuous, and the auto flag is cleared.
///
/// And the tree's finding: the **flag** does not change the curve. The
/// curve follows the eases that were written; the flag is a memory of
/// the writing. So a continuous keyframe is smooth because its two
/// eases are the same, and not because it is marked.
#[derive(Command)]
#[command(id = "fx.keyframe.continuous", area = "fx", layer = "fx", icon = "pencil", mutates, undoable)]
pub struct SetContinuous {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
    /// Whether the keyframe is continuous. The default is that it is.
    pub continuous: Option<Bool>,
    /// The speed of both sides, in value units per frame. The default
    /// is the speed the keyframe has on the side it is entered on.
    pub speed: Option<Number>,
    /// The influence of both sides: the fraction of the segment each
    /// handle takes.
    pub influence: Option<Number>,
}

impl SetContinuous {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.continuous".to_string();
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
        let continuous = self.continuous.map(|continuous| continuous.0).unwrap_or(true);
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
        let Some(key) = keys.iter_mut().find(|key| key.frame == frame) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{path}` has no keyframe at frame {frame}"),
            });
        };
        if !continuous {
            // The **flag** is cleared and nothing else changes: the
            // eases stay as they are, as they do in the tree.
            key.continuous = false;
            let data = with_keys(&current, &keys);
            if let Some(node) = session.document_mut()?.node_mut(layer) {
                let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
                set_at(&mut props, &segments, data)
                    .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
                node.data["props"] = props;
            }
            return Ok(Outcome::changed(format!("the keyframe at frame {frame} is not continuous"))
                .with_data(serde_json::json!({
                    "layer": layer,
                    "path": path,
                    "frame": frame,
                    "continuous": false,
                })));
        }
        // The **ease**: the one that is given, or the one the keyframe
        // has on the side it is entered on — the tree's rule that the
        // side that is not named takes the ease of the side that is.
        let mut ease = key.in_ease;
        if let Some(speed) = &self.speed {
            if !speed.0.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a speed is a number that is there".to_string(),
                });
            }
            ease.speed = speed.0;
        }
        if let Some(influence) = &self.influence {
            if !influence.0.is_finite() || !(0.0..=1.0).contains(&influence.0) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "an influence is a fraction of the segment, from 0 to 1, and `{}` is not one",
                        influence.0
                    ),
                });
            }
            ease.influence = influence.0;
        }
        // Both sides are **beziers** with the same ease, so the
        // handles are mirrors of each other and the curve is smooth
        // at the keyframe.
        key.interp = Interp::Bezier;
        key.in_interp = Interp::Bezier;
        key.ease = ease;
        key.in_ease = ease;
        // A **manual** ease is not an auto one.
        key.auto = false;
        key.continuous = true;
        let data = with_keys(&current, &keys);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            set_at(&mut props, &segments, data)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            node.data["props"] = props;
        }
        Ok(Outcome::changed(format!("the keyframe at frame {frame} is continuous")).with_data(
            serde_json::json!({
                "layer": layer,
                "path": path,
                "frame": frame,
                "continuous": true,
                "ease": { "speed": ease.speed, "influence": ease.influence },
            }),
        ))
    }
}

/// **Set** or **clear** the **auto bezier** of a keyframe.
///
/// An auto keyframe's ease is not written down: it is the **slope** to
/// the keyframes on the other side of it, and its influence is a
/// sixth. So an auto keyframe follows its neighbours: move one and the
/// curve moves with it.
///
/// A keyframe with no neighbour on one side is not an auto one, and
/// the ease that is written down is used instead.
#[derive(Command)]
#[command(id = "fx.keyframe.auto", area = "fx", layer = "fx", icon = "pencil", mutates, undoable)]
pub struct SetAuto {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
    /// Whether the keyframe is an auto bezier.
    pub auto: Bool,
}

impl SetAuto {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.auto".to_string();
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
        let Some(key) = keys.iter_mut().find(|key| key.frame == frame) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{path}` has no keyframe at frame {frame}"),
            });
        };
        key.auto = self.auto.0;
        let new = with_keys(&current, &keys);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            node.data["props"] = props;
        }
        let state = if self.auto.0 { "an auto bezier" } else { "not an auto bezier" };
        Ok(Outcome::changed(format!("the keyframe at frame {frame} is {state}")).with_data(
            serde_json::json!({ "layer": layer, "path": path, "frame": frame, "auto": self.auto.0 }),
        ))
    }
}

/// The **interpolation** of a keyframe.
///
/// The tree's keyframe has an interpolation on the side it leaves on,
/// and it is one of three: `linear`, `bezier` and `hold`. A **bezier**
/// has an **ease** — a speed and an influence — and the ease is what
/// shapes the curve.
///
/// A keyframe that is **not there** is refused, and so is an
/// interpolation that is not one of the three, and an influence that
/// is not a fraction of the segment.
#[derive(Command)]
#[command(id = "fx.keyframe.interp", area = "fx", layer = "fx", icon = "pencil", mutates, undoable)]
pub struct SetInterpolation {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// The frame the keyframe is on.
    pub frame: Number,
    /// `linear`, `bezier` or `hold`.
    pub interp: Text,
    /// The speed of a bezier, in value units per frame.
    pub speed: Option<Number>,
    /// The influence of a bezier: the fraction of the segment its
    /// handle takes.
    pub influence: Option<Number>,
    /// Which **side** of the keyframe: `out` for the side it leaves
    /// on and `in` for the side it is entered on. The default is
    /// `out`.
    pub side: Option<Text>,
}

impl SetInterpolation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.keyframe.interp".to_string();
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
        let interp = Interp::parse(&self.interp.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!(
                "`{}` is not an interpolation: a keyframe is `linear`, `bezier` or `hold`",
                self.interp.0.trim()
            ),
        })?;
        let mut ease = Ease::default();
        if let Some(speed) = &self.speed {
            if !speed.0.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a speed is a number that is there".to_string(),
                });
            }
            ease.speed = speed.0;
        }
        if let Some(influence) = &self.influence {
            if !influence.0.is_finite() || !(0.0..=1.0).contains(&influence.0) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "an influence is a fraction of the segment, from 0 to 1, and `{}` is not one",
                        influence.0
                    ),
                });
            }
            ease.influence = influence.0;
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
        let Some(mut keys) = keys_of(&current) else {
            return Err(EngineError::Failed { id: id.clone(), message: format!("`{path}` is not animated") });
        };
        let Some(key) = keys.iter_mut().find(|key| key.frame == frame) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{path}` has no keyframe at frame {frame}"),
            });
        };
        // The **side**: the side the keyframe leaves on, or the side
        // it is entered on.
        let side = match &self.side {
            Some(side) => {
                let name = side.0.trim();
                match name.to_ascii_lowercase().as_str() {
                    "out" => Side::Out,
                    "in" => Side::In,
                    _ => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("`{name}` is not a side of a keyframe: a side is `out` or `in`"),
                        });
                    }
                }
            }
            None => Side::Out,
        };
        match side {
            Side::Out => {
                key.interp = interp;
                key.ease = ease;
            }
            Side::In => {
                key.in_interp = interp;
                key.in_ease = ease;
            }
        }
        // A **written** ease is a manual one, and the tree's manual
        // ease is not an auto one: the keyframe follows its neighbours
        // again only when nobody has said otherwise.
        if self.speed.is_some() || self.influence.is_some() {
            key.auto = false;
        }
        let new = with_keys(&current, &keys);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            node.data["props"] = props;
        }
        let (verb, side_name) = match side {
            Side::Out => ("leaves", "out"),
            Side::In => ("is entered", "in"),
        };
        Ok(Outcome::changed(format!("`{path}` {verb} frame {frame} on a {}", interp.name())).with_data(
            serde_json::json!({
                "layer": layer,
                "path": path,
                "frame": frame,
                "side": side_name,
                "interp": interp.name(),
                "ease": { "speed": ease.speed, "influence": ease.influence },
            }),
        ))
    }
}

/// The **value** of a property at a frame.
///
/// A property that is not animated has the same value at every frame,
/// and an animated one has the **interpolation** of its keyframes: the
/// first key's value before the first key, the last key's value after
/// the last key, and the curve between them in between.
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
/// the curve between the two keyframes around it — the line, the hold
/// or the bezier, as the keyframe it leaves says — and the earlier
/// value when the values do not interpolate.
fn value_at(keys: &[Key], frame: f64) -> Option<serde_json::Value> {
    let first = keys.first()?;
    let last = keys.last()?;
    if keys.len() == 1 || frame <= first.frame as f64 {
        return Some(first.value.clone());
    }
    if frame >= last.frame as f64 {
        return Some(last.value.clone());
    }
    let index = keys.partition_point(|key| key.frame as f64 <= frame).saturating_sub(1);
    let start = keys.get(index)?;
    let end = keys.get(index + 1)?;
    let span = (end.frame - start.frame) as f64;
    if span <= 0.0 {
        return Some(start.value.clone());
    }
    let x = (frame - start.frame as f64) / span;
    // The tree's rule: the side the keyframe **leaves** on decides the
    // shape of the segment, and the side it is **entered** on shapes
    // the handle that comes in. An in side that is not a bezier is a
    // linear side — a hold in side shapes the segment it ends like a
    // linear one.
    let lin_out = start.interp == Interp::Linear;
    let lin_in = end.in_interp != Interp::Bezier;
    Some(match start.interp {
        // The tree's hold: the value does not move until the next
        // keyframe.
        Interp::Hold => start.value.clone(),
        Interp::Linear | Interp::Bezier => {
            if lin_out && lin_in {
                interpolate(&start.value, &end.value, x)
            } else {
                // The tree's **auto** bezier: the ease of an auto
                // keyframe is not written down, it is the slope to its
                // neighbours — and a keyframe without a neighbour on
                // each side is not an auto one.
                let out = if lin_out {
                    Handle::Linear
                } else if start.auto {
                    match (index.checked_sub(1).and_then(|at| keys.get(at)), Some(end)) {
                        (Some(previous), Some(next)) => {
                            Handle::Auto { previous: previous.clone(), next: next.clone() }
                        }
                        _ => Handle::Written(start.ease),
                    }
                } else {
                    Handle::Written(start.ease)
                };
                let in_handle = if lin_in {
                    Handle::Linear
                } else if end.auto {
                    match (keys.get(index), keys.get(index + 2)) {
                        (Some(previous), Some(next)) => {
                            Handle::Auto { previous: previous.clone(), next: next.clone() }
                        }
                        _ => Handle::Written(end.in_ease),
                    }
                } else {
                    Handle::Written(end.in_ease)
                };
                eased(&start.value, &end.value, x, span, &out, &in_handle)
            }
        }
    })
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

/// The value between two keyframes on a **Bezier** curve.
///
/// The tree's shape of it: the curve is a cubic Bezier in the (time,
/// value) plane, and the two handles are the eases of the two
/// keyframes — the **speed** says how fast the value is moving and the
/// **influence** says how much of the segment the handle takes.
///
/// A side that is **not** a bezier is a linear side, and a linear side
/// is a third of the segment: the handle is a third of the way to the
/// other value, and the curve is the straight line.
fn eased(
    a: &serde_json::Value,
    b: &serde_json::Value,
    x: f64,
    span: f64,
    out: &Handle,
    in_side: &Handle,
) -> serde_json::Value {
    if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
        let difference = b - a;
        let (out_inf, out_offset) = handle(out, 0, difference, span);
        let (into, in_offset) = handle(in_side, 0, difference, span);
        let u = solve_u(0.0, out_inf, 1.0 - into, 1.0, x);
        let value = bez(a, a + out_offset, b - in_offset, b, u);
        return serde_json::json!(value);
    }
    let (Some(list_a), Some(list_b)) = (a.as_array(), b.as_array()) else {
        return a.clone();
    };
    if list_a.len() != list_b.len() {
        return a.clone();
    }
    let mut out_values: Vec<serde_json::Value> = Vec::with_capacity(list_a.len());
    for (index, (from, to)) in list_a.iter().zip(list_b.iter()).enumerate() {
        let (Some(from), Some(to)) = (from.as_f64(), to.as_f64()) else {
            return a.clone();
        };
        let difference = to - from;
        let (out_inf, out_offset) = handle(out, index, difference, span);
        let (into, in_offset) = handle(in_side, index, difference, span);
        let u = solve_u(0.0, out_inf, 1.0 - into, 1.0, x);
        let value = bez(from, from + out_offset, to - in_offset, to, u);
        out_values.push(serde_json::json!(value));
    }
    serde_json::Value::Array(out_values)
}

/// The **handle** of one side of a segment.
#[derive(Debug, Clone)]
enum Handle {
    /// A **linear** side: it takes a third of the segment, as the
    /// tree's linear side does.
    Linear,
    /// A **bezier** side with an ease that is written down.
    Written(Ease),
    /// An **auto** bezier side: the ease is the slope to the
    /// keyframes on the other side of the keyframe, and the influence
    /// is a sixth.
    Auto {
        /// The keyframe before, and its value.
        previous: Key,
        /// The keyframe after, and its value.
        next: Key,
    },
}

/// The **influence** of a side and how far its handle is from the value
/// it is on.
///
/// A side with no ease is a **linear** side: it takes a third of the
/// segment, as the tree's linear side does. An **auto** side is the
/// slope to the keyframes on the other side of its keyframe, and it
/// takes a sixth.
fn handle(handle: &Handle, index: usize, difference: f64, span: f64) -> (f64, f64) {
    match handle {
        Handle::Linear => (1.0 / 3.0, difference / 3.0),
        Handle::Written(ease) => {
            let influence = ease.influence.clamp(0.0, 1.0);
            (influence, ease.speed * influence * span)
        }
        Handle::Auto { previous, next } => {
            let influence = 1.0 / 6.0;
            let span_of_neighbours = next.frame as f64 - previous.frame as f64;
            if span_of_neighbours <= 0.0 {
                return (influence, 0.0);
            }
            let (Some(from), Some(to)) = (component(&previous.value, index), component(&next.value, index))
            else {
                return (influence, 0.0);
            };
            let slope = (to - from) / span_of_neighbours;
            (influence, slope * influence * span)
        }
    }
}

/// The **component** of a value: the number itself, or the number at
/// that place in a list.
fn component(value: &serde_json::Value, index: usize) -> Option<f64> {
    match value {
        serde_json::Value::Array(list) => list.get(index).and_then(|value| value.as_f64()),
        _ => value.as_f64(),
    }
}

/// A cubic Bezier in one dimension.
fn bez(p0: f64, p1: f64, p2: f64, p3: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    v * v * v * p0 + 3.0 * v * v * u * p1 + 3.0 * v * u * u * p2 + u * u * u * p3
}

/// The slope of a cubic Bezier in one dimension.
fn bez_d(p0: f64, p1: f64, p2: f64, p3: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    3.0 * v * v * (p1 - p0) + 6.0 * v * u * (p2 - p1) + 3.0 * u * u * (p3 - p2)
}

/// The parameter at which a cubic Bezier reaches `x`.
///
/// The tree's `solve_u`: a Newton step and then a bisection, so the
/// answer is found and not merely approached.
fn solve_u(x0: f64, x1: f64, x2: f64, x3: f64, x: f64) -> f64 {
    if x <= x0 {
        return 0.0;
    }
    if x >= x3 {
        return 1.0;
    }
    let mut u = (x - x0) / (x3 - x0);
    for _ in 0..8 {
        let f = bez(x0, x1, x2, x3, u) - x;
        let d = bez_d(x0, x1, x2, x3, u);
        if d.abs() < 1e-12 {
            break;
        }
        if (f / d).abs() < 1e-12 {
            return u;
        }
        let next = u - f / d;
        if !(0.0..=1.0).contains(&next) {
            break;
        }
        u = next;
    }
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..60 {
        let middle = (low + high) * 0.5;
        if bez(x0, x1, x2, x3, middle) < x {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) * 0.5
}
