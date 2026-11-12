//! **Rippling** a clip away.
//!
//! The tree's delete leaves a gap and the tree's **ripple delete**
//! closes it: the clips that were after the deleted one move up by the
//! length of the clip that went, and the time the clip took is time
//! that is free again.
//!
//! The tree's ripple **refuses** when the gap cannot close — "no gap
//! can close: the other clips on these tracks cover the deleted time" —
//! and so does this one, in the same words: the gap may only close when
//! the time the clip took is free.
//!
//! # What it does not do
//!
//! - **The sync-locked tracks are not here.** The tree's ripple moves
//!   the tracks that are **sync-locked** with the track that lost the
//!   clip, so that a group of tracks stays together.
//! - **The tree's spans are not here.** The tree computes the
//!   stretches that can close and closes each of them; here the whole
//!   range of the clip closes, or the ripple is refused.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// **Take away** a clip and close the gap.
    ///
    /// The clips that were after it move **up** by the length of the clip
    /// that went, and the time the clip took is free again. The ripple is
    /// **refused** when the time is not free: another clip covers it.
    film.clip.ripple {
        area: "film",
        layer: "film",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The clip.

            clip: NodeRef,
        },
        run: {

            let id = "film.clip.ripple".to_string();
            let clip = self.clip.0;
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
            let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
            let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
            let end = start + duration;
            // A **locked** track cannot be edited.
            if let Some(track) = crate::track_of(session, clip)
                && crate::is_locked(session, track)
            {
                return Err(crate::locked_error(&id, track));
            }
            // The **clips** on the track: the gap may only close when the
            // time the clip took is free.
            let mut others: Vec<(u64, u64, u64)> = Vec::new();
            for child in session
                .document()
                .walk()
                .filter(|node| node.kind == "film.track")
                .flat_map(|node| node.children.iter())
                .filter(|child| child.kind == "film.clip" && child.id != clip)
            {
                let Some(other_start) = child.data.get("start").and_then(|start| start.as_u64()) else {
                    continue;
                };
                let Some(other_duration) = child.data.get("duration").and_then(|duration| duration.as_u64())
                else {
                    continue;
                };
                let other_end = other_start + other_duration;
                if other_start < end && other_end > start {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "no gap can close: the clip on the track runs from frame {other_start} to frame {other_end}, and it covers the time the clip took"
                        ),
                    });
                }
                others.push((child.id, other_start, other_duration));
            }
            // The gap closes: the clips that were after it move **up**.
            let mut moved: Vec<serde_json::Value> = Vec::new();
            for (other, other_start, other_duration) in others {
                if other_start < end {
                    continue;
                }
                let new_start = other_start - duration;
                let source_in = session
                    .document()
                    .node(other)
                    .and_then(|node| node.data.get("source_in").cloned())
                    .unwrap_or(serde_json::Value::Null);
                if let Some(node) = session.document_mut()?.node_mut(other) {
                    node.data = crate::clocks_of(&node.data, new_start, other_duration, &source_in);
                    crate::set_clip_name(node, new_start);
                }
                moved.push(serde_json::json!({ "id": other, "start": new_start }));
            }
            if !session.document_mut()?.remove_node(clip) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            }
            Ok(Outcome::changed(format!("the clip `{clip}` is gone and the gap is closed")).with_data(
                serde_json::json!({ "id": clip, "start": start, "duration": duration, "moved": moved }),
            ))

        }
    }
}
