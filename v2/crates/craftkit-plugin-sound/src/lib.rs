//! `craftkit-plugin-sound` — the **sound battery**.
//!
//! # What this is
//!
//! The port of a sound application: a document of tracks, of clips on
//! them, of a tempo map, of markers and of the media the clips point at.
//! It is a **battery** — an optional plugin — because a program that
//! edits pictures has no use for a tempo map, and a battery is how v2
//! says that: the crate is not there at all unless a feature asks for it.
//!
//! It contributes:
//!
//! - a **layer**, `sound`, which the rest of it is in;
//! - **seven kinds**: `sound.track`, `sound.playlist`, `sound.clip`,
//!   `sound.tempo`, `sound.meter`, `sound.marker` and `sound.source`;
//! - **seven commands**, one for each thing a person does to a sound
//!   document: make a track, make a clip, set the tempo, set the meter,
//!   make a marker, make a source, set the session;
//! - a **tool**, `tool.sound.clip`;
//! - a **panel**, `panel.sound`;
//! - an **evaluator**, `sound.timeline`, which says what the document
//!   means: the tempo map, and every clip in the place it sits.
//!
//! # What the port found
//!
//! Three things, and two of them are the core's business.
//!
//! **The core's node had a place.** A clip in a sound document is at a
//! **time**, and a time is not a place. The core changed: a node may have
//! no place, and the sound kinds have none.
//!
//! **The core's document had nowhere to put the session.** A sound
//! session's sample rate and timecode belong to the session and not to any
//! one node, so the document grew a payload of its own, and the sound
//! battery keeps its settings under one name in it.
//!
//! **The kinds of a sound document are open.** In the tree this ports, the
//! kind of a track is a closed enum of eight variants and the kind of a
//! clip is a closed enum of three, and every one of them is matched in
//! the mixer. Here a track's kind is a **string in the track's payload**
//! and a clip's content is a **field in the clip's payload**, so a third
//! party can add one without touching the core — which is the whole
//! point of the seam, arrived at by a port rather than by an argument.
//!
//! # What this does not do
//!
//! - **It draws nothing.** A sound document is not a picture. v2 has no
//!   timeline and no camera, and a plugin that drew one would be the
//!   second set of camera controls in the tree. The kinds say what they
//!   are; the drawing is a question for the tree.
//! - **It makes no sound.** There is no audio thread and no real-time
//!   door: a command in v2 runs on the caller's thread, which is the wrong
//!   thread for a block of audio.
//! - **It reads no media.** A source is a **reference**: a path, a channel
//!   count and a length. The audio itself is not in the document, and
//!   nothing here decodes it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel, Tool};

// The mixer: buses, routes, inserts, sends and automation lanes. It is
// the part of a sound document that is a **graph** and not a tree, so it
// is written in relations.
pub mod mixer;

use craftkit_sdk::{
    Canvas, Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Point, Session, Text,
};
pub use mixer::{
    AddBus, AddInsert, AddLane, AddRoute, AddSend, BusKind, BusPayload, InsertKind, InsertPayload, LaneKind,
    LanePayload, SendKind, SendPayload,
};

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;

// The time model: a library, not a plugin.
use craftkit_sound_time::{MAX_POSITION, TempoMap};

mod timeline;

/// The sound layer.
///
/// It needs the core, so it says so, and the ecosystem checks the graph
/// when the program starts.
#[derive(Layer)]
#[layer(id = "sound", name = "Sound", requires("core"))]
pub struct SoundLayer;

// ------------------------------------------------------------------ the session --

/// The sound session's own settings.
///
/// They belong to the **session**, not to any one node: the sample rate is
/// the rate of every clip in the document, and the timecode is where the
/// document starts on a clock. The document's payload is where they live,
/// under one name, so that another plugin cannot collide with them by
/// accident.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundSession {
    /// The sample rate, in samples a second.
    pub sample_rate: u32,
    /// Where the document starts on a timecode, in samples.
    pub timecode_start: i64,
}

impl Default for SoundSession {
    fn default() -> Self {
        SoundSession { sample_rate: 48000, timecode_start: 0 }
    }
}

/// The name the sound battery keeps its session settings under.
const SESSION_KEY: &str = "sound";

/// The session's settings, as the document holds them.
pub(crate) fn session_of(document: &Document) -> SoundSession {
    let settings = document.data.get(SESSION_KEY);
    let sample_rate = settings
        .and_then(|settings| settings.get("sample_rate"))
        .and_then(|rate| rate.as_u64())
        .and_then(|rate| u32::try_from(rate).ok())
        .filter(|rate| *rate > 0)
        .unwrap_or(SoundSession::default().sample_rate);
    let timecode_start = settings
        .and_then(|settings| settings.get("timecode_start"))
        .and_then(|start| start.as_i64())
        .unwrap_or(SoundSession::default().timecode_start);
    SoundSession { sample_rate, timecode_start }
}

/// Put the session's settings back into the document.
fn set_session(document: &mut Document, session: &SoundSession) {
    let mut data = match document.data.as_object() {
        Some(object) => object.clone(),
        None => serde_json::Map::new(),
    };
    data.insert(
        SESSION_KEY.to_string(),
        serde_json::json!({
            "sample_rate": session.sample_rate,
            "timecode_start": session.timecode_start,
        }),
    );
    document.data = Json::Object(data);
}

// -------------------------------------------------------------------- the kinds --

/// The kind of a track.
///
/// The kind of a track — audio, instrument, auxiliary, master — is a
/// **string in the track's payload**, not a variant in the core and not a
/// variant in this plugin. The tree this ports has eight variants in its
/// model and a match on them in the mixer; here a track says what it is,
/// and a third party can say something the core has never heard of.
#[derive(NodeKind)]
#[node_kind(kind = "sound.track", label = "Track", layer = "sound", evaluates, place)]
pub struct TrackKind;

impl TrackKind {
    /// How a track is drawn.
    ///
    /// The **row**: a line along the top of the track's band, so that
    /// the timeline shows where one track ends and the next begins.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::track_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.w, rect.y),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Colour::NONE,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
            }),
        );
    }

    /// Whether a point is inside a track.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::track_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a track **is**: its row.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::track_rect(document, node)
    }

    /// What a track **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let track = TrackPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "track_kind": track.track_kind,
            "channels": track.channels,
            "playlists": node.children.len(),
        }))
    }
}

/// The kind of a playlist.
///
/// A track holds **playlists**: the alternative takes of the same track.
/// The tree this ports holds them as a list on the track with an index
/// saying which one is playing; here a playlist is a node, because a node
/// can hold other nodes, and the alternative takes are the clips under
/// it.
#[derive(NodeKind)]
#[node_kind(kind = "sound.playlist", label = "Playlist", layer = "sound", evaluates, place)]
pub struct PlaylistKind;

