//! The **edit decision list** of a sequence.
//!
//! An EDL is a **list of cuts**: it says, for every clip of a
//! sequence, what was cut, where it came from and where it sits on the
//! timeline. It is the oldest interchange format there is — CMX 3600 —
//! and it is a **text**, so it can be read by anything.
//!
//! The list is **derived**: it is worked out from the sequence as it
//! is now, and it is never written down anywhere else.
//!
//! The shape of a line is the tree's shape:
//!
//! ```text
//! TITLE: THE SEQUENCE
//! FCM: NON-DROP FRAME
//!
//! 001  REEL01   V     C        00:00:00:00 00:00:05:00 00:00:00:00 00:00:05:00
//! ```
//!
//! - the **title** is the name of the sequence, or the name that is
//!   given;
//! - the **frame count** is `NON-DROP FRAME`, because the tree's
//!   rates are counted and the list does not drop frames;
//! - every **event** is a number, the **reel** the clip comes from, the
//!   **channel** it is on (`V` for a video track, `A`, `AA`, `A3` for
//!   the audio ones), the **edit** (`C` for a cut) and the four
//!   **timecodes**: the source in and out, and the record in and out.
//!
//! The rules:
//!
//! - a **transition** is a dissolve, and a dissolve is a line this
//!   tree does not write: a sequence with a transition is **refused**,
//!   and the refusal says that the list would be wrong;
//! - a clip with **no item** has no reel and no source, and the list
//!   is refused;
//! - the **reel** is the name of the item, in **eight characters** —
//!   the tree's rule — with everything that is not a letter or a digit
//!   turned into an underscore.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The most a **reel** may be long: the tree's rule.
const MAX_REEL: usize = 8;

/// The rate a sequence with **no rate** runs at: the tree's own.
const DEFAULT_RATE: (i64, i64) = (25, 1);

/// A **timecode**: hours, minutes, seconds and frames.
fn timecode(frames: u64, per_second: u64) -> String {
    let per_second = per_second.max(1);
    let frames = frames as u128;
    let per_second = per_second as u128;
    let total_seconds = frames / per_second;
    let whole = (frames % per_second) as u64;
    let seconds = (total_seconds % 60) as u64;
    let minutes = ((total_seconds / 60) % 60) as u64;
    let hours = (total_seconds / 3600) as u64;
    format!("{hours:02}:{minutes:02}:{seconds:02}:{whole:02}")
}

/// The **reel** of an item: its name, in eight characters.
fn reel_of(name: &str) -> String {
    let mut reel = String::new();
    for character in name.trim().chars() {
        if reel.chars().count() >= MAX_REEL {
            break;
        }
        if character.is_ascii_alphanumeric() {
            reel.push(character.to_ascii_uppercase());
            continue;
        }
        reel.push('_');
    }
    let reel = reel.trim_matches('_').to_string();
    if reel.is_empty() { "REEL".to_string() } else { reel }
}

/// The **channel** of a track: `V` for a video track, and the tree's
/// own names for the audio ones.
fn channel_of(track: &Node, audio: usize) -> String {
    let kind = track.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default();
    if kind == "audio" {
        return match audio {
            0 => "A".to_string(),
            1 => "AA".to_string(),
            other => format!("A{}", other + 1),
        };
    }
    "V".to_string()
}

/// The **rate** of a sequence, and the frames of a second it counts.
fn rate_of(sequence: &Node) -> (i64, i64) {
    let stored = sequence.data.get("rate");
    let number = |name: &str| stored.and_then(|rate| rate.get(name)).and_then(|value| value.as_i64());
    match (number("num"), number("den")) {
        (Some(num), Some(den)) if num > 0 && den > 0 => (num, den),
        _ => DEFAULT_RATE,
    }
}

/// The **item** a clip plays.
fn item_of(document: &craftkit_sdk::Document, clip: u64) -> Option<Node> {
    let item = document
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)?;
    document.node(item).cloned()
}

/// **Write** the edit decision list of a sequence.
///
/// The list is derived from the sequence as it is now, and it is
/// returned as a text.
#[derive(Command)]
#[command(id = "film.export.edl", area = "film", layer = "film", icon = "list")]
pub struct ExportEdl {
    /// The sequence.
    pub sequence: NodeRef,
    /// The title of the list. Without it, the name of the sequence.
    pub title: Option<Text>,
}

impl ExportEdl {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.export.edl".to_string();
        let sequence = self.sequence.0;
        let Some(node) = session.document().node(sequence) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sequence `{sequence}`"),
            });
        };
        if node.kind != "film.item" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sequence}` is a `{}`, and a sequence is an item", node.kind),
            });
        }
        let title = match &self.title {
            Some(title) => title.0.trim().to_string(),
            None => node.name.clone(),
        };
        if title.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a list has a title".to_string() });
        }
        let (num, den) = rate_of(node);
        let per_second = ((num as f64 / den as f64).round() as u64).max(1);
        let document = session.document();
        // The **tracks**: the video ones first, and then the audio
        // ones, and each of them in the order it is in the sequence.
        let mut tracks: Vec<std::sync::Arc<Node>> =
            node.children.iter().filter(|child| child.kind == "film.track").cloned().collect();
        let is_audio = |track: &Node| {
            track.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default() == "audio"
        };
        tracks.sort_by_key(|track| is_audio(track.as_ref()));
        let mut lines: Vec<String> = Vec::new();
        let mut number = 0usize;
        let mut audio = 0usize;
        for track in tracks {
            if is_audio(track.as_ref()) {
                audio += 1;
            }
            // A **transition** is a dissolve, and this list does not
            // write dissolves: the list would be wrong.
            if track.children.iter().any(|child| child.kind == "film.transition") {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the track `{}` has a transition, and a transition is a dissolve, and this list does not write dissolves",
                        track.name
                    ),
                });
            }
            let channel = channel_of(track.as_ref(), audio.saturating_sub(1));
            let mut clips: Vec<&std::sync::Arc<Node>> =
                track.children.iter().filter(|child| child.kind == "film.clip").collect();
            clips.sort_by_key(|clip| clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
            for clip in clips {
                let Some(item) = item_of(document, clip.id) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{}` has no item, and it has no reel", clip.name),
                    });
                };
                let start = clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration = clip.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                let source_in =
                    clip.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
                number += 1;
                lines.push(format!(
                    "{number:03}  {:<MAX_REEL$} {:<4} {:<8} {} {} {} {}",
                    reel_of(&item.name),
                    channel,
                    "C",
                    timecode(source_in, per_second),
                    timecode(source_in + duration, per_second),
                    timecode(start, per_second),
                    timecode(start + duration, per_second),
                ));
            }
        }
        let mut text = format!("TITLE: {title}\nFCM: NON-DROP FRAME\n");
        if !lines.is_empty() {
            text.push('\n');
            text.push_str(&lines.join("\n"));
            text.push('\n');
        }
        Ok(Outcome::new(format!("the list of the sequence `{title}`")).with_data(json!({
            "sequence": sequence,
            "title": title,
            "edl": text,
            "events": number,
            "rate": { "num": num, "den": den },
        })))
    }
}
