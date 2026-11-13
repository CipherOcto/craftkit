//! The **marks** of a sequence: the tree's own **program in** and
//! **program out**.
//!
//! The tree's own: a sequence carries a **mark in** and a **mark out**,
//! and they are the range of the sequence the editor is working in —
//! the tree's own `mark_in` and `mark_out` on the sequence itself, and
//! not the marks of a media.
//!
//! # The rules
//!
//! - **The mark in is the first frame of the range and the mark out is
//!   the last one**: the tree's own range is the mark in and the mark
//!   out **plus one frame**.
//! - **Setting the mark in clears the mark out** when the mark out is
//!   before it, and **setting the mark out clears the mark in** when
//!   the mark in is after it: the tree's own rule — a range that
//!   begins after it ends is not a range, and the tree takes the end
//!   away rather than refusing the beginning.
//! - **A frame is a whole number**, and it is not before the beginning
//!   of the timeline: the tree's own rule.
//! - The marks are **the sequence's own**, and they are kept under keys
//!   of their own: a sequence is an item in this tree, and the marks of
//!   a **media** are the item's own facts.
//!
//! # What it does not settle
//!
//! - **The tree's split points are not here.** The tree takes the split
//!   of the video and the split of the audio away when a mark is set,
//!   and this tree has no split points.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The most frames a mark may be at.
const MAX_FRAME: u64 = 1_000_000_000;

/// The key the **mark in** of a sequence is kept under.
const MARK_IN: &str = "sequence_mark_in";

/// The key the **mark out** of a sequence is kept under.
const MARK_OUT: &str = "sequence_mark_out";

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

/// A **frame** a caller has named: a whole number, and not before the
/// beginning of the timeline.
fn frame_of(command: &str, value: f64) -> Result<u64, EngineError> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "a number of frames is a whole number".to_string(),
        });
    }
    if value < 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the timeline begins at frame 0".to_string(),
        });
    }
    let frame = value as u64;
    if frame > MAX_FRAME {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a frame is at most {MAX_FRAME}, and `{frame}` is not"),
        });
    }
    Ok(frame)
}

/// The **marks** a sequence carries.
pub(crate) fn marks_of(node: &craftkit_sdk::Node) -> (Option<u64>, Option<u64>) {
    let mark_in = node.data.get(MARK_IN).and_then(|frame| frame.as_u64());
    let mark_out = node.data.get(MARK_OUT).and_then(|frame| frame.as_u64());
    (mark_in, mark_out)
}

commands! {
    /// **Set** the **mark in** of a sequence.
    ///
    /// The tree's own rule: the mark in is the frame the caller says, and
    /// a mark out that is **before** it is **taken away** — the tree's own
    /// `mark`.
    film.sequence.mark.in {
        area: "film",
        layer: "film",
        icon: "flag",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// The frame the mark in is at.
            frame: Number,
        },
        run: {
            let id = "film.sequence.mark.in".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            let frame = frame_of(&id, self.frame.0)?;
            let (_mark_in, mark_out) = marks_of(session.document().node(sequence).ok_or_else(|| {
                EngineError::Failed { id: id.clone(), message: format!("there is no sequence `{sequence}`") }
            })?);
            // The tree's own rule: a mark out **before** the new mark in
            // is taken away.
            let mark_out = match mark_out {
                Some(mark_out) if mark_out < frame => None,
                mark_out => mark_out,
            };
            if let Some(node) = session.document_mut()?.node_mut(sequence) {
                node.data[MARK_IN] = json!(frame);
                if mark_out.is_none() {
                    if let craftkit_sdk::serde_json::Value::Object(map) = &mut node.data {
                        map.remove(MARK_OUT);
                    }
                } else {
                    node.data[MARK_OUT] = json!(mark_out);
                }
            }
            Ok(Outcome::changed(format!("the mark in of the sequence `{sequence}` is frame {frame}")).with_data(
                json!({
                    "sequence": sequence,
                    "mark_in": frame,
                    "mark_out": mark_out,
                }),
            ))

        }
    }

    /// **Set** the **mark out** of a sequence.
    ///
    /// The tree's own rule: the mark out is the frame the caller says, and
    /// a mark in that is **after** it is **taken away** — the tree's own
    /// `mark`.
    film.sequence.mark.out {
        area: "film",
        layer: "film",
        icon: "flag",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// The frame the mark out is at.
            frame: Number,
        },
        run: {
            let id = "film.sequence.mark.out".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            let frame = frame_of(&id, self.frame.0)?;
            let (mark_in, _mark_out) = marks_of(session.document().node(sequence).ok_or_else(|| {
                EngineError::Failed { id: id.clone(), message: format!("there is no sequence `{sequence}`") }
            })?);
            // The tree's own rule: a mark in **after** the new mark out is
            // taken away.
            let mark_in = match mark_in {
                Some(mark_in) if mark_in > frame => None,
                mark_in => mark_in,
            };
            if let Some(node) = session.document_mut()?.node_mut(sequence) {
                node.data[MARK_OUT] = json!(frame);
                if mark_in.is_none() {
                    if let craftkit_sdk::serde_json::Value::Object(map) = &mut node.data {
                        map.remove(MARK_IN);
                    }
                } else {
                    node.data[MARK_IN] = json!(mark_in);
                }
            }
            Ok(Outcome::changed(format!("the mark out of the sequence `{sequence}` is frame {frame}")).with_data(
                json!({
                    "sequence": sequence,
                    "mark_in": mark_in,
                    "mark_out": frame,
                }),
            ))

        }
    }

    /// **Take** the **marks** of a sequence away.
    ///
    /// The tree's own three commands — the clear in, the clear out and the
    /// clear of both — are one command here, and the caller says which.
    film.sequence.mark.clear {
        area: "film",
        layer: "film",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// Which mark is taken away: `in`, `out` or `both`.
            which: Text,
        },
        run: {
            let id = "film.sequence.mark.clear".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            let which = self.which.0.trim().to_ascii_lowercase();
            if !matches!(which.as_str(), "in" | "out" | "both") {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "`{}` is not a mark to take away: it is `in`, `out` or `both`",
                        self.which.0
                    ),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed {
                    id: "film.sequence.mark.clear".to_string(),
                    message: format!("there is no sequence `{sequence}`"),
                });
            };
            if let craftkit_sdk::serde_json::Value::Object(map) = &mut node.data {
                if which == "in" || which == "both" {
                    map.remove(MARK_IN);
                }
                if which == "out" || which == "both" {
                    map.remove(MARK_OUT);
                }
            }
            Ok(Outcome::changed(format!("the sequence `{sequence}` has no `{which}` mark"))
                .with_data(json!({ "sequence": sequence, "which": which })))

        }
    }
}
