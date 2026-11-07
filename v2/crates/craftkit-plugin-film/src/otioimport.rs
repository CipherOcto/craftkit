//! The **OTIO** read back: a timeline made into a sequence.
//!
//! OTIO is a **JSON** text: a timeline, the tracks of it, and the clips
//! on them. This module reads one and makes the **sequence** it names.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - A file that is **not OTIO** is refused: it is not a JSON text, or
//!   it has no **timeline** in it — the tree's own refusal.
//! - The **rate** of the sequence is the rate of the timeline's
//!   `global_start_time`, and a timeline with none has the rate of the
//!   first time in it, and a timeline with no time at all has the
//!   default rate of the battery — the tree's own rule.
//! - A **track** whose kind is `Audio` is an audio track, and every
//!   other track is a video one — the tree's own rule.
//! - A **gap** is a hole in the timeline: the clips after it move
//!   along.
//! - The **media** of a clip is an item of a bin, with the name and
//!   the file of the clip's reference, and the **clip** points at it —
//!   the same shape as a clip that was made by hand.
//! - A **nested** clip — a stack of tracks where a clip stands — is a
//!   clip that holds tracks, as it is everywhere else in this tree.
//! - A **transition** is a transition over the cut before it: it
//!   joins the clip before it and the clip after it, and where it
//!   sits on the cut says how it is **aligned** — the tree's own
//!   three ways, worked out from the offsets the text carries.
//! - A **marker** is a marker of the sequence: the stack of the
//!   tracks and the tracks themselves carry them, and the colour they
//!   name is the tree's own colour.
//!
//! # What it does not do
//!
//! - The **effects**, the **groups** and the **links** of the text are
//!   not read. Only the timeline, the tracks, the clips, the media,
//!   the transitions and the markers are.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{self, Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Session, Text};

/// The most an OTIO file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The **colour** a marker names, and the label of the tree's own it
/// is.
///
/// The tree has its own words for the colours — red, pink, orange,
/// yellow, green, cyan, blue, purple, black and white — and they are
/// not the names of the tree's labels: red is rose and white is
/// lavender. A name that is already a label is that label.
fn colour_of(word: &str) -> Option<&'static str> {
    let word = word.trim();
    match word.to_ascii_uppercase().as_str() {
        "RED" => Some("Rose"),
        "PINK" | "MAGENTA" => Some("Magenta"),
        "ORANGE" => Some("Mango"),
        "YELLOW" => Some("Yellow"),
        "GREEN" => Some("Green"),
        "CYAN" => Some("Teal"),
        "BLUE" => Some("Blue"),
        "PURPLE" => Some("Purple"),
        "BLACK" => Some("Brown"),
        "WHITE" => Some("Lavender"),
        _ => crate::labels::label_of(word),
    }
}

/// The **schema** of a value: `Clip.2` is a `Clip`.
fn schema_of(value: &Json) -> &str {
    value
        .get("OTIO_SCHEMA")
        .and_then(|schema| schema.as_str())
        .map(|schema| schema.split('.').next().unwrap_or_default())
        .unwrap_or_default()
}

/// The **name** of a value.
fn name_of(value: &Json) -> &str {
    value.get("name").and_then(|name| name.as_str()).unwrap_or_default()
}

/// The **frames** a rational time holds.
fn frames_of(value: &Json) -> u64 {
    value
        .get("value")
        .and_then(|value| value.as_f64())
        .filter(|frames| frames.is_finite() && *frames >= 0.0)
        .map(|frames| frames.round() as u64)
        .unwrap_or(0)
}

/// The **frames** a range holds: where it begins and how long it is.
fn range_of(value: &Json) -> (u64, u64) {
    let start = value.get("start_time").map(frames_of).unwrap_or(0);
    let duration = value.get("duration").map(frames_of).unwrap_or(0);
    (start, duration)
}

/// The **first rate** in a value, when it holds one.
fn first_rate(value: &Json) -> Option<f64> {
    if let Some(rate) = value
        .get("source_range")
        .filter(|range| !range.is_null())
        .and_then(|range| range.get("duration"))
        .and_then(|duration| duration.get("rate"))
        .and_then(|rate| rate.as_f64())
    {
        return Some(rate);
    }
    value.get("children").and_then(|children| children.as_array()).into_iter().flatten().find_map(first_rate)
}

