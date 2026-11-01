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
//! - **thirteen kinds**: `sound.track`, `sound.playlist`, `sound.clip`,
//!   `sound.bus`, `sound.send`, `sound.insert`, `sound.lane`,
//!   `sound.tempo`, `sound.meter`, `sound.marker`, `sound.ruler`,
//!   `sound.playhead` and `sound.source`;
//! - **thirty-two commands**, one for each thing a person does to a
//!   sound document: make a track, set a track, make a clip, mute a
//!   clip, space the clips against each other, snap a clip to its
//!   neighbour, loop a clip and take the
//!   loop away, make a take, choose the
//!   take that is played, set the tempo,
//!   set the meter, make a marker, make a source, set the session, and
//!   the rest that came with the mixer — the bus, the send, the insert,
//!   the lane, the route, the input, the peaks and the render;
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
mod automate;
mod mixer;

use std::collections::HashMap;

use craftkit_sdk::document::{Path, Rect, Segment, SubPath};
use craftkit_sdk::{
    Bool, Canvas, Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Point, Session, Text,
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
    /// Whether the **metronome** is heard.
    ///
    /// The click is mixed in after everything else, so it reaches the
    /// output without passing through a track, a send or an insert.
    pub metronome: bool,
    /// How loud the click is, in decibels.
    pub click_volume_db: f64,
    /// The click's pitch, as a note number.
    pub click_note: f64,
    /// The **accent's** pitch: the click on the first beat of a bar.
    pub click_accent_note: f64,
}

impl Default for SoundSession {
    fn default() -> Self {
        SoundSession {
            sample_rate: 48000,
            timecode_start: 0,
            metronome: false,
            click_volume_db: -6.0,
            click_note: 72.0,
            click_accent_note: 84.0,
        }
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
    let metronome = settings
        .and_then(|settings| settings.get("metronome"))
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false);
    let click_volume_db = settings
        .and_then(|settings| settings.get("click_volume_db"))
        .and_then(|volume| volume.as_f64())
        .filter(|volume| volume.is_finite())
        .unwrap_or(SoundSession::default().click_volume_db);
    let click_note = settings
        .and_then(|settings| settings.get("click_note"))
        .and_then(|note| note.as_f64())
        .filter(|note| note.is_finite())
        .unwrap_or(SoundSession::default().click_note);
    let click_accent_note = settings
        .and_then(|settings| settings.get("click_accent_note"))
        .and_then(|note| note.as_f64())
        .filter(|note| note.is_finite())
        .unwrap_or(SoundSession::default().click_accent_note);
    SoundSession { sample_rate, timecode_start, metronome, click_volume_db, click_note, click_accent_note }
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
            "metronome": session.metronome,
            "click_volume_db": session.click_volume_db,
            "click_note": session.click_note,
            "click_accent_note": session.click_accent_note,
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
        let stroke = craftkit_sdk::document::Stroke {
            colour: craftkit_sdk::document::Colour::BLACK,
            width: 0.01,
            ..Default::default()
        };
        canvas.path(&line, craftkit_sdk::document::Colour::NONE, Some(stroke));
        // A **soloed** track is a double rule: the row says which
        // track is heard alone.
        if track_is_soloed(document, node.id) {
            let under = craftkit_sdk::document::Path::polygon(&[
                Point::new(rect.x, rect.y + 0.05),
                Point::new(rect.x + rect.w, rect.y + 0.05),
            ]);
            canvas.path(&under, craftkit_sdk::document::Colour::NONE, Some(stroke));
        }
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
    ///
    /// The **fades** are notches in the bar: the quiet part of a clip
    /// is the paper showing through the ink, and the notch is the
    /// fade's own curve.
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
        if node.muted {
            return;
        }
        let Ok(payload) = ClipPayload::from_json(&node.data) else { return };
        let rate = session_of(document).sample_rate as f64;
        if rate <= 0.0 {
            return;
        }
        let paper = craftkit_sdk::document::Colour::WHITE;
        // The **waveform**: the shape of the sound the clip plays, drawn
        // on the bar in the paper's colour, the way the fade's notch is.
        for path in waveform_of(document, node, &payload, &rect) {
            canvas.path(&path, paper, None);
        }
        let width = payload.fade_in.len as f64 / rate;
        if width > 0.0
            && width < rect.w
            && let Some(path) = fade_in_path(&rect, width, payload.fade_in.fade())
        {
            canvas.path(&path, paper, None);
        }
        let width = payload.fade_out.len as f64 / rate;
        if width > 0.0
            && width < rect.w
            && let Some(path) = fade_out_path(&rect, width, payload.fade_out.fade())
        {
            canvas.path(&path, paper, None);
        }
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
            "fade_in": clip.fade_in.to_json(),
            "fade_out": clip.fade_out.to_json(),
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                            ..Default::default()
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
    /// Whether the track is **soloed**.
    ///
    /// A soloed track silences the others: when one track is soloed,
    /// the rest are not heard.
    solo: bool,
    /// The track's **trim**, in decibels: the gain of the track's
    /// input, before its inserts.
    ///
    /// The tree's trim runs from minus one hundred and forty-four
    /// decibels to twenty-four; it is the level of the input and not
    /// the level of the output.
    trim_db: f64,
    /// Whether the track's **phase is inverted**.
    ///
    /// The phase is a sign and not a level: the sound is the same and
    /// upside down.
    phase_invert: bool,
    /// The name of the **take** that is heard.
    ///
    /// A track may have several takes — the tree's playlists — and one
    /// of them is the one that is played. A track with no name here
    /// plays its first take.
    active_playlist: Option<String>,
}

impl TrackPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "track_kind": self.track_kind,
            "channels": self.channels,
            "solo": self.solo,
            "trim_db": self.trim_db,
            "phase_invert": self.phase_invert,
            "active_playlist": self.active_playlist,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(TrackPayload {
            track_kind: value.get("track_kind").and_then(|kind| kind.as_str()).unwrap_or("audio").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
            solo: value.get("solo").and_then(|solo| solo.as_bool()).unwrap_or(false),
            trim_db: value.get("trim_db").and_then(|trim| trim.as_f64()).unwrap_or(0.0),
            phase_invert: value.get("phase_invert").and_then(|phase| phase.as_bool()).unwrap_or(false),
            active_playlist: value
                .get("active_playlist")
                .and_then(|playlist| playlist.as_str())
                .map(|playlist| playlist.to_string()),
        })
    }
}

/// The **take** that is heard: the track's active playlist, or the
/// first one it has.
///
/// A track may have several takes and one of them is played; a track
/// whose active take is not there plays its first one, so that a track
/// is never silent by accident.
fn active_playlist(_document: &Document, track: &Node) -> Option<u64> {
    let playlists: Vec<&Node> =
        track.children.iter().filter(|child| child.kind == "sound.playlist").map(|child| &**child).collect();
    let wanted = TrackPayload::from_json(&track.data).ok().and_then(|payload| payload.active_playlist);
    match wanted {
        Some(name) => playlists
            .iter()
            .find(|playlist| playlist.name == name)
            .map(|playlist| playlist.id)
            .or_else(|| playlists.first().map(|playlist| playlist.id)),
        None => playlists.first().map(|playlist| playlist.id),
    }
}

/// Whether a track is a **master**: a track whose fader is over the
/// whole mix and not over a part of it.
///
/// The kinds of a track are open — they are a string in the track's
/// payload — and `master` is one of them: the tree's mixer runs the
/// master faders over the main mix, and a master holds no clips.
fn is_master(node: &Node) -> bool {
    TrackPayload::from_json(&node.data).map(|payload| payload.track_kind == "master").unwrap_or(false)
}

/// What a **fade** says about itself.
///
/// A fade is a length, a shape and a bend, and it is the same thing at
/// both ends of a clip: the head fades in and the tail fades out.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FadePayload {
    /// How long it is, in samples. Zero is no fade at all.
    len: i64,
    /// The curve it follows.
    shape: craftkit_audio::FadeShape,
    /// How much the curve is bent, from -1 to 1.
    curvature: f32,
}

impl FadePayload {
    /// No fade at all.
    fn none() -> FadePayload {
        FadePayload { len: 0, shape: craftkit_audio::FadeShape::default(), curvature: 0.0 }
    }

    fn to_json(self) -> Json {
        serde_json::json!({
            "len": self.len,
            "shape": self.shape.label(),
            "curvature": self.curvature,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let len = value.get("len").and_then(|len| len.as_i64()).unwrap_or(0);
        if len < 0 {
            return Err("a fade does not run backwards".to_string());
        }
        let shape = match value.get("shape").and_then(|shape| shape.as_str()) {
            Some(label) => craftkit_audio::FadeShape::from_label(label)
                .ok_or_else(|| format!("there is no fade shape called `{label}`"))?,
            None => craftkit_audio::FadeShape::default(),
        };
        let curvature = value.get("curvature").and_then(|curvature| curvature.as_f64()).unwrap_or(0.0);
        if !curvature.is_finite() {
            return Err("a bend is a number".to_string());
        }
        Ok(FadePayload { len, shape, curvature: curvature as f32 })
    }

    /// The fade itself, as the audio crate has it.
    fn fade(&self) -> craftkit_audio::Fade {
        craftkit_audio::Fade { len: self.len.max(0) as u64, shape: self.shape, curvature: self.curvature }
    }
}

/// What a clip says about itself.
///
/// Every number here is in the **session's samples**: the clip's place
/// on the timeline, how long it is there, and how far into its source
/// it starts. The source has its own rate, and the render works out
/// the difference; the document speaks the timeline's language.
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
    /// The fade at its head.
    fade_in: FadePayload,
    /// The fade at its tail.
    fade_out: FadePayload,
    /// How much **longer** it is played than the source it plays.
    ///
    /// One is none. Two is twice as long and an octave lower, because
    /// the pitch follows the speed.
    stretch: f64,
    /// Whether the clip is **muted**.
    ///
    /// A muted clip is not played and is still there: the tree's
    /// renderer skips it and the tree's editor keeps it.
    muted: bool,
}

/// The most a clip may be stretched.
///
/// A stretch is a length ratio, and a clip that is stretched a
/// thousandfold would read a thousandth of a sample of its source and
/// hold it for a thousand times as long. The bound is here so that a
/// number from outside cannot ask for an absurd thing.
const MAX_STRETCH: f64 = 64.0;

impl ClipPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "start": self.start,
            "length": self.length,
            "source": self.source,
            "offset": self.offset,
            "gain_db": self.gain_db,
            "fade_in": self.fade_in.to_json(),
            "fade_out": self.fade_out.to_json(),
            "stretch": self.stretch,
            "muted": self.muted,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let start = value.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let length = value.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
        if start < 0 || length < 0 {
            return Err("a clip does not start before the beginning and does not run backwards".to_string());
        }
        let stretch = value.get("stretch").and_then(|stretch| stretch.as_f64()).unwrap_or(1.0);
        if !stretch.is_finite() || stretch <= 0.0 || stretch > MAX_STRETCH {
            return Err(format!(
                "a stretch is greater than zero and no more than {MAX_STRETCH}, and this one is {stretch}"
            ));
        }
        Ok(ClipPayload {
            start,
            length,
            source: value.get("source").and_then(|source| source.as_u64()),
            offset: value.get("offset").and_then(|offset| offset.as_i64()).unwrap_or(0),
            gain_db: value.get("gain_db").and_then(|gain| gain.as_f64()).unwrap_or(0.0),
            fade_in: match value.get("fade_in") {
                Some(fade) => FadePayload::from_json(fade)?,
                None => FadePayload::none(),
            },
            fade_out: match value.get("fade_out") {
                Some(fade) => FadePayload::from_json(fade)?,
                None => FadePayload::none(),
            },
            stretch,
            muted: value.get("muted").and_then(|muted| muted.as_bool()).unwrap_or(false),
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
    /// The **peaks** of the sound: the extent of the samples in each
    /// bucket, as a flat table of a least and a most.
    ///
    /// The sound is a great many numbers and a drawing needs few: the
    /// peaks are the shape of the sound, and the shape is what a reader
    /// sees. They are **measured** — a command reads the file and puts
    /// them here — because a file that is read at every frame is a file
    /// that is read a hundred times a second.
    peaks: Vec<f32>,
}

impl SourcePayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "path": self.path,
            "channels": self.channels,
            "frames": self.frames,
            "sample_rate": self.sample_rate,
            "peaks": self.peaks,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        // The peaks are **bounded**: the table comes from outside, and
        // a table of a million numbers is not a shape of a sound.
        let peaks = value
            .get("peaks")
            .and_then(|peaks| peaks.as_array())
            .map(|peaks| {
                peaks
                    .iter()
                    .filter_map(|peak| peak.as_f64())
                    .take(PEAK_NUMBERS)
                    .map(|peak| peak as f32)
                    .collect()
            })
            .unwrap_or_default();
        Ok(SourcePayload {
            path: value.get("path").and_then(|path| path.as_str()).unwrap_or("").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
            frames: value.get("frames").and_then(|frames| frames.as_i64()).unwrap_or(0),
            sample_rate: value.get("sample_rate").and_then(|rate| rate.as_u64()).unwrap_or(48000) as u32,
            peaks,
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
        let payload = TrackPayload {
            track_kind,
            channels,
            solo: false,
            trim_db: 0.0,
            phase_invert: false,
            active_playlist: None,
        };
        let document = session.document_mut()?;
        let track = document.add_node_with(name.clone(), "sound.track", None, payload.to_json());
        document.add_node_with_under(Some(track), "A".to_string(), "sound.playlist", None, Json::Null);
        Ok(Outcome::changed(format!("added a track `{name}`"))
            .with_data(serde_json::json!({ "id": track, "name": name })))
    }
}

/// Add a **take** to a track.
///
/// A take is the tree's playlist: an alternative set of clips on the
/// same track, and one of the takes is the one that is played.
#[derive(Command)]
#[command(
    id = "sound.playlist.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "Takes"),
    mutates,
    undoable
)]
pub struct AddPlaylist {
    /// The track the take is on.
    pub track: NodeRef,
    /// What to call it.
    pub name: Text,
}