impl PlaylistKind {
    /// How a playlist is drawn.
    ///
    /// **Nothing.** A playlist is the track's row with a different set of
    /// clips on it, and the row is already drawn by the track.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a playlist.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::playlist_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a playlist **is**: the row of the track it is under.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::playlist_rect(document, node)
    }

    /// What a playlist **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(serde_json::json!({ "clips": node.children.len() }))
    }
}

/// The kind of a clip.
///
/// A clip is a **span of time** with **content** in it. The content is a
/// field of the payload: an audio clip points at a source and an offset
/// into it, and a MIDI clip would hold notes — which the tree this ports
/// keeps in a closed enum of three kinds, matched in the mixer.
#[derive(NodeKind)]
#[node_kind(kind = "sound.clip", label = "Clip", layer = "sound", evaluates, place)]
pub struct ClipKind;

impl ClipKind {
    /// How a clip is drawn: a **bar** on the track's row.
    ///
    /// The bar is the clip's **span**: where it starts and how long it
    /// is, in the world's seconds. A muted clip is drawn as nothing,
    /// which is what a muted clip is.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::clip_rect(document, node) else { return };
        let fill = if node.muted {
            craftkit_sdk::document::Colour::NONE
        } else {
            document
                .appearance_of(node)
                .map(|appearance| appearance.fill)
                .unwrap_or(craftkit_sdk::document::Colour::BLACK)
        };
        canvas.rect(rect, fill);
    }

    /// Whether a point is inside a clip.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::clip_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a clip **is**: a bar on the track's row.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::clip_rect(document, node)
    }

    /// What a clip **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let clip = ClipPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "start": clip.start,
            "length": clip.length,
            "end": clip.start.saturating_add(clip.length),
            "source": clip.source,
            "gain_db": clip.gain_db,
        }))
    }
}

/// The kind of a tempo event.
#[derive(NodeKind)]
#[node_kind(kind = "sound.tempo", label = "Tempo", layer = "sound", evaluates, place)]
pub struct TempoKind;

impl TempoKind {
    /// How a tempo event is drawn: a **mark in the ruler**.
    ///
    /// The ruler is the row **above** the tracks, and it is where the
    /// things that are not on a track live: the tempo and the meter.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::event_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Colour::NONE,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
            }),
        );
    }

    /// Whether a point is inside a tempo event.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::event_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a tempo event **is**: a mark in the ruler.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::event_rect(document, node)
    }

    /// What a tempo event **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let tempo = TempoPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({ "tick": tempo.tick, "bpm": tempo.bpm }))
    }
}

/// The kind of a meter event.
#[derive(NodeKind)]
#[node_kind(kind = "sound.meter", label = "Meter", layer = "sound", evaluates, place)]
pub struct MeterKind;

impl MeterKind {
    /// How a meter event is drawn: a **mark in the ruler**.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::event_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Colour::NONE,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
            }),
        );
    }

    /// Whether a point is inside a meter event.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::event_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a meter event **is**: a mark in the ruler.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::event_rect(document, node)
    }

    /// What a meter event **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let meter = MeterPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "tick": meter.tick,
            "numerator": meter.numerator,
            "denominator": meter.denominator,
        }))
    }
}

/// The kind of a marker.
#[derive(NodeKind)]
#[node_kind(kind = "sound.marker", label = "Marker", layer = "sound", evaluates, place)]
pub struct MarkerKind;

impl MarkerKind {
    /// How a marker is drawn: a **line** down the timeline.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::marker_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Colour::NONE,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
            }),
        );
    }

    /// Whether a point is inside a marker.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::marker_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a marker **is**: a line at the time it is at.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::marker_rect(document, node)
    }

    /// What a marker **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let marker = MarkerPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({ "start": marker.start, "end": marker.end }))
    }
}

/// The kind of a **ruler**: the row above the tracks, and its scale.
///
/// The ruler is a **node** because the ruler has a **setting**: the
/// scale it counts by. A setting is a thing in a document, and a node is
/// where settings live — the same rule as the measure of a paragraph
/// (§58) and the appearance of a node (§95).
///
/// The ruler's **marks** are not the ruler's: the tempo and the meter
/// are drawn in the ruler's row by their own kinds (§126), and the ruler
/// draws the **numbers**.
#[derive(NodeKind)]
#[node_kind(kind = "sound.ruler", label = "Ruler", layer = "sound", evaluates, place)]
pub struct RulerKind;

impl RulerKind {
    /// How a ruler is drawn: its **numbers**.
    ///
    /// The numbers are the kit's own letters (§128): the tree has no
    /// font file and no asset of any kind, and a number is a word of one
    /// or two letters.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let marks = timeline::scale_marks(document, node);
        let font = craftkit_font::Font::builtin();
        let Some(metrics) = craftkit_font::Metrics::new(timeline::SCALE_EM) else { return };
        let unit = metrics.unit();
        for mark in marks {
            let mut pen = mark.x;
            for character in mark.label.chars() {
                let Some(glyph) = font.glyph(character) else { continue };
                for stroke in &glyph.strokes {
                    if stroke.len() < 2 {
                        continue;
                    }
                    let mut segments = Vec::with_capacity(stroke.len() - 1);
                    for point in stroke.iter().skip(1) {
                        segments.push(craftkit_sdk::document::Segment::Line {
                            to: Point::new(pen + point[0] * unit, baseline() + point[1] * unit),
                        });
                    }
                    let start = stroke[0];
                    let path = craftkit_sdk::document::Path::single(craftkit_sdk::document::SubPath {
                        start: Point::new(pen + start[0] * unit, baseline() + start[1] * unit),
                        segments,
                        closed: false,
                    });
                    canvas.path(
                        &path,
                        craftkit_sdk::document::Colour::NONE,
                        Some(craftkit_sdk::document::Stroke {
                            colour: craftkit_sdk::document::Colour::BLACK,
                            width: 0.01,
                        }),
                    );
                }
                pen += glyph.advance * unit;
            }
        }
    }

    /// Whether a point is on a **number**.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        let marks = timeline::scale_marks(document, node);
        let font = craftkit_font::Font::builtin();
        let Some(metrics) = craftkit_font::Metrics::new(timeline::SCALE_EM) else { return false };
        let unit = metrics.unit();
        marks.iter().any(|mark| {
            let width: f64 = mark
                .label
                .chars()
                .filter_map(|character| font.glyph(character))
                .map(|glyph| glyph.advance * unit)
                .sum();
            let box_of = craftkit_sdk::document::Rect::new(mark.x, baseline(), width, metrics.em);
            box_of.contains(point)
        })
    }

    /// Where a ruler **is**: the row above the tracks.
    fn place(_node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        Some(craftkit_sdk::document::Rect::new(
            0.0,
            -timeline::RULER_HEIGHT,
            timeline::length_of(document),
            timeline::RULER_HEIGHT,
        ))
    }

    /// What a ruler **means**: its scale and how many numbers it has.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let marks = timeline::scale_marks(document, node);
        Some(serde_json::json!({
            "scale": node.data.get("scale").and_then(|value| value.as_str()).unwrap_or("bars"),
            "numbers": marks.len(),
        }))
    }
}

