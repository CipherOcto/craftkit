//! The **MIDI** of a clip: the notes it holds and the file they came
//! from.
//!
//! The tree's sound editor has a MIDI clip: a clip whose content is a
//! **sequence** of notes and controllers, and the tree reads and writes
//! Standard MIDI Files. v2 had none of it — a clip was a clip of sound
//! and nothing else — and the notes had nowhere to live.
//!
//! # What a MIDI clip is here
//!
//! A clip whose **content** is `midi`. The content is a **field in the
//! clip's payload**, and that is the whole of the tree's closed enum of
//! three kinds of clip: the tree says "this clip is audio, this one is
//! MIDI and this one is video" with a variant, and here the clip says
//! what its content is with a **string** — so a third party may add a
//! kind of content without touching the core.
//!
//! The **notes** are in the clip's payload as well, and so are the
//! **controllers**. They are values and not nodes: a note is a pitch
//! and a place and a length, and it has no life of its own — the tree's
//! note has none either, and the tree's sequence is a value in the
//! clip's record.
//!
//! The notes are in **ticks**, and a tick is the kit's own tick: nine
//! hundred and sixty to a quarter note, which is what the MIDI library
//! counts in and what the sound battery's tempo map counts in. So a
//! note's place in a clip and a clip's place on the timeline are two
//! readings of the same scale.
//!
//! # What a MIDI clip is not
//!
//! It is **not sound**. There is no synthesiser here: a MIDI clip has
//! no source, and the render skips a clip that has no source — so a
//! MIDI clip is silent, and it is silent **honestly**: what it holds
//! is the notes, and the notes are not a sound until something makes
//! them one.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::{ClipPayload, samples, session_of, tempo_map};

/// The content a clip holds when it holds **notes**.
pub(crate) const CONTENT_MIDI: &str = "midi";

/// The most notes a clip may hold.
///
/// Sixty-five thousand notes is more than a person plays in a lifetime,
/// and a clip with more of them is a clip the kit cannot draw, cannot
/// save and cannot undo.
const MAX_NOTES: usize = 65_536;

/// The most a MIDI file may weigh, in bytes.
///
/// A MIDI file is a few kilobytes for a piece of music, and sixteen
/// megabytes is far past any such file.
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// A **note** of a clip.
///
/// The times are in ticks, and the tick is nine hundred and sixty to a
/// quarter note — the kit's tick and the MIDI library's alike.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NotePayload {
    /// The pitch: zero to a hundred and twenty-seven.
    pitch: u8,
    /// How hard the note is struck: one to a hundred and twenty-seven.
    velocity: u8,
    /// The channel the note is on: zero to fifteen.
    channel: u8,
    /// Where it starts, in ticks from the start of the clip.
    start: i64,
    /// How long it is, in ticks. Never less than one.
    length: i64,
}

impl NotePayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "pitch": self.pitch,
            "velocity": self.velocity,
            "channel": self.channel,
            "start": self.start,
            "length": self.length,
        })
    }

    fn from_json(value: &Json) -> Result<NotePayload, String> {
        let pitch = value.get("pitch").and_then(|pitch| pitch.as_u64()).unwrap_or(60);
        if pitch > 127 {
            return Err(format!("a pitch is from 0 to 127, and {pitch} is not one"));
        }
        let velocity = value.get("velocity").and_then(|velocity| velocity.as_u64()).unwrap_or(64);
        if velocity > 127 {
            return Err(format!("a velocity is from 0 to 127, and {velocity} is not one"));
        }
        let channel = value.get("channel").and_then(|channel| channel.as_u64()).unwrap_or(0);
        if channel > 15 {
            return Err(format!("a channel is from 0 to 15, and {channel} is not one"));
        }
        let start = value.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        if start < 0 {
            return Err("a note does not start before the beginning of the clip".to_string());
        }
        let length = value.get("length").and_then(|length| length.as_i64()).unwrap_or(1);
        if length < 1 {
            return Err("a note is at least one tick long".to_string());
        }
        Ok(NotePayload {
            pitch: pitch as u8,
            velocity: velocity as u8,
            channel: channel as u8,
            start,
            length,
        })
    }
}

