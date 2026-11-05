//! The **OTIO** of a sequence.
//!
//! OTIO is the interchange format of a non-linear editor that is a
//! **JSON** text: a timeline, the tracks of it, and the clips on them.
//! It is **derived** — worked out from the sequence as it is now — and
//! it is returned as a text, because a command about the machine is
//! not a command about the document (§457).
//!
//! The shape of the text is the tree's shape:
//!
//! ```text
//! {
//!   "OTIO_SCHEMA": "Timeline.1",
//!   "name": "the sequence",
//!   "global_start_time": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 0.0 },
//!   "tracks": { "OTIO_SCHEMA": "Stack.1", "children": [ ... ] }
//! }
//! ```
//!
//! - the **tracks** are the tracks of the sequence: the video ones
//!   first, then the audio ones, and each says which kind it is;
//! - the **children** of a track are its clips, with a **gap** for
//!   every hole between them and a **transition** for every dissolve;
//! - a **clip** says where in its source it begins and how long it is,
//!   and it points at the media it plays;
//! - the **markers** of the sequence are on the stack of the tracks;
//! - the **times** are rational: a rate and a value, and the value is
//!   the number of frames.
//!
//! The rules:
//!
//! - a clip with **no item** is refused: the text would point at a
//!   source that is not there;
//! - a clip that is **retimed** carries a `LinearTimeWarp`, and a
//!   clip that is **held** carries a `FreezeFrame`.
//!
//! # What the port found
//!
//! - **The tree's export writes a file and knows the media**: its
//!   reference says what the media is, whether it is a generator, and
//!   what the media's own range is. Here the item knows its name, its
//!   path and its length, and the reference says what it knows: a
//!   **missing reference** for an item with no path, which is what
//!   the tree writes for a clip whose media is not there.
//! - **The tree's export writes the tree's own metadata** beside every
//!   object. Here the text says what the document says and nothing
//!   else.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{self, Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session};

/// The most characters the text may have.
const MAX_TEXT: usize = 4_000_000;

/// The rate a sequence with **no rate** runs at: the tree's own.
const DEFAULT_RATE: (i64, i64) = (25, 1);

/// A **rational time**: the rate and the number of frames.
fn time(frames: u64, rate: (i64, i64)) -> Json {
    json!({
        "OTIO_SCHEMA": "RationalTime.1",
        "rate": rate.0 as f64 / rate.1 as f64,
        "value": frames as f64,
    })
}

/// A **range** of the source: where it begins and how long it is.
fn range(start: u64, duration: u64, rate: (i64, i64)) -> Json {
    json!({
        "OTIO_SCHEMA": "TimeRange.1",
        "start_time": time(start, rate),
        "duration": time(duration, rate),
    })
}

/// The **rate** of a sequence: the frames of a second it counts.
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

/// The **tracks** of a sequence: the video ones first, then the audio
/// ones, and each in the order it is in the sequence.
fn tracks_of(sequence: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut tracks: Vec<std::sync::Arc<Node>> =
        sequence.children.iter().filter(|child| child.kind == "film.track").cloned().collect();
    let is_audio =
        |track: &Node| track.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default() == "audio";
    tracks.sort_by_key(|track| is_audio(track.as_ref()));
    tracks
}

/// Whether a track is an **audio** track.
fn is_audio(track: &Node) -> bool {
    track.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default() == "audio"
}