/// The **baseline** of the ruler's numbers: the bottom of the ruler's
/// row.
///
/// The numbers stand on the bottom of the row and rise into it, so that
/// the marks of the tempo and the meter are above them.
fn baseline() -> f64 {
    -timeline::RULER_HEIGHT
}

/// Put a **ruler** in the document: the row above the tracks, and the
/// scale it counts by.
#[derive(Command)]
#[command(
    id = "sound.ruler.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Ruler"),
    mutates,
    undoable
)]
pub struct AddRuler {
    /// What to call it.
    pub name: Text,
    /// The scale the ruler counts by: `bars` or `seconds`.
    pub scale: Option<Text>,
}

impl AddRuler {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let scale = match &self.scale {
            Some(scale) => {
                let scale = scale.0.as_str();
                if scale != "bars" && scale != "seconds" {
                    return Err(EngineError::Failed {
                        id: "sound.ruler.add".to_string(),
                        message: format!("a ruler counts by bars or by seconds, and not by `{scale}`"),
                    });
                }
                scale.to_string()
            }
            None => "bars".to_string(),
        };
        let name = self.name.0.clone();
        let id = session.document_mut()?.add_node_with(
            name.clone(),
            "sound.ruler",
            None,
            serde_json::json!({ "scale": scale }),
        );
        Ok(Outcome::changed(format!("added a ruler `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "scale": scale })))
    }
}

/// The kind of a source.
///
/// A source is a **reference** to media: where it is, how many channels it
/// has, how long it is. The audio itself is not in the document, and the
/// tree this ports keeps it in a pool of shared buffers beside the
/// document rather than in it.
#[derive(NodeKind)]
#[node_kind(kind = "sound.source", label = "Source", layer = "sound", evaluates)]
pub struct SourceKind;

impl SourceKind {
    /// How a source is drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a source.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What a source **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let source = SourcePayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "path": source.path,
            "channels": source.channels,
            "frames": source.frames,
            "sample_rate": source.sample_rate,
        }))
    }
}

// ------------------------------------------------------------------- the payloads --

/// What a track says about itself.
#[derive(Debug, Clone, PartialEq)]
struct TrackPayload {
    /// What kind of track it is: `audio`, `instrument`, `aux`, `master`, …
    ///
    /// A **string**, and not a variant: the kinds of a track are open.
    track_kind: String,
    /// How many channels it has.
    channels: u32,
}

impl TrackPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({ "track_kind": self.track_kind, "channels": self.channels })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(TrackPayload {
            track_kind: value.get("track_kind").and_then(|kind| kind.as_str()).unwrap_or("audio").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
        })
    }
}

/// What a clip says about itself.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClipPayload {
    /// Where it starts, in samples.
    pub(crate) start: i64,
    /// How long it is, in samples.
    pub(crate) length: i64,
    /// The source it plays, if it plays one.
    source: Option<u64>,
    /// How far into the source it starts, in samples.
    offset: i64,
    /// Its gain, in decibels.
    gain_db: f64,
}

impl ClipPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "start": self.start,
            "length": self.length,
            "source": self.source,
            "offset": self.offset,
            "gain_db": self.gain_db,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let start = value.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let length = value.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
        if start < 0 || length < 0 {
            return Err("a clip does not start before the beginning and does not run backwards".to_string());
        }
        Ok(ClipPayload {
            start,
            length,
            source: value.get("source").and_then(|source| source.as_u64()),
            offset: value.get("offset").and_then(|offset| offset.as_i64()).unwrap_or(0),
            gain_db: value.get("gain_db").and_then(|gain| gain.as_f64()).unwrap_or(0.0),
        })
    }
}

/// What a tempo event says.
#[derive(Debug, Clone, PartialEq)]
struct TempoPayload {
    /// Where it happens, in ticks.
    tick: i64,
    /// How fast, in beats a minute.
    bpm: f64,
}

impl TempoPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({ "tick": self.tick, "bpm": self.bpm })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(TempoPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            bpm: value.get("bpm").and_then(|bpm| bpm.as_f64()).unwrap_or(120.0),
        })
    }
}

/// What a meter event says.
#[derive(Debug, Clone, PartialEq)]
struct MeterPayload {
    /// Where it happens, in ticks.
    tick: i64,
    /// How many beats a bar has.
    numerator: u32,
    /// Which beat a bar has.
    denominator: u32,
}

impl MeterPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "tick": self.tick,
            "numerator": self.numerator,
            "denominator": self.denominator,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(MeterPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            numerator: value.get("numerator").and_then(|value| value.as_u64()).unwrap_or(4) as u32,
            denominator: value.get("denominator").and_then(|value| value.as_u64()).unwrap_or(4) as u32,
        })
    }
}

/// What a marker says.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarkerPayload {
    /// Where it is, in samples.
    pub(crate) start: i64,
    /// Where it ends, in samples.
    end: i64,
}

impl MarkerPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({ "start": self.start, "end": self.end })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(MarkerPayload {
            start: value.get("start").and_then(|start| start.as_i64()).unwrap_or(0),
            end: value.get("end").and_then(|end| end.as_i64()).unwrap_or(0),
        })
    }
}

/// What a source says.
#[derive(Debug, Clone, PartialEq)]
struct SourcePayload {
    /// Where the media is.
    path: String,
    /// How many channels it has.
    channels: u32,
    /// How long it is, in frames.
    frames: i64,
    /// The rate it was recorded at.
    sample_rate: u32,
}

impl SourcePayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "path": self.path,
            "channels": self.channels,
            "frames": self.frames,
            "sample_rate": self.sample_rate,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(SourcePayload {
            path: value.get("path").and_then(|path| path.as_str()).unwrap_or("").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
            frames: value.get("frames").and_then(|frames| frames.as_i64()).unwrap_or(0),
            sample_rate: value.get("sample_rate").and_then(|rate| rate.as_u64()).unwrap_or(48000) as u32,
        })
    }
}

// -------------------------------------------------------------------- the commands --

/// Make a track.
///
/// A track comes with one playlist, because a track with no takes is not a
/// track yet: it is a row with nothing in it.
#[derive(Command)]
#[command(
    id = "sound.track.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "New"),
    mutates,
    undoable
)]
pub struct AddTrack {
    /// What to call it.
    pub name: Text,
    /// What kind of track it is: `audio`, `instrument`, `aux`, `master`.
    pub track_kind: Option<Text>,
    /// How many channels it has.
    pub channels: Option<Number>,
}

