//! The **ALE** of a sequence: an Avid log exchange.
//!
//! An ALE is a **clip log**: a tab-delimited text with three
//! sections — the **heading**, the **columns** and the **data** — and
//! it is the tree's own format for a list of media.
//!
//! ```text
//! Heading
//! FIELD_DELIM  TABS
//! VIDEO_FORMAT  1080
//! AUDIO_FORMAT  48khz
//! FPS  25
//!
//! Column
//! Name  Tracks  Start  End  Tape  Source File
//!
//! Data
//! the clip  V  00:00:00:00  00:00:05:00  THECLIP  /media/the clip.mov
//! ```
//!
//! # The rules
//!
//! - The lines end with **CR LF**, the tree's own: Avid's files end
//!   that way.
//! - A value never holds a **tab** or a **line break**: they are
//!   turned into spaces — the tree's own rule.
//! - The **start** and the **end** of a row are the timecodes of the
//!   **source** of the clip, not of the timeline: the tree's rule.
//! - A clip with **no item** is left out: it plays no media, and the
//!   tree's log holds media.
//!
//! # What the port found
//!
//! - **The tree's log is of the media and this tree's log is of the
//!   clips.** The tree writes a row for each media item of the
//!   project — with the media's own timecodes and the media's own
//!   name. Here a row is a clip of the timeline, and the row names
//!   the media the clip plays.
//! - **The tree's six metadata columns are not here.** The tree's log
//!   ends with `Description`, `Scene`, `Take`, `Shot`, `Log Note` and
//!   `Comment`, and they hold the metadata of the media. The items of
//!   this tree carry no metadata, and a column that would always be
//!   empty is not written.
//! - **The tree's heading knows the height of the video and the rate
//!   of the audio**, because it probes the media. This tree knows
//!   neither: the heading says `1080` and `48khz` — the tree's own
//!   defaults for a media it knows nothing about — and the `FPS` is
//!   the rate of the sequence.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session};

/// The **tracks** of a sequence.
fn tracks_of(sequence: &Node) -> Vec<std::sync::Arc<Node>> {
    sequence.children.iter().filter(|child| child.kind == "film.track").cloned().collect()
}

/// The **clips** of a track, in the order they are on the timeline.
fn clips_of(track: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut clips: Vec<std::sync::Arc<Node>> =
        track.children.iter().filter(|child| child.kind == "film.clip").cloned().collect();
    clips.sort_by_key(|clip| clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
    clips
}

/// Whether a track is an **audio** one.
fn is_audio(track: &Node) -> bool {
    track.data.get("kind").and_then(|kind| kind.as_str()).map(|kind| kind == "audio").unwrap_or(false)
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

/// The **rate** of a sequence: a number and a number.
fn rate_of(sequence: &Node) -> (i64, i64) {
    let stored = sequence.data.get("rate");
    let number = |name: &str| stored.and_then(|rate| rate.get(name)).and_then(|value| value.as_i64());
    match (number("num"), number("den")) {
        (Some(num), Some(den)) if num > 0 && den > 0 => (num, den),
        _ => (25, 1),
    }
}

/// The **FPS** of a rate, as the tree writes it: a whole number, or
/// three decimals with the zeros trimmed.
fn fps_text(rate: (i64, i64)) -> String {
    let fps = rate.0 as f64 / rate.1 as f64;
    if (fps - fps.round()).abs() < 1e-6 {
        format!("{}", fps.round() as i64)
    } else {
        format!("{fps:.3}").trim_end_matches('0').to_string()
    }
}

/// A **timecode**: the hours, the minutes, the seconds and the frames.
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
        if reel.chars().count() >= 8 {
            break;
        }
        if character.is_ascii_alphanumeric() {
            reel.push(character.to_ascii_uppercase());
        } else {
            reel.push('_');
        }
    }
    reel
}

/// A **value** of the log: no tab and no line break in it — the
/// tree's own rule.
fn clean(text: &str) -> String {
    text.replace(['\t', '\r', '\n'], " ")
}

/// **Write** the Avid log exchange of a sequence.
///
/// The log is derived from the sequence as it is now, and it is
/// returned as a text.
#[derive(Command)]
#[command(id = "film.export.ale", area = "film", layer = "film", icon = "list")]
pub struct ExportAle {
    /// The sequence.
    pub sequence: NodeRef,
}

impl ExportAle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.export.ale".to_string();
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
                message: format!("the node `{sequence}` is a `{}` and not a sequence", node.kind),
            });
        }
        let (num, den) = rate_of(node);
        let per_second = ((num as f64 / den as f64).round() as u64).max(1);
        let mut out = String::from("Heading\r\n");
        out.push_str("FIELD_DELIM\tTABS\r\n");
        // The tree probes the media and says what it finds; this tree
        // knows neither the height nor the rate of the audio, and the
        // tree's own defaults are said.
        out.push_str("VIDEO_FORMAT\t1080\r\n");
        out.push_str("AUDIO_FORMAT\t48khz\r\n");
        out.push_str(&format!("FPS\t{}\r\n", fps_text((num, den))));
        out.push_str("\r\nColumn\r\n");
        out.push_str("Name\tTracks\tStart\tEnd\tTape\tSource File\r\n");
        out.push_str("\r\nData\r\n");
        let tracks = tracks_of(node);
        // The **audio** tracks are numbered: `A1`, `A2` and so on.
        let mut audio = 0u32;
        for track in &tracks {
            if is_audio(track) {
                audio += 1;
            }
            for clip in clips_of(track) {
                // A clip with **no item** plays no media, and the log
                // holds media.
                let Some(item) = item_of(session.document(), clip.id) else { continue };
                let source_in = clip.data.get("source_in").and_then(|source| source.as_u64()).unwrap_or(0);
                let duration = clip.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                let tracks_column = if is_audio(track) { format!("A{audio}") } else { "V".to_string() };
                let path =
                    item.data.get("path").and_then(|path| path.as_str()).unwrap_or_default().to_string();
                let row = [
                    clean(&item.name),
                    tracks_column,
                    timecode(source_in, per_second),
                    timecode(source_in + duration, per_second),
                    reel_of(&item.name),
                    clean(&path),
                ];
                out.push_str(&row.join("\t"));
                out.push_str("\r\n");
            }
        }
        Ok(Outcome::new(format!("the sequence `{sequence}` has an Avid log")).with_data(json!({
            "sequence": sequence,
            "ale": out,
        })))
    }
}
