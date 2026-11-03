//! The **fades** of a range of a track.
//!
//! The tree's `create_fades` over a **selection**: a clip that starts
//! inside the range fades in, and a clip that ends inside it fades
//! out. And the tree's `delete_fades` takes them away again.
//!
//! And the tree's **crossfade** is here too: a range that covers the
//! boundary between two clips makes them **overlap**, and each is
//! extended into the other's territory using its **handle** — the
//! material before it and the material after it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::{ClipPayload, FadePayload, SourcePayload};

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
        // The tree's **crossfade** first: a range that covers the
        // boundary between two clips makes them **overlap**, and each
        // is extended into the other's territory using its handle — the
        // material before it and the material after it.
        let mut made: Vec<(u64, ClipPayload)> = Vec::new();
        let mut crossfaded: Vec<u64> = Vec::new();
        for index in 0..clips.len().saturating_sub(1) {
            let (a_id, a) = &clips[index];
            let (b_id, b) = &clips[index + 1];
            let boundary = a.start + a.length;
            if b.start != boundary || from >= boundary || to <= boundary {
                continue;
            }
            let before = boundary - from.max(a.start);
            let after = to.min(b.start + b.length) - boundary;
            let (_, a_after) = handles(&document, a);
            let (b_before, _) = handles(&document, b);
            let extend_a = after.min(a_after).max(0);
            let extend_b = before.min(b_before).max(0);
            // The first clip is longer and fades out over the overlap.
            let mut first = a.clone();
            first.length += extend_a;
            first.fade_out =
                FadePayload { len: (extend_a + before).min(first.length), shape, curvature: 0.0 };
            // The second clip starts earlier and fades in over the
            // overlap, and it plays the material it did not play
            // before.
            let mut second = b.clone();
            second.start -= extend_b;
            second.length += extend_b;
            second.offset -= extend_b;
            second.fade_in =
                FadePayload { len: (extend_b + after).min(second.length), shape, curvature: 0.0 };
            made.push((*a_id, first));
            made.push((*b_id, second));
            crossfaded.push(*a_id);
            crossfaded.push(*b_id);
        }
        // The tree's rule: a clip that starts inside the range fades
        // in, and a clip that ends inside it fades out.
        for (clip_id, mut clip) in clips {
            if crossfaded.contains(&clip_id) {
                continue;
            }
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
                message: "the range covers neither the start nor the end of a clip, nor the boundary between two clips"
                    .to_string(),
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
        // The **overlap** of a crossfade is trimmed back to the
        // midpoint: the two clips are as long as they were, and the
        // overlap is gone.
        let mut clips = clips_of(&document, track);
        for index in 0..clips.len().saturating_sub(1) {
            let (a_id, a) = &clips[index];
            let (b_id, b) = &clips[index + 1];
            let touched_here = touched.iter().any(|(clip_id, _)| clip_id == a_id)
                || touched.iter().any(|(clip_id, _)| clip_id == b_id);
            let a_end = a.start + a.length;
            if !touched_here || a_end <= b.start {
                continue;
            }
            let middle = b.start + (a_end - b.start) / 2;
            // The clip that was **touched** keeps the fades it was
            // given — none — and the one that was not keeps its own.
            let mut first = touched
                .iter()
                .find(|(clip_id, _)| clip_id == a_id)
                .map(|(_, clip)| clip.clone())
                .unwrap_or_else(|| a.clone());
            first.length = middle - first.start;
            if first.length < 1 {
                first.length = 1;
            }
            if let Some(sync) = first.sync
                && sync > first.length
            {
                first.sync = None;
            }
            let mut second = touched
                .iter()
                .find(|(clip_id, _)| clip_id == b_id)
                .map(|(_, clip)| clip.clone())
                .unwrap_or_else(|| b.clone());
            let delta = middle - second.start;
            second.start = middle;
            second.length -= delta;
            second.offset += delta;
            if let Some(node) = session.document_mut()?.node_mut(*a_id) {
                node.data = first.to_json();
            }
            if let Some(node) = session.document_mut()?.node_mut(*b_id) {
                node.data = second.to_json();
            }
            let a_id = *a_id;
            let b_id = *b_id;
            clips[index] = (a_id, first);
            clips[index + 1] = (b_id, second);
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

/// The **handles** of a clip: the material before it and the material
/// after it, in the timeline's samples.
///
/// The tree's handles are the material the clip does not play: the
/// source's frames less what the clip plays for the head, and the
/// clip's offset for the tail. The tree measures them in the source's
/// samples and does not follow the **stretch**; here they are measured
/// in the timeline's samples and the two agree when the clip is not
/// stretched.
fn handles(document: &Document, clip: &ClipPayload) -> (i64, i64) {
    let Some(source) = clip.source else { return (0, 0) };
    let Some(node) = document.node(source) else { return (0, 0) };
    let Ok(payload) = SourcePayload::from_json(&node.data) else { return (0, 0) };
    let stretch = if clip.stretch.is_finite() && clip.stretch > 0.0 { clip.stretch } else { 1.0 };
    let before = (clip.offset as f64 * stretch).round() as i64;
    let played = (clip.length as f64 / stretch).round() as i64;
    let after = ((payload.frames - clip.offset - played) as f64 * stretch).round() as i64;
    (before.max(0), after.max(0))
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
