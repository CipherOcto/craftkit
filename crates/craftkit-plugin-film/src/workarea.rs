//! The **work area** of a sequence.
//!
//! The work area is the range of the sequence the editor is working
//! in: an **in** point and an **out** point. It is a **fact of the
//! sequence**, and it is saved with the sequence and not with the
//! window that happens to be open.
//!
//! # The rules
//!
//! - **A work area is a whole number of frames.** A frame that is not
//!   a whole number is not a frame, and a frame before the beginning
//!   of the timeline is not a frame at all.
//! - **The work area does not end before it begins.** The out point
//!   is not before the in point, and a work area that says otherwise
//!   is refused by name.
//! - **A sequence may have no work area.** Nothing is said, and
//!   nothing is read back: the answer says there is none, and it does
//!   not say a frame of zero, because a work area of one frame at the
//!   start is a work area.
//! - **The work area is not moved.** The tree moves a work area back
//!   to the end of the sequence when the sequence is shorter than it;
//!   here the work area is what the caller says it is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The most frames a work area may reach.
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

/// A **frame** a caller has named: a whole number, and not before the
/// beginning of the timeline.
fn frame_of(command: &str, value: f64, what: &str) -> Result<u64, EngineError> {
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
    let _ = what;
    Ok(frame)
}

/// The **work area** of a sequence: its in point and its out point.
///
/// A sequence that says nothing has **no** work area, and that is not
/// the same as a work area of one frame at the start.
pub(crate) fn work_area_of(node: &craftkit_sdk::Node) -> Option<(u64, u64)> {
    let start = node.data.get("work_in").and_then(|frame| frame.as_u64())?;
    let end = node.data.get("work_out").and_then(|frame| frame.as_u64())?;
    Some((start, end))
}

commands! {
    /// **Set** the work area of a sequence.
    film.workarea.set {
        area: "film",
        layer: "film",
        icon: "marker",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// The in point, in frames.
            start: Number,
            /// The out point, in frames.
            end: Number,
        },
        run: {
            let id = "film.workarea.set".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            let start = frame_of(&id, self.start.0, "the in point")?;
            let end = frame_of(&id, self.end.0, "the out point")?;
            if end < start {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the work area ends at frame {end}, and it begins at frame {start}"),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed { id, message: format!("there is no sequence `{sequence}`") });
            };
            node.data["work_in"] = json!(start);
            node.data["work_out"] = json!(end);
            Ok(Outcome::changed(format!("the work area runs from frame {start} to frame {end}"))
                .with_data(json!({ "sequence": sequence, "start": start, "end": end })))

        }
    }

    /// **Read** the work area of a sequence.
    film.workarea.get {
        area: "film",
        layer: "film",
        icon: "list",
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            let id = "film.workarea.get".to_string();
            let sequence = self.sequence.0;
            let node = the_sequence(session, &id, sequence)?;
            let area = match work_area_of(node) {
                Some((start, end)) => json!({ "start": start, "end": end, "length": end - start + 1 }),
                None => json!(null),
            };
            Ok(Outcome::new(format!("the work area of the sequence `{sequence}`")).with_data(json!({
                "sequence": sequence,
                "work_area": area,
            })))

        }
    }

    /// **Take the work area away** from a sequence.
    film.workarea.clear {
        area: "film",
        layer: "film",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            let id = "film.workarea.clear".to_string();
            let sequence = self.sequence.0;
            the_sequence(session, &id, sequence)?;
            let Some(node) = session.document_mut()?.node_mut(sequence) else {
                return Err(EngineError::Failed { id, message: format!("there is no sequence `{sequence}`") });
            };
            if let craftkit_sdk::serde_json::Value::Object(map) = &mut node.data {
                map.remove("work_in");
                map.remove("work_out");
            }
            Ok(Outcome::changed(format!("the sequence `{sequence}` has no work area"))
                .with_data(json!({ "sequence": sequence })))

        }
    }
}