impl AddPlaylist {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: "a take says what it is called".to_string(),
            });
        }
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        if node.children.iter().any(|child| child.kind == "sound.playlist" && child.name == name) {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: format!("the track `{track}` already has a take called `{name}`"),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            Some(track),
            name.clone(),
            "sound.playlist",
            None,
            Json::Null,
        );
        Ok(Outcome::changed(format!("the track `{track}` has a take called `{name}`"))
            .with_data(serde_json::json!({ "id": id, "track": track, "name": name })))
    }
}

/// Say which **take** of a track is played.
///
/// The tree's track has several playlists and an index of the one that
/// is played; here the take is named, because a name is what a person
/// says and an index is what a program says.
#[derive(Command)]
#[command(
    id = "sound.playlist.active",
    area = "sound",
    layer = "sound",
    icon = "check",
    menu("Track", "Takes"),
    mutates,
    undoable
)]
pub struct SetActivePlaylist {
    /// The track the take is on.
    pub track: NodeRef,
    /// The take that is played.
    pub playlist: Text,
}

impl SetActivePlaylist {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let name = self.playlist.0.clone();
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        if !node.children.iter().any(|child| child.kind == "sound.playlist" && child.name == name) {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("the track `{track}` has no take called `{name}`"),
            });
        }
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.playlist.active".to_string(),
            message: error.to_string(),
        })?;
        payload.active_playlist = Some(name.clone());
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{track}` plays the take `{name}`"))
            .with_data(serde_json::json!({ "track": track, "playlist": name })))
    }
}

/// Set a track's **trim** and **phase**.
///
/// The trim is the level of the track's **input**, before its inserts,
/// and the phase is a sign and not a level: the sound is the same and
/// upside down.
#[derive(Command)]
#[command(
    id = "sound.track.set",
    area = "sound",
    layer = "sound",
    icon = "sliders",
    menu("Track", "Trim"),
    mutates,
    undoable
)]
pub struct SetTrack {
    /// The track to set.
    pub track: NodeRef,
    /// Its trim, in decibels: from minus one hundred and forty-four to
    /// twenty-four.
    pub trim_db: Option<Number>,
    /// Whether its phase is inverted.
    pub phase: Option<Bool>,
}

impl SetTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.set".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.set".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.set".to_string(),
            message: error.to_string(),
        })?;
        if let Some(trim) = &self.trim_db {
            let trim = trim.0;
            if !trim.is_finite() || !(-144.0..=24.0).contains(&trim) {
                return Err(EngineError::Failed {
                    id: "sound.track.set".to_string(),
                    message:
                        "a trim is a number of decibels from minus one hundred and forty-four to twenty-four"
                            .to_string(),
                });
            }
            payload.trim_db = trim;
        }
        if let Some(phase) = &self.phase {
            payload.phase_invert = phase.0;
        }
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.set".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{id}` is set")).with_data(
            serde_json::json!({ "id": id, "trim_db": payload.trim_db, "phase": payload.phase_invert }),
        ))
    }
}

/// **Solo** a track.
///
/// A soloed track silences the others: when one track is soloed, the
/// rest are not heard — and their sends carry the silence, because a
/// send is a copy of the track's signal.
///
/// Soloing nothing is not the same as soloing everything: a document
/// with no soloed track is heard whole, and a document with one is
/// heard alone.
#[derive(Command)]
#[command(
    id = "sound.track.solo",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Solo"),
    mutates,
    undoable
)]
pub struct SoloTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether the track is soloed.
    pub solo: Bool,
}

impl SoloTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.solo".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.solo".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let solo = self.solo.0;
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.solo".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        let mut payload = TrackPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.track.solo".to_string(), message: error })?;
        payload.solo = solo;
        node.data = payload.to_json();
        Ok(Outcome::changed(format!("the track `{id}` is soloed: {solo}"))
            .with_data(serde_json::json!({ "id": id, "solo": solo })))
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
    /// How much **longer** it is played than its source: one is none.
    pub stretch: Option<Number>,
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
        let stretch = match self.stretch {
            Some(stretch) => stretch_of(stretch.0, "sound.clip.add")?,
            None => 1.0,
        };
        // A **master** holds a fader and not a playlist: a clip on a
        // master would be a clip that is never heard.
        match session.document().node(self.track.0) {
            Some(node) if node.kind == "sound.track" && is_master(node) => {
                return Err(EngineError::Failed {
                    id: "sound.clip.add".to_string(),
                    message: "a master holds no clips: it has a fader and not a playlist".to_string(),
                });
            }
            _ => {}
        }
        // A **reader** takes a bus into the mix: it holds a bus and not
        // a playlist.
        if mixer::reads_bus(session.document(), self.track.0) {
            return Err(EngineError::Failed {
                id: "sound.clip.add".to_string(),
                message: "a track that reads a bus holds no clips: it has a bus and not a playlist"
                    .to_string(),
            });
        }
        let playlist = playlist_of(session, self.track.0, self.playlist.as_ref().map(|name| name.0.clone()))?;
        let payload = ClipPayload {
            start,
            length,
            source: self.source.map(|source| source.0),
            offset: 0,
            gain_db: 0.0,
            fade_in: FadePayload::none(),
            fade_out: FadePayload::none(),
            stretch,
            muted: false,
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

/// Stretch a clip.
///
/// A stretched clip is **longer and lower**: the source is read more
/// slowly, so the sound lasts longer and the pitch falls with it. This
/// is a **varispeed**, and it is not a time stretch — a time stretch
/// keeps the pitch and changes the length, and that is a much harder
/// thing to do well.
///
/// The clip keeps its place and its length: the stretch says **how the
/// source is played**, and not how long the clip is. One is no stretch.
///
/// The clip's place in the source moves with it: trimming a stretched
/// clip moves the content by the delta **divided by the stretch**,
/// because the source is read that much more slowly.
#[derive(Command)]
#[command(
    id = "sound.clip.stretch",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Stretch"),
    mutates,
    undoable
)]
pub struct SetClipStretch {
    /// The clip to stretch.
    pub clip: NodeRef,
    /// How much longer it is played than its source: one is none.
    pub stretch: Number,
}

impl SetClipStretch {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let stretch = stretch_of(self.stretch.0, "sound.clip.stretch")?;
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.stretch".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.stretch".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.stretch".to_string(), message: error })?;
        payload.stretch = stretch;
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("stretched the clip `{}` to {stretch}", self.clip.0))
            .with_data(serde_json::json!({ "id": self.clip.0, "stretch": stretch })))
    }
}

/// Fade a clip.
///
/// A fade is a **curve from silence to full**, and it is at the two
/// ends of a clip: the head fades in and the tail fades out. A fade
/// that is longer than the clip it fades is **refused**: a fade that
/// does not finish is a fade that is not there, and a fade that is
/// quietly shortened is a fade the caller did not ask for.
///
/// The **shape** is the curve: linear, equal_power, s_curve,
/// exponential or logarithmic. The **bend** pushes the curve away from
/// itself, and a bend of zero is the shape exactly.
///
/// A fade does not change the clip's length and does not change its
/// place: it changes how the clip is heard.
#[derive(Command)]
#[command(
    id = "sound.clip.fade",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Fade"),
    mutates,
    undoable
)]
pub struct SetClipFade {
    /// The clip to fade.
    pub clip: NodeRef,
    /// How long the fade in is, in samples.
    pub fade_in: Option<Number>,
    /// How long the fade out is, in samples.
    pub fade_out: Option<Number>,
    /// The shape of the curve: linear, equal_power, s_curve, exponential or logarithmic.
    pub shape: Option<Text>,
    /// How much the curve is bent, from -1 to 1.
    pub curvature: Option<Number>,
}

impl SetClipFade {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.fade".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.fade".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.fade".to_string(), message: error })?;
        if self.fade_in.is_none() && self.fade_out.is_none() {
            return Err(EngineError::Failed {
                id: "sound.clip.fade".to_string(),
                message: "a fade needs a length: how long is the fade in, and how long is the fade out?"
                    .to_string(),
            });
        }
        // The **shape** and the **bend** are the curve, and they are
        // read once: they apply to the fades that are being set.
        let shape = match &self.shape {
            Some(shape) => {
                craftkit_audio::FadeShape::from_label(&shape.0).ok_or_else(|| EngineError::Failed {
                    id: "sound.clip.fade".to_string(),
                    message: format!("there is no fade shape called `{}`", shape.0),
                })?
            }
            None => craftkit_audio::FadeShape::default(),
        };
        let curvature = match self.curvature {
            Some(curvature) => {
                if !curvature.0.is_finite() {
                    return Err(EngineError::Failed {
                        id: "sound.clip.fade".to_string(),
                        message: "a bend is a number".to_string(),
                    });
                }
                curvature.0.clamp(-1.0, 1.0) as f32
            }
            None => 0.0,
        };
        if let Some(fade_in) = self.fade_in {
            let len = samples(fade_in.0, "how long the fade in is")?;
            if len > payload.length {
                return Err(EngineError::Failed {
                    id: "sound.clip.fade".to_string(),
                    message: format!("the fade in is {len} samples long and the clip is {}", payload.length),
                });
            }
            payload.fade_in = FadePayload { len, shape, curvature };
        }
        if let Some(fade_out) = self.fade_out {
            let len = samples(fade_out.0, "how long the fade out is")?;
            if len > payload.length {
                return Err(EngineError::Failed {
                    id: "sound.clip.fade".to_string(),
                    message: format!("the fade out is {len} samples long and the clip is {}", payload.length),
                });
            }
            payload.fade_out = FadePayload { len, shape, curvature };
        }
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("faded the clip `{}`", self.clip.0)).with_data(serde_json::json!({
            "id": self.clip.0,
            "fade_in": payload.fade_in.to_json(),
            "fade_out": payload.fade_out.to_json(),
        })))
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
        // starts later in the source. A stretched clip's source is read
        // more slowly, so the content moves by the delta **divided by
        // the stretch**: the same stretch that makes the clip longer
        // makes it slower, and the two are the same fact.
        let delta = from as i64 - payload.start;
        trimmed.offset = payload.offset + ((delta as f64 / payload.stretch).round() as i64);
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

/// Space the **clips** against each other.
///
/// The clips are butted together: each one starts where the last one
/// ended, with a **gap** between them. The tree's editor does this when
/// the pieces of a take are laid out by hand.
#[derive(Command)]
#[command(
    id = "sound.clip.space",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip"),
    mutates,
    undoable
)]
pub struct SpaceClips {
    /// The clips to space: their ids.
    pub clips: serde_json::Value,
    /// The gap between them, in samples.
    pub gap: Number,
    /// `end-to-start` — each clip starts where the last one ended — or
    /// `start-to-start`, which keeps their starts the same distance
    /// apart. The first one if it is left out.
    pub mode: Option<Text>,
}