impl AddTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        let track_kind = match &self.track_kind {
            Some(kind) => kind.0.clone(),
            None => "audio".to_string(),
        };
        let channels = match &self.channels {
            Some(channels) => channels.0 as u32,
            None => 2,
        };
        let payload = TrackPayload { track_kind, channels };
        let document = session.document_mut()?;
        let track = document.add_node_with(name.clone(), "sound.track", None, payload.to_json());
        document.add_node_with_under(Some(track), "A".to_string(), "sound.playlist", None, Json::Null);
        Ok(Outcome::changed(format!("added a track `{name}`"))
            .with_data(serde_json::json!({ "id": track, "name": name })))
    }
}

/// Put a clip on a track.
///
/// The track may be given as an id, and the playlist as a name: a track
/// holds playlists, and a clip goes in one of them.
#[derive(Command)]
#[command(
    id = "sound.clip.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "New"),
    mutates,
    undoable
)]
pub struct AddClip {
    /// The track the clip goes on.
    pub track: NodeRef,
    /// The playlist it goes in, if the track has more than one.
    pub playlist: Option<Text>,
    /// Where it starts, in samples.
    pub start: Number,
    /// How long it is, in samples.
    pub length: Number,
    /// The source it plays.
    pub source: Option<NodeRef>,
    /// What to call it.
    pub name: Option<Text>,
}

impl AddClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let start = samples(self.start.0, "where the clip starts")?;
        let length = samples(self.length.0, "how long the clip is")?;
        if length == 0 {
            return Err(EngineError::Failed {
                id: "sound.clip.add".to_string(),
                message: "a clip with no length is not a clip".to_string(),
            });
        }
        let playlist = playlist_of(session, self.track.0, self.playlist.as_ref().map(|name| name.0.clone()))?;
        let payload = ClipPayload {
            start,
            length,
            source: self.source.map(|source| source.0),
            offset: 0,
            gain_db: 0.0,
        };
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("clip at {start}"),
        };
        let id = session.document_mut()?.add_node_with_under(
            Some(playlist),
            name.clone(),
            "sound.clip",
            None,
            payload.to_json(),
        );
        // The clip plays the source: that is a **relation**, not
        // containment. The clip is not in the source.
        if let Some(source) = self.source {
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(id, source.0, "source"));
        }
        Ok(Outcome::changed(format!("added a clip `{name}`")).with_data(
            serde_json::json!({ "id": id, "playlist": playlist, "start": start, "length": length }),
        ))
    }
}

/// Trim a clip.
///
/// A trim is about a **range**, and not about a node: the clip keeps its
/// place on the track and its place in the source, and what changes is
/// how much of it is heard. The range may be given, or it may come from
/// the selection — the selection holds a **part** of a node as well as
/// whole nodes, and a part of a clip is exactly what a trim is about.
///
/// The range is in **samples on the track's timeline**, and it is half
/// open: `from` is in the clip and `to` is not.
#[derive(Command)]
#[command(
    id = "sound.clip.trim",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Trim"),
    mutates,
    undoable
)]
pub struct TrimClip {
    /// The clip to trim, if the caller names one.
    pub clip: Option<NodeRef>,
    /// Where the clip starts, in samples.
    pub from: Option<Number>,
    /// Where the clip ends, in samples.
    pub to: Option<Number>,
}

impl TrimClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The clip: named, or the one that is selected.
        let (clip_id, span) = match (self.clip, selection_span(session)) {
            (Some(clip), Some((_, span))) => (clip.0, Some(span)),
            (Some(clip), None) => (clip.0, None),
            (None, Some((node, span))) => (node, Some(span)),
            (None, None) => {
                return Err(EngineError::Failed {
                    id: "sound.clip.trim".to_string(),
                    message: "which clip, and which part of it?".to_string(),
                });
            }
        };
        let (from, to) = match (self.from, self.to) {
            (Some(from), Some(to)) => (from.0, to.0),
            (None, None) => match span {
                Some((from, to)) => (from, to),
                None => {
                    return Err(EngineError::Failed {
                        id: "sound.clip.trim".to_string(),
                        message: "which part of the clip?".to_string(),
                    });
                }
            },
            _ => {
                return Err(EngineError::Failed {
                    id: "sound.clip.trim".to_string(),
                    message: "a range has a start and an end".to_string(),
                });
            }
        };
        if !from.is_finite() || !to.is_finite() {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: "a range is made of numbers".to_string(),
            });
        }
        if to <= from {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the range ends at {to} and starts at {from}"),
            });
        }
        let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.trim".to_string(),
            message: format!("there is no node `{clip_id}`"),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
            });
        }
        let payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.trim".to_string(), message: error })?;
        if from < payload.start as f64 {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the clip starts at {} and cannot be trimmed before it", payload.start),
            });
        }
        let mut trimmed = payload.clone();
        trimmed.start = from as i64;
        trimmed.length = (to - from) as i64;
        // The content moves with the start: a clip trimmed at its head
        // starts later in the source.
        trimmed.offset = payload.offset + (from as i64 - payload.start);
        if let Some(node) = session.document_mut()?.node_mut(clip_id) {
            node.data = trimmed.to_json();
        }
        Ok(Outcome::changed(format!("trimmed the clip `{clip_id}` to {from}..{to}")).with_data(
            serde_json::json!({
                "id": clip_id,
                "from": from,
                "to": to,
                "offset": trimmed.offset,
            }),
        ))
    }
}

/// Set the tempo.
///
/// The tempo map is a **list of events** and not a single number, so this
/// command sets the tempo at a tick: a second call at the same tick
/// changes the one that is there.
#[derive(Command)]
#[command(
    id = "sound.tempo.set",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Event", "Tempo"),
    mutates,
    undoable
)]
pub struct SetTempo {
    /// How fast, in beats a minute.
    pub bpm: Number,
    /// Where, in ticks.
    pub tick: Option<Number>,
}

impl SetTempo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let bpm = self.bpm.0;
        if !bpm.is_finite() || !(5.0..=1000.0).contains(&bpm) {
            return Err(EngineError::Failed {
                id: "sound.tempo.set".to_string(),
                message: format!("{bpm} is not a tempo between 5 and 1000 beats a minute"),
            });
        }
        let tick = match &self.tick {
            Some(tick) => ticks(tick.0)?,
            None => 0,
        };
        let payload = TempoPayload { tick, bpm };
        let existing = find_event(session, "sound.tempo", tick);
        let id = match existing {
            Some(id) => {
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = payload.to_json();
                }
                id
            }
            None => session.document_mut()?.add_node_with(
                format!("tempo {bpm}"),
                "sound.tempo",
                None,
                payload.to_json(),
            ),
        };
        Ok(Outcome::changed(format!("the tempo at tick {tick} is {bpm} beats a minute"))
            .with_data(serde_json::json!({ "id": id, "tick": tick, "bpm": bpm })))
    }
}

