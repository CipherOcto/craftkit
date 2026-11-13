//! **Reading** a clip.
//!
//! A clip may be read whole: the answer is everything the clip holds
//! and everything that is said about it.
//!
//! The facts of a clip are where it **starts**, how long it is, where
//! in the media it **begins**, how fast it **plays**, whether it
//! **plays** at all, its **label**, its **link** to the clip beside it,
//! the **frame hold** in it and whether it runs **backwards**.
//!
//! Three of the facts of the answer are **derived**: where the clip
//! **ends** is its start and its length, the **material** it plays is
//! its length times its speed, and the **track**, the **sequence** and
//! the **item** it belongs to are its place in the tree and the edges
//! that name it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// Read a **clip**.
    film.clip.get {
        area: "film",
        layer: "film",
        icon: "list",
        params: {
            /// The clip to read.
            clip: NodeRef,
        },
        run: {
            let id = "film.clip.get".to_string();
            let clip = self.clip.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed { id, message: format!("there is no clip `{clip}`") });
            };
            if node.kind != "film.clip" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let data = &node.data;
            let start = data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
            let duration = data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
            let speed = data.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(1.0);
            let source_in = data.get("source_in").cloned().unwrap_or(Json::Null);
            let label = data.get("label").cloned().unwrap_or(Json::Null);
            let link = data.get("link").cloned().unwrap_or(Json::Null);
            let frame_hold = data.get("frame_hold").cloned().unwrap_or(Json::Null);
            let reverse = data.get("reverse").and_then(|reverse| reverse.as_bool()).unwrap_or(false);
            let enabled = crate::enabling::is_enabled(data);
            // The **name** a clip is shown by: the name it was given, or the
            // place it is at.
            let name = crate::clip_name_of(data, start);
            // The **material** the clip plays: its length times its speed.
            let media = crate::speed::media_frames(duration, speed);
            // The **track** the clip is on, the **sequence** the track is in
            // and the **item** the clip plays.
            let track = crate::track_of(session, clip);
            let sequence = crate::sequence_of(session, clip);
            let item = session.document().relations_from(clip, Some("clip")).first().map(|relation| relation.to);
            // The **nested clip**: the clips it holds, and the clip it is
            // held by. Both are derived from the tree.
            let members: Vec<u64> = node
                .children
                .iter()
                .filter(|child| child.kind == "film.track")
                .flat_map(|track| track.children.iter())
                .filter(|child| child.kind == "film.clip")
                .map(|child| child.id)
                .collect();
            let nested_in = track.and_then(|track| {
                session
                    .document()
                    .walk()
                    .find(|node| node.kind == "film.clip" && node.children.iter().any(|child| child.id == track))
                    .map(|node| node.id)
            });
            Ok(Outcome::new(format!("the clip `{clip}`")).with_data(json!({
                "id": clip,
                "name": name,
                "start": start,
                "duration": duration,
                "end": start + duration,
                "source_in": source_in,
                "speed": speed,
                "media_duration": media,
                "enabled": enabled,
                "label": label,
                "link": link,
                "frame_hold": frame_hold,
                "reverse": reverse,
                "track": track,
                "sequence": sequence,
                "item": item,
                "nested": !members.is_empty(),
                "members": members,
                "nested_in": nested_in,
            })))

        }
    }
}
