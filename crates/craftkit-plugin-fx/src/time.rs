//! The **time** of a composition.
//!
//! The time is the frame the composition is looked at — the tree's
//! **time cursor**, and the tree measures it in **seconds**. Here it
//! is a **frame**, because a composition here is a number of frames
//! long and a time that is not a frame is not a place in it.
//!
//! The time is a **fact of the composition** — its payload carries the
//! frame — so that it is saved with the composition and read back
//! with it.
//!
//! The tree's rules, and they are the finding:
//!
//! - the time is **never before the start** of the composition;
//! - the tree's **end** is the end of the composition, and so the
//!   time may be set beyond the last frame and is not refused for it;
//! - a **step** of frames that would go before the start is a step
//!   that stops at the start.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The most frames a time may be at.
const MAX_FRAME: u64 = 1_000_000_000;

/// The **composition** a node is, if it is one.
fn the_comp<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no composition `{node}`"),
        });
    };
    if found.kind != "fx.comp" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a composition", found.kind),
        });
    }
    Ok(found)
}

/// The **time** of a composition: the frame it is at.
fn time_of(node: &craftkit_sdk::Node) -> u64 {
    node.data.get("time").and_then(|frame| frame.as_u64()).unwrap_or(0)
}

/// The **length** of a composition in frames.
fn length_of(node: &craftkit_sdk::Node) -> u64 {
    node.data.get("duration").and_then(|frames| frames.as_u64()).unwrap_or(0)
}

/// **Set** the time of a composition.
#[derive(Command)]
#[command(id = "fx.time.set", area = "fx", layer = "fx", icon = "clock", mutates, undoable)]
pub struct SetTime {
    /// The composition.
    pub comp: NodeRef,
    /// The frame.
    pub frame: Number,
}

impl SetTime {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.time.set".to_string();
        let comp = self.comp.0;
        the_comp(session, &id, comp)?;
        if !self.frame.0.is_finite() || self.frame.0.fract() != 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a number of frames is a whole number".to_string(),
            });
        }
        let frame = if self.frame.0 < 0.0 { 0 } else { self.frame.0 as u64 };
        if frame > MAX_FRAME {
            return Err(EngineError::Failed {
                id,
                message: format!("a frame is at most {MAX_FRAME}, and `{frame}` is not"),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(comp) else {
            return Err(EngineError::Failed {
                id: "fx.time.set".to_string(),
                message: format!("there is no composition `{comp}`"),
            });
        };
        node.data["time"] = json!(frame);
        Ok(Outcome::changed(format!("the time is at frame {frame}"))
            .with_data(json!({ "comp": comp, "frame": frame })))
    }
}

/// **Read** the time of a composition.
#[derive(Command)]
#[command(id = "fx.time.get", area = "fx", layer = "fx", icon = "list")]
pub struct GetTime {
    /// The composition.
    pub comp: NodeRef,
}

impl GetTime {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.time.get".to_string();
        let comp = self.comp.0;
        let node = the_comp(session, &id, comp)?;
        let frame = time_of(node);
        let frames = length_of(node);
        Ok(Outcome::new(format!("the time is at frame {frame}")).with_data(json!({
            "comp": comp,
            "frame": frame,
            "frames": frames,
        })))
    }
}

/// **Go to** the start of a composition.
#[derive(Command)]
#[command(id = "fx.time.start", area = "fx", layer = "fx", icon = "rewind", mutates, undoable)]
pub struct GoToStart {
    /// The composition.
    pub comp: NodeRef,
}

impl GoToStart {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        go_to(session, "fx.time.start", self.comp.0, 0)
    }
}

/// **Go to** the end of a composition.
#[derive(Command)]
#[command(id = "fx.time.end", area = "fx", layer = "fx", icon = "forward", mutates, undoable)]
pub struct GoToEnd {
    /// The composition.
    pub comp: NodeRef,
}

impl GoToEnd {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.time.end".to_string();
        let comp = self.comp.0;
        let node = the_comp(session, &id, comp)?;
        let frame = length_of(node);
        go_to(session, &id, comp, frame)
    }
}

/// The time goes to a **frame**.
fn go_to(session: &mut Session, id: &str, comp: u64, frame: u64) -> Result<Outcome, EngineError> {
    let Some(node) = session.document_mut()?.node_mut(comp) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no composition `{comp}`"),
        });
    };
    node.data["time"] = json!(frame);
    Ok(Outcome::changed(format!("the time is at frame {frame}"))
        .with_data(json!({ "comp": comp, "frame": frame })))
}

/// **Step** the time of a composition.
#[derive(Command)]
#[command(id = "fx.time.step", area = "fx", layer = "fx", icon = "step", mutates, undoable)]
pub struct StepTime {
    /// The composition.
    pub comp: NodeRef,
    /// The frames: a negative number steps back.
    pub frames: Number,
}

impl StepTime {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.time.step".to_string();
        let comp = self.comp.0;
        let node = the_comp(session, &id, comp)?;
        let here = time_of(node);
        if !self.frames.0.is_finite() || self.frames.0.fract() != 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a number of frames is a whole number".to_string(),
            });
        }
        let steps = self.frames.0 as i64;
        let target = match steps.unsigned_abs() {
            0 => here,
            magnitude => match (steps > 0, here.checked_add(magnitude)) {
                (true, Some(frame)) => frame.min(MAX_FRAME),
                (true, None) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "the target frame exceeds the supported time range".to_string(),
                    });
                }
                (false, _) => here.saturating_sub(magnitude),
            },
        };
        go_to(session, &id, comp, target)
    }
}