/// Set the meter.
///
/// The meter is a list of events too: a bar may be three beats long from
/// here on.
#[derive(Command)]
#[command(
    id = "sound.meter.set",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Event", "Meter"),
    mutates,
    undoable
)]
pub struct SetMeter {
    /// How many beats a bar has.
    pub numerator: Number,
    /// Which beat a bar has.
    pub denominator: Number,
    /// Where, in ticks.
    pub tick: Option<Number>,
}

impl SetMeter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let numerator = self.numerator.0 as u32;
        let denominator = self.denominator.0 as u32;
        if !(1..=64).contains(&numerator) || ![1, 2, 4, 8, 16, 32, 64].contains(&denominator) {
            return Err(EngineError::Failed {
                id: "sound.meter.set".to_string(),
                message: format!("{numerator}/{denominator} is not a meter"),
            });
        }
        let tick = match &self.tick {
            Some(tick) => ticks(tick.0)?,
            None => 0,
        };
        let payload = MeterPayload { tick, numerator, denominator };
        let existing = find_event(session, "sound.meter", tick);
        let id = match existing {
            Some(id) => {
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = payload.to_json();
                }
                id
            }
            None => session.document_mut()?.add_node_with(
                format!("meter {numerator}/{denominator}"),
                "sound.meter",
                None,
                payload.to_json(),
            ),
        };
        Ok(Outcome::changed(format!("the meter at tick {tick} is {numerator}/{denominator}"))
            .with_data(serde_json::json!({ "id": id, "tick": tick })))
    }
}

/// Put a marker in the document.
#[derive(Command)]
#[command(
    id = "sound.marker.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Marker"),
    mutates,
    undoable
)]
pub struct AddMarker {
    /// What to call it.
    pub name: Text,
    /// Where it is, in samples.
    pub start: Number,
    /// Where it ends, in samples.
    pub end: Option<Number>,
}

impl AddMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let start = samples(self.start.0, "where the marker is")?;
        let end = match &self.end {
            Some(end) => samples(end.0, "where the marker ends")?,
            None => start,
        };
        let payload = MarkerPayload { start, end };
        let name = self.name.0.clone();
        let id = session.document_mut()?.add_node_with(name.clone(), "sound.marker", None, payload.to_json());
        Ok(Outcome::changed(format!("added a marker `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "start": start, "end": end })))
    }
}

/// Add a source: a reference to media.
///
/// The path is **relative to the document**, because a document that
/// travels carries its media with it. Nothing here reads the file.
#[derive(Command)]
#[command(
    id = "sound.source.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("File", "Import"),
    mutates,
    undoable
)]
pub struct AddSource {
    /// Where the media is.
    pub path: Text,
    /// How many channels it has.
    pub channels: Option<Number>,
    /// How long it is, in frames.
    pub frames: Option<Number>,
}

impl AddSource {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let channels = match &self.channels {
            Some(channels) => channels.0 as u32,
            None => 2,
        };
        let frames = match &self.frames {
            Some(frames) => samples(frames.0, "how long the source is")?,
            None => 0,
        };
        let session_settings = session_of(session.document());
        let payload =
            SourcePayload { path: path.clone(), channels, frames, sample_rate: session_settings.sample_rate };
        let id = session.document_mut()?.add_node_with(path.clone(), "sound.source", None, payload.to_json());
        // The document now says that it needs this file. The bytes are
        // not in the document: they travel beside it, in a bundle.
        session.document_mut()?.add_asset(craftkit_sdk::Asset::new(path.clone(), "audio", None));
        Ok(Outcome::changed(format!("added a source `{path}`"))
            .with_data(serde_json::json!({ "id": id, "path": path })))
    }
}

/// Set the session: the sample rate and the timecode.
///
/// These are the settings that belong to the session and not to any one
/// node, and they live in the document's own payload.
#[derive(Command)]
#[command(
    id = "sound.session.set",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Options"),
    mutates,
    undoable
)]
pub struct SetSession {
    /// The sample rate, in samples a second.
    pub sample_rate: Option<Number>,
    /// Where the document starts on a timecode, in samples.
    pub timecode_start: Option<Number>,
}

impl SetSession {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut settings = session_of(session.document());
        if let Some(rate) = &self.sample_rate {
            if !rate.0.is_finite() || rate.0 < 1.0 || rate.0 > 768_000.0 {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: format!("{} is not a sample rate", rate.0),
                });
            }
            settings.sample_rate = rate.0 as u32;
        }
        if let Some(start) = &self.timecode_start {
            settings.timecode_start = samples(start.0, "where the timecode starts")?;
        }
        set_session(session.document_mut()?, &settings);
        Ok(Outcome::changed(format!("the session runs at {} samples a second", settings.sample_rate))
            .with_data(serde_json::json!({
                "sample_rate": settings.sample_rate,
                "timecode_start": settings.timecode_start,
            })))
    }
}

// ------------------------------------------------------------------ the tool, the panel --

/// The clip tool.
///
/// A tool is a gesture that runs a command. It is not a command: it has no
/// parameters of its own, it has an icon, and it says which command it
/// runs.
#[derive(Tool)]
#[tool(id = "tool.sound.clip", label = "Clip", layer = "sound", icon = "plus", command = "sound.clip.add")]
pub struct ClipTool;

/// The panel that shows the sound commands.
#[derive(Panel)]
#[panel(
    id = "panel.sound",
    label = "Sound",
    layer = "sound",
    commands(
        "sound.track.add",
        "sound.clip.add",
        "sound.tempo.set",
        "sound.meter.set",
        "sound.marker.add",
        "sound.ruler.add",
        "sound.source.add",
        "sound.session.set"
    )
)]
pub struct SoundPanel;

// ------------------------------------------------------------------- the evaluator --

/// The evaluator that says what a sound document means.
///
/// A kind may speak only about its own nodes; this speaks about the whole
/// document: the **tempo map**, which is derived from the tempo and meter
/// events rather than stored, and every **clip**, in samples and in bars
/// and beats.
#[derive(Evaluator)]
#[evaluator(id = "sound.timeline", label = "Timeline", layer = "sound")]
pub struct TimelineEvaluator;

impl TimelineEvaluator {
    /// The tempo map, derived from the document's events.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        let settings = session_of(document);
        let map = tempo_map(document);

        let mut tracks = Vec::new();
        for track in document.walk().filter(|node| node.kind == "sound.track") {
            let mut playlists = Vec::new();
            for playlist in &track.children {
                if playlist.kind != "sound.playlist" {
                    continue;
                }
                let mut clips = Vec::new();
                for clip in &playlist.children {
                    if clip.kind != "sound.clip" {
                        continue;
                    }
                    let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
                    let position = map.samples_to_ticks(payload.start, settings.sample_rate).max(0);
                    let bar_beat = map.bar_beat_at_tick(position);
                    clips.push(serde_json::json!({
                        "id": clip.id,
                        "name": clip.name,
                        "start": payload.start,
                        "length": payload.length,
                        "end": payload.start.saturating_add(payload.length),
                        "source": payload.source,
                        "position": format!("{}.{}.{}", bar_beat.bar, bar_beat.beat, bar_beat.tick),
                    }));
                }
                playlists.push(serde_json::json!({
                    "id": playlist.id,
                    "name": playlist.name,
                    "clips": clips,
                }));
            }
            tracks.push(serde_json::json!({
                "id": track.id,
                "name": track.name,
                "playlists": playlists,
            }));
        }

