//! The **edit points** of a sequence, and the **razor**.
//!
//! An **edit point** is a place where a clip begins or ends: the
//! places where the sequence can be cut. The tree's playhead walks
//! them, and the tree's razor cuts at one.
//!
//! The tree's rules, and they are the finding:
//!
//! - the edit points are the **starts and the ends of the clips** of
//!   every track, sorted, and a place that is both is one place;
//! - the playhead goes to the **next** edit point that is after it
//!   and to the **previous** one that is before it, and when there is
//!   none it **stays**;
//! - the tree's razor cuts the clip that is **under the playhead**, and
//!   it cuts the **group** of that clip and not the clip alone.
//!
//! Here the edit points are the same, and the razor is the split of
//! the clip under the playhead — the same command and the same edit
//! (the module `splitting`).

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **edit points** of a sequence: the starts and the ends of the
/// clips of its tracks.
fn edit_points(session: &Session, sequence: u64) -> Vec<u64> {
    let Some(node) = session.document().node(sequence) else {
        return Vec::new();
    };
    let mut points: Vec<u64> = Vec::new();
    for track in node.children.iter().filter(|child| child.kind == "film.track") {
        for clip in track.children.iter().filter(|child| child.kind == "film.clip") {
            let start = clip.data.get("start").and_then(|start| start.as_u64());
            let duration = clip.data.get("duration").and_then(|duration| duration.as_u64());
            if let (Some(start), Some(duration)) = (start, duration) {
                points.push(start);
                points.push(start + duration);
            }
        }
    }
    points.sort_unstable();
    points.dedup();
    points
}

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

commands! {
    /// **Go to** the next edit point of a sequence.
    film.playhead.nextedit {
        area: "film",
        layer: "film",
        icon: "next",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            go_to_edit(session, "film.playhead.nextedit", self.sequence.0, true)

        }
    }

    /// **Go to** the previous edit point of a sequence.
    film.playhead.prevedit {
        area: "film",
        layer: "film",
        icon: "previous",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
        },
        run: {
            go_to_edit(session, "film.playhead.prevedit", self.sequence.0, false)

        }
    }

    /// **Cut** the clip that is under the playhead with the **razor**.
    film.timeline.razor {
        area: "film",
        layer: "film",
        icon: "razor",
        mutates,
        undoable,
        params: {
            /// The sequence.
            sequence: NodeRef,
            /// The track the clip is on.
            track: NodeRef,
            /// The frame to cut at. The playhead of the sequence when it is
            /// not said.
            at: Option<Number>,
        },
        run: {
            let id = "film.timeline.razor".to_string();
            let sequence = self.sequence.0;
            let track = self.track.0;
            let node = the_sequence(session, &id, sequence)?;
            let at = match self.at {
                Some(at) => at.0,
                None => crate::playhead::playhead_of(node) as f64,
            };
            if !at.is_finite() || at.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "a number of frames is a whole number".to_string(),
                });
            }
            let at = at as u64;
            // The **track**: a child of the sequence.
            let Some(track_node) = node.children.iter().find(|child| child.id == track) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no track `{track}` in the sequence `{sequence}`"),
                });
            };
            if track_node.kind != "film.track" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{track}` is a `{}` and not a track", track_node.kind),
                });
            }
            // The **clip** that is under the place.
            let clip = track_node.children.iter().find(|child| {
                child.kind == "film.clip"
                    && child.data.get("start").and_then(|start| start.as_u64()).is_some_and(|start| {
                        child
                            .data
                            .get("duration")
                            .and_then(|duration| duration.as_u64())
                            .is_some_and(|duration| start <= at && at < start + duration)
                    })
            });
            let Some(clip) = clip.map(|clip| clip.id) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no clip at frame {at} on the track `{track}`"),
                });
            };
            crate::splitting::split_at(session, &id, clip, at as f64)

        }
    }
}

/// The playhead goes to the next or the previous edit point.
fn go_to_edit(session: &mut Session, id: &str, sequence: u64, forward: bool) -> Result<Outcome, EngineError> {
    let node = the_sequence(session, id, sequence)?;
    let here = crate::playhead::playhead_of(node);
    let points = edit_points(session, sequence);
    let target = if forward {
        points.iter().copied().find(|point| *point > here)
    } else {
        points.iter().copied().rev().find(|point| *point < here)
    };
    // The tree's rule: when there is no edit point to go to, the
    // playhead stays where it is.
    let frame = target.unwrap_or(here);
    let Some(node) = session.document_mut()?.node_mut(sequence) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sequence `{sequence}`"),
        });
    };
    node.data["playhead"] = json!(frame);
    Ok(Outcome::changed(format!("the playhead is at frame {frame}"))
        .with_data(json!({ "sequence": sequence, "frame": frame })))
}