impl SpaceClips {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.space".to_string();
        let list = self.clips.as_array().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: "the clips to space are a list of their ids".to_string(),
        })?;
        let mut ids: Vec<u64> = Vec::with_capacity(list.len());
        for item in list {
            let Some(number) = item.as_u64() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a clip's id is a number".to_string(),
                });
            };
            ids.push(number);
        }
        if ids.len() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "two or more clips are spaced".to_string(),
            });
        }
        let gap = self.gap.0;
        if !gap.is_finite() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a gap is a number of samples".to_string(),
            });
        }
        let gap = gap.round() as i64;
        if gap < 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a gap is not a negative number of samples".to_string(),
            });
        }
        let start_to_start = match &self.mode {
            Some(mode) => match mode.0.trim().to_ascii_lowercase().as_str() {
                "end-to-start" | "end_to_start" => false,
                "start-to-start" | "start_to_start" => true,
                other => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the mode `{other}` is not one of the two"),
                    });
                }
            },
            None => false,
        };
        // The **clips** and where they are now.
        let mut clips: Vec<(u64, i64, i64)> = Vec::with_capacity(ids.len());
        for clip_id in &ids {
            let node = session.document().node(*clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            clips.push((*clip_id, payload.start, payload.length));
        }
        clips.sort_by_key(|(clip_id, start, _)| (*start, *clip_id));
        // The **new** places: each clip butts against the last one.
        let mut moved: Vec<(u64, i64)> = Vec::with_capacity(clips.len());
        let mut current = clips[0].1;
        for (clip_id, start, length) in &clips {
            let at = if moved.is_empty() { *start } else { current };
            moved.push((*clip_id, at.max(0)));
            current = if start_to_start { at + gap } else { at + length + gap };
        }
        for (clip_id, at) in &moved {
            let node = session.document().node(*clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            let mut payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            payload.start = *at;
            if let Some(node) = session.document_mut()?.node_mut(*clip_id) {
                node.data = payload.to_json();
            }
        }
        let count = moved.len();
        Ok(Outcome::changed(format!("spaced {count} clip(s)")).with_data(serde_json::json!({
            "clips": moved.iter().map(|(clip_id, at)| serde_json::json!({ "id": clip_id, "start": at })).collect::<Vec<_>>(),
            "gap": gap,
        })))
    }
}

/// The **relation** that says a clip is followed by its loop.
///
/// The tree's editor remembers a loop in a field on the clip, and has to
/// find the copies again — to take them away, or to draw them as one
/// thing — by looking at what is next to the clip on the track.
///
/// Here the loop is an **edge**: the copy is named by the clip it
/// follows, and the chain is a walk. Nothing has to be guessed.
const LOOP_RELATION: &str = "loop";

/// The most **loops** one clip may be given.
///
/// A clip that loops a thousand times is a thousand bars on the
/// timeline and a thousand copies of the clip in the document. The
/// bound is here so that a number from outside cannot ask for an
/// absurd thing — and the answer is a **refusal**, and not a shorter
/// loop: a loop that is not the loop that was asked for is a loop that
/// is wrong in a way nobody can hear.
const MAX_LOOPS: usize = 1000;

/// How many times a clip is played when nothing else is said.
///
/// The tree's dialog asks; the tree's default is the clip and three
/// loops.
const DEFAULT_LOOPS: i64 = 4;

/// Loop a clip: the copies of it that follow it.
///
/// The tree's editor pastes a copy of the clip onto the track, and the
/// copies are ordinary clips — they may be trimmed, muted and faded on
/// their own. This tree does the same, and says what the tree can only
/// infer: the copy is **named by the clip it follows**, in a relation,
/// so that the loop can be found again without guessing.
///
/// Three ways to say how long the loop is:
///
/// - **count** — the clip and its loops: four is the clip and three
///   copies;
/// - **length** — how long the whole looped clip is, in samples, and
///   the last copy is cut short to fit;
/// - **fill** — up to the next clip on the track, or else to the end of
///   the timeline.
///
/// They are taken in that order: a fill beats a length, and a length
/// beats a count.
#[derive(Command)]
#[command(
    id = "sound.clip.loop",
    area = "sound",
    layer = "sound",
    icon = "link",
    menu("Clip", "Loop"),
    mutates,
    undoable
)]
pub struct LoopClip {
    /// The clip to loop.
    pub clip: NodeRef,
    /// How many times the clip is played: the clip and its loops.
    pub count: Option<Number>,
    /// How long the whole looped clip is, in samples. The last loop is
    /// cut short to fit.
    pub length: Option<Number>,
    /// Loop up to the next clip on the track, or else to the end of the
    /// timeline.
    pub fill: Option<Bool>,
}

impl LoopClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.loop".to_string();
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        if payload.length <= 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a clip with no length is not a clip".to_string(),
            });
        }
        // The **playlist** the clip is in: the copies go beside it.
        let playlist =
            timeline::parent_of(session.document(), self.clip.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{}` is on no playlist", self.clip.0),
            })?;
        // A clip that is **already followed** by a copy is refused: the
        // chain is a line and not a tree, and a branch would have two
        // answers to the same question.
        if !session.document().relations_from(self.clip.0, Some(LOOP_RELATION)).is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{}` is already looped: unloop it first", self.clip.0),
            });
        }
        // How many copies, and how long the last one is.
        let (reps, last) = self.the_plan(session, node, &payload)?;
        if reps == 0 {
            return Ok(Outcome::new("there is no room for a loop").with_data(serde_json::json!({
                "loops": 0,
                "clips": Vec::<u64>::new(),
            })));
        }
        // The **copies**: each one starts where the last one ended, and
        // the last one is cut short to fit.
        let name = node.name.clone();
        let playlist_id = playlist.id;
        let mut made: Vec<u64> = Vec::with_capacity(reps);
        let mut previous = self.clip.0;
        for step in 1..=reps {
            let mut copy = payload.clone();
            copy.start = payload.start.saturating_add(payload.length.saturating_mul(step as i64));
            if step == reps && last < payload.length {
                copy.length = last;
                // The **fades** are shortened to fit: a fade that is
                // longer than the clip is a fade past the end of it.
                copy.fade_in.len = copy.fade_in.len.min(copy.length);
                copy.fade_out.len = copy.fade_out.len.min(copy.length);
            }
            let made_id = session.document_mut()?.add_node_with_under(
                Some(playlist_id),
                name.clone(),
                "sound.clip",
                None,
                copy.to_json(),
            );
            // The copy plays the same source, and it is said so.
            if let Some(source) = payload.source {
                session.document_mut()?.add_relation(craftkit_sdk::Relation::new(made_id, source, "source"));
            }
            // And the copy **follows** the clip before it: that is the
            // loop, and it is an edge and not a guess.
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(
                previous,
                made_id,
                LOOP_RELATION,
            ));
            previous = made_id;
            made.push(made_id);
        }
        let count = made.len();
        Ok(Outcome::changed(format!("looped the clip {count} time(s)")).with_data(serde_json::json!({
            "loops": count,
            "clips": made,
        })))
    }

    /// How many **copies** the loop has, and how long the last one is.
    ///
    /// The three ways to say it are taken in order: a fill beats a
    /// length, and a length beats a count.
    fn the_plan(
        &self,
        session: &Session,
        node: &Node,
        payload: &ClipPayload,
    ) -> Result<(usize, i64), EngineError> {
        let id = "sound.clip.loop".to_string();
        // **Fill**: up to the next clip on the track, or else to the end
        // of the timeline.
        if self.fill.map(|fill| fill.0) == Some(true) {
            let end = the_next_start(session.document(), node, payload)
                .unwrap_or_else(|| timeline::end_in_samples(session.document()));
            let span = end.saturating_sub(payload.start.saturating_add(payload.length));
            return Self::the_reps(span, payload.length, &id);
        }
        // **Length**: the whole looped clip is this long, and the last
        // copy is cut short to fit.
        if let Some(length) = self.length {
            let total = samples(length.0, "how long the looped clip is")?;
            let span = total.saturating_sub(payload.length);
            return Self::the_reps(span, payload.length, &id);
        }
        // **Count**: the clip and its loops.
        let count = match self.count {
            Some(count) => samples(count.0, "how many times the clip is played")?,
            None => DEFAULT_LOOPS,
        };
        if count < 1 {
            return Err(EngineError::Failed {
                id,
                message: "a clip that is played no times is not a clip".to_string(),
            });
        }
        let reps = count - 1;
        if reps > MAX_LOOPS as i64 {
            return Err(EngineError::Failed {
                id,
                message: format!("a clip loops at most {MAX_LOOPS} times, and this one was asked for {reps}"),
            });
        }
        Ok((reps as usize, payload.length))
    }

    /// How many copies fit in a **span**, and how long the last one is.
    ///
    /// A span that is not a whole number of clips has a last copy that
    /// is cut short — which is the tree's own rule, and the reason a
    /// loop of a whole number of bars ends exactly where it began.
    fn the_reps(span: i64, length: i64, id: &str) -> Result<(usize, i64), EngineError> {
        if span <= 0 || length <= 0 {
            return Ok((0, length));
        }
        let whole = span / length;
        let rest = span % length;
        let reps = whole + i64::from(rest > 0);
        if reps > MAX_LOOPS as i64 {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("a clip loops at most {MAX_LOOPS} times, and this one would need {reps}"),
            });
        }
        let last = if rest > 0 { rest } else { length };
        Ok((reps as usize, last))
    }
}

/// Unloop a clip: take the copies away.
///
/// The tree's editor has to **find** the copies before it can take them
/// away, and it finds them by looking at the track: a clip that starts
/// where this one ends, of the same material. This tree knows them —
/// the copy is named by the clip it follows — so the copies are found
/// by walking the chain.
///
/// Two ways to be rid of a loop:
///
/// - **remove** — the copies are taken away, and the clip is a clip
///   again;
/// - **flatten** — the copies stay, and they are not a loop any more:
///   they are clips in their own right.
#[derive(Command)]
#[command(
    id = "sound.clip.unloop",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Unloop"),
    mutates,
    undoable
)]
pub struct UnloopClip {
    /// The clip whose loop is to be taken away.
    pub clip: NodeRef,
    /// `remove` — the copies are taken away. `flatten` — the copies
    /// stay, and are not a loop any more. The first one if it is left
    /// out.
    pub mode: Option<Text>,
}