/// A **controller** of a clip.
///
/// The kind is a **string** and not a variant, for the same reason the
/// kinds of a sound document are strings: the kinds are open here. A
/// kind this battery does not know is **refused**, because a
/// controller that does nothing is worse than no controller at all.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CtrlPayload {
    /// Where it happens, in ticks from the start of the clip.
    tick: i64,
    /// The channel it happens on: zero to fifteen.
    channel: u8,
    /// What it changes.
    kind: String,
    /// The number it changes, for the kinds that have one.
    number: Option<u8>,
    /// The value it changes it to.
    value: i32,
}

/// The kinds of controller a clip may hold.
///
/// The names are the MIDI library's kinds, in the library's own words:
/// a control change, a pitch bend, the pressure of the channel, the
/// pressure of one pitch and a program change.
const CTRL_KINDS: [&str; 5] = ["control", "pitch_bend", "channel_pressure", "poly_pressure", "program"];

impl CtrlPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "tick": self.tick,
            "channel": self.channel,
            "kind": self.kind,
            "number": self.number,
            "value": self.value,
        })
    }

    fn from_json(value: &Json) -> Result<CtrlPayload, String> {
        let tick = value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0);
        if tick < 0 {
            return Err("a controller does not happen before the beginning of the clip".to_string());
        }
        let channel = value.get("channel").and_then(|channel| channel.as_u64()).unwrap_or(0);
        if channel > 15 {
            return Err(format!("a channel is from 0 to 15, and {channel} is not one"));
        }
        let kind = value.get("kind").and_then(|kind| kind.as_str()).unwrap_or("").to_string();
        if !CTRL_KINDS.contains(&kind.as_str()) {
            return Err(format!("there is no controller called `{kind}`"));
        }
        let number = value.get("number").and_then(|number| number.as_u64());
        if let Some(number) = number
            && number > 127
        {
            return Err(format!("a controller's number is from 0 to 127, and {number} is not one"));
        }
        Ok(CtrlPayload {
            tick,
            channel: channel as u8,
            kind,
            number: number.map(|number| number as u8),
            value: value.get("value").and_then(|value| value.as_i64()).unwrap_or(0) as i32,
        })
    }

    /// The controller as the MIDI library has it.
    fn to_ctrl(&self) -> craftkit_midi::CtrlEvent {
        let kind = match (self.kind.as_str(), self.number) {
            ("control", Some(number)) => craftkit_midi::CtrlKind::Control(number),
            ("poly_pressure", Some(number)) => craftkit_midi::CtrlKind::PolyPressure(number),
            ("pitch_bend", _) => craftkit_midi::CtrlKind::PitchBend,
            ("channel_pressure", _) => craftkit_midi::CtrlKind::ChannelPressure,
            _ => craftkit_midi::CtrlKind::Program,
        };
        craftkit_midi::CtrlEvent { tick: self.tick, channel: self.channel, kind, value: self.value }
    }
}

/// The **sequence** of a MIDI clip: its notes and its controllers.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MidiPayload {
    /// The notes, in the order they were in.
    notes: Vec<NotePayload>,
    /// The controllers, in the order they were in.
    ctrls: Vec<CtrlPayload>,
}

impl MidiPayload {
    /// The sequence as the MIDI library has it.
    fn to_sequence(&self) -> craftkit_midi::Sequence {
        let notes = self
            .notes
            .iter()
            .map(|note| craftkit_midi::Note {
                pitch: note.pitch,
                velocity: note.velocity,
                release_velocity: 64,
                channel: note.channel,
                start: note.start,
                length: note.length,
            })
            .collect();
        let ctrls = self.ctrls.iter().map(CtrlPayload::to_ctrl).collect();
        craftkit_midi::Sequence { notes, ctrls }
    }

