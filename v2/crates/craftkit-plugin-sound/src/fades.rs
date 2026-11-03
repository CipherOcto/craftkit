//! The **fades** of a range of a track.
//!
//! The tree's `create_fades` over a **selection**: a clip that starts
//! inside the range fades in, and a clip that ends inside it fades
//! out. And the tree's `delete_fades` takes them away again.
//!
//! The tree's **crossfade** — the range that covers the boundary
//! between two clips — is a thing of its own and is not here.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::{ClipPayload, FadePayload};

/// The **fades** of a range of a track.
///
/// The tree's `create_fades`: a clip that **starts** inside the range
/// fades in over the rest of the range, and a clip that **ends**
/// inside it fades out over the beginning of the range.
///
/// A range that covers neither a start nor an end is **refused**: the
/// tree says the selection must cover a clip start, an end or the
/// boundary between two clips, and a command that quietly does nothing
/// is a command that answers a question nobody asked.
#[derive(Command)]
#[command(
    id = "sound.track.fade",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Fade"),
    mutates,
    undoable
)]
pub struct FadeRange {
    /// The track the clips are on.
    pub track: NodeRef,
    /// Where the range starts, in samples.
    pub from: Number,
    /// Where the range ends, in samples.
    pub to: Number,
    /// The shape of the curve: linear, equal_power, s_curve, exponential or logarithmic.
    pub shape: Option<Text>,
}

impl FadeRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.fade".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let from = crate::samples(self.from.0, "where the range starts")?;
        let to = crate::samples(self.to.0, "where the range ends")?;
        if to <= from {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the range is empty: it is from {from} to {to}"),
            });
        }
        let shape = match &self.shape {
            Some(shape) => {
                craftkit_audio::FadeShape::from_label(&shape.0).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no fade shape called `{}`", shape.0),
                })?
            }
            None => craftkit_audio::FadeShape::default(),
        };
        let document = session.document().clone();
        let clips = clips_of(&document, track);
        // The tree's rule: a clip that starts inside the range fades
        // in, and a clip that ends inside it fades out.
        let mut made: Vec<(u64, ClipPayload)> = Vec::new();
        for (clip_id, mut clip) in clips {
            let end = clip.start + clip.length;
            if from <= clip.start && to > clip.start && to < end {
                clip.fade_in = FadePayload { len: to - clip.start, shape, curvature: 0.0 };
                made.push((clip_id, clip));
            } else if to >= end && from > clip.start && from < end {
                clip.fade_out = FadePayload { len: end - from, shape, curvature: 0.0 };
                made.push((clip_id, clip));
            }
        }
        if made.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the range covers neither the start nor the end of a clip".to_string(),
            });
        }
        for (clip_id, clip) in &made {
            if let Some(node) = session.document_mut()?.node_mut(*clip_id) {
                node.data = clip.to_json();
            }
        }
        let count = made.len();
        Ok(Outcome::changed(format!("faded {count} clip(s) of the track `{track}`")).with_data(
            serde_json::json!({
                "track": track,
                "from": from,
                "to": to,
                "clips": made.iter().map(|(clip_id, _)| *clip_id).collect::<Vec<u64>>(),
            }),
        ))
    }
}

/// **Take away** the fades of a range of a track.
///
/// The tree's `delete_fades`: every clip that touches the range loses
/// its fade in and its fade out.
#[derive(Command)]
#[command(
    id = "sound.track.fade.delete",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Fade"),
    mutates,
    undoable
)]
pub struct DeleteRangeFades {
    /// The track the clips are on.
    pub track: NodeRef,
    /// Where the range starts, in samples.
    pub from: Number,
    /// Where the range ends, in samples.
    pub to: Number,
}

impl DeleteRangeFades {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.fade.delete".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let from = crate::samples(self.from.0, "where the range starts")?;
        let to = crate::samples(self.to.0, "where the range ends")?;
        if to < from {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the range is empty: it is from {from} to {to}"),
            });
        }
        let document = session.document().clone();
        let clips = clips_of(&document, track);
        // Every clip that **touches** the range loses its fades.
        let mut touched: Vec<(u64, ClipPayload)> = Vec::new();
        let mut count = 0usize;
        for (clip_id, mut clip) in clips {
            let end = clip.start + clip.length;
            let overlaps = clip.start < to && from < end;
            let holds = to == from && clip.start <= from && from < end;
            if !overlaps && !holds {
                continue;
            }
            if clip.fade_in.len > 0 || clip.fade_out.len > 0 {
                count += 1;
            }
            clip.fade_in = FadePayload::none();
            clip.fade_out = FadePayload::none();
            touched.push((clip_id, clip));
        }
        for (clip_id, clip) in &touched {
            if let Some(node) = session.document_mut()?.node_mut(*clip_id) {
                node.data = clip.to_json();
            }
        }
        Ok(Outcome::changed(format!("took away the fades of {count} clip(s) of the track `{track}`"))
            .with_data(serde_json::json!({
                "track": track,
                "from": from,
                "to": to,
                "clips": touched.iter().map(|(clip_id, _)| *clip_id).collect::<Vec<u64>>(),
            })))
    }
}

/// The **clips** of a track, in the order they are played.
fn clips_of(document: &Document, track: u64) -> Vec<(u64, ClipPayload)> {
    let mut clips: Vec<(u64, ClipPayload)> = Vec::new();
    for clip in document.walk().filter(|node| node.kind == "sound.clip") {
        if crate::timeline::track_of(document, clip).map(|owner| owner.id) != Some(track) {
            continue;
        }
        let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
        clips.push((clip.id, payload));
    }
    clips.sort_by_key(|(_, payload)| payload.start);
    clips
}