        plan.items.push(PlanItem {
            node: 0,
            kind: "sound.timeline".to_string(),
            value: serde_json::json!({
                "sample_rate": settings.sample_rate,
                "timecode_start": settings.timecode_start,
                "tempos": map
                    .tempos()
                    .iter()
                    .map(|tempo| serde_json::json!({ "tick": tempo.tick, "bpm": tempo.bpm }))
                    .collect::<Vec<_>>(),
                "meters": map
                    .meters()
                    .iter()
                    .map(|meter| serde_json::json!({
                        "tick": meter.tick,
                        "numerator": meter.numerator,
                        "denominator": meter.denominator,
                    }))
                    .collect::<Vec<_>>(),
                "tracks": tracks,
            }),
        });
        plan
    }
}

/// The evaluator that works out the routing.
///
/// The buses, in the order they are fed in. It is **derived state**: the
/// document holds the edges and this works out what they mean.
#[derive(Evaluator)]
#[evaluator(id = "sound.routing", label = "Routing", layer = "sound")]
pub struct RoutingEvaluator;

impl RoutingEvaluator {
    /// The routing, in order.
    fn evaluate(_document: &Document) -> Plan {
        let mut plan = Plan::new();
        plan.items.push(PlanItem {
            node: 0,
            kind: "sound.routing".to_string(),
            value: serde_json::json!({ "buses": mixer::routing(_document) }),
        });
        plan
    }
}

// ---------------------------------------------------------------------- the helpers --

/// The tempo map, read from the document's events.
///
/// The map is **derived**: the document holds the events, and the map is
/// what they mean. That is the evaluator's job and not the document's.
pub(crate) fn tempo_map(document: &Document) -> TempoMap {
    let mut map = TempoMap::new();
    for node in document.walk() {
        if node.kind == "sound.tempo"
            && let Ok(payload) = TempoPayload::from_json(&node.data)
        {
            let _ = map.set_tempo(payload.tick, payload.bpm);
        }
        if node.kind == "sound.meter"
            && let Ok(payload) = MeterPayload::from_json(&node.data)
        {
            let _ = map.set_meter(payload.tick, payload.numerator, payload.denominator);
        }
    }
    map
}

/// The event node at a tick, if the document has one.
fn find_event(session: &Session, kind: &str, tick: i64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| {
            if node.kind != kind {
                return false;
            }
            let at = match kind {
                "sound.tempo" => TempoPayload::from_json(&node.data).map(|payload| payload.tick),
                "sound.meter" => MeterPayload::from_json(&node.data).map(|payload| payload.tick),
                _ => Ok(-1),
            };
            matches!(at, Ok(at) if at == tick)
        })
        .map(|node| node.id)
}

/// The playlist a clip goes in.
///
/// The playlist is named, or it is the first one the track has.
fn playlist_of(session: &Session, track: u64, name: Option<String>) -> Result<u64, EngineError> {
    let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
        id: "sound.clip.add".to_string(),
        message: format!("there is no node {track}"),
    })?;
    if node.kind != "sound.track" {
        return Err(EngineError::Failed {
            id: "sound.clip.add".to_string(),
            message: format!("node {track} is a `{}` and not a track", node.kind),
        });
    }
    let playlists: Vec<&Node> =
        node.children.iter().filter(|child| child.kind == "sound.playlist").map(|child| &**child).collect();
    match name {
        Some(name) => {
            playlists.iter().find(|playlist| playlist.name == name).map(|playlist| playlist.id).ok_or_else(
                || EngineError::Failed {
                    id: "sound.clip.add".to_string(),
                    message: format!("the track has no playlist called `{name}`"),
                },
            )
        }
        None => playlists.first().map(|playlist| playlist.id).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.add".to_string(),
            message: format!("the track {track} has no playlist"),
        }),
    }
}

/// The part of a node that is selected, if a part is selected.
///
/// The selection holds whole nodes and **parts** of nodes, and a part is
/// a range in the node's own units. A sound document's units are
/// samples, so a part of a clip is a range of samples.
fn selection_span(session: &Session) -> Option<(u64, (f64, f64))> {
    session.selection().iter().find_map(|item| {
        let from = item.from?;
        let to = item.to?;
        Some((item.node, (from, to)))
    })
}

/// A number of samples, from a number a caller sent.
///
/// A number from outside is hostile: it may not be a number at all, and it
/// may be a number so large that it is not a position.
fn samples(value: f64, what: &str) -> Result<i64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed {
            id: "sound".to_string(),
            message: format!("{what} is not a number"),
        });
    }
    let rounded = value.round();
    if rounded.abs() > MAX_POSITION as f64 {
        return Err(EngineError::Failed {
            id: "sound".to_string(),
            message: format!("{what} is past the end of time"),
        });
    }
    Ok(rounded as i64)
}

/// A number of ticks, from a number a caller sent.
fn ticks(value: f64) -> Result<i64, EngineError> {
    let ticks = samples(value, "where the event happens")?;
    if ticks < 0 {
        return Err(EngineError::Failed {
            id: "sound".to_string(),
            message: "an event does not happen before the beginning".to_string(),
        });
    }
    Ok(ticks)
}

// The plugin's anchor.
//
// One name that keeps every item above in the program, on every target —
// including `wasm32-unknown-unknown`, whose linker drops a crate that
// nothing names. See the SDK's `anchor` module.
craftkit_sdk::plugin_anchor!(
    SoundLayer,
    TrackKind,
    PlaylistKind,
    ClipKind,
    TempoKind,
    MeterKind,
    MarkerKind,
    SourceKind,
    AddTrack,
    AddClip,
    SetTempo,
    SetMeter,
    AddMarker,
    AddSource,
    SetSession,
    ClipTool,
    SoundPanel,
    TimelineEvaluator,
    RoutingEvaluator,
    BusKind,
    InsertKind,
    SendKind,
    LaneKind,
    AddBus,
    AddRoute,
    AddInsert,
    AddSend,
    AddLane
);

#[cfg(test)]
mod tests {
    use super::*;

    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// The plugin's own kinds are in the registry.
    #[test]
    fn the_kinds_are_there() {
        let ecosystem = ecosystem();
        assert!(ecosystem.kinds.get("sound.track").is_some());
        assert!(ecosystem.kinds.get("sound.clip").is_some());
        assert!(ecosystem.commands.get("sound.track.add").is_some());
        assert!(ecosystem.evaluators.get("sound.timeline").is_some());
    }

