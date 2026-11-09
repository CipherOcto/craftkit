//! The **edges** of a caption: trimmed, moved, split and merged.
//!
//! A cue of a caption track may be cut shorter at its **in** edge or at
//! its **out** edge, it may be **moved**, it may be **split** in two,
//! and two cues may be **merged** into one.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A cue keeps its **room**: a trimmed cue does not run into the cue
//!   before it or the cue after it, and it does not become shorter
//!   than a frame.
//! - A cue that is **moved** stays at or after the start of the
//!   sequence and does not run into its neighbours.
//! - A cue is **split** at a frame that leaves a frame on either side
//!   of it, and the two halves are the two cues.
//! - Cues are **merged** when they are on one track and they are
//!   **next to each other**: the merged cue runs from the start of the
//!   first to the end of the last, and the texts are joined with line
//!   breaks, the empty ones dropped.
//!
//! # What it does not do
//!
//! - **The tree clamps the delta** of a trim and of a move into the
//!   room there is. Here a delta that does not fit is **refused**.
//! - **The tree's cues are on a timeline of ticks** and its commands
//!   may be given the selection. Here the times are the **frames** of
//!   the sequence, and the cues are named.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The **shortest** a cue may be: a frame.
const MIN_DURATION: i64 = 1;

/// The **cue** a command is about, and the **track** it is on.
fn the_caption(session: &Session, command: &str, caption: u64) -> Result<(Node, u64), EngineError> {
    let Some(node) = session.document().node(caption).cloned() else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no cue `{caption}`"),
        });
    };
    if node.kind != "film.caption" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{caption}` is a `{}` and not a cue", node.kind),
        });
    }
    let track = session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == caption))
        .map(|node| node.id)
        .unwrap_or(caption);
    Ok((node, track))
}

/// The **cues** of a track, in the order they start in.
fn cues_of(node: &Node) -> Vec<(u64, i64, i64, String)> {
    let mut cues: Vec<(u64, i64, i64, String)> = Vec::new();
    for child in node.children.iter().filter(|child| child.kind == "film.caption") {
        let start = child.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = child.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        let text = child.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        cues.push((child.id, start, end, text));
    }
    cues.sort_by_key(|(_, start, _, _)| *start);
    cues
}

/// The **room** a cue has: the end of the cue before it and the start
/// of the cue after it.
fn room_of(cues: &[(u64, i64, i64, String)], caption: u64) -> (i64, i64) {
    let at = cues.iter().position(|cue| cue.0 == caption).unwrap_or(0);
    let before = if at == 0 { 0 } else { cues[at - 1].2 };
    let after = cues.get(at + 1).map(|cue| cue.1).unwrap_or(i64::MAX);
    (before, after)
}

/// A **delta** of whole frames.
fn delta_of(command: &str, delta: Number) -> Result<i64, EngineError> {
    let number = delta.0;
    if !number.is_finite() || number.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a delta is a whole number of frames: {number} is not one"),
        });
    }
    Ok(number as i64)
}

/// **Trim** a cue at its **in** edge or at its **out** edge.
///
/// The delta is in frames, and it may be negative: a negative delta
/// makes the cue longer. A delta that does not fit the room the cue
/// has is refused.
#[derive(Command)]
#[command(id = "film.caption.trim", area = "film", layer = "film", icon = "trim", mutates, undoable)]
pub struct TrimCaption {
    /// The cue.
    pub caption: NodeRef,
    /// The edge: `in` or `out`.
    pub edge: Text,
    /// The frames to take away, or to add.
    pub delta: Number,
}

impl TrimCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.trim";
        let caption = self.caption.0;
        let (node, track) = the_caption(session, command, caption)?;
        let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = node.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        let edge = self.edge.0.trim().to_ascii_lowercase();
        if edge != "in" && edge != "out" {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the edge is `in` or `out`".to_string(),
            });
        }
        let delta = delta_of(command, self.delta)?;
        if delta == 0 {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a trim of nothing is nothing".to_string(),
            });
        }
        let Some(track_node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        let cues = cues_of(track_node);
        let (before, after) = room_of(&cues, caption);
        // The **room** the trim has: the cue keeps a frame, and it does
        // not run into its neighbours.
        let (new_start, new_end) = if edge == "in" {
            let lowest = before - start;
            let highest = end - start - MIN_DURATION;
            if delta < lowest || delta > highest {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "the in edge of the cue may move between {lowest} and {highest} frames, and {delta} is not one of them"
                    ),
                });
            }
            (start + delta, end)
        } else {
            let lowest = MIN_DURATION - (end - start);
            let highest = if after == i64::MAX { i64::MAX - end } else { after - end };
            if delta < lowest || delta > highest {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "the out edge of the cue may move between {lowest} and {highest} frames, and {delta} is not one of them"
                    ),
                });
            }
            (start, end + delta)
        };
        if let Some(node) = session.document_mut()?.node_mut(caption) {
            node.data["start"] = json!(new_start);
            node.data["end"] = json!(new_end);
            node.name = format!("{new_start}–{new_end}");
        }
        Ok(Outcome::changed(format!("the cue `{caption}` is from {new_start} to {new_end}")).with_data(
            json!({
                "caption": caption,
                "start": new_start,
                "end": new_end,
            }),
        ))
    }
}

/// **Move** a cue: it stays at or after the start of the sequence and
/// it does not run into its neighbours.
#[derive(Command)]
#[command(id = "film.caption.move", area = "film", layer = "film", icon = "arrow", mutates, undoable)]
pub struct MoveCaption {
    /// The cue.
    pub caption: NodeRef,
    /// The frames to move it by.
    pub delta: Number,
}

impl MoveCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.move";
        let caption = self.caption.0;
        let (node, track) = the_caption(session, command, caption)?;
        let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = node.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        let delta = delta_of(command, self.delta)?;
        if delta == 0 {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a move of nothing is nothing".to_string(),
            });
        }
        let Some(track_node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        let cues = cues_of(track_node);
        let (before, after) = room_of(&cues, caption);
        let lowest = before - start;
        let highest = if after == i64::MAX { i64::MAX - end } else { after - end };
        if delta < lowest || delta > highest {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!(
                    "the cue may move between {lowest} and {highest} frames, and {delta} is not one of them"
                ),
            });
        }
        let (new_start, new_end) = (start + delta, end + delta);
        if let Some(node) = session.document_mut()?.node_mut(caption) {
            node.data["start"] = json!(new_start);
            node.data["end"] = json!(new_end);
            node.name = format!("{new_start}–{new_end}");
        }
        Ok(Outcome::changed(format!("the cue `{caption}` is moved to {new_start}")).with_data(json!({
            "caption": caption,
            "start": new_start,
            "end": new_end,
        })))
    }
}

/// **Split** a cue in two at a frame.
///
/// The frame leaves a frame on either side of it, and the two halves
/// are the two cues.
#[derive(Command)]
#[command(id = "film.caption.split", area = "film", layer = "film", icon = "split", mutates, undoable)]
pub struct SplitCaption {
    /// The cue.
    pub caption: NodeRef,
    /// The frame to split it at.
    pub at: Number,
}

impl SplitCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.split";
        let caption = self.caption.0;
        let (node, track) = the_caption(session, command, caption)?;
        let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = node.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        let at = delta_of(command, self.at)?;
        if at - start < MIN_DURATION || end - at < MIN_DURATION {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the cue may be split at a frame that leaves a frame on either side of it"
                    .to_string(),
            });
        }
        // The **second half** is a new cue of its own.
        let id = session.document_mut()?.add_node_with_under(
            Some(track),
            format!("{at}–{end}"),
            "film.caption",
            None,
            json!({ "start": at, "end": end, "text": text }),
        );
        if let Some(node) = session.document_mut()?.node_mut(caption) {
            node.data["end"] = json!(at);
            node.name = format!("{start}–{at}");
        }
        Ok(Outcome::changed(format!("the cue `{caption}` is split at {at}")).with_data(json!({
            "caption": caption,
            "split": id,
            "at": at,
        })))
    }
}

/// **Merge** the cues that are next to each other into one.
///
/// The merged cue runs from the start of the first to the end of the
/// last, and the texts are joined with line breaks.
#[derive(Command)]
#[command(id = "film.caption.merge", area = "film", layer = "film", icon = "merge", mutates, undoable)]
pub struct MergeCaptions {
    /// The cues, in the order they stand in.
    pub captions: Option<Json>,
}

impl MergeCaptions {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.merge";
        let given = match self.captions.clone() {
            Some(Json::Array(items)) => {
                let mut ids = Vec::new();
                for item in items {
                    let Some(id) = item.as_u64() else {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: "the cues are a list of ids".to_string(),
                        });
                    };
                    ids.push(id);
                }
                ids
            }
            Some(_) => {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "the cues are a list of ids".to_string(),
                });
            }
            None => Vec::new(),
        };
        if given.len() < 2 {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "two cues are merged, and there is less".to_string(),
            });
        }
        // The **track**: every cue is on one track.
        let mut track = None;
        for cue in &given {
            let (_, of) = the_caption(session, command, *cue)?;
            match track {
                None => track = Some(of),
                Some(track) if track != of => {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: "the cues to merge must be on one track".to_string(),
                    });
                }
                Some(_) => {}
            }
        }
        let track = track.unwrap_or(0);
        let Some(track_node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        let cues = cues_of(track_node);
        // The **cues** must be next to each other.
        let mut at: Vec<usize> =
            given.iter().filter_map(|id| cues.iter().position(|cue| cue.0 == *id)).collect();
        at.sort_unstable();
        at.dedup();
        if at.len() < 2 || at.windows(2).any(|pair| pair[1] != pair[0] + 1) {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the cues to merge must be next to each other".to_string(),
            });
        }
        let first = at[0];
        let last = at[at.len() - 1];
        let start = cues[first].1;
        let end = cues[last].2;
        let text: Vec<String> =
            at.iter().map(|at| cues[*at].3.trim().to_string()).filter(|text| !text.is_empty()).collect();
        let keep = cues[first].0;
        if let Some(node) = session.document_mut()?.node_mut(keep) {
            node.data["start"] = json!(start);
            node.data["end"] = json!(end);
            node.data["text"] = json!(text.join("\n"));
            node.name = format!("{start}–{end}");
        }
        for cue in at.iter().skip(1) {
            session.document_mut()?.remove_node(cues[*cue].0);
        }
        Ok(Outcome::changed(format!("the cues are merged into `{keep}`")).with_data(json!({
            "caption": keep,
            "start": start,
            "end": end,
        })))
    }
}