    /// The sequence of a MIDI library file, as this payload.
    fn from_sequence(sequence: &craftkit_midi::Sequence) -> MidiPayload {
        let notes = sequence
            .notes
            .iter()
            .map(|note| NotePayload {
                pitch: note.pitch,
                velocity: note.velocity,
                channel: note.channel,
                start: note.start,
                length: note.length,
            })
            .collect();
        let ctrls = sequence
            .ctrls
            .iter()
            .map(|ctrl| {
                let (kind, number) = match ctrl.kind {
                    craftkit_midi::CtrlKind::Control(number) => ("control", Some(number)),
                    craftkit_midi::CtrlKind::PolyPressure(number) => ("poly_pressure", Some(number)),
                    craftkit_midi::CtrlKind::PitchBend => ("pitch_bend", None),
                    craftkit_midi::CtrlKind::ChannelPressure => ("channel_pressure", None),
                    craftkit_midi::CtrlKind::Program => ("program", None),
                };
                CtrlPayload {
                    tick: ctrl.tick,
                    channel: ctrl.channel,
                    kind: kind.to_string(),
                    number,
                    value: ctrl.value,
                }
            })
            .collect();
        MidiPayload { notes, ctrls }
    }

    pub(crate) fn to_json(&self) -> Json {
        serde_json::json!({
            "notes": self.notes.iter().map(NotePayload::to_json).collect::<Vec<Json>>(),
            "ctrls": self.ctrls.iter().map(CtrlPayload::to_json).collect::<Vec<Json>>(),
        })
    }

    pub(crate) fn from_json(value: &Json) -> Result<MidiPayload, String> {
        let notes = match value.get("notes") {
            Some(notes) => {
                let list = notes.as_array().ok_or("the notes of a clip are a list")?;
                let mut out = Vec::with_capacity(list.len());
                for note in list {
                    out.push(NotePayload::from_json(note)?);
                }
                out
            }
            None => Vec::new(),
        };
        if notes.len() > MAX_NOTES {
            return Err(format!(
                "a clip holds at most {MAX_NOTES} notes, and this one holds {}",
                notes.len()
            ));
        }
        let ctrls = match value.get("ctrls") {
            Some(ctrls) => {
                let list = ctrls.as_array().ok_or("the controllers of a clip are a list")?;
                let mut out = Vec::with_capacity(list.len());
                for ctrl in list {
                    out.push(CtrlPayload::from_json(ctrl)?);
                }
                out
            }
            None => Vec::new(),
        };
        Ok(MidiPayload { notes, ctrls })
    }

    /// The tick the last thing of the sequence reaches.
    fn end_tick(&self) -> i64 {
        let note = self.notes.iter().map(|note| note.start.saturating_add(note.length)).max();
        let ctrl = self.ctrls.iter().map(|ctrl| ctrl.tick).max();
        match (note, ctrl) {
            (Some(note), Some(ctrl)) => note.max(ctrl),
            (Some(note), None) | (None, Some(note)) => note,
            (None, None) => 0,
        }
    }
}

/// **Import a MIDI file** as a clip.
///
/// The file is a Standard MIDI File, and the notes of **all** its
/// tracks become the notes of the clip: a clip holds one sequence, and
/// a file may hold many.
///
/// The **tempo** of the file is not the tempo of the document, and the
/// document's is the one that is used: the notes are in ticks, and the
/// ticks are read at the tempo the document says. A file that carries
/// a tempo of its own is imported at the document's tempo, and the
/// file's tempo is not read into the document — because a second tempo
/// map would be a second truth about the same timeline.
///
/// The **length** of the clip is **derived**: it is as long as the
/// notes need, at the tempo the document says where the clip starts. A
/// file with no notes is **refused**: a clip of no length is not a
/// clip.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.import",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Import MIDI"),
    mutates,
    undoable
)]
pub struct ImportMidi {
    /// The track the clip goes on.
    pub track: NodeRef,
    /// Where the clip starts, in samples.
    pub start: Number,
    /// The MIDI file to read.
    pub path: Text,
}

