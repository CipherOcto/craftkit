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
//! - A **dissolve** is an event that is not a cut: it is a
//!   **transition** over the cut before it, it joins the clip that
//!   ends there and the clip that begins there, and its length is the
//!   number the line carries. The other edits — a wipe, a key — are
//!   left out, and said.
//! - A **marker** is a `* LOC:` comment: the timecode is where it
//!   starts, the colour it names is the tree's own colour, and the
//!   rest is its name.
//!
//! # What it does not do
//!
//! - The **comments** of the tree — the `M2` reel of a file, the black
//!   of a `BL` reel, the `* FROM CLIP NAME` of a nested clip — are not
//!   read, and the **bins** of the list are not made.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Text};

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

/// The **colour** a marker names, and the label of the tree's own it
/// is.
fn colour_of(word: &str) -> Option<&'static str> {
    match word.trim().to_ascii_uppercase().as_str() {
        "RED" => Some("Rose"),
        "GREEN" => Some("Green"),
        "BLUE" => Some("Blue"),
        "CYAN" => Some("Teal"),
        "MAGENTA" => Some("Magenta"),
        "YELLOW" => Some("Yellow"),
        "WHITE" => Some("Lavender"),
        "BLACK" => Some("Brown"),
        _ => None,
    }
}

/// One **clip** of a track: the clip itself, where it starts and
/// where it ends.
type Placed = (u64, u64, u64);

/// The **clips** of every track: the track and its clips.
type TrackClips = (u64, Vec<Placed>);

/// An **event** of the list: a cut, and where it came from and where
/// it sits.
struct Event {
    reel: String,
    channels: Vec<Channel>,
    cut: bool,
    /// The **kind** of edit: `C` is a cut and `D` is a dissolve.
    edit: char,
    /// The **length** of a dissolve, in frames.
    length: u64,
    /// The **name** of the effect, when the event names one.
    effect: Option<String>,
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
    // The **kind** of edit, and the length of a dissolve: the second
    // number of the line.
    let edit = first.chars().next().unwrap_or('C');
    let length = middle.get(1).and_then(|length| length.parse::<u64>().ok()).unwrap_or(0);
    let cut = edit == 'C';
    Some(Event { reel, channels, cut, edit, length, effect: None, source_in, record_in, record_out })
}

commands! {
    /// **Read** an edit decision list and make the **sequence** it names.
    ///
    /// The media of the list are items, and the events of the list are
    /// clips on the tracks of the sequence.
    film.import.edl {
        area: "film",
        layer: "film",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The file to read.

            path: Text,
            /// What to call the sequence.
            ///
            /// Without it, the title of the list.

            name: Option<Text>,
        },
        run: {

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
            // The **markers** of the list: where they start, what they are
            // called and what colour they are.
            let mut markers: Vec<(u64, String, String)> = Vec::new();
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
                    // The **comments** of the event above: a marker, and
                    // the name of an effect.
                    let comment = line.trim_start_matches('*').trim();
                    if let Some(rest) = comment.strip_prefix("LOC:") {
                        let mut parts = rest.split_whitespace();
                        let Some(timecode) = parts.next() else { continue };
                        let Some(start) = frames_of(timecode, per_second) else { continue };
                        let rest: Vec<&str> = parts.collect();
                        let (colour, name) = match rest.split_first() {
                            Some((colour, name)) if colour_of(colour).is_some() => {
                                (colour_of(colour).unwrap_or("Green").to_string(), name.join(" "))
                            }
                            _ => ("Green".to_string(), rest.join(" ")),
                        };
                        markers.push((start, name, colour));
                    } else if let Some(rest) = comment.strip_prefix("EFFECT NAME:") {
                        // The comment belongs to the **event above** it.
                        if let Some(event) = events.last_mut() {
                            event.effect = Some(rest.trim().to_string());
                        }
                    }
                    continue;
                }
                match event_of(line, per_second) {
                    Some(event) => {
                        // A **dissolve** is a transition over the cut
                        // before it, and it is kept. The other edits — a
                        // wipe, a key — are not cuts, and they are left
                        // out, and said.
                        if !event.cut && event.edit != 'D' {
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
            // The **clips** of every track, for the dissolves to name the
            // clip they come from.
            let mut track_clips: Vec<TrackClips> = Vec::new();
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
                    // A **dissolve** is a transition over the cut before
                    // this clip: it joins the clip that ends there and
                    // this one.
                    if event.edit == 'D' {
                        // The **clip** the dissolve comes from: the one
                        // that ends where this one begins.
                        let from = track_clips
                            .iter()
                            .find(|(other, _)| *other == track)
                            .map(|(_, clips)| clips)
                            .unwrap_or(&Vec::new())
                            .iter()
                            .filter(|(_, start, end)| *end == event.record_in && *start < event.record_in)
                            .max_by_key(|(_, start, _)| *start)
                            .map(|(clip, _, _)| *clip);
                        // The **effect**: the name the event gives, or
                        // the tree's own default.
                        let name = event.effect.clone().unwrap_or_else(|| "Cross Dissolve".to_string());
                        let Some(effect) = crate::transitions::effect_of(&name) else {
                            said.push(format!(
                                "the transition `{name}` is not a transition this tree has, and it is left out"
                            ));
                            continue;
                        };
                        let align = if event.reel.eq_ignore_ascii_case("BL") { "end" } else { "start" };
                        session.document_mut()?.add_node_with_under(
                            Some(track),
                            format!("transition at {}", event.record_in),
                            "film.transition",
                            None,
                            json!({
                                "from": from,
                                "to": clip,
                                "start": event.record_in,
                                "duration": event.length,
                                "align": align,
                                "effect": effect,
                            }),
                        );
                    }
                    match track_clips.iter_mut().find(|(other, _)| *other == track) {
                        Some((_, clips)) => clips.push((clip, event.record_in, event.record_out)),
                        None => track_clips.push((track, vec![(clip, event.record_in, event.record_out)])),
                    }
                }
            }
            // The **markers** of the list: they are markers of the
            // sequence.
            for (start, name, colour) in &markers {
                session.document_mut()?.add_node_with_under(
                    Some(sequence),
                    name.clone(),
                    "film.marker",
                    None,
                    json!({
                        "start": start,
                        "duration": 0,
                        "name": name,
                        "comment": "",
                        "color": colour,
                        "kind": "Comment",
                    }),
                );
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
                "markers": markers.len(),
                "rate": { "num": num, "den": den },
                "left_out": said,
            })))

        }
    }
}
