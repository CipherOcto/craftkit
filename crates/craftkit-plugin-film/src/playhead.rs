//! The **playhead** of a sequence.
//!
//! The playhead is where the sequence is looked at: a **frame** in it.
//!
//! The tree keeps the playhead in its **session**, one for each
//! sequence, and the tree's playhead is measured in **ticks**. Here
//! the playhead is a **fact of the sequence**: the sequence's payload
//! carries the frame the playhead is at, so that it is saved with the
//! sequence and not with the window that happens to be open.
//!
//! The tree's rules, and they are the finding:
//!
//! - a playhead is **snapped to a frame** and never before the start
//!   of the sequence;
//! - the tree's **end** of a sequence is the end of its last clip, and
//!   so the playhead may be set beyond the last frame of a sequence
//!   and is not refused for it;
//! - a **step** of frames that would go before the start is a step
//!   that stops at the start.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The most frames a playhead may be at.
const MAX_FRAME: u64 = 1_000_000_000;

/// The **sequence** a node is, if it is one.
fn the_sequence<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no sequence `{node}`"),
        });
    };
    if found.kind != "film.item" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}`, and a sequence is an item", found.kind),
        });
    }
    Ok(found)
}

/// The **playhead** of a sequence: the frame it is at.
pub(crate) fn playhead_of(node: &craftkit_sdk::Node) -> u64 {
    node.data.get("playhead").and_then(|frame| frame.as_u64()).unwrap_or(0)
}

/// The **length** of a sequence in frames: the end of the longest of
/// its tracks.
fn length_in_frames(sequence: &craftkit_sdk::Node) -> u64 {
    sequence
        .children
        .iter()
        .filter(|child| child.kind == "film.track")
        .map(|track| {
            track
                .children
                .iter()
                .filter(|child| child.kind == "film.clip")
                .filter_map(|child| {
                    let start = child.data.get("start").and_then(|start| start.as_u64())?;
                    let duration = child.data.get("duration").and_then(|duration| duration.as_u64())?;
                    Some(start + duration)
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0)
}

commands! {
    /// **Set** the playhead of a sequence.
    film.playhead.set {
        area: "film",
        layer: "film",
        icon: "playhead",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// The frame.
            frame: Number,
        },
        run: {
            let id = "film.playhead.set".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            // The tree's playhead is never before the start of the
            // sequence, and so a frame below nothing is nothing.
            let frame = if !self.frame.0.is_finite() || self.frame.0.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "a number of frames is a whole number".to_string(),
                });
            } else if self.frame.0 < 0.0 {
                0
            } else {
                self.frame.0 as u64
            };
            if frame > MAX_FRAME {
                return Err(EngineError::Failed {
                    id,
                    message: format!("a frame is at most {MAX_FRAME}, and `{frame}` is not"),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed {
                    id: "film.playhead.set".to_string(),
                    message: format!("there is no sequence `{sequence}`"),
                });
            };
            node.data["playhead"] = json!(frame);
            Ok(Outcome::changed(format!("the playhead is at frame {frame}"))
                .with_data(json!({ "sequence": sequence, "frame": frame })))

        }
    }

    /// **Read** the playhead of a sequence.
    film.playhead.get {
        area: "film",
        layer: "film",
        icon: "list",
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            let id = "film.playhead.get".to_string();
            let sequence = self.sequence.0;
            let node = the_sequence(session, &id, sequence)?;
            let frame = playhead_of(node);
            let frames = length_in_frames(node);
            Ok(Outcome::new(format!("the playhead is at frame {frame}")).with_data(json!({
                "sequence": sequence,
                "frame": frame,
                "frames": frames,
            })))

        }
    }

    /// **Go to** the start of a sequence.
    film.playhead.start {
        area: "film",
        layer: "film",
        icon: "rewind",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            let id = "film.playhead.start".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed {
                    id: "film.playhead.start".to_string(),
                    message: format!("there is no sequence `{sequence}`"),
                });
            };
            node.data["playhead"] = json!(0);
            Ok(Outcome::changed("the playhead is at the start").with_data(json!({ "sequence": sequence })))

        }
    }

    /// **Go to** the end of a sequence.
    film.playhead.end {
        area: "film",
        layer: "film",
        icon: "forward",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            let id = "film.playhead.end".to_string();
            let sequence = self.sequence.0;
            let node = the_sequence(session, &id, sequence)?;
            let frames = length_in_frames(node);
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed {
                    id: "film.playhead.end".to_string(),
                    message: format!("there is no sequence `{sequence}`"),
                });
            };
            node.data["playhead"] = json!(frames);
            Ok(Outcome::changed(format!("the playhead is at frame {frames}"))
                .with_data(json!({ "sequence": sequence, "frame": frames })))

        }
    }

    /// **Go to** the **mark** of a sequence: the mark in or the mark out.
    ///
    /// The tree's own rules — the tree's `goToIn` and its `goToOut`:
    ///
    /// - the playhead goes to the **mark in**, and to the **start** of the
    ///   sequence when there is no mark in;
    /// - the playhead goes to the **mark out**, and to the **end** of the
    ///   sequence when there is no mark out.
    film.playhead.goto.mark {
        area: "film",
        layer: "film",
        icon: "playhead",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// Which mark the playhead goes to: `in` or `out`.
            which: Text,
        },
        run: {
            let id = "film.playhead.goto.mark".to_string();
            let sequence = self.sequence.0;
            let node = the_sequence(session, &id, sequence)?;
            let which = self.which.0.trim().to_ascii_lowercase();
            if !matches!(which.as_str(), "in" | "out") {
                return Err(EngineError::Failed {
                    id,
                    message: format!("`{}` is not a mark to go to: it is `in` or `out`", self.which.0),
                });
            }
            let (mark_in, mark_out) = crate::sequencemarks::marks_of(node);
            // The tree's own rule: a mark that is not there sends the
            // playhead to the start or to the end of the sequence.
            let frames = length_in_frames(node);
            let target = if which == "in" { mark_in.unwrap_or(0) } else { mark_out.unwrap_or(frames) };
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed {
                    id: "film.playhead.goto.mark".to_string(),
                    message: format!("there is no sequence `{sequence}`"),
                });
            };
            node.data["playhead"] = json!(target);
            Ok(Outcome::changed(format!("the playhead is at frame {target}"))
                .with_data(json!({ "sequence": sequence, "frame": target, "which": which })))

        }
    }

    /// **Step** the playhead of a sequence.
    film.playhead.step {
        area: "film",
        layer: "film",
        icon: "step",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// The frames: a negative number steps back.
            frames: Number,
        },
        run: {
            let id = "film.playhead.step".to_string();
            let sequence = self.sequence.0;
            let node = the_sequence(session, &id, sequence)?;
            let here = playhead_of(node);
            let steps = self.frames.0;
            if !steps.is_finite() || steps.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "a number of frames is a whole number".to_string(),
                });
            }
            let steps = steps as i64;
            let target = match steps.unsigned_abs() {
                0 => here,
                magnitude => match (steps > 0, here.checked_add(magnitude)) {
                    (true, Some(frame)) => frame,
                    (true, None) => {
                        return Err(EngineError::Failed {
                            id,
                            message: "the target frame exceeds the supported time range".to_string(),
                        });
                    }
                    (false, _) => here.saturating_sub(magnitude),
                },
            };
            let target = target.min(MAX_FRAME);
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed {
                    id: "film.playhead.step".to_string(),
                    message: format!("there is no sequence `{sequence}`"),
                });
            };
            node.data["playhead"] = json!(target);
            Ok(Outcome::changed(format!("the playhead is at frame {target}"))
                .with_data(json!({ "sequence": sequence, "frame": target })))

        }
    }
}
