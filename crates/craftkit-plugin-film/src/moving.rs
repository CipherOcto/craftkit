//! **Moving** a clip.
//!
//! A clip has a place on a track, and the place may change: the clip
//! moves along the track, or it moves to another track. The tree moves
//! a clip in two modes — the **overwrite**, which cuts what is under
//! the clip, and the **insert**, which pushes it along — and here the
//! move is the **plain** one: the clip goes where it is told to go,
//! and a clip that would land on another clip is **refused**.
//!
//! The refusal is the point. The tree's overwrite is a thing a command
//! does **quietly**: a clip that is dragged over another one cuts it,
//! and the cut is not in the answer the tree gives back. Here the
//! answer says what would have been cut, and the caller may do it in a
//! command of its own.
//!
//! # What it does not do
//!
//! - **The insert is not here.** The tree's insert mode pushes the
//!   clips after the moved one along the track, so that nothing is
//!   cut. That is a different command.
//! - **The ripple is not here.** The tree's ripple pushes the clips
//!   after the moved one along the track, so that nothing is cut. That
//!   is a different command.
//! - **The link is here, and the move honours it**: the tree's linked
//!   clips move together, by the same offset on their own tracks (the
//!   module `linking`).

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

commands! {
    /// **Move** a clip to a place on a track.
    ///
    /// The clip goes where it is told to go, and a clip that would land on
    /// another clip is **refused**: the tree's overwrite cuts the clip
    /// that is under it and says nothing about the cut, and here the
    /// refusal names the clip that would have been cut.
    film.clip.move {
        area: "film",
        layer: "film",
        icon: "arrow",
        mutates,
        undoable,
        params: {
            /// The clip.
            clip: NodeRef,
            /// The track it goes on. The track it is on, if it stays there.
            track: NodeRef,
            /// Where it starts on that track, in frames.
            start: Number,
        },
        run: {
            let id = "film.clip.move".to_string();
            let clip = self.clip.0;
            let track = self.track.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            };
            if node.kind != "film.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let Some(duration) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` does not say how long it is"),
                });
            };
            let source_in = node.data.get("source_in").cloned().unwrap_or(serde_json::Value::Null);
            let Some(node) = session.document().node(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{track}`"),
                });
            };
            if node.kind != "film.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            let start = self.start.0;
            if !start.is_finite() || start.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a number of frames is a whole number".to_string(),
                });
            }
            if start < 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the timeline begins at frame 0".to_string(),
                });
            }
            let start = start as u64;
            let end = start + duration;
            // The **clips** that are on the track it goes on: the move may
            // not land on any of them.
            if let Some((_other, other_start, other_end)) = the_clip_under(session, track, start, end, clip) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip would run from frame {start} to frame {end}, and the clip on the track runs from frame {other_start} to frame {other_end}"
                    ),
                });
            }
            // The **partners** of a linked clip move with it: the tree's
            // rule is the same offset on their own tracks, so that the
            // picture and the sound stay in step. A partner on a locked
            // track stays where it is.
            let old_start = session
                .document()
                .node(clip)
                .and_then(|node| node.data.get("start").and_then(|start| start.as_u64()))
                .unwrap_or(start);
            let offset = start as i64 - old_start as i64;
            let mut partner_moves: Vec<(u64, u64, u64)> = Vec::new();
            for partner in crate::linking::partners(session, clip) {
                let Some(node) = session.document().node(partner) else {
                    continue;
                };
                let Some(partner_track) = crate::track_of(session, partner) else {
                    continue;
                };
                if crate::is_locked(session, partner_track) {
                    // A **locked** track cannot be edited: the partner
                    // stays where it is.
                    continue;
                }
                let Some(partner_start) = node.data.get("start").and_then(|start| start.as_u64()) else {
                    continue;
                };
                let Some(partner_duration) = node.data.get("duration").and_then(|duration| duration.as_u64())
                else {
                    continue;
                };
                let partner_new = partner_start as i64 + offset;
                if partner_new < 0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the clip `{partner}` would start at frame {partner_new}, and the timeline begins at frame 0"
                        ),
                    });
                }
                let partner_new = partner_new as u64;
                let partner_end = partner_new + partner_duration;
                if let Some((other, other_start, other_end)) =
                    the_clip_under(session, partner_track, partner_new, partner_end, partner)
                {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the clip `{partner}` would run from frame {partner_new} to frame {partner_end}, and the clip `{other}` on its track runs from frame {other_start} to frame {other_end}"
                        ),
                    });
                }
                partner_moves.push((partner, partner_new, partner_duration));
            }
            // The **track** it is on now, if it is another one.
            let from = session
                .document()
                .walk()
                .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == clip))
                .map(|node| node.id);
            // A **locked** track cannot be edited: not the one the clip is
            // on, and not the one it goes on.
            if let Some(track) = from
                && crate::is_locked(session, track)
            {
                return Err(crate::locked_error(&id, track));
            }
            if crate::is_locked(session, track) {
                return Err(crate::locked_error(&id, track));
            }
            if from == Some(track) {
                if let Some(node) = session.document_mut()?.node_mut(clip) {
                    node.data = crate::clocks_of(&node.data, start, duration, &source_in);
                    crate::set_clip_name(node, start);
                }
            } else {
                // The clip **changes track**: it is taken out of the tree
                // and put under the other track, with the same id, so that
                // the edge that says which item it plays still says it.
                let Some(mut node) = session.document_mut()?.take_node(clip) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{clip}`"),
                    });
                };
                node.data = crate::clocks_of(&node.data, start, duration, &source_in);
                crate::set_clip_name(&mut node, start);
                let Some(track_node) = session.document_mut()?.node_mut(track) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{track}`"),
                    });
                };
                track_node.add_child(node);
            }
            // The **partners** move after the clip, on their own tracks.
            for (partner, partner_start, partner_duration) in &partner_moves {
                if let Some(node) = session.document_mut()?.node_mut(*partner) {
                    let source_in = node.data.get("source_in").cloned().unwrap_or(serde_json::Value::Null);
                    node.data = crate::clocks_of(&node.data, *partner_start, *partner_duration, &source_in);
                    crate::set_clip_name(node, *partner_start);
                }
            }
            let partners: Vec<u64> = partner_moves.iter().map(|(partner, _, _)| *partner).collect();
            Ok(Outcome::changed(format!("the clip `{clip}` is at frame {start} of the track `{track}`"))
                .with_data(serde_json::json!({
                    "id": clip,
                    "track": track,
                    "start": start,
                    "duration": duration,
                    "partners": partners,
                })))

        }
    }
}

/// The **clip** on a track that a clip of that span would run into.
fn the_clip_under(session: &Session, track: u64, start: u64, end: u64, clip: u64) -> Option<(u64, u64, u64)> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "film.track" && node.id == track)
        .flat_map(|node| node.children.iter())
        .filter(|child| child.kind == "film.clip" && child.id != clip)
        .find_map(|other| {
            let other_start = other.data.get("start").and_then(|start| start.as_u64())?;
            let other_duration = other.data.get("duration").and_then(|duration| duration.as_u64())?;
            let other_end = other_start + other_duration;
            if start < other_end && other_start < end {
                Some((other.id, other_start, other_end))
            } else {
                None
            }
        })
}
