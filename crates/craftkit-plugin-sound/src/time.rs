//! The **time of the session**: inserting silence into it, and cutting
//! time out of it.
//!
//! The tree's editor can **insert time** at a place and **cut time**
//! out of a range. Both are changes to the whole of the session's
//! timeline: every track moves, every lane moves and every marker
//! moves. The tree's commands take a **selection** and do this to
//! every track; here the place is **given** and the whole document is
//! the thing that moves.
//!
//! What the tree does and this tree does, in one line:
//!
//! - **insert**: the clips at or after the place move later by the
//!   length; the lanes' breakpoints at or after it move later; the
//!   markers at or after it move later.
//! - **cut**: the clips inside the range are thrown away and the ones
//!   after it move earlier by the length of the range; the lanes'
//!   breakpoints inside the range are thrown away and the ones after
//!   it move earlier; the markers inside the range are thrown away and
//!   the ones after it move earlier.
//!
//! A **locked** clip does not move: a lock says the clip stays where
//! it is, and a clip that is locked against editing is finished.

use craftkit_derive::Command;
use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, Number, Outcome, Session};

use crate::ClipPayload;
use crate::MarkerPayload;
use crate::mixer::LanePayload;

/// The clips of the document, by their id.
fn clips_of(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "sound.clip").map(|node| node.id).collect()
}

/// The lanes of the document, by their id.
fn lanes_of(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "sound.lane").map(|node| node.id).collect()
}

/// The markers of the document, by their id.
fn markers_of(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "sound.marker").map(|node| node.id).collect()
}

/// **Insert time** into the session: silence at a place, and everything
/// after it moves later.
///
/// The tree's editor inserts time at a place in the selection, and
/// every track of the session moves: the clips at or after the place
/// move later by the length of the silence, the lanes' breakpoints at
/// or after it move later, and the markers at or after it move later.
///
/// Here the place is **given** — a place is a fact, and the tree's
/// selection is the editor's state, which this tree does not keep
/// (§ the grid) — and the whole of the document is the thing that
/// moves.
///
/// A **locked** clip does not move: a lock says the clip stays where it
/// is.
#[derive(Command)]
#[command(
    id = "sound.time.insert",
    area = "sound",
    layer = "sound",
    icon = "move",
    menu("Event", "Time"),
    mutates,
    undoable
)]
pub struct InsertTime {
    /// Where the silence goes, in samples.
    pub at: Number,
    /// How much silence there is, in samples.
    pub length: Number,
}

impl InsertTime {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.time.insert".to_string();
        let at = crate::samples(self.at.0, "where the silence goes")?;
        let length = crate::samples(self.length.0, "how much silence there is")?;
        if length <= 0 {
            return Err(EngineError::Failed {
                id,
                message: "the silence is a length of samples, and not nothing".to_string(),
            });
        }
        // The **clips** that move: the ones at or after the place, and
        // not the locked ones.
        let mut moved = 0;
        for clip in clips_of(session) {
            let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            })?;
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if payload.start < at {
                continue;
            }
            if crate::locks::edit_is_locked(&payload) || crate::locks::time_is_locked(&payload) {
                continue;
            }
            let mut moved_payload = payload.clone();
            moved_payload.start += length;
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = moved_payload.to_json();
            }
            moved += 1;
        }
        // The **lanes**: the breakpoints at or after the place move
        // later, and the ones before it stay.
        let mut lanes = 0;
        for lane in lanes_of(session) {
            let node = session.document().node(lane).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no lane `{lane}`"),
            })?;
            let mut payload = LanePayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            let mut changed = false;
            for point in &mut payload.points {
                if point.0 >= at {
                    point.0 += length;
                    changed = true;
                }
            }
            if changed {
                if let Some(node) = session.document_mut()?.node_mut(lane) {
                    node.data = payload.to_json();
                }
                lanes += 1;
            }
        }
        // The **markers**: the ones at or after the place move later,
        // and they move as a whole — a marker keeps its span.
        let mut markers = 0;
        for marker in markers_of(session) {
            let node = session.document().node(marker).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no marker `{marker}`"),
            })?;
            let mut payload = MarkerPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if payload.start < at {
                continue;
            }
            payload.start += length;
            payload.end += length;
            if let Some(node) = session.document_mut()?.node_mut(marker) {
                node.data = payload.to_json();
            }
            markers += 1;
        }
        Ok(Outcome::changed(format!("inserted {length} samples of silence at {at}")).with_data(
            serde_json::json!({
                "at": at,
                "length": length,
                "clips": moved,
                "lanes": lanes,
                "markers": markers,
            }),
        ))
    }
}

