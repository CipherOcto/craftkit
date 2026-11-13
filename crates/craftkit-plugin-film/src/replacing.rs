//! **Replacing** the media a clip plays.
//!
//! A clip plays a media, and it may play a **different** media: the
//! tree's *replace from the bin* points the clip at another item of
//! the project and moves the clip's **in point** to the beginning of
//! the new media.
//!
//! The clip's media is an **edge** — a relation — and not a field of
//! the clip, so replacing it is replacing one edge with another: the
//! old edge dies and the new one is made, and the clip keeps its id
//! and its place.
//!
//! # What the port found
//!
//! - **The tree's replace is three commands and here it is one.** The
//!   tree has *replace from source*, *replace from source, match
//!   frame* and *replace from bin*, and the three differ in where the
//!   new in point comes from: the source monitor, the frame under the
//!   playhead, or the beginning of the new media. Here the new in
//!   point is the **beginning of the new media** — the tree's *from
//!   bin* — because this tree has no source monitor and no playhead.
//! - **The tree clamps and this tree refuses.** The tree's replace
//!   keeps a clip's own in point when the caller says
//!   `keepSourceIn`, and clamps it to the nearest frame of a subclip
//!   that restricts its trims — and it says which clips were clamped.
//!   Here the in point is the beginning of the new media and a clip
//!   that would run past the end of it is **refused** and the clip is
//!   named, which is the rule this tree follows everywhere: refuse, and
//!   do not clamp.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// The media a clip plays, as the **edge** says it.
fn media_of(session: &Session, clip: u64) -> Option<u64> {
    session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)
}

commands! {
    /// **Replace** the media a clip plays.
    ///
    /// The clip plays the new media from its **beginning**, and a clip that
    /// would run past the end of the new media is refused.
    film.clip.replace {
        area: "film",
        layer: "film",
        icon: "swap",
        mutates,
        undoable,
        params: {
            /// The clip.
            clip: NodeRef,
            /// The media it plays now.
            item: NodeRef,
        },
        run: {
            let id = "film.clip.replace".to_string();
            let clip = self.clip.0;
            let item = self.item.0;
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
            let Some(media) = session.document().node(item) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{item}`"),
                });
            };
            if media.kind != "film.item" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{item}` is a `{}` and not an item", media.kind),
                });
            }
            // The old media, for the answer.
            let old = media_of(session, clip);
            // A **locked** track cannot be edited, and a clip on it is an
            // edit.
            if let Some(track) = crate::track_of(session, clip)
                && crate::is_locked(session, track)
            {
                return Err(crate::locked_error(&id, track));
            }
            // The new media must be **long enough** for the clip: the clip
            // plays from the beginning of the new media, and a clip that
            // would run past the end of it is refused and not shortened.
            let length = media.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
            let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
            let speed = node.data.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(1.0);
            let played = if speed > 0.0 { (duration as f64 / speed).ceil() as u64 } else { duration };
            if length == 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the media `{item}` has no frames to play"),
                });
            }
            if played > length {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip would run to frame {played} of the media, and the media is {length} frames long"
                    ),
                });
            }
            // The edge that says which media the clip plays: the old one
            // goes and the new one comes.
            session.document_mut()?.remove_relations(|relation| relation.kind == "clip" && relation.from == clip);
            let relation = craftkit_sdk::Relation::with_payload(clip, item, "clip", serde_json::json!({}));
            session.document_mut()?.add_relation(relation);
            // The clip plays the new media from its beginning.
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data["source_in"] = serde_json::json!(0u64);
                node.data["speed"] = serde_json::json!(1.0);
                let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                crate::set_clip_name(node, start);
            }
            Ok(Outcome::changed(format!("the clip `{clip}` plays the media `{item}` now"))
                .with_data(serde_json::json!({ "clip": clip, "item": item, "old_item": old })))

        }
    }
}
