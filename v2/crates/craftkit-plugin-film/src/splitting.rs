//! **Splitting** a clip.
//!
//! A clip may be divided at a place: the piece before the place keeps
//! the clip's id and the piece after it is a **new clip**. The media
//! moves with the cut — the piece after the place begins that much
//! later in the media — because a clip has **two clocks** and cutting
//! one of them cuts the other with it.
//!
//! The tree's `split_track_at` does exactly this, and the tree's razor
//! does it on purpose. Here it is a command of its own, and not a side
//! effect of the insert (§266): a command that splits a clip while it
//! moves another one is a command that does two things and says one.
//!
//! # What it does not do
//!
//! - **The tree's links are not here.** The tree's split gives the new
//!   piece a **new link**, so that the two pieces are still linked
//!   together. Here a clip is a clip.
//! - **The tree's transitions are not here**, and neither are the
//!   tree's **reversed** clips: a reversed clip's cut moves the media
//!   the other way.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// **Split** a clip at a place.
///
/// The piece before the place keeps the clip's id and is shortened, and
/// the piece after it is a **new clip** that begins that much later in
/// the media. The place must be **inside** the clip: a place at its
/// edge would cut nothing, and a place outside it is not in it.
#[derive(Command)]
#[command(id = "film.clip.split", area = "film", layer = "film", icon = "scissors", mutates, undoable)]
pub struct SplitClip {
    /// The clip.
    pub clip: NodeRef,
    /// Where it is divided, in frames.
    pub at: Number,
}

impl SplitClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.split".to_string();
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
        let Some(start) = node.data.get("start").and_then(|start| start.as_u64()) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say where it is"),
            });
        };
        let Some(duration) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say how long it is"),
            });
        };
        let source_in = node.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
        let at = self.at.0;
        if !at.is_finite() || at.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a number of frames is a whole number".to_string(),
            });
        }
        let at = at as u64;
        if at <= start || at >= start + duration {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the frame {at} is not inside the clip, which runs from frame {start} to frame {}",
                    start + duration
                ),
            });
        }
        // The **item** the clip plays, and the **track** it is on.
        let Some(item) = session
            .document()
            .relations
            .iter()
            .find(|relation| relation.kind == "clip" && relation.from == clip)
            .map(|relation| relation.to)
        else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say which item it plays"),
            });
        };
        let Some(track) = session
            .document()
            .walk()
            .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == clip))
            .map(|node| node.id)
        else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` is not on a track"),
            });
        };
        let cut = at - start;
        // A clip that plays **backwards** shows the *later* media at
        // its head: the piece before the place takes the media the
        // piece after it had, and the content stays continuous.
        let reversed = crate::is_reversed(&node.data);
        let original = node.data.clone();
        let (left_source_in, right_source_in) =
            if reversed { (source_in + (duration - cut), source_in) } else { (source_in, source_in + cut) };
        // The piece before the place: the same clip, shorter.
        let left_source_in = serde_json::json!(left_source_in);
        let data = crate::clocks_of(&original, start, cut, &left_source_in);
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data = data;
            node.name = format!("clip at {start}");
        }
        // The piece after it: a **new clip**, and the media moves with
        // the cut.
        let right_source_in = serde_json::json!(right_source_in);
        let data = crate::clocks_of(&original, at, duration - cut, &right_source_in);
        let name = format!("clip at {at}");
        let new =
            session.document_mut()?.add_node_with_under(Some(track), name.clone(), "film.clip", None, data);
        let relation = craftkit_sdk::Relation::with_payload(new, item, "clip", serde_json::json!({}));
        session.document_mut()?.add_relation(relation);
        Ok(Outcome::changed(format!("the clip `{clip}` is split at frame {at}")).with_data(
            serde_json::json!({
                "id": clip,
                "new": new,
                "at": at,
                "duration": cut,
            }),
        ))
    }
}