impl ImportMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.import".to_string();
        let start = samples(self.start.0, "where the clip starts")?;
        let path = self.path.0.clone();
        let file = read_midi_file(&id, &path)?;
        // The notes of every track of the file: a clip holds one
        // sequence, and a file may hold many.
        let mut sequence = craftkit_midi::Sequence::default();
        for track in &file.tracks {
            sequence.notes.extend(track.sequence.notes.iter().copied());
            sequence.ctrls.extend(track.sequence.ctrls.iter().copied());
        }
        sequence.sort();
        if sequence.notes.is_empty() {
            return Err(EngineError::Failed { id, message: format!("the file `{path}` holds no notes") });
        }
        if sequence.notes.len() > MAX_NOTES {
            return Err(EngineError::Failed {
                id,
                message: format!("the file `{path}` holds more than {MAX_NOTES} notes"),
            });
        }
        let payload = MidiPayload::from_sequence(&sequence);
        // The **length** of the clip is derived: it is as long as the
        // notes need, at the tempo of the document where the clip is.
        let settings = session_of(session.document());
        let map = tempo_map(session.document());
        let start_tick = map.samples_to_ticks(start, settings.sample_rate);
        let end_tick = start_tick.saturating_add(payload.end_tick());
        let length = map
            .ticks_to_samples(end_tick, settings.sample_rate)
            .saturating_sub(map.ticks_to_samples(start_tick, settings.sample_rate));
        if length <= 0 {
            return Err(EngineError::Failed {
                id,
                message: format!("the notes of `{path}` are no longer than a moment"),
            });
        }
        let playlist = crate::playlist_of(session, self.track.0, None)?;
        let mut clip = ClipPayload::new();
        clip.start = start;
        clip.length = length;
        clip.content = Some(CONTENT_MIDI.to_string());
        clip.midi = Some(payload);
        let name = file_name_of(&path);
        let clip_id = session.document_mut()?.add_node_with_under(
            Some(playlist),
            name.clone(),
            "sound.clip",
            None,
            clip.to_json(),
        );
        Ok(Outcome::changed(format!("imported the MIDI of `{path}` as a clip")).with_data(
            serde_json::json!({
                "id": clip_id,
                "notes": sequence.notes.len(),
                "ctrls": sequence.ctrls.len(),
                "start": start,
                "length": length,
            }),
        ))
    }
}

/// **Export the MIDI** of a clip to a file.
///
/// The file is a Standard MIDI File at nine hundred and sixty ticks a
/// quarter note — the kit's tick and the MIDI library's alike — and it
/// holds the clip's notes and its controllers.
///
/// A clip that is **not** a MIDI clip is refused: a file of notes is
/// not a file of nothing.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.export",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Export MIDI"),
    mutates,
    undoable
)]
pub struct ExportMidi {
    /// The clip to export.
    pub clip: NodeRef,
    /// Where the file is written.
    pub path: Text,
}

impl ExportMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.export".to_string();
        let clip_id = self.clip.0;
        let path = self.path.0.clone();
        let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
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
        let Some(midi) = payload.midi.clone() else {
            return Err(EngineError::Failed { id, message: format!("the clip `{clip_id}` holds no notes") });
        };
        let file = craftkit_midi::Smf {
            format: 1,
            tracks: vec![craftkit_midi::SmfTrack { name: node.name.clone(), sequence: midi.to_sequence() }],
            ..craftkit_midi::Smf::default()
        };
        let bytes = craftkit_midi::write_smf(&file).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the MIDI of the clip `{clip_id}` cannot be written: {error}"),
        })?;
        std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
            id,
            message: format!("the MIDI cannot be written to `{path}`: {error}"),
        })?;
        Ok(Outcome::changed(format!("wrote the MIDI of the clip `{clip_id}` to `{path}`")).with_data(
            serde_json::json!({
                "clip": clip_id,
                "path": path,
                "notes": midi.notes.len(),
                "ctrls": midi.ctrls.len(),
                "bytes": bytes.len(),
            }),
        ))
    }
}

