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
//! And the **transitions** follow the cut: a transition that begins at
//! or after the cut now joins the piece after the cut, and a transition
//! that spans the cut keeps the piece before it. The tree's rule, and
//! the rule is about the transition's **start** and not about the cut
//! the transition sits on.
//!
//! # What it does not do
//!
//! - **The link is here, and the split honours it.** The tree's split
//!   gives the new piece a **new link**, so that the two pieces are
//!   still linked together and not to the left-hand ones, and the
//!   members of a group are cut in one edit (the module `linking`).
//! - **The tree's reversed clips are not here**: a reversed clip's cut
//!   moves the media the other way.

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
        // A clip that is **linked** is split with its partners: the
        // tree splits the group as one edit and the right-hand pieces
        // keep a link to each other and not to the left-hand ones.
        let members = crate::linking::group(session, clip);
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
        // A **locked** track cannot be edited.
        if let Some(track) = crate::track_of(session, clip)
            && crate::is_locked(session, track)
        {
            return Err(crate::locked_error(&id, track));
        }

        // Every member of the group that **covers** the place is
        // split: the tree splits the members of a group that are under
        // the place, and the others are left alone.
        let mut split: Vec<(u64, u64)> = Vec::new();
        // The **transitions** that follow the cut: they now join the
        // piece after it.
        let mut moved: Vec<u64> = Vec::new();
        for member in &members {
            let Some(node) = session.document().node(*member) else {
                continue;
            };
            let member_start = node.data.get("start").and_then(|start| start.as_u64());
            let member_duration = node.data.get("duration").and_then(|duration| duration.as_u64());
            let (Some(member_start), Some(member_duration)) = (member_start, member_duration) else {
                continue;
            };
            if at <= member_start || at >= member_start + member_duration {
                continue;
            }
            let (new, followed) = split_one(session, *member, at)?;
            split.push((*member, new));
            moved.extend(followed);
        }
        if split.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the frame {at} is not inside the clip, which runs from frame {start} to frame {}",
                    start + duration
                ),
            });
        }
        // The **new group**: the right-hand pieces are linked to each
        // other and not to the left-hand ones, as the tree's split
        // does. The group's name is the smallest id in it.
        let name = split.iter().map(|(_, new)| *new).min();
        if let Some(name) = name {
            for (_, new) in &split {
                if let Some(node) = session.document_mut()?.node_mut(*new) {
                    node.data["link"] = serde_json::json!(name);
                }
            }
        }
        let new = split.iter().find(|(member, _)| *member == clip).map(|(_, new)| *new);
        Ok(Outcome::changed(format!("the clip `{clip}` is split at frame {at}")).with_data(
            serde_json::json!({
                "id": clip,
                "new": new,
                "at": at,
                "duration": at - start,
                "group": split.iter().map(|(member, _)| *member).collect::<Vec<u64>>(),
                "new_group": split.iter().map(|(_, new)| *new).collect::<Vec<u64>>(),
                "link": name,
                "transitions": moved,
            }),
        ))
    }
}

/// **Split** one clip at a place, and give back the **new clip** — the
/// piece after the place — and the **transitions** that now join it.
///
/// The tree's rule: a transition that **begins at or after the cut**
/// and joins the clip now joins the piece after the cut. The rule is
/// about the transition's **start** and not about the cut the
/// transition sits on, so a transition that spans the cut keeps the
/// piece before it.
fn split_one(session: &mut Session, clip: u64, at: u64) -> Result<(u64, Vec<u64>), EngineError> {
    let id = "film.clip.split".to_string();
    let Some(node) = session.document().node(clip) else {
        return Err(EngineError::Failed { id: id.clone(), message: format!("there is no node `{clip}`") });
    };
    let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    let source_in = node.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
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
    // The **speed**: the media a frame of the clip plays is the
    // speed's worth of the media, so the cut is a cut in the media
    // too.
    let speed = crate::speed::speed_of(&node.data);
    let original = node.data.clone();
    let media_of = |frames: u64| crate::speed::media_frames(frames, speed);
    let (left_source_in, right_source_in) = if reversed {
        (source_in + media_of(duration - cut), source_in)
    } else {
        (source_in, source_in + media_of(cut))
    };
    // The piece before the place: the same clip, shorter.
    let left_source_in = serde_json::json!(left_source_in);
    let data = crate::clocks_of(&original, start, cut, &left_source_in);
    if let Some(node) = session.document_mut()?.node_mut(clip) {
        node.data = data;
        crate::set_clip_name(node, start);
    }
    // The piece after it: a **new clip**, and the media moves with
    // the cut.
    //
    // The name the caller gave belongs to the clip that kept its id:
    // the new piece is a new clip and it is shown by the place it is
    // at.
    let right_source_in = serde_json::json!(right_source_in);
    let mut data = crate::clocks_of(&original, at, duration - cut, &right_source_in);
    data["name"] = serde_json::Value::Null;
    let name = format!("clip at {at}");
    let new = session.document_mut()?.add_node_with_under(Some(track), name.clone(), "film.clip", None, data);
    let relation = craftkit_sdk::Relation::with_payload(new, item, "clip", serde_json::json!({}));
    session.document_mut()?.add_relation(relation);
    // The **transitions** that follow the cut now join the piece after
    // it: the tree's rule, and the rule is about the transition's
    // **start**.
    let followed: Vec<u64> = session
        .document()
        .walk()
        .filter(|node| node.kind == "film.transition")
        .filter(|node| node.data.get("from").and_then(|from| from.as_u64()) == Some(clip))
        .filter(|node| node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0) >= at)
        .map(|node| node.id)
        .collect();
    for transition in &followed {
        if let Some(node) = session.document_mut()?.node_mut(*transition) {
            node.data["from"] = serde_json::json!(new);
        }
    }
    Ok((new, followed))
}
