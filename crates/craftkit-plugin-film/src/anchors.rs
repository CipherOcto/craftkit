//! The **synchronisation** of a clip by its anchor.
//!
//! A clip has two clocks: where it starts on the **timeline** and
//! where it begins in the **media**. The frame of the media the clip
//! begins at is its **anchor**, and it is the anchor that says what
//! the clip shows.
//!
//! Two clips are **in step** when their anchors agree: each of them
//! begins at the same moment of its own media. The **method** says
//! which point of the reference the clip lines up with:
//!
//! - `in` — the **in points**: the two clips begin at the same frame
//!   of their media;
//! - `out` — the **out points**: the two clips end at the same frame
//!   of their media.
//!
//! And the **link** is here, and the synchronisation honours it: the
//! tree moves the linked clips together, so that the picture and the
//! sound stay in step, and this tree does the same (the module
//! `linking`).
//!
//! # What the port found
//!
//! - **The tree's synchronisation is a common time**: every clip is
//!   reduced to an anchor, and the clips are moved in the timeline
//!   until their common times line up. Here the **media** lines up
//!   and the timeline is left alone: a clip that is in step shows the
//!   same thing as the clip it is in step with, and a synchronisation
//!   that moved clips about would be a second edit wearing the
//!   synchronisation's coat.
//! - **The tree has five methods and here there are two.** The tree's
//!   timecode and marker methods need a clock and a marker, and its
//!   audio method needs the waveforms; this tree has the frames of
//!   the media and nothing else.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// Which point of the reference a clip lines up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    /// The **in points**: the clips begin at the same frame of their
    /// media.
    In,
    /// The **out points**: the clips end at the same frame of their
    /// media.
    Out,
}

impl Method {
    /// The method a name says, if it says one this tree knows.
    fn parse(name: &str) -> Option<Method> {
        match name.trim().to_lowercase().as_str() {
            "in" | "inpoint" | "start" => Some(Method::In),
            "out" | "outpoint" | "end" => Some(Method::Out),
            _ => None,
        }
    }

    /// The name of the method.
    fn name(self) -> &'static str {
        match self {
            Method::In => "in",
            Method::Out => "out",
        }
    }
}

/// The **clip** a node is, if it is one, and the frames it needs.
struct Clip {
    /// How long it is.
    duration: u64,
    /// The frame of the media it begins at: its **anchor**.
    anchor: u64,
    /// The speed it plays at.
    speed: f64,
}

/// The **clip** a node is, if it is one.
fn the_clip(session: &Session, id: &str, node: u64) -> Result<Clip, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no clip `{node}`"),
        });
    };
    if found.kind != "film.clip" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{node}` is a `{}` and not a clip", found.kind),
        });
    }
    let duration = found.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    let anchor = found.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
    let speed = crate::speed::speed_of(&found.data);
    Ok(Clip { duration, anchor, speed })
}

/// The frame of the media the clip must begin at to be in step with
/// the reference.
///
/// The method says which point of the reference lines up: its in
/// point, or its out point.
fn target_of(clip: &Clip, reference: &Clip, method: Method) -> Option<i64> {
    match method {
        Method::In => Some(reference.anchor as i64),
        Method::Out => Some(reference.anchor as i64 + reference.duration as i64 - clip.duration as i64),
    }
}

commands! {
    /// **Synchronise** a clip with another one.
    film.clip.synchronise {
        area: "film",
        layer: "film",
        icon: "link",
        mutates,
        undoable,
        params: {
            /// The clip.

            clip: NodeRef,
            /// The clip it lines up with.

            reference: NodeRef,
            /// Which point of the reference lines up: `in` or `out`. The
            /// default is `in`.

            method: Option<Text>,
        },
        run: {

            let id = "film.clip.synchronise".to_string();
            let clip = self.clip.0;
            let reference = self.reference.0;
            if clip == reference {
                return Err(EngineError::Failed {
                    id,
                    message: "a clip cannot be in step with itself".to_string(),
                });
            }
            let method = match &self.method {
                Some(method) => Method::parse(&method.0).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a method: a method is `in` or `out`", method.0),
                })?,
                None => Method::In,
            };
            let clip_info = the_clip(session, &id, clip)?;
            let reference_info = the_clip(session, &id, reference)?;
            // A **locked** track cannot be edited.
            if let Some(track) = crate::track_of(session, clip)
                && crate::is_locked(session, track)
            {
                return Err(crate::locked_error(&id, track));
            }
            let Some(target) = target_of(&clip_info, &reference_info, method) else {
                return Err(EngineError::Failed { id, message: "the clips cannot be in step".to_string() });
            };
            if target < 0 {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "the clip is {} frame(s) longer than the reference, and their out points cannot be the same frame",
                        clip_info.duration as i64 - reference_info.duration as i64
                    ),
                });
            }
            let target = target as u64;
            // The **delta** in the timeline: the media moves by the speed's
            // worth of it, and the delta is a whole number of frames.
            let moved = target as f64 - clip_info.anchor as f64;
            let delta = (moved / clip_info.speed).round() as i64;
            if delta == 0 {
                return Err(EngineError::Failed { id, message: "the clip is already in step".to_string() });
            }
            // The **media** the clip may play: the clip may not begin
            // before it and may not run past the end of it.
            if let Some(media) = crate::media_range_of(session, clip) {
                let end = target + clip_info.duration;
                if target < media.start || end > media.end {
                    return Err(EngineError::Failed {
                        id,
                        message: format!(
                            "the clip would play the frames {target} to {end} of the media, and the media runs from {} to {}",
                            media.start, media.end
                        ),
                    });
                }
            }
            // The **link** is here: the tree moves the linked clips
            // together, and this tree does the same.
            let members = crate::linking::group(session, clip);
            let mut planned: Vec<(u64, u64)> = Vec::new();
            let mut skipped: Vec<u64> = Vec::new();
            for member in &members {
                if *member != clip
                    && let Some(track) = crate::track_of(session, *member)
                    && crate::is_locked(session, track)
                {
                    skipped.push(*member);
                    continue;
                }
                let Some(node) = session.document().node(*member) else { continue };
                let Some(anchor) = node.data.get("source_in").and_then(|source_in| source_in.as_u64()) else {
                    continue;
                };
                let speed = crate::speed::speed_of(&node.data);
                let shifted = (anchor as f64 + delta as f64 * speed).round();
                if shifted < 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{member}` would begin before the media begins"),
                    });
                }
                planned.push((*member, shifted as u64));
            }
            for (member, anchor) in &planned {
                if let Some(node) = session.document_mut()?.node_mut(*member) {
                    node.data["source_in"] = serde_json::json!(*anchor);
                }
            }
            let anchor =
                planned.iter().find(|(member, _)| *member == clip).map(|(_, anchor)| *anchor).unwrap_or(target);
            let partners: Vec<u64> = members.iter().copied().filter(|member| *member != clip).collect();
            Ok(Outcome::changed(format!("the clip `{clip}` is in step with the clip `{reference}`")).with_data(
                serde_json::json!({
                    "clip": clip,
                    "reference": reference,
                    "method": method.name(),
                    "anchor": anchor,
                    "group": members,
                    "partners": partners,
                    "skipped": skipped,
                }),
            ))

        }
    }
}
