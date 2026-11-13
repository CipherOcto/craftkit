//! **Nesting** the clips of a sequence.
//!
//! A **nested clip** is a clip that holds other clips: it is a little
//! timeline of its own, standing on a track of the sequence around it.
//! The tree calls it a **compound clip**, and the tree nests a group
//! of clips that are beside each other on the timeline.
//!
//! Here a nested clip is a **clip** that holds **tracks**, and each of
//! those tracks holds the clips that were on the track it came from.
//! The clip it stands on keeps the place of the earliest of the clips
//! it holds, and it lasts as long as they do.
//!
//! # The rules
//!
//! - **The clips must be of one sequence.** A clip of one sequence
//!   cannot be nested with a clip of another: there is nowhere to put
//!   the one that holds them.
//! - **A clip that holds nothing is not a clip.** There is nothing to
//!   hold, and the tree's compound clip is made of what is chosen.
//! - **The tracks of the clips must not be locked.** A locked track
//!   cannot be edited, and a clip that is taken off it is an edit.
//! - **The nested clip takes no media of its own.** It plays the
//!   clips it holds, and the media it lasts is the span of them.
//! - **A clip that is taken apart goes back where it was.** The track
//!   it came from is remembered, and the clip goes back to it — and a
//!   clip that would land on another one is **refused**, as it is
//!   when it is moved.
//!
//! # What it does not do
//!
//! - **The clips after the nested one do not move.** The tree closes
//!   the gap the nested clips leave and pushes the rest of the track
//!   along; here the gap is a gap, and the tree's own commands for
//!   gaps and for rippling are there to close it.
//! - **A nested clip is not a sequence.** It has no item of its own,
//!   and it is written out as a **stack** of the tracks it holds
//!   (§1367).

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session, Text};

/// The most clips a nested clip may hold.
const MOST_NESTED: usize = 64;

/// The **clips** a command is given.
fn listed(value: &Json) -> Vec<u64> {
    match value {
        Json::Array(list) => list.iter().filter_map(|entry| entry.as_u64()).collect(),
        one => one.as_u64().map(|entry| vec![entry]).unwrap_or_default(),
    }
}

/// The **start** and the **length** of a clip.
fn clocks(document: &Document, clip: u64) -> Option<(u64, u64)> {
    let node = document.node(clip)?;
    let start = node.data.get("start").and_then(|start| start.as_u64())?;
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64())?;
    Some((start, duration))
}

/// The **track** a clip is on, and the **sequence** that track is in.
///
/// The sequence is the **outermost** one: a clip that is inside a
/// nested clip is in the sequence the nested clip stands in.
fn place_of(session: &Session, clip: u64) -> Option<(u64, u64)> {
    let mut current = clip;
    loop {
        let track = crate::track_of(session, current)?;
        let parent = session
            .document()
            .walk()
            .find(|node| node.children.iter().any(|child| child.id == track))
            .map(|node| node.id)?;
        let kind = session.document().node(parent).map(|node| node.kind.clone())?;
        if kind == "film.item" {
            return Some((track, parent));
        }
        if kind != "film.clip" {
            return None;
        }
        current = parent;
    }
}

/// The **clip** on a track that a clip of that span would run into.
///
/// The clips that are **ignored** are the ones that are on their way
/// into the nested clip: they are not in the way of it.
fn the_clip_under(
    session: &Session,
    track: u64,
    start: u64,
    end: u64,
    ignored: &[u64],
) -> Option<(u64, u64, u64)> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "film.track" && node.id == track)
        .flat_map(|node| node.children.iter())
        .filter(|child| child.kind == "film.clip")
        .filter(|child| !ignored.contains(&child.id))
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

/// The **nested tracks** of a clip: the tracks it holds.
fn nested_tracks(document: &Document, clip: u64) -> Vec<u64> {
    document
        .node(clip)
        .map(|node| {
            node.children.iter().filter(|child| child.kind == "film.track").map(|child| child.id).collect()
        })
        .unwrap_or_default()
}