/// The **timeline** of a document: the first one, when it holds one.
fn timeline_of(value: &Json) -> Option<&Json> {
    match schema_of(value) {
        "Timeline" | "Stack" => Some(value),
        "SerializableCollection" => value
            .get("children")
            .and_then(|children| children.as_array())
            .into_iter()
            .flatten()
            .find_map(timeline_of),
        _ => None,
    }
}

/// The **stack** of tracks of a timeline.
fn stack_of(value: &Json) -> Option<&Json> {
    match schema_of(value) {
        "Timeline" => value.get("tracks"),
        _ => Some(value),
    }
}

/// The **reader**: it makes the nodes of a sequence.
struct Reader<'a> {
    session: &'a mut Session,
    bin: u64,
    number: u64,
    clips: usize,
    said: Vec<String>,
}

impl Reader<'_> {
    /// A **node** under a parent.
    fn node(
        &mut self,
        parent: Option<u64>,
        name: String,
        kind: &str,
        data: Json,
    ) -> Result<u64, EngineError> {
        let document = self.session.document_mut()?;
        Ok(document.add_node_with_under(parent, name, kind, None, data))
    }

    /// The **item** of the bin for a piece of media.
    fn item(&mut self, name: &str, path: &str, duration: u64, offline: bool) -> Result<u64, EngineError> {
        self.number += 1;
        let number = self.number;
        let data = json!({
            "item": number,
            "path": if path.is_empty() { Json::Null } else { Json::String(path.to_string()) },
            "duration": duration,
            "offline": offline,
        });
        self.node(Some(self.bin), name.to_string(), "film.item", data)
    }

    /// The **clips** of a track, from the children of the text.
    ///
    /// The answer is where the track ends.
    fn track_children(&mut self, children: &Json, track: u64) -> Result<u64, EngineError> {
        let Some(children) = children.as_array() else { return Ok(0) };
        let mut at = 0u64;
        // The **clips** of the track, in order, and the
        // **transitions** that sit between them: a transition is
        // counted where it stands, and the clip it comes into is the
        // one that follows it.
        let mut laid: Vec<Option<u64>> = Vec::new();
        let mut transitions: Vec<(usize, &Json, u64)> = Vec::new();
        for child in children {
            match schema_of(child) {
                "Gap" => {
                    let (_, duration) = child.get("source_range").map(range_of).unwrap_or((0, 0));
                    at += duration;
                    laid.push(None);
                }
                "Transition" => {
                    transitions.push((laid.len(), child, at));
                }
                "Stack" => {
                    let length = self.nested(child, at)?;
                    at += length;
                    laid.push(None);
                }
                "Clip" => {
                    let (source_in, duration) = child.get("source_range").map(range_of).unwrap_or((0, 0));
                    let name = name_of(child);
                    let (path, media_length, offline) = self.reference(child);
                    let item = self.item(
                        if name.is_empty() { "the media" } else { name },
                        &path,
                        media_length,
                        offline,
                    )?;
                    let speed = self.speed_of(child);
                    let clip = self.node(
                        Some(track),
                        if name.is_empty() { "a clip".to_string() } else { name.to_string() },
                        "film.clip",
                        json!({
                            "start": at,
                            "duration": duration,
                            "source_in": source_in,
                            "speed": speed,
                            "link": Json::Null,
                        }),
                    )?;
                    let relation = Relation::with_payload(clip, item, "clip", json!({}));
                    let document = self.session.document_mut()?;
                    document.add_relation(relation);
                    self.clips += 1;
                    at += duration;
                    laid.push(Some(clip));
                }
                other => {
                    self.said.push(format!("a `{other}` is left out"));
                }
            }
        }
        // The **transitions** of the track: each one joins the clip
        // before it and the clip after it.
        for (position, child, cut) in transitions {
            let from = position.checked_sub(1).and_then(|index| laid.get(index)).copied().flatten();
            let to = laid.get(position).copied().flatten();
            if from.is_none() && to.is_none() {
                continue;
            }
            // The **offsets** the text carries: how far the transition
            // reaches on either side of the cut.
            let inner = child.get("in_offset").map(frames_of).unwrap_or(0);
            let outer = child.get("out_offset").map(frames_of).unwrap_or(0);
            // The **alignment** is the tree's own: one that reaches
            // back over the cut only starts at it, one that reaches
            // forward over it only ends at it, and any other is
            // centred on it.
            let align = if inner == 0 {
                "start"
            } else if outer == 0 {
                "end"
            } else {
                "center"
            };
            // The **effect**: the name of the transition, or the kind
            // of it.
            let name = name_of(child);
            let kind = child.get("transition_type").and_then(|kind| kind.as_str()).unwrap_or_default();
            let named = if name.is_empty() { kind } else { name };
            let Some(effect) = crate::transitions::effect_of(named) else {
                self.said.push(format!(
                    "the transition `{named}` is not a transition this tree has, and it is left out"
                ));
                continue;
            };
            self.node(
                Some(track),
                format!("transition at {}", cut.saturating_sub(inner)),
                "film.transition",
                json!({
                    "from": from,
                    "to": to,
                    "start": cut.saturating_sub(inner),
                    "duration": inner + outer,
                    "align": align,
                    "effect": effect,
                }),
            )?;
        }
        Ok(at)
    }

    /// The **markers** of a value: they are markers of the sequence.
    fn markers(&mut self, value: &Json, sequence: u64) -> Result<(), EngineError> {
        let Some(markers) = value.get("markers").and_then(|markers| markers.as_array()) else {
            return Ok(());
        };
        for marker in markers {
            let (start, duration) = marker.get("marked_range").map(range_of).unwrap_or((0, 0));
            let name = name_of(marker).to_string();
            let comment =
                marker.get("comment").and_then(|comment| comment.as_str()).unwrap_or_default().to_string();
            // The **colour** the marker names, and green when it names
            // none.
            let colour =
                marker.get("color").and_then(|colour| colour.as_str()).and_then(colour_of).unwrap_or("Green");
            self.node(
                Some(sequence),
                name.clone(),
                "film.marker",
                json!({
                    "start": start,
                    "duration": duration,
                    "name": name,
                    "comment": comment,
                    "color": colour,
                    "kind": "Comment",
                }),
            )?;
        }
        Ok(())
    }

    /// The **media** a clip plays: the path of its reference, the
    /// length of it, and whether it is there.
    fn reference(&self, clip: &Json) -> (String, u64, bool) {
        let references = clip.get("media_references");
        let reference = references
            .and_then(|references| references.get("DEFAULT_MEDIA"))
            .or_else(|| {
                references
                    .and_then(|references| references.as_object())
                    .and_then(|references| references.values().next())
            })
            .cloned()
            .unwrap_or(Json::Null);
        let (_, length) = reference.get("available_range").map(range_of).unwrap_or((0, 0));
        match schema_of(&reference) {
            "ExternalReference" => {
                let url = reference.get("target_url").and_then(|url| url.as_str()).unwrap_or_default();
                let path = url.strip_prefix("file://").unwrap_or(url);
                (path.to_string(), length, false)
            }
            _ => (String::new(), length, true),
        }
    }

    /// The **speed** a clip plays at: the warp of its effects.
    fn speed_of(&self, clip: &Json) -> f64 {
        let Some(effects) = clip.get("effects").and_then(|effects| effects.as_array()) else {
            return 1.0;
        };
        for effect in effects {
            match schema_of(effect) {
                "FreezeFrame" => return 0.0,
                "LinearTimeWarp" => {
                    if let Some(scalar) = effect.get("time_scalar").and_then(|scalar| scalar.as_f64()) {
                        return scalar.abs();
                    }
                }
                _ => {}
            }
        }
        1.0
    }

    /// A **nested clip**: a stack of tracks where a clip stands.
    ///
    /// The answer is how long it is.
    fn nested(&mut self, stack: &Json, at: u64) -> Result<u64, EngineError> {
        let name = name_of(stack);
        let nested = self.node(
            None,
            if name.is_empty() { "a nested clip".to_string() } else { name.to_string() },
            "film.clip",
            json!({
                "start": at,
                "duration": 0,
                "source_in": 0,
                "speed": 1.0,
                "link": Json::Null,
                "nested": true,
            }),
        )?;
        let mut end = 0u64;
        let Some(children) = stack.get("children").and_then(|children| children.as_array()) else {
            return Ok(0);
        };
        for child in children {
            if schema_of(child) != "Track" {
                continue;
            }
            let kind = match child.get("kind").and_then(|kind| kind.as_str()) {
                Some("Audio") => "audio",
                _ => "video",
            };
            let track_name = name_of(child);
            let track = self.node(
                Some(nested),
                if track_name.is_empty() { "the track".to_string() } else { track_name.to_string() },
                "film.track",
                json!({ "kind": kind }),
            )?;
            let length = self.track_children(child.get("children").unwrap_or(&Json::Null), track)?;
            end = end.max(length);
        }
        // The **length** of the nested clip is what it holds.
        if let Some(node) = self.session.document_mut().ok().and_then(|document| document.node_mut(nested)) {
            node.data["duration"] = Json::from(end);
        }
        Ok(end)
    }
}