/// **Read the MIDI** of a clip.
///
/// The notes and the controllers are given back, and so is the **next**
/// place the sequence reaches — the tick the clip would end at.
#[derive(Command)]
#[command(id = "sound.clip.midi.read", area = "sound", layer = "sound", icon = "list")]
pub struct ReadMidi {
    /// The clip to read.
    pub clip: NodeRef,
}

impl ReadMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.read".to_string();
        let clip_id = self.clip.0;
        let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
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
        let Some(midi) = payload.midi.clone() else {
            return Ok(Outcome::new("the clip holds no notes").with_data(serde_json::json!({
                "clip": clip_id,
                "midi": false,
            })));
        };
        Ok(Outcome::new(format!("the clip holds {} notes", midi.notes.len())).with_data(serde_json::json!({
            "clip": clip_id,
            "midi": true,
            "notes": midi.to_json()["notes"],
            "ctrls": midi.to_json()["ctrls"],
            "end_tick": midi.end_tick(),
        })))
    }
}

/// **Quantise the notes** of a clip onto the session's **grid**.
///
/// The grid is the session's (§200) and the lines are musical, so they
/// move with the tempo and follow the meter. Every note is put on the
/// nearest line, and a note that is exactly between two lines goes to
/// the **earlier** one — the same tie the grid breaks everywhere else.
///
/// The notes keep their **length**, and the **length of the clip** is
/// worked out again from the notes: a clip that is as long as its
/// notes need is a clip that says how long it is.
///
/// The notes are read at the tempo of the document, and a note's place
/// is a **fact**: there is no random component here, because a note
/// that is somewhere else each time it is asked is a note nobody can
/// rely on.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.quantize",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Clip", "Quantize MIDI"),
    mutates,
    undoable
)]
pub struct QuantizeMidi {
    /// The clip to quantise.
    pub clip: NodeRef,
}

impl QuantizeMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.quantize".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let settings = session_of(session.document());
        let map = tempo_map(session.document());
        let mut moved = 0;
        for note in midi.notes.iter_mut() {
            // The note's place on the timeline, and the line nearest to
            // it.
            let at = payload.start.saturating_add(map.ticks_to_samples(note.start, settings.sample_rate));
            let line = settings.grid.snap(at, &map, settings.sample_rate);
            let offset = line.saturating_sub(payload.start).max(0);
            let tick = map.samples_to_ticks(offset, settings.sample_rate);
            if tick != note.start {
                note.start = tick;
                moved += 1;
            }
        }
        // The **length** of the clip is derived from the notes.
        let end_tick = midi.end_tick();
        let start_tick = map.samples_to_ticks(payload.start, settings.sample_rate);
        let length = map
            .ticks_to_samples(start_tick.saturating_add(end_tick), settings.sample_rate)
            .saturating_sub(map.ticks_to_samples(start_tick, settings.sample_rate));
        payload.midi = Some(midi);
        payload.length = length.max(1);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("quantised the notes of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "notes": moved, "length": payload.length })))
    }
}

/// **Transpose** the notes of a clip.
///
/// The pitch of a MIDI note is from zero to a hundred and twenty-seven,
/// and a transpose that would take a note out of that range is
/// **refused**: the notes are not changed at all. The tree clamps such
/// a pitch into the range, and a note that was asked for and is not
/// there is a note the caller does not know about.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.transpose",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Clip", "Transpose MIDI"),
    mutates,
    undoable
)]
pub struct TransposeMidi {
    /// The clip to transpose.
    pub clip: NodeRef,
    /// How many semitones, up or down.
    pub semitones: Number,
}