impl UnloopClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.unloop".to_string();
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let remove = match &self.mode {
            None => true,
            Some(mode) => match mode.0.trim().to_ascii_lowercase().as_str() {
                "remove" => true,
                "flatten" => false,
                other => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the mode `{other}` is not one of the two"),
                    });
                }
            },
        };
        // The **chain**: the clip, and the copies that follow it.
        let chain = the_chain(session.document(), self.clip.0);
        if chain.len() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{}` is not a looped clip", self.clip.0),
            });
        }
        let copies = chain.len() - 1;
        if remove {
            // The copies go, and their edges go with them: a document
            // that keeps the edge to a clip that is not there is a
            // document that says a clip plays a source that is gone.
            for copy in chain.iter().skip(1) {
                session.document_mut()?.remove_node(*copy);
            }
        } else {
            // The copies stay, and the **edges** go: they are clips in
            // their own right, and nothing says they are a loop.
            let members = chain.clone();
            session.document_mut()?.remove_relations(|relation| {
                relation.kind == LOOP_RELATION
                    && members.contains(&relation.from)
                    && members.contains(&relation.to)
            });
        }
        let message = if remove {
            format!("took away {copies} cop(y|ies) of the loop")
        } else {
            format!("the {copies} cop(y|ies) are not a loop any more")
        };
        Ok(Outcome::changed(message).with_data(serde_json::json!({
            "copies": copies,
            "removed": remove,
            "clips": chain.iter().skip(1).collect::<Vec<_>>(),
        })))
    }
}

/// The **chain** of a loop: the clip and the copies that follow it.
///
/// The chain is a walk: back along the edges to the head, and then
/// forward along the copies. A clip that is in no loop is a chain of
/// one.
fn the_chain(document: &craftkit_sdk::document::Document, clip: u64) -> Vec<u64> {
    // The **head**: the clip that nothing follows into.
    let mut head = clip;
    loop {
        let incoming = document.relations_to(head, Some(LOOP_RELATION));
        let Some(relation) = incoming.first() else { break };
        let from = relation.from;
        if from == head {
            break;
        }
        head = from;
    }
    // The **copies**: the head, and then each one after it.
    let mut chain = vec![head];
    let mut current = head;
    loop {
        let outgoing = document.relations_from(current, Some(LOOP_RELATION));
        let Some(relation) = outgoing.first() else { break };
        let to = relation.to;
        if chain.contains(&to) {
            break;
        }
        chain.push(to);
        current = to;
    }
    chain
}

/// The start of the **next clip** on the clip's track, if there is one.
///
/// The tree's editor looks for the next clip that starts at or after
/// this one ends: a clip that overlaps is not a neighbour, because the
/// loop would run into the middle of it.
fn the_next_start(
    document: &craftkit_sdk::document::Document,
    clip: &Node,
    payload: &ClipPayload,
) -> Option<i64> {
    let track = timeline::track_of(document, clip)?;
    let end = payload.start.saturating_add(payload.length);
    let mut next: Option<i64> = None;
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        if node.id == clip.id {
            continue;
        }
        if timeline::track_of(document, node).map(|owner| owner.id) != Some(track.id) {
            continue;
        }
        let Ok(other) = ClipPayload::from_json(&node.data) else { continue };
        if other.start < end {
            continue;
        }
        next = Some(next.map_or(other.start, |best: i64| best.min(other.start)));
    }
    next
}

/// Snap a clip to its **neighbour**.
///
/// The tree's editor has a menu — *Edit ▸ Snap to* — with two items in
/// it: **Next** moves a clip so that its end meets the start of the
/// clip that follows it, and **Previous** moves a clip so that its
/// start meets the end of the clip that comes before it.
///
/// The tree's editor snaps the clip **nearest the destination first**,
/// so that the neighbours stay where they are: a row of clips that are
/// snapped together does not pile itself up on one clip.
///
/// The clips that are named are the destinations too. The tree's
/// editor found that out the hard way — ignoring the selected
/// neighbours made *Snap to Next* pile every selected clip onto the
/// same later clip, and made *Snap to Previous* leave the gap between
/// the selected clips in place.
#[derive(Command)]
#[command(
    id = "sound.clip.snap",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Snap to"),
    mutates,
    undoable
)]
pub struct SnapClip {
    /// The clips to snap: their ids.
    pub clips: serde_json::Value,
    /// `next` — each clip's end meets the start of the clip that
    /// follows it. `previous` — each clip's start meets the end of the
    /// clip that comes before it.
    pub direction: Text,
}

impl SnapClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.snap".to_string();
        let list = self.clips.as_array().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: "the clips to snap are a list of their ids".to_string(),
        })?;
        let mut ids: Vec<u64> = Vec::with_capacity(list.len());
        for item in list {
            let Some(number) = item.as_u64() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a clip's id is a number".to_string(),
                });
            };
            ids.push(number);
        }
        if ids.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "there is no clip to snap".to_string(),
            });
        }
        let next = match self.direction.0.trim().to_ascii_lowercase().as_str() {
            "next" => true,
            "previous" => false,
            other => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the direction `{other}` is not one of the two"),
                });
            }
        };
        // The **clips** and where they are now.
        let mut order: Vec<(u64, i64, i64, u64)> = Vec::with_capacity(ids.len());
        for clip_id in &ids {
            let node = session.document().node(*clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            // The **list** the clip is in: the playlist under the
            // track, or the track itself. A clip's neighbours are the
            // clips of its own list — a take is a list of clips of its
            // own, and a clip in another take is not on this timeline.
            let list =
                timeline::parent_of(session.document(), *clip_id).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` is on no playlist"),
                })?;
            order.push((*clip_id, payload.start, payload.length, list.id));
        }
        // The clip **nearest the destination** is snapped first, so that
        // the neighbours stay where they are.
        order.sort_by_key(|(_, start, _, _)| if next { -*start } else { *start });
        let mut moved: Vec<(u64, i64)> = Vec::with_capacity(order.len());
        for (clip_id, start, length, list) in order {
            let end = start.saturating_add(length);
            // The **neighbour**: the clip the destination is. The clips
            // that are named are neighbours too.
            let delta = if next {
                clips_in(session.document(), list)
                    .into_iter()
                    .filter(|(other, _)| *other != clip_id)
                    .map(|(_, payload)| payload.start)
                    .filter(|other_start| *other_start >= end)
                    .min()
                    .map(|other_start| other_start - end)
            } else {
                clips_in(session.document(), list)
                    .into_iter()
                    .filter(|(other, _)| *other != clip_id)
                    .map(|(_, payload)| (payload.start, payload.start.saturating_add(payload.length)))
                    .filter(|(_, other_end)| *other_end <= start)
                    .map(|(_, other_end)| other_end)
                    .max()
                    .map(|other_end| other_end - start)
            };
            // A clip that is already against its neighbour is not
            // moved: there is nothing to snap.
            let Some(delta) = delta.filter(|delta| *delta != 0) else { continue };
            let at = start.saturating_add(delta).max(0);
            if let Some(node) = session.document_mut()?.node_mut(clip_id) {
                let mut payload = ClipPayload::from_json(&node.data)
                    .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
                payload.start = at;
                node.data = payload.to_json();
            }
            moved.push((clip_id, at));
        }
        let count = moved.len();
        Ok(Outcome::changed(format!("snapped {count} clip(s)")).with_data(serde_json::json!({
            "moved": count,
            "clips": moved.iter().map(|(clip_id, at)| serde_json::json!({ "id": clip_id, "start": at })).collect::<Vec<_>>(),
        })))
    }
}

/// The **clips** in a list, with what they say.
///
/// A list is a playlist under a track, or a track that holds its clips
/// itself. A clip's neighbours are the clips of the list it is in.
fn clips_in(document: &craftkit_sdk::document::Document, list: u64) -> Vec<(u64, ClipPayload)> {
    let mut clips: Vec<(u64, ClipPayload)> = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        if timeline::parent_of(document, node.id).map(|parent| parent.id) != Some(list) {
            continue;
        }
        let Ok(payload) = ClipPayload::from_json(&node.data) else { continue };
        clips.push((node.id, payload));
    }
    clips
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
        let payload = SourcePayload {
            path: path.clone(),
            channels,
            frames,
            sample_rate: session_settings.sample_rate,
            peaks: Vec::new(),
        };
        let id = session.document_mut()?.add_node_with(path.clone(), "sound.source", None, payload.to_json());
        // The document now says that it needs this file. The bytes are
        // not in the document: they travel beside it, in a bundle.
        session.document_mut()?.add_asset(craftkit_sdk::Asset::new(path.clone(), "audio", None));
        Ok(Outcome::changed(format!("added a source `{path}`"))
            .with_data(serde_json::json!({ "id": id, "path": path })))
    }
}

/// **Measure** the peaks of a source.
///
/// The peaks are the shape of the sound in a small table, and the shape
/// is what a clip's bar is drawn from.
///
/// Measuring them reads the file, so it is a **command** and not a
/// thing that happens at every frame: a file that is read a hundred
/// times a second is a file that is read a hundred times too often.
/// The measurement is in the document, so it is undone with everything
/// else and it is saved with the document.
#[derive(Command)]
#[command(id = "sound.source.peaks", area = "sound", layer = "sound", icon = "list", mutates, undoable)]
pub struct MeasurePeaks {
    /// The source to measure.
    pub source: NodeRef,
}

impl MeasurePeaks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.source.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.source.peaks".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "sound.source" {
            return Err(EngineError::Failed {
                id: "sound.source.peaks".to_string(),
                message: format!("the node `{id}` is a `{}` and not a source", node.kind),
            });
        }
        let payload = SourcePayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.source.peaks".to_string(),
            message: format!("the source `{id}` cannot be read: {error}"),
        })?;
        let sound = read_sound("sound.source.peaks", &payload.path)?;
        let peaks = peaks_of(&sound);
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.source.peaks".to_string(),
                message: format!("there is no source `{id}`"),
            });
        };
        // The file says what it is: how long it is, how many channels
        // it has and the rate it was recorded at. A source that has
        // not been measured does not know.
        node.data["frames"] = serde_json::json!(sound.frames as i64);
        node.data["channels"] = serde_json::json!(sound.channels as u32);
        node.data["sample_rate"] = serde_json::json!(sound.rate);
        node.data["peaks"] = serde_json::json!(peaks);
        Ok(Outcome::changed(format!("measured the peaks of the source {id}")).with_data(serde_json::json!({
            "id": id,
            "numbers": PEAK_NUMBERS,
            "frames": sound.frames as i64,
            "channels": sound.channels as u32,
            "sample_rate": sound.rate,
        })))
    }
}

/// The kind of a **playhead**: where the sound is.
///
/// The playhead is a **node** because the playhead has a **setting** —
/// the position — and a setting is a thing in a document, and a node is
/// where settings live: the same rule as the ruler's scale (§131).
///
/// The position is a **number of samples** and not an event: the
/// transport is not a stream of "and then" messages, it is a place,
/// and everything else is worked out from it.
#[derive(NodeKind)]
#[node_kind(kind = "sound.playhead", label = "Playhead", layer = "sound", evaluates, place)]
pub struct PlayheadKind;

impl PlayheadKind {
    /// How a playhead is drawn: a **line** down the timeline.
    ///
    /// The line runs from the ruler's row through the tracks, because a
    /// playhead is where the sound is and the sound is under the ruler.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::playhead_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x + rect.w / 2.0, rect.y),
            Point::new(rect.x + rect.w / 2.0, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Colour::NONE,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is on the playhead.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::playhead_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where the playhead **is**: the line at the position.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::playhead_rect(document, node)
    }

    /// What a playhead **means**: where it is, in samples and in
    /// seconds.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let position = timeline::playhead_position(node)?;
        let rate = session_of(document).sample_rate;
        Some(serde_json::json!({
            "position": position,
            "seconds": position as f64 / rate as f64,
        }))
    }
}

/// Put the **playhead** where it is, or move it.
///
/// The playhead is a **node**, so there is one of it and it is moved.
#[derive(Command)]
#[command(
    id = "sound.playhead.set",
    area = "sound",
    layer = "sound",
    icon = "play",
    menu("Playhead"),
    mutates,
    undoable
)]
pub struct SetPlayhead {
    /// Where it is, in samples.
    pub position: Number,
}

impl SetPlayhead {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let position = samples(self.position.0, "where the playhead is")?;
        if position < 0 {
            return Err(EngineError::Failed {
                id: "sound.playhead.set".to_string(),
                message: "a playhead cannot be before the beginning".to_string(),
            });
        }
        let existing =
            session.document().walk().find(|node| node.kind == "sound.playhead").map(|node| node.id);
        let id = match existing {
            Some(id) => {
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = serde_json::json!({ "position": position });
                }
                id
            }
            None => session.document_mut()?.add_node_with(
                "playhead".to_string(),
                "sound.playhead",
                None,
                serde_json::json!({ "position": position }),
            ),
        };
        Ok(Outcome::changed(format!("the playhead is at {position} samples"))
            .with_data(serde_json::json!({ "id": id, "position": position })))
    }
}

/// **Mute** a clip, or unmute it.
///
/// A muted clip is not played and is still there: the tree's renderer
/// skips it and the tree's editor keeps it, so that a clip that is
/// quiet is a clip that may be loud again.
#[derive(Command)]
#[command(
    id = "sound.clip.mute",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Track", "Mute"),
    mutates,
    undoable
)]
pub struct MuteClip {
    /// The clip to mute.
    pub clip: NodeRef,
    /// Whether it is muted. Without it, the clip is muted.
    pub muted: Option<Bool>,
}

impl MuteClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.clip.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.clip.mute".to_string(),
                message: format!("there is no clip `{id}`"),
            });
        };
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.mute".to_string(),
                message: format!("the node `{id}` is a `{}` and not a clip", node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.clip.mute".to_string(),
            message: error.to_string(),
        })?;
        let muted = self.muted.map(|muted| muted.0).unwrap_or(true);
        payload.muted = muted;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.clip.mute".to_string(),
                message: format!("there is no clip `{id}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(if muted {
            format!("the clip `{id}` is muted")
        } else {
            format!("the clip `{id}` is not muted")
        })
        .with_data(serde_json::json!({ "id": id, "muted": muted })))
    }
}

/// Render the document into a **sound file**.
///
/// The renderer is the first thing in this tree that makes a sound: it
/// reads the sources the clips play, mixes them and writes the file.
/// It is **offline** — it writes a file and it stops — because the kit
/// has no audio device and no audio thread (§ the kit's own notes), and
/// a file is a thing a program can make without a device.
///
/// What it does:
/// - the clips are the **tracks**: each clip plays a source from an
///   offset, for a length, at a gain;
/// - the sources are read from the **files** the document names, and
///   the samples are the ones the WAV reader gives back (§133);
/// - the tracks are mixed (§134) and the mix is written as a WAV.
#[derive(Command)]
#[command(id = "sound.render", area = "sound", layer = "sound", icon = "play", menu("Render"))]
pub struct RenderSound {
    /// Where to write the sound.
    pub path: Text,
}

