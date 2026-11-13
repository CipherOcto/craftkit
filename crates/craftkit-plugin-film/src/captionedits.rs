//! The **text** and the **times** of a caption.
//!
//! A cue of a caption track may be **edited**: its text may be
//! changed, and the two times it is on the screen may be moved.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - The **text** of a cue is a text, and a line break is a line
//!   break: the tree takes the carriage return out of it.
//! - The **speaker** of a cue is a name, and an empty name takes it
//!   away.
//! - A command that changes neither the text nor the speaker is
//!   refused: it would do nothing.
//! - The **times** of a cue are whole frames, and the cue ends after
//!   it starts.
//! - The cues of a track never **overlap**, and a cue that is moved
//!   over another one is refused. A cue that ends exactly when another
//!   one starts is not an overlap.
//!
//! # What it does not do
//!
//! - **The tree's times are ticks** and the tree's command may be
//!   given a timecode, a number of frames or a number of seconds.
//!   Here the times are the **frames** of the sequence, as they are
//!   everywhere in this battery.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The most characters the **text** of a cue may have.
const MAX_TEXT: usize = 4096;

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

/// The **cues** of a track: the start, the end and the id of each.
fn cues_of(node: &Node) -> Vec<(u64, i64, i64)> {
    let mut cues: Vec<(u64, i64, i64)> = Vec::new();
    for child in node.children.iter().filter(|child| child.kind == "film.caption") {
        let start = child.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = child.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        cues.push((child.id, start, end));
    }
    cues.sort_by_key(|(_, start, _)| *start);
    cues
}

/// A **time**: whole frames, and not before the start.
fn frames_of(command: &str, value: Number) -> Result<i64, EngineError> {
    let number = value.0;
    if !number.is_finite() || number.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a time is a whole number of frames: {number} is not one"),
        });
    }
    if number < 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a time is not before the start: {number} is"),
        });
    }
    Ok(number as i64)
}

commands! {
    /// **Set** the text of a cue, and the **speaker** of it.
    ///
    /// A command that changes neither is refused.
    film.caption.set {
        area: "film",
        layer: "film",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The cue.
            caption: NodeRef,
            /// Its new text.
            text: Option<Text>,
            /// Its new speaker. An empty speaker takes it away.
            speaker: Option<Text>,
        },
        run: {
            let command = "film.caption.set";
            let caption = self.caption.0;
            let (node, _track) = the_caption(session, command, caption)?;
            if self.text.is_none() && self.speaker.is_none() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a cue has a text or a speaker".to_string(),
                });
            }
            // The **text**: the line breaks of the tree are line breaks.
            let text = self.text.as_ref().map(|text| text.0.replace("\r\n", "\n"));
            if let Some(text) = text.as_ref()
                && text.chars().count() > MAX_TEXT
            {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the text of a cue is not longer than {MAX_TEXT} characters"),
                });
            }
            // The **speaker**: a name, and an empty name takes it away.
            let speaker = self.speaker.as_ref().map(|speaker| {
                let speaker = speaker.0.trim();
                if speaker.is_empty() { None } else { Some(speaker.to_string()) }
            });
            if let Some(node) = session.document_mut()?.node_mut(caption) {
                if let Some(text) = text.clone() {
                    node.data["text"] = json!(text);
                }
                if let Some(speaker) = speaker.clone() {
                    node.data["speaker"] = match speaker {
                        Some(speaker) => json!(speaker),
                        None => Json::Null,
                    };
                }
            }
            let _ = node;
            Ok(Outcome::changed(format!("the cue `{caption}` is set")).with_data(json!({
                "caption": caption,
                "text": text,
                "speaker": speaker,
            })))

        }
    }

    /// **Set** the times of a cue.
    ///
    /// The times that are not given keep the ones the cue has, and a cue
    /// that would overlap another one is refused.
    film.caption.times {
        area: "film",
        layer: "film",
        icon: "clock",
        mutates,
        undoable,
        params: {
            /// The cue.
            caption: NodeRef,
            /// The frame it starts at.
            start: Option<Number>,
            /// The frame it ends at.
            end: Option<Number>,
        },
        run: {
            let command = "film.caption.times";
            let caption = self.caption.0;
            let (node, track) = the_caption(session, command, caption)?;
            let held_start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            let held_end = node.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
            // The **times**: the ones that are given, and the ones that
            // are not keep what they were.
            let start = match self.start {
                Some(start) => frames_of(command, start)?,
                None => held_start,
            };
            let end = match self.end {
                Some(end) => frames_of(command, end)?,
                None => held_end,
            };
            if end <= start {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a cue ends after it starts".to_string(),
                });
            }
            // The **overlap**: the tree's own rule, and the cue itself is
            // not an overlap.
            let Some(track_node) = session.document().node(track) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no track `{track}`"),
                });
            };
            for (id, other_start, other_end) in cues_of(track_node) {
                if id == caption {
                    continue;
                }
                if start < other_end && other_start < end {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: format!("the cue from {start} to {end} overlaps a cue of the track"),
                    });
                }
            }
            if let Some(node) = session.document_mut()?.node_mut(caption) {
                node.data["start"] = json!(start);
                node.data["end"] = json!(end);
                node.name = format!("{start}–{end}");
            }
            Ok(Outcome::changed(format!("the cue `{caption}` is from {start} to {end}")).with_data(json!({
                "caption": caption,
                "start": start,
                "end": end,
            })))

        }
    }
}
