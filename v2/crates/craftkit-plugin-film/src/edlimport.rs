//! The **edit decision list** read back: a list of cuts made into a
//! sequence.
//!
//! An EDL is a **text** with a title and a list of events: every event
//! is a number, the reel the clip comes from, the channel it is on,
//! the edit, and the four timecodes — the source in and out, and the
//! record in and out. This module reads one and makes the sequence it
//! names.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - The **rate** is the rate of the battery, 25 frames a second: an
//!   EDL carries no rate of its own, and the tree takes the rate of
//!   the media it reads.
//! - A **channel** is `V` for a video track, and `A`, `A1`, `AA` and
//!   `An` for the audio ones — the tree's own rule: `AA` is **two**
//!   audio channels of one event, and `B` is a video channel and an
//!   audio one.
//! - The **media** of a reel is an **item** of a bin, named after the
//!   reel, and the **clip** points at it — the same shape as a clip
//!   made by hand.
//! - The **start** of a clip is the record in of its event, its
//!   **length** is the record out less the record in, and the
//!   **source** is the source in.
//! - A **dissolve** — an event that is not a cut — is left out, and
//!   said: a dissolve is two cuts, and the tree writes it as two.
//!
//! # What it does not do
//!
//! - The **comments** of the tree — the `M2` reel of a file, the black
//!   of a `BL` reel, the `* FROM CLIP NAME` of a nested clip — are not
//!   read.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Session, Text};

/// The most an EDL file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The rate a sequence with **no rate** runs at: the tree's own.
const DEFAULT_RATE: (i64, i64) = (25, 1);

/// A **channel** of an event.
#[derive(Clone, Copy, PartialEq)]
enum Channel {
    /// A video track.
    Video,
    /// An audio track, counted from one.
    Audio(usize),
}

/// The **channels** a word of an event names, the tree's own rule.
fn channels_of(word: &str) -> Option<Vec<Channel>> {
    let upper = word.to_ascii_uppercase();
    if upper == "B" {
        return Some(vec![Channel::Video, Channel::Audio(1)]);
    }
    if upper == "NONE" {
        return Some(Vec::new());
    }
    let mut channels = Vec::new();
    for part in upper.split('/') {
        match part {
            "V" => channels.push(Channel::Video),
            "A" | "A1" => channels.push(Channel::Audio(1)),
            "AA" => {
                channels.push(Channel::Audio(1));
                channels.push(Channel::Audio(2));
            }
            other => {
                let number = other.strip_prefix('A')?.parse::<usize>().ok()?;
                if !(1..=99).contains(&number) {
                    return None;
                }
                channels.push(Channel::Audio(number));
            }
        }
    }
    (!channels.is_empty()).then_some(channels)
}

/// The **frames** a timecode holds, at a rate.
fn frames_of(text: &str, per_second: u64) -> Option<u64> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if parts.len() != 4 {
        return None;
    }
    let hours: u64 = parts[0].trim().parse().ok()?;
    let minutes: u64 = parts[1].trim().parse().ok()?;
    let seconds: u64 = parts[2].trim().parse().ok()?;
    let frames: u64 = parts[3].trim().parse().ok()?;
    if minutes > 59 || seconds > 59 || frames >= per_second.max(1) {
        return None;
    }
    Some(((hours * 60 + minutes) * 60 + seconds) * per_second.max(1) + frames)
}

/// An **event** of the list: a cut, and where it came from and where
/// it sits.
struct Event {
    reel: String,
    channels: Vec<Channel>,
    cut: bool,
    source_in: u64,
    record_in: u64,
    record_out: u64,
}

/// The **event** a line holds, when it is one.
fn event_of(line: &str, per_second: u64) -> Option<Event> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 8 || !tokens[0].bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let last = tokens.len();
    let source_in = frames_of(tokens[last - 4], per_second)?;
    let _ = frames_of(tokens[last - 3], per_second)?;
    let record_in = frames_of(tokens[last - 2], per_second)?;
    let record_out = frames_of(tokens[last - 1], per_second)?;
    let reel = tokens[1].to_string();
    let channels = channels_of(tokens[2])?;
    let middle = &tokens[3..last - 4];
    let first = middle.first()?.to_ascii_uppercase();
    let cut = match first.as_str() {
        "C" => true,
        // A **dissolve** and the other edits are not cuts: they are
        // left out, and said.
        _ => false,
    };
    Some(Event { reel, channels, cut, source_in, record_in, record_out })
}

/// **Read** an edit decision list and make the **sequence** it names.
///
/// The media of the list are items, and the events of the list are
/// clips on the tracks of the sequence.
#[derive(Command)]
#[command(id = "film.import.edl", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ImportEdl {
    /// The file to read.
    pub path: Text,
    /// What to call the sequence.
    ///
    /// Without it, the title of the list.
    pub name: Option<Text>,
}

