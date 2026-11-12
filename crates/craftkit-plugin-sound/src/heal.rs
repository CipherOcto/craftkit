//! The **healing** of a track: the clips that were separated are put
//! back together.
//!
//! The tree's editor heals a separation: it walks the clips of a track
//! in order and joins each one with the one before it when they are
//! **the same sound continued** — the same source, the second starting
//! where the first's sound carries on, and the same way of playing it.
//!
//! # What "the same sound continued" is
//!
//! Four things, and all four of them must hold:
//!
//! - the **same source**: the two play the same sound;
//! - the **offset lines up**: the second starts in the source where
//!   the first's sound carries on — the distance the cut made, plus
//!   any gap on the timeline, **divided by the stretch**, because a
//!   stretched clip's source is read more slowly;
//! - the **same way of playing it**: the same mute, the same stretch,
//!   the same gain and the same pitch;
//! - and the second must **not start before** the first ends: two
//!   clips that overlap are not a separation.
//!
//! # What the joined clip is
//!
//! The **first** clip, made longer: it keeps its own head, its own
//! fade in and its own beat, and it takes the second's fade out — the
//! tail of the sound is the tail of the sound whichever piece it was
//! in.
//!
//! The second clip's **lanes** move into the first and their
//! breakpoints move with them: a lane that was over the second piece
//! is over the second piece of the sound, and the sound is now one
//! clip.

use std::sync::Arc;

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session};

use crate::ClipPayload;
use crate::mixer::LanePayload;

/// **Heal** a track: the clips that were separated are joined.
///
/// The clips of the track are walked in order and each one is joined
/// with the one before it when they are the same sound continued. The
/// answer says how many joins were made.
#[derive(Command)]
#[command(
    id = "sound.clip.heal",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Heal"),
    mutates,
    undoable
)]
pub struct HealTrack {
    /// The track to heal.
    pub track: NodeRef,
}

impl HealTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.heal".to_string();
        let track_id = self.track.0;
        let node = session.document().node(track_id).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track_id}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track_id}` is a `{}` and not a track", node.kind),
            });
        }
        let playlists: Vec<u64> = node
            .children
            .iter()
            .filter(|child| child.kind == "sound.playlist")
            .map(|child| child.id)
            .collect();
        let mut healed = 0;
        for playlist in playlists {
            healed += heal_playlist(session.document_mut()?, playlist);
        }
        Ok(Outcome::changed(format!("healed {healed} separations of the track `{track_id}`"))
            .with_data(serde_json::json!({ "track": track_id, "healed": healed })))
    }
}

/// The clips of a playlist, in the order they are on the timeline.
fn clips_of(document: &Document, playlist: u64) -> Vec<(u64, ClipPayload)> {
    let mut clips: Vec<(u64, ClipPayload)> = document
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .filter(|node| {
            crate::timeline::parent_of(document, node.id).map(|parent| parent.id) == Some(playlist)
        })
        .filter_map(|node| ClipPayload::from_json(&node.data).ok().map(|payload| (node.id, payload)))
        .collect();
    clips.sort_by_key(|(_, payload)| payload.start);
    clips
}

/// Joins the clips of a playlist, and says how many joins were made.
fn heal_playlist(document: &mut Document, playlist: u64) -> usize {
    let clips = clips_of(document, playlist);
    let mut healed = 0;
    // The clip that is left, and the one that may join it.
    let mut kept: Option<(u64, ClipPayload)> = None;
    for (clip_id, payload) in clips {
        match kept.take() {
            Some((kept_id, kept_payload)) => {
                if continues(&kept_payload, &payload) {
                    join(document, kept_id, &kept_payload, clip_id, &payload);
                    healed += 1;
                    // The joined clip is the one that is kept, and the
                    // next clip may join it.
                    kept = Some((kept_id, joined(&kept_payload, &payload)));
                } else {
                    // The two are apart, and the second may join the
                    // next one.
                    kept = Some((clip_id, payload));
                }
            }
            None => kept = Some((clip_id, payload)),
        }
    }
    healed
}

/// How many source samples a length of timeline is, at a stretch.
///
/// A stretched clip's source is read more slowly, so a length of
/// timeline is fewer samples of the source.
fn source_len(length: i64, stretch: f64) -> i64 {
    if stretch.is_finite() && stretch > 0.0 && (stretch - 1.0).abs() >= 1e-9 {
        (length as f64 / stretch).round() as i64
    } else {
        length
    }
}

/// Whether the second clip is **the same sound continued** by the
/// first.
fn continues(first: &ClipPayload, second: &ClipPayload) -> bool {
    // Two clips that overlap are not a separation.
    let end = first.start + first.length;
    if second.start < end {
        return false;
    }
    let gap = second.start - end;
    // The same source, and the second starting where the first's sound
    // carries on.
    let (Some(source), Some(other)) = (first.source, second.source) else { return false };
    if source != other {
        return false;
    }
    if second.offset != first.offset + source_len(first.length + gap, first.stretch) {
        return false;
    }
    // The same way of playing it.
    first.muted == second.muted
        && (first.stretch - second.stretch).abs() < 1e-9
        && (first.gain_db - second.gain_db).abs() < 1e-6
        && first.gain_bypassed == second.gain_bypassed
        && (first.pitch_semitones - second.pitch_semitones).abs() < 1e-9
}

/// The clip that is left when the two are joined.
fn joined(first: &ClipPayload, second: &ClipPayload) -> ClipPayload {
    let mut joined = first.clone();
    joined.length = second.start + second.length - first.start;
    // The tail of the sound is the tail of the sound whichever piece
    // it was in.
    joined.fade_out = second.fade_out;
    // The **beat**: the first piece's beat is the beat of the sound,
    // and the second piece's beat is dropped unless the first piece
    // has none — then it moves with the sound it is in.
    joined.sync = match first.sync {
        Some(sync) => Some(sync),
        None => second.sync.map(|sync| sync + (second.start - first.start)),
    };
    joined
}

/// Joins the second clip into the first, in the document.
fn join(document: &mut Document, first_id: u64, first: &ClipPayload, second_id: u64, second: &ClipPayload) {
    let seam = second.start - first.start;
    // The **lanes** of the second clip move into the first, and their
    // breakpoints move with them: a lane over the second piece is
    // over the second piece of the sound.
    let lanes: Vec<u64> = document
        .walk()
        .filter(|node| node.kind == "sound.lane")
        .filter(|node| {
            crate::timeline::parent_of(document, node.id).map(|parent| parent.id) == Some(second_id)
        })
        .map(|node| node.id)
        .collect();
    for lane in lanes {
        let Some(node) = document.take_node(lane) else { continue };
        let mut node = node;
        if let Ok(mut payload) = LanePayload::from_json(&node.data) {
            payload.points = payload.points.into_iter().map(|(at, value)| (at + seam, value)).collect();
            node.data = payload.to_json();
        }
        if let Some(parent) = document.node_mut(first_id) {
            parent.children.push(Arc::new(node));
        }
    }
    // The first clip is now the joined one.
    if let Some(node) = document.node_mut(first_id) {
        node.data = joined(first, second).to_json();
    }
    // And the second clip is gone.
    document.remove_node(second_id);
}