    /// A track comes with a playlist.
    #[test]
    fn a_track_comes_with_a_playlist() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there");
        assert_eq!(track.kind, "sound.track");
        assert_eq!(track.rect, None, "a track is at no place: it is at a time");
        assert_eq!(track.children.len(), 1, "the track has a playlist");
        assert_eq!(track.children[0].kind, "sound.playlist");
    }

    /// A clip is a **bar** on the track's row: the timeline is the sound
    /// document's own world.
    ///
    /// The place is **derived** — from the clip's span and from the row
    /// of the track it is under — and it is not in the document, because
    /// a rectangle in the document would go stale the moment a track was
    /// added above it.
    #[test]
    fn a_clip_is_a_bar_on_the_rows() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there");
        session
            .execute("sound.clip.add", &json!({ "track": track.id, "start": 48000, "length": 96000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
        let rect = ClipKind::place(clip, session.document()).expect("the clip has a place");
        assert_eq!(rect.x, 1.0, "the clip starts one second in");
        assert_eq!(rect.w, 2.0, "and it is two seconds long");
        assert_eq!(rect.y, 0.0, "on the first row");
        assert_eq!(rect.h, timeline::ROW_HEIGHT, "and the row is one unit tall");
        assert_eq!(clip.rect, None, "and the document holds no rectangle for it");
    }

    /// The **second** track is on the second row, and the clip on it
    /// moves with it.
    #[test]
    fn a_second_track_is_on_the_second_row() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the first track");
        session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the second track");
        let bass = session.document().node_by_name("bass").expect("the track is there");
        session
            .execute("sound.clip.add", &json!({ "track": bass.id, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
        let rect = ClipKind::place(clip, session.document()).expect("the clip has a place");
        assert_eq!(rect.y, timeline::ROW_HEIGHT, "the clip is on the second row");
    }

    /// A clip is **drawn**, and a clip is **picked**.
    #[test]
    fn a_clip_is_drawn_and_picked() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there");
        session
            .execute("sound.clip.add", &json!({ "track": track.id, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
        let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
        ClipKind::paint(clip, session.document(), &mut canvas);
        assert_eq!(canvas.drawn.len(), 1, "the clip is one bar");
        assert!(canvas.drawn[0].starts_with("rect("), "and it is a rectangle: {}", canvas.drawn[0]);
        assert!(
            ClipKind::hit(clip, session.document(), Point::new(0.5, 0.5)),
            "a point inside the bar is in the clip"
        );
        assert!(
            !ClipKind::hit(clip, session.document(), Point::new(5.0, 0.5)),
            "and a point outside it is not"
        );
    }

    /// A tempo event is a **mark in the ruler**, and the ruler is above
    /// the tracks.
    ///
    /// The event's time is in **ticks** and the world is in **seconds**,
    /// so the tick is worked out against the tempo map: a tick is a
    /// musical position, and it is not a length until the tempo says how
    /// fast the music is.
    #[test]
    fn a_tempo_event_is_a_mark_in_the_ruler() {
        let mut session = session();
        session.execute("sound.tempo.set", &json!({ "tick": 960, "bpm": 120.0 })).expect("the tempo is set");
        let tempo =
            session.document().walk().find(|node| node.kind == "sound.tempo").expect("the tempo is there");
        let rect = TempoKind::place(tempo, session.document()).expect("the tempo has a place");
        assert_eq!(rect.y, -timeline::RULER_HEIGHT, "the ruler is above the tracks");
        // At a hundred and twenty beats a minute, a beat is half a
        // second, and tick 960 is one beat.
        assert!((rect.x - 0.5).abs() < 0.001, "and the beat is half a second in: {}", rect.x);
    }

    /// The ruler carries the **bar numbers**.
    ///
    /// The bars come from the tempo map: at a hundred and twenty beats
    /// a minute a bar of four beats is two seconds long.
    #[test]
    fn the_ruler_carries_the_bar_numbers() {
        let mut session = session();
        session.execute("sound.tempo.set", &json!({ "tick": 0, "bpm": 120.0 })).expect("the tempo is set");
        session
            .execute("sound.meter.set", &json!({ "tick": 0, "numerator": 4, "denominator": 4 }))
            .expect("the meter is set");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480000 }))
            .expect("the clip is added");
        session.execute("sound.ruler.add", &json!({ "name": "ruler" })).expect("the ruler is added");
        let ruler =
            session.document().walk().find(|node| node.kind == "sound.ruler").expect("the ruler is there");
        let marks = timeline::scale_marks(session.document(), ruler);
        assert!(!marks.is_empty(), "the ruler has no numbers");
        assert_eq!(marks[0].label, "1", "the first bar is bar one");
        assert!((marks[0].x - 0.0).abs() < 0.001, "and it is at the beginning");
        let second = marks.iter().find(|mark| mark.label == "2").expect("the second bar is numbered");
        assert!((second.x - 2.0).abs() < 0.001, "and a bar is two seconds long: {}", second.x);
    }

    /// The ruler can count by **seconds** instead.
    #[test]
    fn the_ruler_can_count_by_seconds() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480000 }))
            .expect("the clip is added");
        session
            .execute("sound.ruler.add", &json!({ "name": "ruler", "scale": "seconds" }))
            .expect("the ruler is added");
        let ruler =
            session.document().walk().find(|node| node.kind == "sound.ruler").expect("the ruler is there");
        let marks = timeline::scale_marks(session.document(), ruler);
        assert_eq!(marks[0].label, "1", "the first second is second one");
        assert!((marks[0].x - 1.0).abs() < 0.001, "and it is one second in");
    }

    /// The ruler is the **row above the tracks**.
    #[test]
    fn the_ruler_is_the_row_above_the_tracks() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.ruler.add", &json!({ "name": "ruler" })).expect("the ruler is added");
        let ruler =
            session.document().walk().find(|node| node.kind == "sound.ruler").expect("the ruler is there");
        let rect = RulerKind::place(ruler, session.document()).expect("the ruler has a place");
        assert_eq!(rect.y, -timeline::RULER_HEIGHT, "the ruler is above the tracks");
        assert_eq!(rect.h, timeline::RULER_HEIGHT, "and it is a row");
    }

    /// A scale that is not a scale is **refused**.
    #[test]
    fn a_scale_that_is_not_a_scale_is_refused() {
        let mut session = session();
        let error = session
            .execute("sound.ruler.add", &json!({ "name": "ruler", "scale": "eighths" }))
            .expect_err("that is not a scale");
        assert!(error.to_string().contains("bars"), "the refusal says what is wrong: {error}");
    }

    /// The kind of a track is a string, and it is open.
    #[test]
    fn the_kind_of_a_track_is_a_string() {
        let mut session = session();
        session
            .execute("sound.track.add", &json!({ "name": "bass", "track_kind": "instrument" }))
            .expect("the track is added");
        let track = session.document().node_by_name("bass").expect("the track is there");
        let payload = TrackPayload::from_json(&track.data).expect("the payload is read");
        assert_eq!(payload.track_kind, "instrument");
    }

    /// A clip goes on a track, in its playlist.
    #[test]
    fn a_clip_goes_on_a_track() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        let clip = session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip");
        let payload = ClipPayload::from_json(&clip.data).expect("the payload is read");
        assert_eq!(payload.start, 0);
        assert_eq!(payload.length, 48000);
        assert_eq!(clip.rect, None, "a clip is at a time and not at a place");
    }

    /// A clip on something that is not a track is refused.
    #[test]
    fn a_clip_on_something_else_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let error = session
            .execute("sound.clip.add", &json!({ "track": track + 100, "start": 0, "length": 1 }))
            .expect_err("the clip is refused");
        assert!(error.to_string().contains("no node"), "{error}");
    }

    /// A clip with no length is refused.
    #[test]
    fn a_clip_with_no_length_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let error = session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 0 }))
            .expect_err("the clip is refused");
        assert!(error.to_string().contains("no length"), "{error}");
    }

    /// The tempo map is a list of events, and the second one at a tick
    /// changes the first.
    #[test]
    fn the_tempo_is_a_list_of_events() {
        let mut session = session();
        session.execute("sound.tempo.set", &json!({ "bpm": 120.0 })).expect("the tempo is set");
        session.execute("sound.tempo.set", &json!({ "bpm": 140.0, "tick": 960 })).expect("the tempo is set");
        session.execute("sound.tempo.set", &json!({ "bpm": 100.0 })).expect("the tempo is set");
        let map = tempo_map(session.document());
        assert_eq!(map.tempos().len(), 2, "the second tempo at tick zero changed the first");
        assert_eq!(map.tempo_at_tick(0), 100.0);
        assert_eq!(map.tempo_at_tick(960), 140.0);
    }

    /// A tempo that is not a tempo is refused.
    #[test]
    fn a_tempo_that_is_not_a_tempo_is_refused() {
        let mut session = session();
        let error =
            session.execute("sound.tempo.set", &json!({ "bpm": 0.0 })).expect_err("the tempo is refused");
        assert!(error.to_string().contains("not a tempo"), "{error}");
    }

    /// The session's settings live in the document's payload.
    #[test]
    fn the_session_lives_in_the_document() {
        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 44100 })).expect("the session is set");
        let settings = session_of(session.document());
        assert_eq!(settings.sample_rate, 44100);
        assert_eq!(
            session.document().data.get("sound").and_then(|sound| sound.get("sample_rate")),
            Some(&json!(44100)),
            "the settings are in the document's payload"
        );
    }

    /// The timeline says where every clip is, in samples and in bars.
    #[test]
    fn the_timeline_says_where_the_clips_are() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 48000, "length": 24000 }))
            .expect("the clip is added");
        let plan = TimelineEvaluator::evaluate(session.document());
        let item = &plan.items[0];
        let tracks = item.value.get("tracks").and_then(|tracks| tracks.as_array()).expect("the tracks");
        let clips = tracks[0]
            .get("playlists")
            .and_then(|playlists| playlists.as_array())
            .and_then(|playlists| playlists[0].get("clips").and_then(|clips| clips.as_array()))
            .expect("the clips");
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].get("start"), Some(&json!(48000)));
        // At 48 kHz and 120 bpm a beat is 24000 samples, so the clip at
        // 48000 sits on the third beat of the first bar.
        assert_eq!(clips[0].get("position"), Some(&json!("1.3.0")));
    }

    /// A source is a file the document needs.
    ///
    /// The document says so in one place, so that a program can tell what
    /// is missing before it starts editing.
    #[test]
    fn a_source_is_an_asset() {
        let mut session = session();
        session.execute("sound.source.add", &json!({ "path": "kit/kick.wav" })).expect("the source is added");
        let assets = &session.document().assets;
        assert_eq!(assets.len(), 1, "the document needs the file: {assets:?}");
        assert_eq!(assets[0].path, "kit/kick.wav");
        assert_eq!(assets[0].kind, "audio");
        assert_eq!(assets[0].length, None, "nobody has read the file, so nobody knows its length");
    }

    /// The kinds of a sound document are open: a track whose kind the core
    /// has never heard of is still a track.
    #[test]
    fn the_kinds_are_open() {
        let mut document = Document::new("open");
        document.add_node_with("odd", "sound.track", None, json!({ "track_kind": "quantum" }));
        let track = document.nodes[0].clone();
        let payload = TrackPayload::from_json(&track.data).expect("the payload is read");
        assert_eq!(payload.track_kind, "quantum");
    }
}