/// **Cut time** out of the session: a range of the timeline is thrown
/// away and everything after it moves earlier.
///
/// The tree's editor cuts time out of a range of the selection, and
/// every track of the session moves: the clips inside the range are
/// thrown away and the ones after it move earlier by the length of the
/// range, the lanes' breakpoints inside the range are thrown away and
/// the ones after it move earlier, and the markers inside the range
/// are thrown away and the ones after it move earlier.
///
/// Here the range is **given** and the whole of the document is the
/// thing that moves.
#[derive(Command)]
#[command(
    id = "sound.time.cut",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Event", "Time"),
    mutates,
    undoable
)]
pub struct CutTime {
    /// Where the range starts, in samples.
    pub start: Number,
    /// Where the range ends, in samples.
    pub end: Number,
}

impl CutTime {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.time.cut".to_string();
        let start = crate::samples(self.start.0, "where the range starts")?;
        let end = crate::samples(self.end.0, "where the range ends")?;
        if end <= start {
            return Err(EngineError::Failed {
                id,
                message: format!("the range from {start} to {end} is empty"),
            });
        }
        let length = end - start;
        // The **clips** inside the range are thrown away, and the ones
        // after it move earlier.
        let mut removed = 0;
        let mut moved = 0;
        for clip in clips_of(session) {
            let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            })?;
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            let clip_end = payload.start + payload.length;
            if payload.start >= start && clip_end <= end {
                session.document_mut()?.remove_node(clip);
                removed += 1;
                continue;
            }
            if payload.start < end {
                // A clip that **crosses** the range is not thrown away:
                // the part of it outside the range is still there. A
                // locked clip stays whole.
                if crate::locks::edit_is_locked(&payload) || crate::locks::time_is_locked(&payload) {
                    continue;
                }
                let mut trimmed = payload.clone();
                if payload.start < start {
                    trimmed.start = start;
                    trimmed.length = clip_end - start;
                } else {
                    trimmed.length = end - payload.start;
                }
                if let Some(node) = session.document_mut()?.node_mut(clip) {
                    node.data = trimmed.to_json();
                }
                continue;
            }
            // A clip after the range moves earlier.
            if crate::locks::edit_is_locked(&payload) || crate::locks::time_is_locked(&payload) {
                continue;
            }
            let mut moved_payload = payload.clone();
            moved_payload.start -= length;
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = moved_payload.to_json();
            }
            moved += 1;
        }
        // The **lanes**: the breakpoints inside the range are thrown
        // away and the ones after it move earlier.
        let mut lanes = 0;
        for lane in lanes_of(session) {
            let node = session.document().node(lane).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no lane `{lane}`"),
            })?;
            let mut payload = LanePayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            let before = payload.points.len();
            payload.points.retain(|(when, _)| *when < start || *when >= end);
            for point in &mut payload.points {
                if point.0 >= end {
                    point.0 -= length;
                }
            }
            if payload.points.len() != before {
                lanes += 1;
            }
            if let Some(node) = session.document_mut()?.node_mut(lane) {
                node.data = payload.to_json();
            }
        }
        // The **markers**: the ones inside the range are thrown away
        // and the ones after it move earlier.
        let mut markers = 0;
        for marker in markers_of(session) {
            let node = session.document().node(marker).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no marker `{marker}`"),
            })?;
            let payload = MarkerPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if payload.start >= start && payload.start < end {
                session.document_mut()?.remove_node(marker);
                markers += 1;
                continue;
            }
            if payload.start < end {
                continue;
            }
            let mut moved_payload = payload.clone();
            moved_payload.start -= length;
            moved_payload.end -= length;
            if let Some(node) = session.document_mut()?.node_mut(marker) {
                node.data = moved_payload.to_json();
            }
            markers += 1;
        }
        Ok(Outcome::changed(format!("cut {length} samples out of the session")).with_data(
            serde_json::json!({
                "start": start,
                "end": end,
                "removed": removed,
                "moved": moved,
                "lanes": lanes,
                "markers": markers,
            }),
        ))
    }
}