/// The **clips** of a track, in the order they are on the timeline.
fn clips_of(track: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut clips: Vec<std::sync::Arc<Node>> =
        track.children.iter().filter(|child| child.kind == "film.clip").cloned().collect();
    clips.sort_by_key(|clip| clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
    clips
}

/// A **transition** of a track: a dissolve over a cut.
struct Transition {
    /// The outgoing clip.
    from: u64,
    /// The incoming clip.
    to: u64,
    /// Where it sits on the timeline.
    start: u64,
    /// How long it is.
    duration: u64,
    /// The name of its effect.
    name: String,
}

/// The **transitions** of a track.
fn transitions_of(track: &Node) -> Vec<Transition> {
    let mut transitions = Vec::new();
    for child in track.children.iter().filter(|child| child.kind == "film.transition") {
        let number = |name: &str| child.data.get(name).and_then(|value| value.as_u64());
        let (Some(from), Some(to)) = (number("from"), number("to")) else { continue };
        transitions.push(Transition {
            from,
            to,
            start: number("start").unwrap_or(0),
            duration: number("duration").unwrap_or(0),
            name: child
                .data
                .get("effect")
                .and_then(|effect| effect.as_str())
                .unwrap_or("cross_dissolve")
                .to_string(),
        });
    }
    transitions
}

/// The **markers** of a sequence, in the order they are on the
/// timeline.
fn markers_of(sequence: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut markers: Vec<std::sync::Arc<Node>> =
        sequence.children.iter().filter(|child| child.kind == "film.marker").cloned().collect();
    markers.sort_by_key(|marker| marker.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
    markers
}

/// The **reference** of a clip: the media it plays.
fn reference_of(item: &Node, rate: (i64, i64)) -> Json {
    let name = item.name.as_str();
    let path = item.data.get("path").and_then(|path| path.as_str()).unwrap_or_default();
    if path.is_empty() {
        return json!({
            "OTIO_SCHEMA": "MissingReference.1",
            "name": name,
            "available_range": Json::Null,
            "metadata": {},
        });
    }
    let duration = item.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    let target = if path.starts_with('/') { format!("file://{path}") } else { path.to_string() };
    json!({
        "OTIO_SCHEMA": "ExternalReference.1",
        "name": name,
        "target_url": target,
        "available_range": range(0, duration, rate),
        "available_image_bounds": Json::Null,
        "metadata": {},
    })
}

/// The **effects** of a clip: what is done to its time.
fn effects_of(clip: &Node) -> Vec<Json> {
    let speed = clip.data.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(1.0);
    let reverse = clip.data.get("reverse").and_then(|reverse| reverse.as_bool()).unwrap_or(false);
    if clip.data.get("hold").and_then(|hold| hold.as_u64()).is_some() {
        return vec![json!({
            "OTIO_SCHEMA": "FreezeFrame.1",
            "name": "",
            "effect_name": "FreezeFrame",
            "time_scalar": 0.0,
            "metadata": {},
        })];
    }
    if speed != 1.0 || reverse {
        let scalar = if reverse { -speed } else { speed };
        return vec![json!({
            "OTIO_SCHEMA": "LinearTimeWarp.1",
            "name": "",
            "effect_name": "LinearTimeWarp",
            "time_scalar": scalar,
            "metadata": {},
        })];
    }
    Vec::new()
}

/// The **text** of the OTIO of a sequence.
///
/// The text is derived from the sequence as it is now, and it is
/// returned as a text.
#[derive(Command)]
#[command(id = "film.export.otio", area = "film", layer = "film", icon = "list")]
pub struct ExportOtio {
    /// The sequence.
    pub sequence: NodeRef,
}

impl ExportOtio {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.export.otio".to_string();
        let sequence = self.sequence.0;
        let Some(node) = session.document().node(sequence).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{sequence}`"),
            });
        };
        if node.kind != "film.item" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sequence}` is a `{}`, and a sequence is an item", node.kind),
            });
        }
        let rate = rate_of(&node);
        let document = session.document();
        let mut tracks: Vec<Json> = Vec::new();
        for track in tracks_of(&node) {
            let audio = is_audio(track.as_ref());
            let transitions = transitions_of(track.as_ref());
            let mut children: Vec<Json> = Vec::new();
            let mut at = 0u64;
            for clip in clips_of(track.as_ref()) {
                let start = clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration = clip.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                if start > at {
                    children.push(json!({
                        "OTIO_SCHEMA": "Gap.1",
                        "name": "",
                        "source_range": range(0, start - at, rate),
                        "effects": Vec::<Json>::new(),
                        "markers": Vec::<Json>::new(),
                        "enabled": true,
                        "metadata": {},
                    }));
                }
                for transition in transitions.iter().filter(|transition| transition.to == clip.id) {
                    let cut = start;
                    children.push(json!({
                        "OTIO_SCHEMA": "Transition.1",
                        "name": transition.name,
                        "transition_type": "SMPTE_Dissolve",
                        "in_offset": time(cut.saturating_sub(transition.start), rate),
                        "out_offset": time((transition.start + transition.duration).saturating_sub(cut), rate),
                        "metadata": {},
                    }));
                }
                let Some(item) = item_of(document, clip.id) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{}` has no item, and it has no source", clip.name),
                    });
                };
                let source_in = clip.data.get("source_in").and_then(|source| source.as_u64()).unwrap_or(0);
                let enabled = clip.data.get("enabled").and_then(|enabled| enabled.as_bool()).unwrap_or(true);
                children.push(json!({
                    "OTIO_SCHEMA": "Clip.2",
                    "name": clip.name,
                    "source_range": range(source_in, duration, rate),
                    "effects": effects_of(&clip),
                    "markers": Vec::<Json>::new(),
                    "enabled": enabled,
                    "media_references": { "DEFAULT_MEDIA": reference_of(&item, rate) },
                    "active_media_reference_key": "DEFAULT_MEDIA",
                    "metadata": {},
                }));
                for transition in transitions
                    .iter()
                    .filter(|transition| transition.from == clip.id && transition.to != clip.id)
                {
                    let cut = start + duration;
                    children.push(json!({
                        "OTIO_SCHEMA": "Transition.1",
                        "name": transition.name,
                        "transition_type": "SMPTE_Dissolve",
                        "in_offset": time(cut.saturating_sub(transition.start), rate),
                        "out_offset": time((transition.start + transition.duration).saturating_sub(cut), rate),
                        "metadata": {},
                    }));
                }
                at = start + duration;
            }
            let enabled = track.data.get("enabled").and_then(|enabled| enabled.as_bool()).unwrap_or(true);
            tracks.push(json!({
                "OTIO_SCHEMA": "Track.1",
                "name": track.name,
                "kind": if audio { "Audio" } else { "Video" },
                "children": children,
                "source_range": Json::Null,
                "effects": Vec::<Json>::new(),
                "markers": Vec::<Json>::new(),
                "enabled": enabled,
                "metadata": {},
            }));
        }
        let markers: Vec<Json> = markers_of(&node)
            .iter()
            .map(|marker| {
                let start = marker.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration =
                    marker.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                json!({
                    "OTIO_SCHEMA": "Marker.2",
                    "name": marker.name,
                    "comment": marker.data.get("comment").cloned().unwrap_or(Json::Null),
                    "color": marker.data.get("color").cloned().unwrap_or(Json::Null),
                    "marked_range": range(start, duration, rate),
                    "metadata": {},
                })
            })
            .collect();
        let timeline = json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": node.name,
            "global_start_time": time(0, rate),
            "metadata": {},
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "name": node.name,
                "children": tracks,
                "source_range": Json::Null,
                "effects": Vec::<Json>::new(),
                "markers": markers,
                "enabled": true,
                "metadata": {},
            },
        });
        let text = serde_json::to_string_pretty(&timeline)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        if text.chars().count() > MAX_TEXT {
            return Err(EngineError::Failed {
                id,
                message: format!("the text is more than {MAX_TEXT} characters long, and it is not written"),
            });
        }
        Ok(Outcome::new(format!("the sequence is {} characters of OTIO", text.chars().count())).with_data(
            json!({
                "sequence": sequence,
                "otio": text,
                "timeline": timeline,
            }),
        ))
    }
}
