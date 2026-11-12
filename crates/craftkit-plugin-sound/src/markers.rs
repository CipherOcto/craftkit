//! The **markers** of the timeline: the places a person has marked, and
//! the way the playhead goes to them.
//!
//! This is not the module `marks`: a **mark** is what a person has said
//! about a clip — a rating, a colour — and a **marker** is a named
//! place on the timeline.
//!
//! The tree's own `markers.recall`, `markers.next` and
//! `markers.previous`.
//!
//! # The tree's rules
//!
//! - a marker is **recalled** by its number, its id or its name: the
//!   recall puts the playhead at the marker's start and chooses the
//!   marker;
//! - the **next** marker is the one that starts **after** where the
//!   selection is, and the **previous** one is the one that starts
//!   **before** it;
//! - a step that has nowhere to go **says so** and changes nothing.
//!
//! # What the port found
//!
//! - **The tree's markers are a list and this tree's are nodes.** The
//!   tree keeps its markers in a list of the session, and each of them
//!   has a number, a name, a colour and a comment. Here a marker is a
//!   **node** with a name and a place, and it is chosen the way every
//!   other node is chosen. So the tree's *number* is not here: the
//!   node is the marker and the name finds it.
//! - **The tree's step reads the selection and this tree's reads the
//!   playhead.** The tree keeps a selection of time and a playhead, and
//!   it steps from the selection; here the playhead is the place, and
//!   it is both what is read and what is moved.
//! - **The tree's transport is asked to locate and here the playhead is
//!   a node.** The tree pushes a *locate* to its transport; here the
//!   playhead is a node in the document and it is moved, so the move is
//!   in the history and it may be undone.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Either, EngineError, NodeRef, Outcome, Session, Text};

use crate::MarkerPayload;

/// **Recall** a marker: the playhead goes to it.
///
/// The tree's own `markers.recall`: the playhead is put at the marker's
/// start, and the marker is chosen.
#[derive(Command)]
#[command(
    id = "sound.marker.recall",
    area = "sound",
    layer = "sound",
    icon = "play",
    menu("Marker"),
    mutates,
    undoable
)]
pub struct RecallMarker {
    /// The marker: the node, or the name it is called.
    #[param(or = "name")]
    pub marker: Either<NodeRef, Text>,
}

/// **Step** to the next marker.
///
/// The tree's own `markers.next`: the marker that starts after where the
/// playhead is.
#[derive(Command)]
#[command(
    id = "sound.marker.next",
    area = "sound",
    layer = "sound",
    icon = "play",
    menu("Marker"),
    mutates,
    undoable
)]
pub struct NextMarker {}

/// **Step** to the previous marker.
///
/// The tree's own `markers.previous`: the marker that starts before
/// where the playhead is.
#[derive(Command)]
#[command(
    id = "sound.marker.previous",
    area = "sound",
    layer = "sound",
    icon = "play",
    menu("Marker"),
    mutates,
    undoable
)]
pub struct PreviousMarker {}

/// The **markers** of the document, by where they start.
fn markers(session: &Session) -> Vec<(u64, String, i64)> {
    let mut list: Vec<(u64, String, i64)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.marker")
        .filter_map(|node| {
            let payload = MarkerPayload::from_json(&node.data).ok()?;
            Some((node.id, node.name.clone(), payload.start))
        })
        .collect();
    list.sort_by_key(|(_, _, start)| *start);
    list
}

/// Where the **playhead** is.
fn playhead(session: &Session) -> Option<i64> {
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.playhead")
        .and_then(crate::timeline::playhead_position)
}

/// Put the **playhead** where it is.
fn move_playhead(session: &mut Session, position: i64) -> Result<(), EngineError> {
    let existing = session.document().walk().find(|node| node.kind == "sound.playhead").map(|node| node.id);
    match existing {
        Some(id) => {
            if let Some(node) = session.document_mut()?.node_mut(id) {
                node.data = json!({ "position": position });
            }
        }
        None => {
            session.document_mut()?.add_node_with(
                "playhead".to_string(),
                "sound.playhead",
                None,
                json!({ "position": position }),
            );
        }
    }
    Ok(())
}

/// The **marker** a command was told about: the node, or the name.
fn marker_of(
    session: &Session,
    marker: &Either<NodeRef, Text>,
    id: &str,
) -> Result<(u64, String), EngineError> {
    match marker {
        Either::Left(reference) => {
            let Some(node) = session.document().node(reference.0) else {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("there is no node `{}`", reference.0),
                });
            };
            if node.kind != "sound.marker" {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("the node `{}` is a `{}` and not a marker", reference.0, node.kind),
                });
            }
            Ok((reference.0, node.name.clone()))
        }
        Either::Right(name) => {
            let wanted = name.0.clone();
            let found =
                session.document().walk().find(|node| node.kind == "sound.marker" && node.name == wanted);
            match found {
                Some(node) => Ok((node.id, node.name.clone())),
                None => Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("there is no marker called `{wanted}`"),
                }),
            }
        }
    }
}

/// The **start** of a marker.
fn start_of(session: &Session, marker: u64, id: &str) -> Result<i64, EngineError> {
    let Some(node) = session.document().node(marker) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no node `{marker}`"),
        });
    };
    MarkerPayload::from_json(&node.data)
        .map(|payload| payload.start)
        .map_err(|error| EngineError::Failed { id: id.to_string(), message: error })
}

impl RecallMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.marker.recall".to_string();
        let (marker, name) = marker_of(session, &self.marker, &id)?;
        let start = start_of(session, marker, &id)?;
        move_playhead(session, start)?;
        session.select_ids(&[marker]);
        Ok(Outcome::changed(format!("the playhead is at the marker `{name}`"))
            .with_data(json!({ "marker": marker, "name": name, "at": start, "position": start })))
    }
}

/// The **step**: the playhead goes to the marker on that side of it.
///
/// A step that has nowhere to go **says so** and changes nothing: the
/// tree's own rule.
fn step(session: &mut Session, forward: bool) -> Result<Outcome, EngineError> {
    let current = playhead(session).unwrap_or(0);
    let list = markers(session);
    // The **next** marker is the one that starts after where the
    // playhead is, and the **previous** one is the one that starts
    // before it: the tree's own rule, and the tree's strictness.
    let target = if forward {
        list.iter().find(|(_, _, start)| *start > current)
    } else {
        list.iter().rev().find(|(_, _, start)| *start < current)
    };
    let Some((marker, name, start)) = target.cloned() else {
        return Ok(Outcome::new(if forward {
            "there is no marker after the playhead".to_string()
        } else {
            "there is no marker before the playhead".to_string()
        })
        .with_data(json!({ "moved": false, "at": current })));
    };
    move_playhead(session, start)?;
    session.select_ids(&[marker]);
    Ok(Outcome::changed(format!("the playhead is at the marker `{name}`"))
        .with_data(json!({ "moved": true, "marker": marker, "name": name, "at": start })))
}

impl NextMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        step(session, true)
    }
}

impl PreviousMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        step(session, false)
    }
}