#[cfg(test)]
mod trim_tests {
    use super::*;

    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// A track with one clip on it, and the clip's id.
    fn with_clip(session: &mut Session) -> u64 {
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip").id
    }

    /// A trim with an explicit range shortens the clip.
    #[test]
    fn a_trim_shortens_the_clip() {
        let mut session = session();
        let clip = with_clip(&mut session);
        session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": 0, "to": 24000 }))
            .expect("the clip is trimmed");
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        assert_eq!(payload.start, 0);
        assert_eq!(payload.length, 24000);
    }

    /// A trim with a range that starts later moves the content with it.
    #[test]
    fn a_trim_moves_the_content_with_it() {
        let mut session = session();
        let clip = with_clip(&mut session);
        session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": 12000, "to": 24000 }))
            .expect("the clip is trimmed");
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        assert_eq!(payload.start, 12000);
        assert_eq!(payload.length, 12000);
        assert_eq!(payload.offset, 12000, "the content starts later in the source");
    }

    /// A trim takes its range from the **selection**.
    ///
    /// The selection holds a part of a node as well as whole nodes, and a
    /// part of a clip is what a trim is about.
    #[test]
    fn a_trim_takes_its_range_from_the_selection() {
        let mut session = session();
        let clip = with_clip(&mut session);
        session
            .execute("select.span", &json!({ "node": clip, "from": 0.0, "to": 24000.0 }))
            .expect("the part is selected");
        session.execute("sound.clip.trim", &json!({})).expect("the clip is trimmed");
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        assert_eq!(payload.length, 24000, "the clip was trimmed to the selection");
    }

    /// A trim before the clip's own start is refused.
    #[test]
    fn a_trim_before_the_clip_is_refused() {
        let mut session = session();
        let clip = with_clip(&mut session);
        let error = session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": -100.0, "to": 24000.0 }))
            .expect_err("the trim is refused");
        assert!(error.to_string().contains("cannot be trimmed before"), "{error}");
    }

    /// A range that ends before it starts is refused.
    #[test]
    fn a_range_that_ends_first_is_refused() {
        let mut session = session();
        let clip = with_clip(&mut session);
        let error = session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": 100.0, "to": 100.0 }))
            .expect_err("the trim is refused");
        assert!(error.to_string().contains("starts at"), "{error}");
    }
}