impl ImportEdl {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.import.edl".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(|message| {
                EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
            })?;
        let text = String::from_utf8(bytes).map_err(|_| EngineError::Failed {
            id: id.clone(),
            message: "the file is not a text".to_string(),
        })?;
        let (num, den) = DEFAULT_RATE;
        let per_second = num as u64;
        // The **title** of the list, and the **events** of it.
        let mut title = String::new();
        let mut events: Vec<Event> = Vec::new();
        let mut said: Vec<String> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("TITLE:") {
                title = rest.trim().to_string();
                continue;
            }
            if line.to_ascii_uppercase().starts_with("FCM:") {
                continue;
            }
            if line.starts_with('*') {
                continue;
            }
            match event_of(line, per_second) {
                Some(event) => {
                    if !event.cut {
                        said.push(format!(
                            "the edit of the reel `{}` is not a cut, and it is left out",
                            event.reel
                        ));
                        continue;
                    }
                    events.push(event);
                }
                None => {
                    if !line.starts_with(|character: char| character.is_ascii_digit()) {
                        continue;
                    }
                    said.push(format!("the event `{}` cannot be read, and it is left out", line));
                }
            }
        }
        if events.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file is not an edit decision list: it holds no event".to_string(),
            });
        }
        // The **name** of the sequence.
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => {
                if title.is_empty() {
                    std::path::Path::new(&path)
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                        .unwrap_or_else(|| "the sequence".to_string())
                } else {
                    title.clone()
                }
            }
        };
        // The **bin** the media goes in, and the **sequence**.
        let bin = session.document_mut()?.add_node_under(None, format!("{name} media"), "film.bin", None);
        // The **bin** the sequence goes in: a sequence is not a media.
        let sequences =
            session.document_mut()?.add_node_under(None, format!("{name} sequences"), "film.bin", None);
        let sequence = session.document_mut()?.add_node_with_under(
            Some(sequences),
            name.clone(),
            "film.item",
            None,
            json!({
                "item": 0,
                "path": Json::Null,
                "duration": 0,
                "offline": false,
                "rate": { "num": num, "den": den },
            }),
        );
        // The **tracks**: one for the video, and one for each audio.
        let mut tracks: Vec<(Channel, u64)> = Vec::new();
        let mut end = 0u64;
        let mut clips = 0usize;
        let mut reels: Vec<(String, u64)> = Vec::new();
        let mut number = 0u64;
        for event in &events {
            let duration = event.record_out.saturating_sub(event.record_in);
            if duration == 0 {
                said.push(format!(
                    "the event of the reel `{}` has no length, and it is left out",
                    event.reel
                ));
                continue;
            }
            // The **media** of the reel: one item for every reel.
            let item = match reels.iter().find(|(reel, _)| *reel == event.reel) {
                Some((_, item)) => *item,
                None => {
                    number += 1;
                    let item = session.document_mut()?.add_node_with_under(
                        Some(bin),
                        event.reel.clone(),
                        "film.item",
                        None,
                        json!({
                            "item": number,
                            "path": Json::Null,
                            "duration": 0,
                            "offline": true,
                        }),
                    );
                    reels.push((event.reel.clone(), item));
                    item
                }
            };
            for channel in &event.channels {
                let track = match tracks.iter().find(|(other, _)| other == channel) {
                    Some((_, track)) => *track,
                    None => {
                        let (kind, name) = match channel {
                            Channel::Video => ("video", "the video".to_string()),
                            Channel::Audio(number) => ("audio", format!("the audio {number}")),
                        };
                        let track = session.document_mut()?.add_node_with_under(
                            Some(sequence),
                            name,
                            "film.track",
                            None,
                            json!({ "kind": kind }),
                        );
                        tracks.push((*channel, track));
                        track
                    }
                };
                let clip = session.document_mut()?.add_node_with_under(
                    Some(track),
                    event.reel.clone(),
                    "film.clip",
                    None,
                    json!({
                        "start": event.record_in,
                        "duration": duration,
                        "source_in": event.source_in,
                        "speed": 1.0,
                        "link": Json::Null,
                    }),
                );
                let relation = Relation::with_payload(clip, item, "clip", json!({}));
                session.document_mut()?.add_relation(relation);
                clips += 1;
                end = end.max(event.record_out);
            }
        }
        // The **length** of the sequence is what its tracks hold.
        if let Some(node) = session.document_mut().ok().and_then(|document| document.node_mut(sequence)) {
            node.data["duration"] = Json::from(end);
        }
        Ok(Outcome::changed(format!("the list of `{path}` made {clips} clip(s)")).with_data(json!({
            "path": path,
            "bin": bin,
            "sequence_bin": sequences,
            "sequence": sequence,
            "title": title,
            "clips": clips,
            "rate": { "num": num, "den": den },
            "left_out": said,
        })))
    }
}