impl RenderSound {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let settings = session_of(session.document());
        let rate = settings.sample_rate;
        // The clips: each one plays a source from an offset, for a
        // length, at a gain.
        let mut clips: Vec<(u64, u64, ClipPayload)> = Vec::new();
        for clip in session.document().walk().filter(|node| node.kind == "sound.clip") {
            // A clip that cannot be read is **refused**, and not left
            // out: a sound that is missing a clip without saying so is
            // a sound that is wrong in a way nobody can hear.
            let payload = ClipPayload::from_json(&clip.data).map_err(|error| EngineError::Failed {
                id: "sound.render".to_string(),
                message: format!("the clip `{}` cannot be read: {error}", clip.id),
            })?;
            let Some(source_id) = payload.source else { continue };
            // A **muted** clip is not played and is still there.
            if payload.muted {
                continue;
            }
            // The **take**: a clip that is not in the take that is
            // played is not heard, and the tree's tracks have takes for
            // exactly that reason.
            if let Some(track) = timeline::track_of(session.document(), clip) {
                let active = active_playlist(session.document(), track);
                let take = timeline::parent_of(session.document(), clip.id).map(|playlist| playlist.id);
                if let (Some(active), Some(take)) = (active, take)
                    && active != take
                {
                    continue;
                }
            }
            clips.push((source_id, clip.id, payload));
        }
        // The sources are read once each, and the clips that play one
        // share it.
        let mut sources: Vec<(u64, craftkit_wav::Sound)> = Vec::new();
        let mut channels = 0_u16;
        for (source_id, _, _) in &clips {
            if sources.iter().any(|(id, _)| id == source_id) {
                continue;
            }
            let Some(node) = session.document().node(*source_id) else { continue };
            let Ok(source) = SourcePayload::from_json(&node.data) else { continue };
            let sound = read_sound("sound.render", &source.path)?;
            if channels == 0 {
                channels = sound.channels;
            } else if channels != sound.channels {
                return Err(EngineError::Failed {
                    id: "sound.render".to_string(),
                    message: "the sources do not have the same number of channels".to_string(),
                });
            }
            sources.push((*source_id, sound));
        }
        // Each clip's samples are **prepared**: a source at another rate
        // is resampled, a **stretched** clip is played at another
        // speed, and the track's **automation** is applied to them,
        // sample by sample.
        let mut prepared: Vec<Vec<f32>> = Vec::with_capacity(clips.len());
        for (source_id, clip_id, payload) in &clips {
            let Some((_, sound)) = sources.iter().find(|(id, _)| id == source_id) else {
                prepared.push(Vec::new());
                continue;
            };
            let (from, to) = the_span(sound, payload, rate);
            let slice = &sound.samples[from..to];
            let samples = if sound.rate == rate {
                slice.to_vec()
            } else {
                // A source that cannot be played at the session's rate
                // is **refused**, and not truncated: a sound that is
                // missing its tail is a sound that is wrong in a way
                // nobody can hear.
                craftkit_audio::resample(
                    slice,
                    sound.channels as usize,
                    sound.rate,
                    rate,
                    &craftkit_audio::Limits::default(),
                )
                .map_err(|error| EngineError::Failed {
                    id: "sound.render".to_string(),
                    message: format!("the source cannot be played at {rate} Hz: {error}"),
                })?
            };
            // The **stretch**: the source is read more slowly, so the
            // sound is longer and the pitch is lower. The clip's own
            // length does not change: the stretch says how the source
            // is played.
            let samples = if (payload.stretch - 1.0).abs() > f64::EPSILON {
                craftkit_audio::varispeed(&samples, sound.channels as usize, payload.stretch)
            } else {
                samples
            };
            // The **fades**: the head of the clip fades in and the tail
            // fades out, and a fade is a gain and not a thing that
            // belongs to a channel.
            let samples = craftkit_audio::faded(
                &samples,
                sound.channels as usize,
                payload.fade_in.fade(),
                payload.fade_out.fade(),
            );
            // The **track's** automation is **not** applied here: the
            // fader comes after the track's inserts, the way the
            // tree's mixer runs it, and a send may be taken before the
            // fader. The lanes under the track move the track's sound
            // and not one clip's.
            //
            // The **clip's own** automation: the lanes under the clip,
            // and their times are the clip's own — a clip carries a
            // curve of its own, and the family's does.
            let own = session
                .document()
                .node(*clip_id)
                .map(|clip| automate::automation_of(session.document(), clip))
                .unwrap_or_default();
            let mut samples = automate::apply(&own, &samples, sound.channels as usize, 0);
            // The clip's **gain**: the level of the clip itself. The
            // tree's mixer multiplies it into the clip before the
            // track's inserts, and so does this one.
            let clip_gain = craftkit_audio::db_to_linear(payload.gain_db) as f32;
            if clip_gain != 1.0 {
                samples.iter_mut().for_each(|sample| *sample *= clip_gain);
            }
            // The clip's **own effects**: the tree's clip effects are an
            // equaliser and a compressor on one clip and not on the
            // track, and they run over the clip's own sound before it
            // is mixed into the track.
            if let Some(clip) = session.document().node(*clip_id) {
                for insert in inserts_of(session.document(), clip) {
                    if insert.bypass {
                        continue;
                    }
                    let mut plugin = craftkit_plugins::make(
                        &insert.plugin,
                        &craftkit_plugins::Params::from_map(insert.params),
                    )
                    .map_err(|error| EngineError::Failed {
                        id: "sound.render".to_string(),
                        message: error.to_string(),
                    })?;
                    plugin.process(&mut samples, sound.channels as usize, rate);
                }
            }
            prepared.push(samples);
        }
        // How long the sound is: the end of the longest clip.
        let mut frames = 0_i64;
        for (index, (_, _, payload)) in clips.iter().enumerate() {
            let end = payload.start.max(0) + prepared[index].len() as i64 / channels.max(1) as i64;
            if end > frames {
                frames = end;
            }
        }
        let frames = frames.max(0) as usize;
        // The **tracks**: each track's signal is the sum of its clips,
        // its **inserts** are run over that sum in the order of the
        // tree, and its **fader** comes after them — the fader is the
        // track's automation, and the tree's mixer runs the fader
        // last.
        //
        // The signal before the fader is kept for the tracks whose
        // sends ask for it: a **pre-fader** send is a copy of the
        // signal before the fader, and the copy is the whole point of
        // it.
        let mut pre_fader: Vec<(u64, Vec<f32>)> = Vec::new();
        let mut tracks: Vec<(u64, Vec<f32>, automate::Automation)> = Vec::new();
        // The **routed tracks**: the tracks that feed a bus, and the
        // signal they feed it with.
        let mut routed: Vec<(u64, Vec<f32>)> = Vec::new();
        // The **readers**: the tracks that read a bus. A reader is
        // finished after the buses are, because its sound is the bus's
        // sound.
        let readers = mixer::readers_of(session.document());
        for track in session.document().walk().filter(|node| {
            node.kind == "sound.track" && !is_master(node) && !mixer::reads_bus(session.document(), node.id)
        }) {
            let mut signal = vec![0.0_f32; frames];
            for (index, (_, clip_id, payload)) in clips.iter().enumerate() {
                let Some(clip) = session.document().node(*clip_id) else { continue };
                if timeline::track_of(session.document(), clip).map(|owner| owner.id) != Some(track.id) {
                    continue;
                }
                // The clip's own gain is already in its samples: the
                // tree's mixer multiplies it into the clip.
                add_into(&mut signal, &prepared[index], payload.start.max(0) as usize, 1.0);
            }
            // The **trim**: the level of the track's input, and its
            // phase. The tree's mixer applies the trim to the strip
            // before the inserts, so an insert sees the trimmed sound.
            let trim = TrackPayload::from_json(&track.data)
                .map(|payload| {
                    let gain = craftkit_audio::db_to_linear(payload.trim_db) as f32;
                    let phase = if payload.phase_invert { -1.0 } else { 1.0 };
                    gain * phase
                })
                .unwrap_or(1.0);
            if trim != 1.0 {
                signal.iter_mut().for_each(|sample| *sample *= trim);
            }
            for insert in inserts_of(session.document(), track) {
                if insert.bypass {
                    continue;
                }
                let mut plugin = craftkit_plugins::make(
                    &insert.plugin,
                    &craftkit_plugins::Params::from_map(insert.params),
                )
                .map_err(|error| EngineError::Failed {
                    id: "sound.render".to_string(),
                    message: error.to_string(),
                })?;
                plugin.process(&mut signal, channels as usize, rate);
            }
            // The **mute** gates the audio and the track's pre-fader
            // tap: a muted track is not heard anywhere.
            let automation = automate::automation_of(session.document(), track);
            let gated = automate::apply_mute(&automation, &signal, channels as usize, 0);
            // The **fader**: the track's volume, over the whole signal
            // and not over one clip.
            let faded = automate::apply_fader(&automation, &gated, channels as usize, 0);
            if sends_of(session.document()).iter().any(|send| send.track == track.id && send.pre_fader) {
                pre_fader.push((track.id, gated));
            }
            // The **route**: a track that feeds a bus is heard on that
            // bus and not in the main mix. The tree's strip goes to
            // its output and to nowhere else.
            match mixer::route_of(session.document(), track.id) {
                Some(bus) => routed.push((bus, faded)),
                None => tracks.push((track.id, faded, automation)),
            }
        }
        // The **solo**: a soloed track silences the others, and their
        // sends carry the silence, because a send is a copy of the
        // track's signal.
        if tracks.iter().any(|(id, _, _)| track_is_soloed(session.document(), *id)) {
            for (id, signal, _) in &mut tracks {
                if !track_is_soloed(session.document(), *id) {
                    signal.iter_mut().for_each(|sample| *sample = 0.0);
                }
            }
            // A soloed-out track's **pre-fader** copy is silent too: the
            // tree's mixer skips the send of a track that is muted, and
            // the copy is not heard.
            for (id, signal) in &mut pre_fader {
                if !track_is_soloed(session.document(), *id) {
                    signal.iter_mut().for_each(|sample| *sample = 0.0);
                }
            }
        }
        // The **buses**: each bus is fed by the sends that name it, and
        // a send carries a copy of its track's signal at its own level.
        let mut buses: Vec<(u64, Vec<f32>)> = Vec::new();
        for bus in session.document().walk().filter(|node| node.kind == "sound.bus") {
            let mut signal = vec![0.0_f32; frames];
            for send in sends_of(session.document()) {
                if send.bus != bus.id {
                    continue;
                }
                // A **pre-fader** send takes the signal from before the
                // fader, and a post-fader send takes it from after.
                let source: Option<&[f32]> = if send.pre_fader {
                    pre_fader.iter().find(|(id, _)| *id == send.track).map(|(_, signal)| signal.as_slice())
                } else {
                    tracks.iter().find(|(id, _, _)| *id == send.track).map(|(_, signal, _)| signal.as_slice())
                };
                let Some(track_signal) = source else {
                    continue;
                };
                if send.mute {
                    // A muted send is a copy of **nothing**: the track
                    // is still heard on its own and the bus hears
                    // nothing.
                    continue;
                }
                let level = craftkit_audio::db_to_linear(send.level_db) as f32;
                let panned = panned(track_signal, send.pan, channels as usize);
                add_into(&mut signal, &panned, 0, level);
            }
            buses.push((bus.id, signal));
        }
        // The **routed tracks**: a track that feeds a bus is heard on
        // that bus.
        for (bus, signal) in &routed {
            if let Some((_, bus_signal)) = buses.iter_mut().find(|(id, _)| id == bus) {
                add_into(bus_signal, signal, 0, 1.0);
            }
        }
        // The **mix order**: the buses and the readers, in the order
        // they are mixed. A bus is mixed before the readers that read
        // it, and a reader is mixed before the bus it feeds — so a
        // reader that feeds a bus is heard there and not by itself.
        let stages = mixer::stage_order(session.document())
            .map_err(|message| EngineError::Failed { id: "sound.render".to_string(), message })?;
        for stage in stages {
            match stage {
                mixer::Stage::Bus(id) => {
                    // A bus that feeds another bus is heard there and
                    // not by itself.
                    let Some(destination) = mixer::route_of(session.document(), id) else { continue };
                    let Some(index) = buses.iter().position(|(bus, _)| *bus == id) else { continue };
                    let signal = buses[index].1.clone();
                    if let Some((_, target)) = buses.iter_mut().find(|(bus, _)| *bus == destination) {
                        add_into(target, &signal, 0, 1.0);
                    }
                }
                mixer::Stage::Reader(reader, bus) => {
                    // A track that reads a bus takes the bus's sound,
                    // with its own inserts and its own fader. A bus
                    // that is read is not heard by itself: the tree's
                    // bus is read by an aux and is silent on its own.
                    let Some((_, bus_signal)) = buses.iter().find(|(id, _)| *id == bus) else { continue };
                    let Some(track) = session.document().node(reader) else { continue };
                    let mut signal = bus_signal.clone();
                    for insert in inserts_of(session.document(), track) {
                        if insert.bypass {
                            continue;
                        }
                        let mut plugin = craftkit_plugins::make(
                            &insert.plugin,
                            &craftkit_plugins::Params::from_map(insert.params),
                        )
                        .map_err(|error| EngineError::Failed {
                            id: "sound.render".to_string(),
                            message: error.to_string(),
                        })?;
                        plugin.process(&mut signal, channels.max(1) as usize, rate);
                    }
                    let automation = automate::automation_of(session.document(), track);
                    let gated = automate::apply_mute(&automation, &signal, channels.max(1) as usize, 0);
                    let faded = automate::apply_fader(&automation, &gated, channels.max(1) as usize, 0);
                    // A reader that **feeds a bus** is heard there and
                    // not in the main mix.
                    match mixer::route_of(session.document(), reader) {
                        Some(destination) => {
                            if let Some((_, target)) = buses.iter_mut().find(|(bus, _)| *bus == destination) {
                                add_into(target, &faded, 0, 1.0);
                            }
                        }
                        None => tracks.push((reader, faded, automation)),
                    }
                }
            }
        }
        // The **master** is the file: every track and every bus.
        let mut master = vec![0.0_f32; frames];
        for (_, signal, automation) in &tracks {
            // The **balance**: the track's own pan is the track's, and
            // it is applied when the track is routed. A copy that a
            // send takes is balanced by the **send's** own pan.
            let own = automate::apply_pan(automation, signal, channels.max(1) as usize, 0);
            add_into(&mut master, &own, 0, 1.0);
        }
        for (bus, signal) in &buses {
            // A bus that is **read** is heard through the reader, and a
            // bus that **feeds another bus** is heard there: neither is
            // heard by itself.
            if readers.iter().any(|(_, read)| read == bus) {
                continue;
            }
            if mixer::route_of(session.document(), *bus).is_some() {
                continue;
            }
            add_into(&mut master, signal, 0, 1.0);
        }
        // The **master faders**: a track of the kind `master` holds a
        // fader for the whole mix and not for a part of it. The tree's
        // mixer runs the master faders over the main mix, in the order
        // of the tree, and the metronome is summed after them.
        for master_track in
            session.document().walk().filter(|node| node.kind == "sound.track" && is_master(node))
        {
            for insert in inserts_of(session.document(), master_track) {
                if insert.bypass {
                    continue;
                }
                let mut plugin = craftkit_plugins::make(
                    &insert.plugin,
                    &craftkit_plugins::Params::from_map(insert.params),
                )
                .map_err(|error| EngineError::Failed {
                    id: "sound.render".to_string(),
                    message: error.to_string(),
                })?;
                plugin.process(&mut master, channels.max(1) as usize, rate);
            }
            // The **fader** of the master: its own lanes, over the whole
            // mix.
            let automation = automate::automation_of(session.document(), master_track);
            master = automate::apply_mute(&automation, &master, channels.max(1) as usize, 0);
            master = automate::apply_fader(&automation, &master, channels.max(1) as usize, 0);
        }
        let channels = channels.max(1);
        // The **metronome**: the click is mixed in after the master, so
        // it reaches the output without passing through a track, a send
        // or an insert. The beats come from the tempo map, and the
        // first beat of a bar is the accent.
        if settings.metronome {
            click_into(&mut master, channels as usize, rate, &tempo_map(session.document()), &settings);
        }
        let mix = craftkit_audio::mix(
            &[craftkit_audio::Track { start: 0, samples: master, gain: 1.0 }],
            rate,
            &craftkit_audio::Limits::default(),
        )
        .map_err(|error| EngineError::Failed {
            id: "sound.render".to_string(),
            message: error.to_string(),
        })?;
        let bytes = craftkit_wav::write(channels, rate, &mix.samples);
        write_file(&path, &bytes)?;
        Ok(Outcome::new(format!("rendered {} frames to `{path}`", mix.frames)).with_data(serde_json::json!({
            "path": path,
            "frames": mix.frames,
            "rate": rate,
            "channels": channels,
            "clips": tracks.len(),
        })))
    }
}

