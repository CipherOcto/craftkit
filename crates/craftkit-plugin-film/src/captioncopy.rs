//! The **copy** of a cue, and the **end** of a track.
//!
//! A cue may be **duplicated** — the copy stands later on the same
//! track — and a caption track may be **deleted**, and its cues go
//! with it.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A **copy** of a cue stands by the delta that is given, and it is
//!   never **before the start of the sequence**: a copy that would be
//!   is refused.
//! - A cue that is copied twice is copied twice, and each copy is a
//!   cue of its own.
//! - A **track** that is deleted takes its cues with it, and a track
//!   that is not there is refused by name.
//!
//! # What it does not do
//!
//! - **The tree clamps the delta** of a copy so that the copy is not
//!   before the start of the sequence. Here such a copy is **refused**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

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

/// The **track** a command is about.
fn the_track<'a>(session: &'a Session, command: &str, track: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(track) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no caption track `{track}`"),
        });
    };
    if node.kind != "film.caption.track" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{track}` is a `{}` and not a caption track", node.kind),
        });
    }
    Ok(node)
}

commands! {
    /// **Duplicate** a cue: the copy stands by the delta that is given.
    ///
    /// The copy is never before the start of the sequence, and a copy that
    /// would be is refused.
    film.caption.duplicate {
        area: "film",
        layer: "film",
        icon: "copy",
        mutates,
        undoable,
        params: {
            /// The cue.

            caption: NodeRef,
            /// The frames to stand it by.

            delta: Number,
        },
        run: {

            let command = "film.caption.duplicate";
            let caption = self.caption.0;
            let (node, track) = the_caption(session, command, caption)?;
            let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            let end = node.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
            let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
            let speaker =
                node.data.get("speaker").and_then(|speaker| speaker.as_str()).map(|speaker| speaker.to_string());
            let delta = self.delta.0;
            if !delta.is_finite() || delta.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("a delta is a whole number of frames: {delta} is not one"),
                });
            }
            let delta = delta as i64;
            if delta == 0 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a copy of nothing is nothing".to_string(),
                });
            }
            if start + delta < 0 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "the copy may not stand before the start of the sequence: {delta} is too far"
                    ),
                });
            }
            let (new_start, new_end) = (start + delta, end + delta);
            // The **overlap**: the cues of a track never overlap, and the
            // tree writes its copies over what is there. Here a copy that
            // runs into another cue is refused.
            if let Some(track_node) = session.document().node(track) {
                for child in track_node.children.iter().filter(|child| child.kind == "film.caption") {
                    if child.id == caption {
                        continue;
                    }
                    let other_start = child.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
                    let other_end = child.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
                    if new_start < other_end && other_start < new_end {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!(
                                "the copy from {new_start} to {new_end} overlaps a cue of the track"
                            ),
                        });
                    }
                }
            }
            let mut data = json!({ "start": new_start, "end": new_end, "text": text });
            if let Some(speaker) = speaker {
                data["speaker"] = json!(speaker);
            }
            let id = session.document_mut()?.add_node_with_under(
                Some(track),
                format!("{new_start}–{new_end}"),
                "film.caption",
                None,
                data,
            );
            Ok(Outcome::changed(format!("the cue `{caption}` is copied into `{id}`")).with_data(json!({
                "caption": caption,
                "copy": id,
                "start": new_start,
                "end": new_end,
            })))

        }
    }
    /// **Delete** a caption track, and its cues go with it.
    film.caption.track.delete {
        area: "film",
        layer: "film",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The caption track.

            track: NodeRef,
        },
        run: {

            let command = "film.caption.track.delete";
            let track = self.track.0;
            let node = the_track(session, command, track)?;
            let cues = node.children.iter().filter(|child| child.kind == "film.caption").count();
            session.document_mut()?.remove_node(track);
            Ok(Outcome::changed(format!("the caption track `{track}` is gone")).with_data(json!({
                "track": track,
                "cues": cues,
            })))

        }
    }
}
