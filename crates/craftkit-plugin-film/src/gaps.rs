//! **Closing** a gap.
//!
//! A track may have a **gap** in it: a stretch of the timeline that no
//! clip covers. Closing the gap moves the clips that were after it
//! **up** by the length of the gap, and the time the gap took is time
//! that is free again.
//!
//! The tree's *close gap* works on the gap that holds a place on a
//! track, and it **refuses** when there is no gap there and when the
//! track is locked. So does this one.
//!
//! # What the port found
//!
//! - **The tree's gap is a selection and this tree's is a place.** The
//!   tree's *close gap* with no parameters closes the **selected** gap,
//!   and with none selected closes every gap of the sequence. Here the
//!   command says the **track** and the **place**, so there is no
//!   selection to read and no gap to guess: the gap is the one that
//!   holds the place.
//! - **The tree's gap is measured in the track's own time and here it
//!   is measured in the track's frames.** The tree's gap runs from the
//!   end of the clip before it to the start of the clip after it, and
//!   the clips that were after it move up by the length of the gap.
//!   Here the gap is the same stretch of the timeline, and the clips
//!   that were after it move up by the length of the gap — so the gap
//!   closes and nothing is cut.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

commands! {
    /// **Close** the gap that holds a place on a track.
    ///
    /// The clips that were after the gap move **up** by the length of the
    /// gap, and the time the gap took is free again.
    film.timeline.closegap {
        area: "film",
        layer: "film",
        icon: "collapse",
        mutates,
        undoable,
        params: {
            /// The track the gap is on.

            track: NodeRef,
            /// The place in the gap, in frames.

            time: Number,
        },
        run: {

            let id = "film.timeline.closegap".to_string();
            let track = self.track.0;
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
            let time = self.time.0;
            if !time.is_finite() || time < 0.0 || time.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a place in time is a whole number of frames".to_string(),
                });
            }
            let time = time as u64;
            // A **locked** track cannot be edited, and closing a gap on it
            // would move the other tracks while it stayed put.
            if crate::is_locked(session, track) {
                return Err(crate::locked_error(&id, track));
            }
            // The clips of the track, in the order they start.
            let mut clips: Vec<(u64, u64, u64)> = Vec::new();
            for child in node.children.iter().filter(|child| child.kind == "film.clip") {
                let Some(start) = child.data.get("start").and_then(|start| start.as_u64()) else {
                    continue;
                };
                let duration = child.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                clips.push((child.id, start, duration));
            }
            // The place must be **in a gap**: a clip that covers it means
            // there is no gap there.
            for (_, start, duration) in &clips {
                if *start <= time && time < start + duration {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no gap at frame {time}: a clip covers it"),
                    });
                }
            }
            // The gap runs from the end of the clip before it to the start
            // of the clip after it.
            let before = clips
                .iter()
                .filter(|(_, start, duration)| start + duration <= time)
                .map(|(_, start, duration)| start + duration)
                .max()
                .unwrap_or(0);
            let after =
                clips.iter().filter(|(_, start, _)| *start > time).map(|(_, start, _)| *start).min().ok_or_else(
                    || EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no gap at frame {time}: the track has no clip after it"),
                    },
                )?;
            let length = after - before;
            if length == 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no gap at frame {time}"),
                });
            }
            // The clips that were after the gap move **up**.
            let mut moved: Vec<serde_json::Value> = Vec::new();
            for (clip, start, duration) in &clips {
                if *start < after {
                    continue;
                }
                let new_start = start - length;
                let source_in = session
                    .document()
                    .node(*clip)
                    .and_then(|node| node.data.get("source_in").cloned())
                    .unwrap_or(serde_json::Value::Null);
                if let Some(node) = session.document_mut()?.node_mut(*clip) {
                    node.data = crate::clocks_of(&node.data, new_start, *duration, &source_in);
                    crate::set_clip_name(node, new_start);
                }
                moved.push(serde_json::json!({ "id": clip, "start": new_start }));
            }
            Ok(Outcome::changed(format!("the gap at frame {time} is closed")).with_data(serde_json::json!({
                "track": track,
                "gap": { "start": before, "end": after, "length": length },
                "moved": moved,
            })))

        }
    }
}