impl TransposeMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.transpose".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        if !self.semitones.0.is_finite() {
            return Err(EngineError::Failed {
                id,
                message: "a transpose is a number of semitones".to_string(),
            });
        }
        let semitones = self.semitones.0.round() as i32;
        let mut sequence = midi.to_sequence();
        craftkit_midi::transpose(&mut sequence.notes, semitones).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!(
                "the note of pitch {} would leave the range with {semitones} semitones",
                error.pitch
            ),
        })?;
        // The notes come back with their new pitches, and the rest of
        // the sequence is as it was.
        for (note, moved) in midi.notes.iter_mut().zip(sequence.notes.iter()) {
            note.pitch = moved.pitch;
        }
        payload.midi = Some(midi);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("transposed the notes of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "semitones": semitones })))
    }
}

/// **Change the velocity** of the notes of a clip.
///
/// The velocity is how hard a note is struck, and it is from one to a
/// hundred and twenty-seven. A velocity outside that range is
/// **refused** and the notes are not changed: the tree clamps such a
/// velocity into the range, and a note asked for as a hundred and fifty
/// is not a note the caller knows about.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.velocity",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Clip", "Velocity"),
    mutates,
    undoable
)]
pub struct VelocityMidi {
    /// The clip to change.
    pub clip: NodeRef,
    /// The velocity every note is struck at.
    pub set: Option<Number>,
    /// The change in the velocity of every note.
    pub delta: Option<Number>,
}

impl VelocityMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.velocity".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let op = match (self.set, self.delta) {
            (Some(set), _) => craftkit_midi::VelocityOp::Set(number_of(&id, set.0, "the velocity")? as u8),
            (None, Some(delta)) => {
                craftkit_midi::VelocityOp::Add(number_of(&id, delta.0, "the change in the velocity")?)
            }
            (None, None) => {
                return Err(EngineError::Failed {
                    id,
                    message: "the velocity, or the change in it, is needed".to_string(),
                });
            }
        };
        let mut sequence = midi.to_sequence();
        craftkit_midi::change_velocity(&mut sequence.notes, &op).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!(
                "the velocity {} is not one a note may have: it is from {} to {}",
                error.velocity,
                craftkit_midi::MIN_VELOCITY,
                craftkit_midi::MAX_VELOCITY
            ),
        })?;
        for (note, moved) in midi.notes.iter_mut().zip(sequence.notes.iter()) {
            note.velocity = moved.velocity;
        }
        payload.midi = Some(midi);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("changed the velocity of the notes of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id })))
    }
}

/// The clip of a command, read.
fn clip_of(session: &Session, id: &str, clip_id: u64) -> Result<ClipPayload, EngineError> {
    let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no node `{clip_id}`"),
    })?;
    if node.kind != "sound.clip" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
        });
    }
    ClipPayload::from_json(&node.data)
        .map_err(|error| EngineError::Failed { id: id.to_string(), message: error })
}

/// Write a clip's payload back into the document.
fn write_clip(session: &mut Session, clip_id: u64, payload: &ClipPayload) -> Result<(), EngineError> {
    if let Some(node) = session.document_mut()?.node_mut(clip_id) {
        node.data = payload.to_json();
    }
    Ok(())
}

/// A whole number from a parameter.
fn number_of(id: &str, value: f64, what: &str) -> Result<i32, EngineError> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(EngineError::Failed { id: id.to_string(), message: format!("{what} is a whole number") });
    }
    if value < i32::MIN as f64 || value > i32::MAX as f64 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is a whole number that fits in a note"),
        });
    }
    Ok(value as i32)
}

/// Reads a MIDI file, within the limits.
fn read_midi_file(id: &str, path: &str) -> Result<craftkit_midi::Smf, EngineError> {
    let metadata = std::fs::metadata(path).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the MIDI file `{path}` cannot be read: {error}"),
    })?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the MIDI file `{path}` is larger than the limits allow"),
        });
    }
    let bytes = std::fs::read(path).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the MIDI file `{path}` cannot be read: {error}"),
    })?;
    craftkit_midi::read_smf(&bytes).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the file `{path}` is not a MIDI file the kit reads: {error}"),
    })
}

/// The name a clip gets from the name of its file.
fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}