/// Whether a track is **soloed**.
fn track_is_soloed(document: &Document, track: u64) -> bool {
    let Some(node) = document.node(track) else { return false };
    TrackPayload::from_json(&node.data).map(|payload| payload.solo).unwrap_or(false)
}

/// The **mix** of a sound document, as a **lane job**.
///
/// This is the real-time work of a sound application: the tracks are
/// prepared on the **control thread** — the sources are read, the
/// clips are prepared and the tracks' signals are built — and the lane
/// does the rest: the block's samples are the sum of the tracks, and
/// nothing is allocated while it happens.
///
/// The tree has **no device**, so nothing plays the job: a caller runs
/// the blocks itself, and the rule that the work does not allocate is
/// **measured** on it rather than asserted in a comment.
pub struct MixJob {
    /// The tracks, prepared: their signals and where they start.
    tracks: Vec<craftkit_audio::Track>,
    /// The block's samples.
    block: Vec<f32>,
    /// The frames the job has mixed.
    played: i64,
}

impl MixJob {
    /// A job that mixes these tracks.
    pub fn new(tracks: Vec<craftkit_audio::Track>) -> MixJob {
        MixJob { tracks, block: Vec::new(), played: 0 }
    }

    /// The block's samples, after the lane has run it.
    pub fn samples(&self) -> &[f32] {
        &self.block
    }

    /// How many frames the job has mixed.
    pub fn played(&self) -> i64 {
        self.played
    }
}

impl craftkit_sdk::LaneJob for MixJob {
    /// Prepare: the **control thread**. The block's buffer is made
    /// here, because the lane may not allocate.
    fn prepare(&mut self, block: usize) -> Result<(), String> {
        self.block = vec![0.0_f32; block];
        Ok(())
    }

    /// Run: the **lane**. The tracks are summed into the block, and
    /// nothing is allocated.
    fn run(&mut self, frames: usize) -> craftkit_sdk::Block {
        let frames = frames.min(self.block.len());
        for sample in self.block.iter_mut().take(frames) {
            *sample = 0.0;
        }
        let block_start = self.played.max(0) as usize;
        let block_end = block_start + frames;
        for track in &self.tracks {
            let start = track.start.max(0) as usize;
            let end = start + track.samples.len();
            // The part of the track that is in this block.
            let from = start.max(block_start);
            let to = end.min(block_end);
            if from >= to {
                continue;
            }
            let gain = track.gain as f32;
            for (index, sample) in track.samples[from - start..to - start].iter().enumerate() {
                if let Some(slot) = self.block.get_mut(from - block_start + index) {
                    *slot += *sample * gain;
                }
            }
        }
        self.played += frames as i64;
        craftkit_sdk::Block::Played
    }
}

/// An **insert**: the plugin a track runs over its signal.
struct Insert {
    /// The plugin's name.
    plugin: String,
    /// Its parameters, by name.
    params: HashMap<String, f64>,
    /// Whether it is bypassed.
    bypass: bool,
}

/// The inserts of a track, in the order of the tree.
///
/// The order of the tree is the order of the drawing (§ the order), and
/// it is the order of the **processing** too: the first insert is the
/// first program the signal meets.
fn inserts_of(document: &Document, track: &Node) -> Vec<Insert> {
    let mut inserts = Vec::new();
    for child in &track.children {
        let Some(node) = document.node(child.id) else { continue };
        if node.kind != "sound.insert" {
            continue;
        }
        let Ok(payload) = InsertPayload::from_json(&node.data) else { continue };
        let mut params: HashMap<String, f64> = HashMap::new();
        if let Some(values) = payload.params.as_object() {
            for (name, value) in values {
                if let Some(number) = value.as_f64() {
                    params.insert(name.clone(), number);
                }
            }
        }
        inserts.push(Insert { plugin: payload.plugin, params, bypass: payload.bypass });
    }
    inserts
}

/// A **send**: the level and the pan with which a track feeds a bus.
struct Send {
    /// The track that feeds.
    track: u64,
    /// The bus that is fed.
    bus: u64,
    /// Its level, in decibels.
    level_db: f64,
    /// Its pan, from minus one to one.
    pan: f64,
    /// Whether it is silent.
    mute: bool,
    /// Whether it is taken before the track's fader.
    pre_fader: bool,
}

/// The sends of the document: the tracks that feed the buses.
fn sends_of(document: &Document) -> Vec<Send> {
    let mut sends = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.send") {
        let Ok(payload) = SendPayload::from_json(&node.data) else { continue };
        let Some(track) = timeline::parent_of(document, node.id) else { continue };
        let Some(relation) = document.relations_from(node.id, Some("send")).into_iter().next() else {
            continue;
        };
        sends.push(Send {
            track: track.id,
            bus: relation.to,
            level_db: payload.level_db,
            pan: payload.pan,
            mute: payload.mute,
            pre_fader: payload.pre_fader,
        });
    }
    sends
}

/// Add a signal into another, at a place, at a gain.
fn add_into(target: &mut [f32], source: &[f32], at: usize, gain: f32) {
    for (index, sample) in source.iter().enumerate() {
        let position = at + index;
        if position >= target.len() {
            break;
        }
        target[position] += sample * gain;
    }
}

/// The signal of a track, **panned** between its channels.
///
/// The pan is a balance: minus one is all to the left and one is all
/// to the right, and the middle is the middle. A **mono** signal is
/// not balanced and is left alone.
fn panned(signal: &[f32], pan: f64, channels: usize) -> Vec<f32> {
    let pan = pan.clamp(-1.0, 1.0);
    if channels < 2 || pan == 0.0 {
        return signal.to_vec();
    }
    let angle = (pan + 1.0) * 0.25 * std::f64::consts::PI;
    let left = angle.cos() as f32;
    let right = angle.sin() as f32;
    let mut out = signal.to_vec();
    for index in 0..signal.len() / channels {
        for channel in 0..channels {
            let at = index * channels + channel;
            out[at] *= if channel % 2 == 0 { left } else { right };
        }
    }
    out
}

/// The span of a source's samples a clip plays: from its offset, for
/// the part of the source its length covers.
///
/// The clip speaks in the **session's samples** and the source speaks in
/// **its own**, and the span is where the two meet: the offset is
/// turned into the source's frames, and the length is turned into the
/// frames the source is read in — which is **less** than the length
/// when the clip is stretched, because a stretched clip reads its
/// source more slowly.
///
/// The span is in **frames**, and the samples of the source are
/// interleaved, so the frames are multiplied by the number of channels.
fn the_span(sound: &craftkit_wav::Sound, payload: &ClipPayload, rate: u32) -> (usize, usize) {
    let width = sound.channels as usize;
    // How many frames of the source one sample of the session is.
    let ratio = sound.rate as f64 / rate.max(1) as f64;
    let from_frame = (payload.offset.max(0) as f64 * ratio).round() as usize;
    let span_frames = ((payload.length.max(0) as f64 / payload.stretch) * ratio).round() as usize;
    let from = (from_frame * width).min(sound.samples.len());
    let to = ((from_frame + span_frames) * width).min(sound.samples.len()).max(from);
    (from, to)
}

/// How many **segments** a drawn fade has.
///
/// A fade is a curve, and a curve drawn with two points is a line. The
/// number is small because a notch in a bar is small, and because the
/// notch is a hint and not the sound.
const FADE_SEGMENTS: usize = 8;

/// How many **numbers** a sound's peaks have.
///
/// The peaks are the shape of a sound in a small table, and the table
/// has to be small enough to live in a payload: a thousand numbers is
/// a shape that a reader can see and a document can carry. The
/// buckets and the channels share it — a sound with two channels has
/// half the buckets of a sound with one, so that the table is the same
/// size either way.
const PEAK_NUMBERS: usize = 1024;

