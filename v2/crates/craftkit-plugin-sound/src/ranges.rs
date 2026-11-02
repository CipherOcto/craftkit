//! The **range** of a track: a place and a length, and the material
//! that is in it.
//!
//! A range is a fact about a track, and the same range is used by
//! every command that works on one: the **repeat** lays the material
//! of a range again, the **copy** puts it in the clipboard and the
//! **cut** takes it away. They share this module so that a range means
//! one thing and not three.

use craftkit_sdk::{EngineError, Number, Session};

use crate::ClipPayload;

/// The **clips** of a track, with their payloads.
pub(crate) fn clips_of_track(session: &Session, track: u64) -> Vec<(u64, ClipPayload)> {
    let document = session.document();
    let mut clips = Vec::new();
    for clip in document.walk().filter(|node| node.kind == "sound.clip") {
        if crate::timeline::track_of(document, clip).map(|owner| owner.id) != Some(track) {
            continue;
        }
        let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
        clips.push((clip.id, payload));
    }
    clips
}

/// The **range** of a track: where it starts and where it ends.
///
/// Without a place, the range is the whole of the track's material:
/// from the beginning of its first clip to the end of its last. A
/// range with **no length** is refused: there is nothing in it.
pub(crate) fn range_of(
    session: &Session,
    track: u64,
    id: &str,
    start: Option<Number>,
    end: Option<Number>,
) -> Result<(i64, i64), EngineError> {
    let clips = clips_of_track(session, track);
    let mut last = 0_i64;
    for (_, payload) in &clips {
        last = last.max(payload.start.max(0) + payload.length.max(0));
    }
    let start = match start {
        Some(start) => crate::samples(start.0, "where the range starts")?,
        None => 0,
    };
    let end = match end {
        Some(end) => crate::samples(end.0, "where the range ends")?,
        None => last,
    };
    if end <= start {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the range ends at {end} and starts at {start}"),
        });
    }
    Ok((start, end))
}

/// The **copies** of the material of a range: each clip of the track
/// that overlaps the range, trimmed to it and moved to the range's own
/// head.
///
/// A copy that is trimmed at its head loses its **fade in** — the fade
/// was of the material before it — and a copy that is trimmed at its
/// tail loses its **fade out**. The **beat** follows the audio: a
/// beat is a place in the clip, and the clip's head has moved.
pub(crate) fn copies_of_range(
    session: &Session,
    track: u64,
    start: i64,
    end: i64,
) -> Vec<(String, ClipPayload)> {
    let mut copies = Vec::new();
    for (clip_id, payload) in clips_of_track(session, track) {
        let clip_start = payload.start.max(0);
        let clip_end = clip_start + payload.length.max(0);
        if clip_end <= start || clip_start >= end {
            continue;
        }
        let mut copy = payload.clone();
        if clip_start < start {
            let cut = start - clip_start;
            copy.length = (copy.length - cut).max(0);
            copy.offset += cut;
            copy.fade_in = crate::FadePayload::none();
            copy.sync = copy.sync.map(|beat| beat - cut);
        }
        if clip_end > end {
            let cut = clip_end - end;
            copy.length = (copy.length - cut).max(0);
            copy.fade_out = crate::FadePayload::none();
        }
        copy.start = clip_start.max(start) - start;
        let name = session
            .document()
            .node(clip_id)
            .map(|node| node.name.clone())
            .unwrap_or_else(|| format!("clip {clip_id}"));
        copies.push((name, copy));
    }
    copies
}

/// The **automation** of a range: the breakpoints of the track's lanes
/// that fall inside it, moved to the range's own head.
///
/// A lane automates one thing and the tree's copy of a range carries
/// the lanes with it, so that a paste brings the curve as well as the
/// clips. A breakpoint outside the range is not carried: it belongs to
/// the material around it.
pub(crate) fn automation_of_range(
    session: &Session,
    track: u64,
    start: i64,
    end: i64,
) -> Vec<(String, Vec<(i64, f64)>)> {
    let document = session.document();
    let mut lanes = Vec::new();
    for lane in document.walk().filter(|node| node.kind == "sound.lane") {
        if crate::timeline::parent_of(document, lane.id).map(|parent| parent.id) != Some(track) {
            continue;
        }
        let Ok(payload) = crate::mixer::LanePayload::from_json(&lane.data) else { continue };
        let points: Vec<(i64, f64)> = payload
            .points
            .iter()
            .filter(|(at, _)| *at >= start && *at < end)
            .map(|(at, value)| (at - start, *value))
            .collect();
        if points.is_empty() {
            continue;
        }
        lanes.push((payload.param, points));
    }
    lanes
}

/// Take the material of a range **away**: the clips inside it are
/// thrown away, and the clips that cross its edge keep the part
/// outside it.
///
/// The number of clips that were thrown away is given back.
pub(crate) fn clear_range_of(session: &mut Session, track: u64, start: i64, end: i64) -> usize {
    let clips = clips_of_track(session, track);
    let mut removed = 0_usize;
    for (clip_id, payload) in clips {
        let clip_start = payload.start.max(0);
        let clip_end = clip_start + payload.length.max(0);
        if clip_end <= start || clip_start >= end {
            continue;
        }
        if clip_start >= start && clip_end <= end {
            if let Ok(document) = session.document_mut() {
                document.remove_node(clip_id);
            }
            removed += 1;
            continue;
        }
        // The part **outside** the range is kept.
        let mut payload = payload;
        if clip_start < start {
            payload.length = (start - clip_start).max(0);
        } else {
            let cut = end - clip_start;
            payload.offset += cut;
            payload.start = end;
            payload.length = (clip_end - end).max(0);
        }
        if let Ok(Some(node)) = session.document_mut().map(|document| document.node_mut(clip_id)) {
            node.data = payload.to_json();
        }
    }
    removed
}