/// **Read** an OTIO file and make the **sequence** it names.
///
/// The media of the file are items, and the clips of its tracks are
/// clips on the tracks of the sequence.
#[derive(Command)]
#[command(id = "film.import.otio", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ImportOtio {
    /// The file to read.
    pub path: Text,
    /// What to call the sequence.
    ///
    /// Without it, the name of the file.
    pub name: Option<Text>,
}

impl ImportOtio {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.import.otio".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(|message| {
                EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
            })?;
        let text: Json = serde_json::from_slice(&bytes).map_err(|_| EngineError::Failed {
            id: id.clone(),
            message: "the file is not OTIO: it is not a JSON text".to_string(),
        })?;
        let Some(timeline) = timeline_of(&text) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the file is not OTIO: there is no timeline in a `{}` document",
                    schema_of(&text)
                ),
            });
        };
        // The **rate**: the timeline's own, or the first time in it.
        let rate = timeline
            .get("global_start_time")
            .and_then(|time| time.get("rate"))
            .and_then(|rate| rate.as_f64())
            .or_else(|| stack_of(timeline).and_then(first_rate))
            .map(crate::rates::rate_from_float)
            .unwrap_or((25, 1));
        // The **name** of the sequence.
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => {
                let from_file = std::path::Path::new(&path)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                if from_file.is_empty() { name_of(timeline).to_string() } else { from_file }
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
                "rate": { "num": rate.0, "den": rate.1 },
            }),
        );
        let mut reader = Reader { session, bin, number: 0, clips: 0, said: Vec::new() };
        let mut end = 0u64;
        let mut tracks = 0usize;
        if let Some(stack) = stack_of(timeline)
            && let Some(children) = stack.get("children").and_then(|children| children.as_array())
        {
            for child in children {
                if schema_of(child) != "Track" {
                    continue;
                }
                let kind = match child.get("kind").and_then(|kind| kind.as_str()) {
                    Some("Audio") => "audio",
                    _ => "video",
                };
                let track_name = name_of(child);
                let track = reader.node(
                    Some(sequence),
                    if track_name.is_empty() { "the track".to_string() } else { track_name.to_string() },
                    "film.track",
                    json!({ "kind": kind }),
                )?;
                tracks += 1;
                // The **markers** of the track are markers of the
                // sequence.
                reader.markers(child, sequence)?;
                let length = reader.track_children(child.get("children").unwrap_or(&Json::Null), track)?;
                end = end.max(length);
            }
            // The **markers** of the stack of the tracks are markers of
            // the sequence too.
            reader.markers(stack, sequence)?;
        }
        // The **length** of the sequence is what its tracks hold.
        if let Some(node) =
            reader.session.document_mut().ok().and_then(|document| document.node_mut(sequence))
        {
            node.data["duration"] = Json::from(end);
        }
        let said = reader.said.clone();
        let clips = reader.clips;
        Ok(Outcome::changed(format!("the timeline of `{path}` made {tracks} track(s) and {clips} clip(s)"))
            .with_data(json!({
                "path": path,
                "bin": bin,
                "sequence_bin": sequences,
                "sequence": sequence,
                "tracks": tracks,
                "clips": clips,
                "rate": { "num": rate.0, "den": rate.1 },
                "left_out": said,
            })))
    }
}