/// The **peaks** of a sound: the extent of the samples in each bucket.
///
/// A sound is a great many numbers and a drawing needs few. The peaks
/// are the least and the most of each bucket of samples, and they are
/// what a clip's bar is drawn from.
fn peaks_of(sound: &craftkit_wav::Sound) -> Vec<f32> {
    let channels = sound.channels.max(1) as usize;
    let frames = sound.frames as usize;
    // The buckets and the channels **share** the table: a table of a
    // thousand numbers is a thousand numbers however many channels
    // there are.
    let buckets = (PEAK_NUMBERS / (channels * 2)).max(1);
    let per_bucket = (frames / buckets).max(1);
    let mut out = Vec::with_capacity(buckets * channels * 2);
    for bucket in 0..buckets {
        let from = bucket * per_bucket;
        let to = (from + per_bucket).min(frames);
        for channel in 0..channels {
            if from >= frames {
                // A sound shorter than the table: the rest of the table
                // is silence, and silence is a shape too.
                out.push(0.0);
                out.push(0.0);
                continue;
            }
            let mut least = f32::MAX;
            let mut most = f32::MIN;
            for frame in from..to {
                let Some(sample) = sound.samples.get(frame * channels + channel) else { continue };
                if *sample < least {
                    least = *sample;
                }
                if *sample > most {
                    most = *sample;
                }
            }
            out.push(least);
            out.push(most);
        }
    }
    out
}

/// The **peaks** of a source, as the payload holds them.
///
/// The peaks are a flat table — a least and a most for each bucket —
/// and the source says how many frames of the sound the table covers.
struct Peaks {
    /// The table: for each bucket, the least and the most of each
    /// channel.
    table: Vec<f32>,
    /// How many channels the sound has.
    channels: usize,
    /// How many frames of the sound the table covers.
    frames: u64,
}

/// The peaks of a source, if it has been measured.
fn peaks_of_source(document: &Document, source: u64) -> Option<Peaks> {
    let node = document.node(source)?;
    let payload = SourcePayload::from_json(&node.data).ok()?;
    let channels = payload.channels.max(1) as usize;
    if payload.peaks.is_empty() {
        return None;
    }
    Some(Peaks { table: payload.peaks, channels, frames: payload.frames.max(0) as u64 })
}

/// The **waveform** of a clip: the shape of the sound it plays.
///
/// The clip plays a **part** of a source, from its offset, for as long
/// as its length says — and a stretched clip reads its source more
/// slowly, so it plays less of it. The bar is the clip, and the shape
/// drawn on the bar is the part of the source the clip plays.
fn waveform_paths<F: Fn(usize, usize) -> f32>(
    rect: &Rect,
    peaks: &Peaks,
    from_frame: usize,
    span_frames: usize,
    gain: F,
) -> Vec<Path> {
    let channels = peaks.channels.max(1);
    let count = peaks.table.len() / (channels * 2);
    if count == 0 || peaks.frames == 0 {
        return Vec::new();
    }
    let from_bucket = from_frame.saturating_mul(count) / peaks.frames as usize;
    let to_bucket = ((from_frame + span_frames).saturating_mul(count) / peaks.frames as usize)
        .min(count)
        .max(from_bucket + 1);
    if from_bucket >= count {
        return Vec::new();
    }
    let buckets = to_bucket - from_bucket;
    let width = rect.w;
    let mut paths: Vec<Path> = Vec::with_capacity(channels);
    for channel in 0..channels {
        // The **band** of the channel: the bar is as tall as the
        // channels it holds, and each channel has its own.
        let band = Rect::new(
            rect.x,
            rect.y + rect.h * channel as f64 / channels as f64,
            rect.w,
            rect.h / channels as f64,
        );
        // The **upper** edge of the shape, left to right, and then the
        // lower edge, right to left: the shape is the sound between
        // them.
        let mut upper: Vec<Point> = Vec::with_capacity(buckets);
        let mut lower: Vec<Point> = Vec::with_capacity(buckets);
        for index in 0..buckets {
            let bucket = from_bucket + index;
            let at = (bucket * channels + channel) * 2;
            let least = peaks.table.get(at).copied().unwrap_or(0.0);
            let most = peaks.table.get(at + 1).copied().unwrap_or(least);
            let gain = gain(index, buckets);
            let x = rect.x + (index as f64 + 0.5) * width / buckets as f64;
            upper.push(Point::new(x, level_y(&band, most * gain)));
            lower.push(Point::new(x, level_y(&band, least * gain)));
        }
        if upper.is_empty() {
            continue;
        }
        let mut segments: Vec<Segment> = Vec::with_capacity(upper.len() * 2);
        for point in upper.iter().skip(1) {
            segments.push(Segment::Line { to: *point });
        }
        for point in lower.iter().rev().skip(1) {
            segments.push(Segment::Line { to: *point });
        }
        let Some(start) = upper.first() else { continue };
        paths.push(Path::single(SubPath { start: *start, segments, closed: true }));
    }
    paths
}
/// The **waveform** of a clip: the shape of the sound it plays.
///
/// The clip plays a part of a source, and the part is in the source's
/// own frames: the offset is turned into the source's frames, and the
/// length is turned into the frames the source is read in — which is
/// less than the length when the clip is stretched, because a stretched
/// clip reads its source more slowly.
fn waveform_of(document: &Document, clip: &Node, payload: &ClipPayload, rect: &Rect) -> Vec<Path> {
    let Some(source) = payload.source else { return Vec::new() };
    let Some(peaks) = peaks_of_source(document, source) else { return Vec::new() };
    let Some(source_node) = document.node(source) else { return Vec::new() };
    let Ok(source_payload) = SourcePayload::from_json(&source_node.data) else { return Vec::new() };
    let session_rate = session_of(document).sample_rate.max(1) as f64;
    let ratio = source_payload.sample_rate.max(1) as f64 / session_rate;
    let from_frame = (payload.offset.max(0) as f64 * ratio).round() as usize;
    let span_frames =
        ((payload.length.max(0) as f64 / payload.stretch.max(f64::EPSILON)) * ratio).round() as usize;
    // The **gain** of the sound at each place: the clip's own gain and
    // the track's automation. A clip that runs across a breakpoint is
    // loud at one end and quiet at the other, and the shape on the bar
    // says so — the shape is the sound as it is heard and not the
    // sound as it is on the disk.
    let length = payload.length.max(0);
    waveform_paths(rect, &peaks, from_frame, span_frames, |index, count| {
        let at = payload.start + (length * index as i64) / count.max(1) as i64;
        gain_at(document, clip, payload, at) as f32
    })
}

/// The **gain** of a clip's sound at that time.
///
/// The clip's own gain and the track's **automation**: the volume lane
/// is in decibels and the mute lane is one or less, and a muted track
/// is silent however loud its sound is.
fn gain_at(document: &Document, clip: &Node, payload: &ClipPayload, at: i64) -> f64 {
    let mut decibels = payload.gain_db;
    if let Some(track) = timeline::track_of(document, clip) {
        let automation = automate::automation_of(document, track);
        if let Some(points) = &automation.volume {
            decibels += automate::value_at(points, at);
        }
        if let Some(points) = &automation.mute
            && automate::value_at(points, at) >= 1.0
        {
            return 0.0;
        }
    }
    craftkit_audio::db_to_linear(decibels)
}

/// Where a **level** of the sound stands in a bar.
///
/// The sound runs from minus one to one, and the bar is the height it
/// has: the top of the bar is the loudest sound and the bottom is the
/// quietest.
fn level_y(rect: &Rect, level: f32) -> f64 {
    let level = level.clamp(-1.0, 1.0) as f64;
    rect.y + (1.0 - level) * rect.h / 2.0
}

/// The **notch** of a fade at the head of a bar.
///
/// The notch is the part of the clip that is quiet: it is the region
/// under the fade's curve, and the curve is the fade's own shape.
fn fade_in_path(rect: &Rect, width: f64, fade: craftkit_audio::Fade) -> Option<Path> {
    let mut segments = Vec::with_capacity(FADE_SEGMENTS);
    for step in 1..=FADE_SEGMENTS {
        let t = step as f64 / FADE_SEGMENTS as f64;
        let x = rect.x + t * width;
        let y = rect.y + rect.h - fade.gain(t as f32) as f64 * rect.h;
        segments.push(Segment::Line { to: Point::new(x, y) });
    }
    Some(Path::single(SubPath { start: Point::new(rect.x, rect.y + rect.h), segments, closed: true }))
}

/// The **notch** of a fade at the tail of a bar.
///
/// The notch is the part of the clip that is quiet, and at the tail the
/// quiet part is **above** the curve: the curve falls from the full
/// height to nothing, and what is left of the bar above it is the
/// silence.
fn fade_out_path(rect: &Rect, width: f64, fade: craftkit_audio::Fade) -> Option<Path> {
    let start = rect.x + rect.w - width;
    let mut segments = Vec::with_capacity(FADE_SEGMENTS);
    for step in 1..=FADE_SEGMENTS {
        let t = step as f64 / FADE_SEGMENTS as f64;
        let x = start + t * width;
        let y = rect.y + rect.h - fade.gain((1.0 - t) as f32) as f64 * rect.h;
        segments.push(Segment::Line { to: Point::new(x, y) });
    }
    Some(Path::single(SubPath { start: Point::new(start, rect.y), segments, closed: true }))
}

/// The **metronome**: a click at every beat.
///
/// The beats come from the tempo map, and the meter says how long a
/// beat is: a click at every beat, and a different click on the first
/// beat of a bar, because that is the beat a musician waits for.
///
/// The click is mixed into the master **after** everything else, so it
/// does not pass through a track, a send or an insert: it is a voice
/// of its own.
fn click_into(
    master: &mut [f32],
    channels: usize,
    rate: u32,
    map: &craftkit_sound_time::TempoMap,
    settings: &SoundSession,
) {
    let level = craftkit_audio::db_to_linear(settings.click_volume_db) as f32;
    if level <= 0.0 {
        return;
    }
    let frames = master.len() / channels.max(1);
    // The walk is over the beats, and it stops when the beats are past
    // the end of the sound. The counter is the bound: a document with
    // a tempo of a million beats a minute would otherwise walk for a
    // very long time.
    let mut tick = 0_i64;
    let mut walked = 0_u32;
    while walked < 1_000_000 {
        let at = map.ticks_to_samples(tick, rate);
        if at < 0 || at as usize >= frames {
            break;
        }
        let accent = map.bar_beat_at_tick(tick).beat == 1;
        let note = if accent { settings.click_accent_note } else { settings.click_note };
        craftkit_audio::blip(master, channels, at as usize, craftkit_audio::midi_to_hz(note), level, rate);
        let (_, denominator) = map.meter_at_tick(tick);
        tick += craftkit_sound_time::ticks_per_beat(denominator).max(1);
        walked += 1;
    }
}

/// Read a sound file, within limits.
///
/// The file came from outside — it is named by a document that came
/// from outside — so it is read **within limits** and a file that is
/// too big is refused before a byte of it is read.
fn read_sound(id: &str, path: &str) -> Result<craftkit_wav::Sound, EngineError> {
    let limits = craftkit_wav::Limits::default();
    let metadata = std::fs::metadata(path).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the source `{path}` cannot be read: {error}"),
    })?;
    if metadata.len() as usize > limits.max_bytes {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the source `{path}` is larger than the limits allow"),
        });
    }
    let bytes = std::fs::read(path).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the source `{path}` cannot be read: {error}"),
    })?;
    craftkit_wav::read(&bytes, &limits).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the source `{path}` is not a sound the kit reads: {error}"),
    })
}