commands! {
    /// **Nest** the clips of a sequence into a clip of their own.
    ///
    /// The clip that is made stands on the track of the earliest of the
    /// clips it holds, and it lasts from there to the end of the last of
    /// them.
    film.clip.nest {
        area: "film",
        layer: "film",
        icon: "group",
        mutates,
        undoable,
        params: {
            /// The clips that are nested.
            clips: Json,
            /// What to call the clip that holds them.
            name: Text,
        },
        run: {
            let id = "film.clip.nest".to_string();
            let name = self.name.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a nested clip has a name".to_string(),
                });
            }
            let members = listed(&self.clips);
            if members.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a clip that holds nothing is not a clip".to_string(),
                });
            }
            if members.len() > MOST_NESTED {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a nested clip holds at most {MOST_NESTED} clips"),
                });
            }
            // Every clip is a clip of the **same** sequence, and every one
            // of them is on a track of it that is not locked.
            let mut sequence: Option<u64> = None;
            let mut places: Vec<(u64, u64, u64, u64)> = Vec::new();
            for member in &members {
                let Some(node) = session.document().node(*member) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no clip `{member}`"),
                    });
                };
                if node.kind != "film.clip" {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{member}` is a `{}` and not a clip", node.kind),
                    });
                }
                let Some((start, duration)) = clocks(session.document(), *member) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{member}` does not say where it starts"),
                    });
                };
                let Some((track, item)) = place_of(session, *member) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{member}` is not on a track of a sequence"),
                    });
                };
                // A clip that is **already nested** is not nested again.
                let nested_already = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "film.track" && node.id == track)
                    .and_then(|node| {
                        session
                            .document()
                            .walk()
                            .find(|parent| parent.children.iter().any(|child| child.id == node.id))
                            .map(|parent| parent.kind.clone())
                    })
                    .map(|kind| kind == "film.clip")
                    .unwrap_or(false);
                if nested_already {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{member}` is already nested"),
                    });
                }
                match sequence {
                    None => sequence = Some(item),
                    Some(sequence) if sequence != item => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the clip `{member}` is in the sequence `{item}`, and the others are in the sequence `{sequence}`"
                            ),
                        });
                    }
                    Some(_) => {}
                }
                // A **locked** track cannot be edited.
                if crate::is_locked(session, track) {
                    return Err(crate::locked_error(&id, track));
                }
                places.push((*member, track, start, duration));
            }
            // The place of the clip that is made: the earliest of them, and
            // as long as the last of them.
            let start = places.iter().map(|place| place.2).min().unwrap_or(0);
            let end = places.iter().map(|place| place.2 + place.3).max().unwrap_or(start);
            let duration = end - start;
            // The track it stands on is the track of the earliest clip.
            let first = places.iter().find(|place| place.2 == start).map(|place| place.1).unwrap_or(0);
            // It may not land on another clip of that track. The clips that
            // are nested are not in the way: they are going inside it.
            if let Some((other, other_start, other_end)) = the_clip_under(session, first, start, end, &members) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the nested clip would run from frame {start} to frame {end}, and the clip `{other}` on the track runs from frame {other_start} to frame {other_end}"
                    ),
                });
            }
            // The clip that is made: it holds the tracks of the clips, and
            // it plays no media of its own.
            let data = json!({
                "start": start,
                "duration": duration,
                "source_in": 0,
                "speed": 1.0,
                "link": Json::Null,
                "nested": true,
            });
            let compound =
                session.document_mut()?.add_node_with_under(Some(first), name.clone(), "film.clip", None, data);
            // The **tracks**: one for each track the clips came from, and
            // each one remembers the track it came from.
            let mut tracks: Vec<u64> = Vec::new();
            let mut nested_of: Vec<(u64, u64)> = Vec::new();
            for (_, track, _, _) in &places {
                if nested_of.iter().any(|(origin, _)| origin == track) {
                    continue;
                }
                let origin = session.document().node(*track);
                let kind = origin.map(|node| node.data.get("kind").cloned().unwrap_or(Json::Null));
                let track_name = origin.map(|node| node.name.clone()).unwrap_or_default();
                let nested_data = json!({ "kind": kind, "origin": track });
                let nested = session.document_mut()?.add_node_with_under(
                    Some(compound),
                    track_name,
                    "film.track",
                    None,
                    nested_data,
                );
                tracks.push(nested);
                nested_of.push((*track, nested));
            }
            // The clips go into the nested tracks, and their **start** is
            // the start they have inside the nested clip.
            for (member, track, member_start, member_duration) in &places {
                let nested =
                    nested_of.iter().find(|(origin, _)| origin == track).map(|(_, nested)| *nested).unwrap_or(0);
                let Some(mut node) = session.document_mut()?.take_node(*member) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no clip `{member}`"),
                    });
                };
                let source_in = node.data.get("source_in").cloned().unwrap_or(Json::Null);
                node.data = crate::clocks_of(&node.data, member_start - start, *member_duration, &source_in);
                crate::set_clip_name(&mut node, member_start - start);
                let Some(track_node) = session.document_mut()?.node_mut(nested) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no track `{nested}`"),
                    });
                };
                track_node.add_child(node);
            }
            let nested_members: Vec<u64> = places.iter().map(|place| place.0).collect();
            Ok(Outcome::changed(format!("the clip `{compound}` holds {} clip(s)", nested_members.len()))
                .with_data(json!({
                    "id": compound,
                    "name": name,
                    "start": start,
                    "duration": duration,
                    "track": first,
                    "sequence": sequence,
                    "tracks": tracks,
                    "members": nested_members,
                })))

        }
    }

    /// **Take a nested clip apart** and put its clips back where they were.
    ///
    /// The clips go back to the tracks they came from, at the places they
    /// had inside the nested clip. A clip that would land on another one
    /// is **refused**.
    film.clip.ungroup {
        area: "film",
        layer: "film",
        icon: "ungroup",
        mutates,
        undoable,
        params: {
            /// The nested clip.
            clip: NodeRef,
        },
        run: {
            let id = "film.clip.ungroup".to_string();
            let clip = self.clip.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no clip `{clip}`"),
                });
            };
            if node.kind != "film.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let tracks = nested_tracks(session.document(), clip);
            if tracks.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` holds no clips"),
                });
            }
            let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
            // The clips go back to the tracks they came from, and none of
            // them may land on another clip.
            let mut places: Vec<(u64, u64, u64, u64)> = Vec::new();
            for track in &tracks {
                let Some(nested) = session.document().node(*track) else { continue };
                let Some(origin) = nested.data.get("origin").and_then(|origin| origin.as_u64()) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the nested track `{track}` does not say where it came from"),
                    });
                };
                if crate::is_locked(session, origin) {
                    return Err(crate::locked_error(&id, origin));
                }
                for member in nested.children.iter().filter(|child| child.kind == "film.clip") {
                    let member_start = member.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                    let member_duration =
                        member.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                    let absolute = start + member_start;
                    let end = absolute + member_duration;
                    // The nested clip itself is not in the way: it is the
                    // thing that is taken apart.
                    if let Some((other, other_start, other_end)) =
                        the_clip_under(session, origin, absolute, end, &[clip])
                    {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the clip `{}` would run from frame {absolute} to frame {end}, and the clip `{other}` on the track runs from frame {other_start} to frame {other_end}",
                                member.id
                            ),
                        });
                    }
                    places.push((member.id, origin, absolute, member_duration));
                }
            }
            // The nested clip is taken out of the tree, and the clips it
            // holds go back to their tracks.
            let Some(mut compound) = session.document_mut()?.take_node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no clip `{clip}`"),
                });
            };
            for track in &tracks {
                let Some(mut nested) = compound.take_child(*track) else { continue };
                let members: Vec<u64> = nested
                    .children
                    .iter()
                    .filter(|child| child.kind == "film.clip")
                    .map(|child| child.id)
                    .collect();
                for member_id in members {
                    let Some(mut member) = nested.take_child(member_id) else { continue };
                    let Some((_, origin, absolute, duration)) =
                        places.iter().find(|place| place.0 == member.id).copied()
                    else {
                        continue;
                    };
                    let source_in = member.data.get("source_in").cloned().unwrap_or(Json::Null);
                    member.data = crate::clocks_of(&member.data, absolute, duration, &source_in);
                    crate::set_clip_name(&mut member, absolute);
                    let Some(track_node) = session.document_mut()?.node_mut(origin) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no track `{origin}`"),
                        });
                    };
                    track_node.add_child(member);
                }
            }
            let members: Vec<u64> = places.iter().map(|place| place.0).collect();
            Ok(Outcome::changed(format!("the clip `{clip}` holds no clips"))
                .with_data(json!({ "clip": clip, "members": members })))

        }
    }
}
