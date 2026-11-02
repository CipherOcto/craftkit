//! The **repeating** of a range: the material of a range, again.
//!
//! The tree's editor repeats a selection: it copies the material of
//! the range and lays the copies after it, as many times as the caller
//! asks for.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

use crate::ClipPayload;

/// The most times a range may be repeated.
///
/// The tree's repeat is a number from one to nine hundred and
/// ninety-nine, and the bound is here too: a repeat of ten thousand
/// times is a track of ten thousand times the material.
const MAX_REPEATS: u32 = 999;

/// **Repeat** the material of a range of a track.
///
/// The clips of the track that overlap the range are **copied** — each
/// copy trimmed to the range and moved to the range's own head — and
/// the copies are laid after the range, as many times as the caller
/// asks for.
///
/// The copies keep everything the clips had: their gain, their fades,
/// their stretch, their pitch and their processes. A copy that is
/// trimmed at its head loses its **fade in** — the fade was of the
/// material before it — and a copy that is trimmed at its tail loses
/// its **fade out**.
///
/// The copies are laid **over** what is there, as the loop's copies
/// are (§ the loop of a clip): the tree's paste clears what it covers,
/// and this one does not, because a document that throws away
/// material nobody asked it to throw away is a document that surprises.
#[derive(Command)]
#[command(
    id = "sound.track.repeat",
    area = "sound",
    layer = "sound",
    icon = "loop",
    menu("Track", "Repeat"),
    mutates,
    undoable
)]
pub struct RepeatRange {
    /// The track whose material is repeated.
    pub track: NodeRef,
    /// Where the range starts, in samples.
    pub start: Option<Number>,
    /// Where the range ends, in samples.
    pub end: Option<Number>,
    /// How many times the range is repeated.
    ///
    /// One is none. The number is **clamped** into its bounds: it is a
    /// knob and not a fact about the material.
    pub count: Option<Number>,
}

impl RepeatRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.repeat".to_string();
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
        // The **count**: how many times, and it is a knob.
        let count = match self.count {
            Some(count) if count.0.is_finite() => (count.0 as u32).clamp(1, MAX_REPEATS),
            Some(_) => {
                return Err(EngineError::Failed {
                    id,
                    message: "the count is a number of times".to_string(),
                });
            }
            None => 1,
        };
        let document = session.document().clone();
        // The **clips** of the track: the ones that overlap the range
        // are the ones that are copied.
        let mut clips: Vec<(u64, ClipPayload)> = Vec::new();
        let mut last = 0_i64;
        for clip in document.walk().filter(|node| node.kind == "sound.clip") {
            if crate::timeline::track_of(&document, clip).map(|owner| owner.id) != Some(track) {
                continue;
            }
            let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
            last = last.max(payload.start.max(0) + payload.length.max(0));
            clips.push((clip.id, payload));
        }
        let start = match &self.start {
            Some(start) => crate::samples(start.0, "where the range starts")?,
            None => 0,
        };
        let end = match &self.end {
            Some(end) => crate::samples(end.0, "where the range ends")?,
            None => last,
        };
        if end <= start {
            return Err(EngineError::Failed {
                id,
                message: format!("the range ends at {end} and starts at {start}"),
            });
        }
        let length = end - start;
        // The **copies**: each clip of the range, trimmed to the range
        // and moved to the range's head.
        let mut copies: Vec<(String, ClipPayload)> = Vec::new();
        for (clip_id, payload) in &clips {
            let clip_start = payload.start.max(0);
            let clip_end = clip_start + payload.length.max(0);
            if clip_end <= start || clip_start >= end {
                continue;
            }
            let mut copy = payload.clone();
            // The **head** of the copy: where the range cuts it.
            if clip_start < start {
                let cut = start - clip_start;
                copy.length = (copy.length - cut).max(0);
                copy.offset += cut;
                // The **fade in** was of the material before it.
                copy.fade_in = crate::FadePayload::none();
                // The **beat** follows the audio: a beat is a place in
                // the clip, and the clip's head has moved.
                copy.sync = copy.sync.map(|beat| beat - cut);
            }
            // The **tail** of the copy: where the range cuts it.
            if clip_end > end {
                let cut = clip_end - end;
                copy.length = (copy.length - cut).max(0);
                // The **fade out** was of the material after it.
                copy.fade_out = crate::FadePayload::none();
            }
            // The copy is measured from the **range's** head.
            copy.start = clip_start.max(start) - start;
            let name = session
                .document()
                .node(*clip_id)
                .map(|node| node.name.clone())
                .unwrap_or_else(|| format!("clip {clip_id}"));
            copies.push((name, copy));
        }
        if copies.is_empty() {
            return Ok(Outcome::new("there is no material in the range to repeat")
                .with_data(serde_json::json!({ "repeats": 0, "clips": Vec::<u64>::new() })));
        }
        let playlist = crate::playlist_of(session, track, None)?;
        // The **repeats**: the copies are laid after the range, again
        // and again.
        let mut made = Vec::new();
        for step in 1..=count {
            let at = start + length * step as i64;
            for (name, copy) in &copies {
                let mut placed = copy.clone();
                placed.start = at + copy.start;
                let placed_id = session.document_mut()?.add_node_with_under(
                    Some(playlist),
                    name.clone(),
                    "sound.clip",
                    None,
                    placed.to_json(),
                );
                // The **source** relation goes with the copy: a copy
                // that plays nothing is a copy of nothing.
                if let Some(source) = placed.source {
                    session
                        .document_mut()?
                        .add_relation(craftkit_sdk::Relation::new(placed_id, source, "source"));
                }
                made.push(placed_id);
            }
        }
        Ok(Outcome::changed(format!("repeated the range {count} time(s)"))
            .with_data(serde_json::json!({ "repeats": count, "clips": made })))
    }
}