/// Write the sound to a file.
fn write_file(path: &str, bytes: &[u8]) -> Result<(), EngineError> {
    std::fs::write(path, bytes).map_err(|error| EngineError::Failed {
        id: "sound.render".to_string(),
        message: format!("the sound cannot be written to `{path}`: {error}"),
    })
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
    /// Whether the **metronome** is heard.
    pub metronome: Option<Bool>,
    /// How loud the click is, in decibels.
    pub click_volume_db: Option<Number>,
    /// The click's pitch, as a note number.
    pub click_note: Option<Number>,
    /// The **accent's** pitch: the click on the first beat of a bar.
    pub click_accent_note: Option<Number>,
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
        if let Some(metronome) = &self.metronome {
            settings.metronome = metronome.0;
        }
        if let Some(volume) = &self.click_volume_db {
            if !volume.0.is_finite() {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: "a level is a number of decibels".to_string(),
                });
            }
            settings.click_volume_db = volume.0;
        }
        if let Some(note) = &self.click_note {
            if !note.0.is_finite() {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: "a pitch is a note number".to_string(),
                });
            }
            settings.click_note = note.0;
        }
        if let Some(note) = &self.click_accent_note {
            if !note.0.is_finite() {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: "a pitch is a note number".to_string(),
                });
            }
            settings.click_accent_note = note.0;
        }
        set_session(session.document_mut()?, &settings);
        Ok(Outcome::changed(format!("the session runs at {} samples a second", settings.sample_rate))
            .with_data(serde_json::json!({
                "sample_rate": settings.sample_rate,
                "timecode_start": settings.timecode_start,
                "metronome": settings.metronome,
                "click_volume_db": settings.click_volume_db,
                "click_note": settings.click_note,
                "click_accent_note": settings.click_accent_note,
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
        "sound.insert.set",
        "sound.playhead.set",
        "sound.source.add",
        "sound.render",
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

/// A stretch, from a number a caller sent.
///
/// The stretch is a **length ratio**, and it is bounded: it is greater
/// than zero, because a clip that plays nothing is not a clip, and it
/// is no more than [`MAX_STRETCH`], because a clip stretched a
/// thousandfold would hold a thousandth of a sample for a thousand
/// times as long.
fn stretch_of(value: f64, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed { id: id.to_string(), message: "a stretch is a number".to_string() });
    }
    if value <= 0.0 || value > MAX_STRETCH {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "a stretch is greater than zero and no more than {MAX_STRETCH}, and this one is {value}"
            ),
        });
    }
    Ok(value)
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
    SoloTrack,
    AddClip,
    SetClipStretch,
    SetClipFade,
    SetTempo,
    SetMeter,
    AddMarker,
    AddSource,
    MeasurePeaks,
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

    /// The document is **rendered**: the clips are played and the sound
    /// is written to a file.
    ///
    /// The renderer is offline — it makes a file and it stops — and the
    /// file it makes is a sound the kit can read back.
    #[test]
    fn the_document_is_rendered_into_a_file() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-render-source.wav");
        let output_path = directory.join("craftkit-v2-render-out.wav");
        // A source: four samples of half.
        let bytes = craftkit_wav::write(1, 48_000, &[0.5, 0.5, 0.5, 0.5]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.frames, 4, "the clip is four samples long");
        assert!((sound.samples[0] - 0.5).abs() < 0.001, "the sample is the source's: {}", sound.samples[0]);
    }

    /// A source at **another rate** is resampled: the sound is the same
    /// sound, at the rate the session plays at.
    ///
    /// The clip's length is in the **session's** samples, so a clip of
    /// four is four samples long whatever the source's rate is — and
    /// it is four samples long in the picture too, because the
    /// timeline draws the clip's length in the session's samples. The
    /// source is read at the rate it was written at and played at the
    /// rate the session plays at, and the clip takes the part of it
    /// that fits.
    #[test]
    fn a_source_at_another_rate_is_resampled() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-render-rate.wav");
        let output_path = directory.join("craftkit-v2-render-rate-out.wav");
        // A source of eight thousand samples a second, four frames.
        let bytes = craftkit_wav::write(1, 8_000, &[0.5, 0.5, 0.5, 0.5]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 16_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.rate, 16_000, "the sound is at the session's rate");
        assert_eq!(sound.frames, 4, "and it is as long as the clip says: {}", sound.frames);
        assert!((sound.samples[0] - 0.5).abs() < 0.001, "and it holds the source's samples");
    }

    /// The **automation** moves the volume: a lane of volume is heard.
    ///
    /// The lane is applied **per sample**, because that is what a curve
    /// is: the volume at the sample's own time.
    #[test]
    fn the_automation_moves_the_volume() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-auto-source.wav");
        let output_path = directory.join("craftkit-v2-auto-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[0.5; 4]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        // A lane of volume: half of it is six decibels down, and the
        // last of it is loud again.
        session
            .execute(
                "sound.lane.add",
                &json!({
                    "track": track,
                    "param": "volume",
                    "points": [[0, -6.0], [3, 0.0]],
                }),
            )
            .expect("the lane is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.frames, 4);
        // The first sample is at the lane's first point: six decibels
        // down is about half.
        assert!((sound.samples[0] - 0.25).abs() < 0.01, "the first sample is quieter: {}", sound.samples[0]);
        // The last is at the second point: nothing down at all.
        assert!((sound.samples[3] - 0.5).abs() < 0.01, "the last sample is loud again: {}", sound.samples[3]);
    }

    /// A **mute** lane mutes the track, and a lane of a parameter the
    /// kit does not know is drawn and not heard.
    #[test]
    fn a_mute_lane_mutes_the_track() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-mute-source.wav");
        let output_path = directory.join("craftkit-v2-mute-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[0.5; 2]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.lane.add", &json!({ "track": track, "param": "mute", "points": [[0, 1.0]] }))
            .expect("the lane is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.frames, 2);
        assert!((sound.samples[0] - 0.0).abs() < 0.001, "the track is muted: {}", sound.samples[0]);
    }

    /// The **routing** is heard: a track that sends to a bus is heard
    /// twice — once on its own and once through the bus.
    #[test]
    fn the_routing_is_heard() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-send-source.wav");
        let output_path = directory.join("craftkit-v2-send-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[0.25, 0.25]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
            .expect("the clip is added");
        // The track sends to a bus, at six decibels down: about half.
        session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
        let bus = session.document().node_by_name("fx").expect("the bus is there").id;
        session
            .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": -6.0 }))
            .expect("the send is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        // A quarter on its own and a quarter at half: about three
        // eighths.
        assert!(
            (sound.samples[0] - 0.375).abs() < 0.01,
            "the track and the bus are both heard: {}",
            sound.samples[0]
        );
    }

    /// An **insert** is run over the track's signal: the insert names a
    /// plugin and the plugin is a program.
    #[test]
    fn an_insert_is_run_over_the_track() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-insert-source.wav");
        let output_path = directory.join("craftkit-v2-insert-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[1.0, 1.0]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
            .expect("the clip is added");
        // A gain insert, at six decibels down.
        session
            .execute("sound.insert.add", &json!({ "track": track, "plugin": "gain" }))
            .expect("the insert is added");
        let insert =
            session.document().walk().find(|node| node.kind == "sound.insert").expect("the insert is there");
        session
            .execute("sound.insert.set", &json!({ "insert": insert.id, "parameters": { "gain_db": -6.0 } }))
            .expect("the insert is changed");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert!((sound.samples[0] - 0.5).abs() < 0.01, "the insert changed the sound: {}", sound.samples[0]);
    }

    /// A plugin that is **not in the catalogue** is refused: a plugin
    /// that cannot be made cannot be run.
    #[test]
    fn a_plugin_that_is_not_in_the_catalogue_is_refused() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-unknown-source.wav");
        let output_path = directory.join("craftkit-v2-unknown-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[1.0]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.insert.add", &json!({ "track": track, "plugin": "reverb" }))
            .expect("the insert is added");
        let error = session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect_err("there is no reverb in the catalogue");
        assert!(error.to_string().contains("reverb"), "the refusal says what is wrong: {error}");
    }

    /// A source that is **not there** is refused: the renderer does not
    /// make a sound out of nothing.
    #[test]
    fn a_source_that_is_not_there_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.source.add", &json!({ "path": "/nowhere/at/all.wav" }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        let directory = std::env::temp_dir();
        let output_path = directory.join("craftkit-v2-render-missing.wav");
        let error = session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect_err("the source is not there");
        assert!(error.to_string().contains("source"), "the refusal says what is wrong: {error}");
    }

    /// The playhead is **where the sound is**: a line at the position.
    #[test]
    fn the_playhead_is_a_line_at_the_position() {
        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.playhead.set", &json!({ "position": 24_000 })).expect("the playhead is set");
        let playhead = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.playhead")
            .expect("the playhead is there");
        let rect = PlayheadKind::place(playhead, session.document()).expect("the playhead has a place");
        // Twenty four thousand samples at forty eight thousand a second
        // is half a second.
        assert!((rect.x + rect.w / 2.0 - 0.5).abs() < 0.001, "the playhead is half a second in: {}", rect.x);
        // And it runs from the ruler's row through the tracks.
        assert_eq!(rect.y, -timeline::RULER_HEIGHT, "it starts at the ruler");
        assert!(rect.h > timeline::RULER_HEIGHT, "and it reaches the tracks");
    }

    /// The playhead is **moved**, and there is still only one of it.
    #[test]
    fn the_playhead_is_moved_and_not_duplicated() {
        let mut session = session();
        session.execute("sound.playhead.set", &json!({ "position": 0 })).expect("the playhead is set");
        session.execute("sound.playhead.set", &json!({ "position": 960 })).expect("the playhead is moved");
        let playheads: Vec<_> =
            session.document().walk().filter(|node| node.kind == "sound.playhead").collect();
        assert_eq!(playheads.len(), 1, "there is one playhead");
        assert_eq!(playheads[0].data["position"], json!(960), "and it is where it was moved");
    }

    /// A position that is **before the beginning** is refused.
    #[test]
    fn a_position_before_the_beginning_is_refused() {
        let mut session = session();
        let error = session
            .execute("sound.playhead.set", &json!({ "position": -1 }))
            .expect_err("that is before the beginning");
        assert!(
            error.to_string().contains("before the beginning"),
            "the refusal says what is wrong: {error}"
        );
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
mod waveform_tests {
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

    /// A track with a clip on it, and the clip's payload.
    fn a_clip(session: &mut Session) -> (u64, u64, ClipPayload) {
        session.execute("sound.track.add", &json!({ "name": "track" })).expect("the track is added");
        let track = session.document().node_by_name("track").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        (track, clip, payload)
    }

    /// The **gain** of a clip's sound at that time, with the track's
    /// volume lane saying what it says.
    #[test]
    fn the_gain_follows_the_automation() {
        let mut session = session();
        let (track, clip, payload) = a_clip(&mut session);
        session
            .execute(
                "sound.lane.add",
                &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [48_000, -20.0]] }),
            )
            .expect("the track is automated");
        let node = session.document().node(clip).expect("the clip is there");
        let document = session.document();
        let loud = gain_at(document, node, &payload, 0);
        let quiet = gain_at(document, node, &payload, 48_000);
        assert!((loud - 1.0).abs() < 1e-6, "the sound starts as it is: {loud}");
        assert!((quiet - 0.1).abs() < 1e-3, "and is twenty decibels down at the end: {quiet}");
    }

    /// A **muted** track is silent however loud its sound is.
    #[test]
    fn a_muted_track_is_silent() {
        let mut session = session();
        let (track, clip, payload) = a_clip(&mut session);
        session
            .execute("sound.lane.add", &json!({ "track": track, "param": "mute", "points": [[0, 1.0]] }))
            .expect("the track is muted");
        let node = session.document().node(clip).expect("the clip is there");
        let document = session.document();
        assert_eq!(gain_at(document, node, &payload, 0), 0.0, "the sound is silent");
    }

    /// The **shape** on the bar is the shape of the sound as it is
    /// heard: a gain that falls makes the shape fall.
    #[test]
    fn the_shape_follows_the_gain() {
        let peaks = Peaks { table: vec![-0.5, 0.5, -0.5, 0.5], channels: 1, frames: 2 };
        let rect = Rect::new(0.0, 0.0, 2.0, 1.0);
        let loud = waveform_paths(&rect, &peaks, 0, 2, |_index, _count| 1.0);
        let quiet = waveform_paths(&rect, &peaks, 0, 2, |_index, _count| 0.25);
        assert_eq!(loud.len(), 1, "a mono sound has one band");
        let loud = loud.first().expect("the shape is drawn");
        let quiet = quiet.first().expect("the shape is drawn");
        // The shape is a path: the upper edge first, and the lower edge
        // after it. The upper edge of the loud shape is at the top of
        // the bar, and the quiet one is a quarter of the way down.
        let loud_top = loud.subpaths.first().map(|subpath| subpath.start.y).unwrap_or(0.0);
        let quiet_top = quiet.subpaths.first().map(|subpath| subpath.start.y).unwrap_or(0.0);
        assert!((loud_top - 0.25).abs() < 1e-6, "half a sound is at the top of the bar: {loud_top}");
        assert!(quiet_top > loud_top, "and a quarter of it is lower: {quiet_top}");
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
