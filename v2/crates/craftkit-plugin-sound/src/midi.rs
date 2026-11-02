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
    pub(crate) notes: Vec<NotePayload>,
    /// The controllers, in the order they were in.
    pub(crate) ctrls: Vec<CtrlPayload>,
}

impl MidiPayload {
    /// The half of the sequence on one side of a **cut**.
    ///
    /// The cut is at a **tick** from the start of the clip, and the
    /// half is named by which side of it is wanted: the first piece
    /// keeps what is **before** the cut, and the second keeps what is
    /// **at or after** it.
    ///
    /// The second piece's notes are **moved back** by the cut, because
    /// they are counted from the second piece's own head: a note that
    /// was a quarter note into the clip is at the head of the piece
    /// that starts a quarter note into it.
    ///
    /// The tree's cut divides the notes the same way — a note is not in
    /// both pieces.
    pub(crate) fn divided(&self, boundary: i64, second: bool) -> MidiPayload {
        let mut notes: Vec<NotePayload> = Vec::new();
        let mut ctrls: Vec<CtrlPayload> = Vec::new();
        for note in &self.notes {
            let after = note.start >= boundary;
            if after != second {
                continue;
            }
            let mut note = note.clone();
            if second {
                note.start = note.start.saturating_sub(boundary);
            }
            notes.push(note);
        }
        for ctrl in &self.ctrls {
            let after = ctrl.tick >= boundary;
            if after != second {
                continue;
            }
            let mut ctrl = ctrl.clone();
            if second {
                ctrl.tick = ctrl.tick.saturating_sub(boundary);
            }
            ctrls.push(ctrl);
        }
        MidiPayload { notes, ctrls }
    }

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

    /// Sorts the notes by their start and their pitch, and the
    /// controllers by their tick.
    ///
    /// The notes of a clip are in order, so that "the third note" is
    /// the third note and not whichever one happens to be there.
    pub(crate) fn sort(&mut self) {
        self.notes.sort_by_key(|note| (note.start, note.pitch, note.channel, note.length, note.velocity));
        self.ctrls.sort_by_key(|ctrl| ctrl.tick);
    }

    /// The tick the last thing of the sequence reaches.
    pub(crate) fn end_tick(&self) -> i64 {
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

/// **Name the notes** of a MIDI clip.
///
/// The tree's MIDI crate carries a table of names: a note is "C3", a
/// drum is "Bass Drum 1" and a program is "Acoustic Grand Piano". The
/// names are here, and this command gives them back with the notes
/// they belong to.
///
/// A note on the **drum channel** — the tenth — is a drum and is
/// named as one, and a note on any other channel is named by its
/// pitch. A pitch outside the drum kit has no drum name, and such a
/// note is named by its pitch instead.
#[derive(Command)]
#[command(id = "sound.clip.midi.names", area = "sound", layer = "sound", icon = "list")]
pub struct ReadMidiNames {
    /// The clip whose notes are named.
    pub clip: NodeRef,
}

impl ReadMidiNames {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.names".to_string();
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
            return Ok(Outcome::new(format!("the clip `{clip_id}` holds no notes"))
                .with_data(serde_json::json!({ "clip": clip_id, "midi": false, "names": [] })));
        };
        // The **names**: the note's own, and the drum's when it is on
        // the drum channel.
        let names: Vec<Json> = midi
            .notes
            .iter()
            .map(|note| {
                let pitch = note.pitch;
                let channel = note.channel;
                serde_json::json!({
                    "pitch": pitch,
                    "channel": channel,
                    "start": note.start,
                    "length": note.length,
                    "velocity": note.velocity,
                    "name": craftkit_midi::names::note_or_drum_name(pitch, channel),
                })
            })
            .collect();
        // The **programs** the clip names, when it has any: a program
        // change is a program, and a program has a name.
        let programs: Vec<Json> = midi
            .ctrls
            .iter()
            .filter(|ctrl| ctrl.kind == "program")
            .map(|ctrl| {
                let number = ctrl.number.unwrap_or(0);
                serde_json::json!({
                    "tick": ctrl.tick,
                    "channel": ctrl.channel,
                    "program": number,
                    "name": craftkit_midi::names::gm_program_name(number),
                })
            })
            .collect();
        Ok(Outcome::new(format!("the clip holds {} notes", names.len())).with_data(serde_json::json!({
            "clip": clip_id,
            "midi": true,
            "names": names,
            "programs": programs,
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
/// not shortened: the room the notes had is the room they have, and a
/// note that reaches beyond the clip makes the clip longer.
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
        // The **length** of the clip: the room its notes need, and the
        // room it has. The room does not shrink because the notes
        // moved.
        payload.midi = Some(midi);
        let length = crate::midi_length(
            payload.length,
            payload.midi.as_ref().unwrap_or(&MidiPayload::default()),
            payload.start,
            &map,
            settings.sample_rate,
        );
        payload.length = length;
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
///
/// The **limit** is the one way a velocity is moved into the range on
/// purpose: a limit says *no softer than this and no harder than
/// that*, so a note outside the limit is moved to its edge. The bounds
/// of the limit are still refused when they are outside the range,
/// because a limit of zero is not a limit of anything.
///
/// The tree's command has the same four ways: set, add, scale and
/// limit. They are tried in that order, and the first one that is
/// given is the one that is done.
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
    /// The factor every note's velocity is multiplied by.
    pub scale: Option<Number>,
    /// The softest a note may be struck: with `max`, it is the limit.
    pub min: Option<Number>,
    /// The hardest a note may be struck: with `min`, it is the limit.
    pub max: Option<Number>,
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
        // The ways the velocity is changed, and how many of them were
        // asked for: the tree's command takes them in the order set,
        // add, scale, limit, and the first one that is given is the
        // one that is done — but asking for two of them at once is
        // refused here, because a caller that meant one of them and
        // got the other would not know.
        let ways = usize::from(self.set.is_some())
            + usize::from(self.delta.is_some())
            + usize::from(self.scale.is_some())
            + usize::from(self.min.is_some() || self.max.is_some());
        if ways > 1 {
            return Err(EngineError::Failed {
                id,
                message: "the velocity is changed one way at a time: set, delta, scale or limit".to_string(),
            });
        }
        let op = match (self.set, self.delta, self.scale, self.min, self.max) {
            (Some(set), _, _, _, _) => {
                craftkit_midi::VelocityOp::Set(number_of(&id, set.0, "the velocity")? as u8)
            }
            (None, Some(delta), _, _, _) => {
                craftkit_midi::VelocityOp::Add(number_of(&id, delta.0, "the change in the velocity")?)
            }
            (None, None, Some(scale), _, _) => {
                if !scale.0.is_finite() {
                    return Err(EngineError::Failed { id, message: "the factor is a number".to_string() });
                }
                craftkit_midi::VelocityOp::Scale(scale.0)
            }
            (None, None, None, Some(min), Some(max)) => {
                let min = number_of(&id, min.0, "the softest a note may be struck")?;
                let max = number_of(&id, max.0, "the hardest a note may be struck")?;
                if !(0..=255).contains(&min) || !(0..=255).contains(&max) {
                    return Err(EngineError::Failed {
                        id,
                        message: format!(
                            "a limit is from {} to {}, and not {min} to {max}",
                            craftkit_midi::MIN_VELOCITY,
                            craftkit_midi::MAX_VELOCITY
                        ),
                    });
                }
                craftkit_midi::VelocityOp::Limit { min: min as u8, max: max as u8 }
            }
            (None, None, None, Some(_), None) | (None, None, None, None, Some(_)) => {
                return Err(EngineError::Failed {
                    id,
                    message: "a limit has two bounds: the softest and the hardest".to_string(),
                });
            }
            _ => {
                return Err(EngineError::Failed {
                    id,
                    message: "the velocity, the change in it, the factor, or the limit is needed".to_string(),
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

/// **Change the length** of the notes of a clip.
///
/// A note is always at least one tick long: a note of no length is a
/// note that cannot be heard, and that is a **shape** and not a bound
/// — so a length of zero is a length of one tick, the way the tree
/// makes it.
///
/// The tree's command changes a note's length four ways: it sets it,
/// adds to it, scales it and makes it **legato** — every note ends a
/// gap of ticks before the next note starts, and the last note keeps
/// its length. They are tried in that order, and here they are tried
/// **one at a time**: asking for two of them at once is refused,
/// because a caller that meant one of them and got the other would
/// not know.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.duration",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Clip", "Duration"),
    mutates,
    undoable
)]
pub struct DurationMidi {
    /// The clip to change.
    pub clip: NodeRef,
    /// The length every note is given, in ticks.
    pub set: Option<Number>,
    /// The change in the length of every note, in ticks.
    pub delta: Option<Number>,
    /// The factor every note's length is multiplied by.
    pub scale: Option<Number>,
    /// The gap of ticks before the next note, for a legato.
    pub gap: Option<Number>,
}

impl DurationMidi {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.duration".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        // The ways the length is changed, and how many of them were
        // asked for.
        let ways = usize::from(self.set.is_some())
            + usize::from(self.delta.is_some())
            + usize::from(self.scale.is_some())
            + usize::from(self.gap.is_some());
        if ways > 1 {
            return Err(EngineError::Failed {
                id,
                message: "the length is changed one way at a time: set, delta, scale or legato".to_string(),
            });
        }
        let op = match (self.set, self.delta, self.scale, self.gap) {
            (Some(set), _, _, _) => {
                let length = whole_of(&id, set, "the length of a note", i64::MIN as f64, i64::MAX as f64)?;
                craftkit_midi::DurationOp::Set(length)
            }
            (None, Some(delta), _, _) => {
                let delta = whole_of(
                    &id,
                    delta,
                    "the change in the length of a note",
                    i64::MIN as f64,
                    i64::MAX as f64,
                )?;
                craftkit_midi::DurationOp::Add(delta)
            }
            (None, None, Some(scale), _) => {
                if !scale.0.is_finite() || scale.0 < 0.0 {
                    return Err(EngineError::Failed {
                        id,
                        message: "the factor is a number that is not negative".to_string(),
                    });
                }
                craftkit_midi::DurationOp::Scale(scale.0)
            }
            (None, None, None, Some(gap)) => {
                let gap = whole_of(&id, gap, "the gap before the next note", 0.0, i64::MAX as f64)?;
                craftkit_midi::DurationOp::Legato { gap }
            }
            _ => {
                return Err(EngineError::Failed {
                    id,
                    message: "the length, the change in it, the factor, or the gap is needed".to_string(),
                });
            }
        };
        let mut sequence = midi.to_sequence();
        craftkit_midi::change_duration(&mut sequence.notes, &op);
        for (note, moved) in midi.notes.iter_mut().zip(sequence.notes.iter()) {
            note.length = moved.length;
        }
        payload.midi = Some(midi);
        // The **length** of the clip: the room its notes need, and the
        // room it has. The room does not shrink because the notes moved.
        let settings = session_of(session.document());
        let length = crate::midi_length(
            payload.length,
            payload.midi.as_ref().unwrap_or(&MidiPayload::default()),
            payload.start,
            &tempo_map(session.document()),
            settings.sample_rate,
        );
        payload.length = length;
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("changed the length of the notes of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "length": payload.length })))
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

/// **Make a MIDI clip** with no notes in it.
///
/// The tree's editor makes a MIDI clip when a person asks for one on a
/// track, and the clip is a bar long unless the selection says
/// otherwise. Here the clip is a bar long at the **document's tempo**
/// unless the caller says how long it is, and it holds no notes: the
/// notes are added afterwards, or imported.
///
/// The **content** of the clip is `midi`, and its **length** is what
/// it was given — a clip with no notes has nothing to derive its
/// length from.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.add",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "New MIDI Clip"),
    mutates,
    undoable
)]
pub struct AddMidiClip {
    /// The track the clip goes on.
    pub track: NodeRef,
    /// Where the clip starts, in samples.
    pub start: Number,
    /// How long the clip is, in samples. A bar at the document's tempo
    /// without it.
    pub length: Option<Number>,
}

impl AddMidiClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.add".to_string();
        let start = samples(self.start.0, "where the clip starts")?;
        let settings = session_of(session.document());
        let map = tempo_map(session.document());
        // The length: a bar at the document's tempo, unless the caller
        // says how long the clip is.
        let length = match self.length {
            Some(length) => samples(length.0, "how long the clip is")?.max(1),
            None => {
                let tick = map.samples_to_ticks(start, settings.sample_rate);
                let (numerator, denominator) = map.meter_at_tick(tick);
                let beat = craftkit_sound_time::ticks_per_beat(denominator.max(1));
                let bar = beat * i64::from(numerator.max(1));
                map.ticks_to_samples(tick.saturating_add(bar), settings.sample_rate)
                    .saturating_sub(map.ticks_to_samples(tick, settings.sample_rate))
                    .max(1)
            }
        };
        let playlist = crate::playlist_of(session, self.track.0, None)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: format!("{error}") })?;
        let mut clip = ClipPayload::new();
        clip.start = start;
        clip.length = length;
        clip.content = Some(CONTENT_MIDI.to_string());
        clip.midi = Some(MidiPayload::default());
        let clip_id = session.document_mut()?.add_node_with_under(
            Some(playlist),
            "MIDI".to_string(),
            "sound.clip",
            None,
            clip.to_json(),
        );
        Ok(Outcome::changed(format!("made a MIDI clip of {length} samples"))
            .with_data(serde_json::json!({ "id": clip_id, "start": start, "length": length })))
    }
}

/// **Add a note** to a MIDI clip.
///
/// The **pitch**, the **velocity** and the **channel** are refused when
/// they are outside the range a MIDI note has: the tree clamps them
/// into the range, and a note asked for as a hundred and forty is not a
/// note the caller knows about.
///
/// The **length** of a note is at least one tick, and the note starts
/// at or after the beginning of the clip.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.note.add",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Add Note"),
    mutates,
    undoable
)]
pub struct AddMidiNote {
    /// The clip the note goes in.
    pub clip: NodeRef,
    /// The pitch: zero to a hundred and twenty-seven.
    pub pitch: Number,
    /// Where the note starts, in ticks from the start of the clip.
    pub start: Number,
    /// How long the note is, in ticks. Half a quarter note without it.
    pub length: Option<Number>,
    /// How hard the note is struck: one to a hundred and twenty-seven.
    pub velocity: Option<Number>,
    /// The channel the note is on: zero to fifteen.
    pub channel: Option<Number>,
}

impl AddMidiNote {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.note.add".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let pitch = whole_of(&id, self.pitch, "the pitch", 0.0, 127.0)? as u8;
        let start = whole_of(&id, self.start, "where the note starts", 0.0, i64::MAX as f64)?;
        let length = match self.length {
            Some(length) => whole_of(&id, length, "how long the note is", 1.0, i64::MAX as f64)?,
            None => 480,
        };
        let velocity = match self.velocity {
            Some(velocity) => whole_of(&id, velocity, "the velocity", 1.0, 127.0)? as u8,
            None => 100,
        };
        let channel = match self.channel {
            Some(channel) => whole_of(&id, channel, "the channel", 0.0, 15.0)? as u8,
            None => 0,
        };
        midi.notes.push(NotePayload { pitch, velocity, channel, start, length });
        midi.sort();
        let index = midi.notes.iter().position(|note| {
            note.pitch == pitch && note.start == start && note.channel == channel && note.velocity == velocity
        });
        payload.midi = Some(midi);
        let length = crate::midi_length(
            payload.length,
            payload.midi.as_ref().unwrap_or(&MidiPayload::default()),
            payload.start,
            &tempo_map(session.document()),
            session_of(session.document()).sample_rate,
        );
        payload.length = length;
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("added a note to the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "index": index, "length": length })))
    }
}

/// **Edit a note** of a MIDI clip.
///
/// The note is named by its **index** in the clip's notes, which are in
/// order of their start. An index that names no note is **refused**.
///
/// The pitch, the velocity and the length are refused when they are
/// outside the range a MIDI note has — the tree clamps them, and a note
/// asked for as a hundred and forty is not a note the caller knows
/// about.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.note.edit",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Edit Note"),
    mutates,
    undoable
)]
pub struct EditMidiNote {
    /// The clip the note is in.
    pub clip: NodeRef,
    /// Which note: its index in the clip's notes.
    pub index: Number,
    /// The new pitch.
    pub pitch: Option<Number>,
    /// The new start, in ticks.
    pub start: Option<Number>,
    /// The new length, in ticks.
    pub length: Option<Number>,
    /// The new velocity.
    pub velocity: Option<Number>,
}

impl EditMidiNote {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.note.edit".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let index = whole_of(&id, self.index, "which note", 0.0, i64::MAX as f64)? as usize;
        let Some(note) = midi.notes.get(index) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the clip `{clip_id}` has no note `{index}`"),
            });
        };
        let mut note = note.clone();
        if let Some(pitch) = self.pitch {
            note.pitch = whole_of(&id, pitch, "the pitch", 0.0, 127.0)? as u8;
        }
        if let Some(start) = self.start {
            note.start = whole_of(&id, start, "where the note starts", 0.0, i64::MAX as f64)?;
        }
        if let Some(length) = self.length {
            note.length = whole_of(&id, length, "how long the note is", 1.0, i64::MAX as f64)?;
        }
        if let Some(velocity) = self.velocity {
            note.velocity = whole_of(&id, velocity, "the velocity", 1.0, 127.0)? as u8;
        }
        if let Some(slot) = midi.notes.get_mut(index) {
            *slot = note;
        }
        midi.sort();
        payload.midi = Some(midi);
        let length = crate::midi_length(
            payload.length,
            payload.midi.as_ref().unwrap_or(&MidiPayload::default()),
            payload.start,
            &tempo_map(session.document()),
            session_of(session.document()).sample_rate,
        );
        payload.length = length;
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("edited a note of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "index": index, "length": length })))
    }
}

/// **Remove notes** from a MIDI clip.
///
/// The notes are named by their **indices**, and an index that names no
/// note is **refused**: a note that is not there is not removed.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.note.remove",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Remove Note"),
    mutates,
    undoable
)]
pub struct RemoveMidiNotes {
    /// The clip the notes are in.
    pub clip: NodeRef,
    /// Which notes: their indices in the clip's notes.
    pub indices: serde_json::Value,
}

impl RemoveMidiNotes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.note.remove".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let indices = crate::clip_ids_of(&self.indices, &id)?;
        for index in &indices {
            if *index as usize >= midi.notes.len() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` has no note `{index}`"),
                });
            }
        }
        let before = midi.notes.len();
        midi.notes.retain(|_| true);
        let mut kept: Vec<NotePayload> = Vec::with_capacity(before);
        for (index, note) in midi.notes.iter().enumerate() {
            if !indices.contains(&(index as u64)) {
                kept.push(note.clone());
            }
        }
        midi.notes = kept;
        payload.midi = Some(midi);
        let length = crate::midi_length(
            payload.length,
            payload.midi.as_ref().unwrap_or(&MidiPayload::default()),
            payload.start,
            &tempo_map(session.document()),
            session_of(session.document()).sample_rate,
        );
        payload.length = length;
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("removed notes from the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "removed": before - payload.midi.as_ref().map(|midi| midi.notes.len()).unwrap_or(0) })))
    }
}

/// **Move notes** of a MIDI clip.
///
/// The notes move **later** or **earlier** by so many ticks, and **up**
/// or **down** by so many semitones. A note that would start before the
/// beginning of the clip, or that would leave the range of a MIDI
/// pitch, is **refused** and no note moves: the tree clamps both, and
/// a note the caller did not ask for is not a note the caller knows
/// about.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.notes.move",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Move Notes"),
    mutates,
    undoable
)]
pub struct MoveMidiNotes {
    /// The clip the notes are in.
    pub clip: NodeRef,
    /// Which notes: their indices in the clip's notes.
    pub indices: serde_json::Value,
    /// How much later or earlier, in ticks.
    pub ticks: Option<Number>,
    /// How much higher or lower, in semitones.
    pub semitones: Option<Number>,
}

impl MoveMidiNotes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.notes.move".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let indices = crate::clip_ids_of(&self.indices, &id)?;
        for index in &indices {
            if *index as usize >= midi.notes.len() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` has no note `{index}`"),
                });
            }
        }
        let ticks = match self.ticks {
            Some(ticks) => whole_of(&id, ticks, "how far the notes move", i64::MIN as f64, i64::MAX as f64)?,
            None => 0,
        };
        let semitones = match self.semitones {
            Some(semitones) => {
                whole_of(&id, semitones, "how far the notes move", i64::MIN as f64, i64::MAX as f64)? as i32
            }
            None => 0,
        };
        // The **bounds** are checked before anything moves: a note that
        // would leave the range is refused, and no note moves.
        for index in &indices {
            let Some(note) = midi.notes.get(*index as usize) else { continue };
            let start = note.start.saturating_add(ticks);
            if start < 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a note would start before the beginning of the clip".to_string(),
                });
            }
            let pitch = i32::from(note.pitch).saturating_add(semitones);
            if !(0..=127).contains(&pitch) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the note of pitch {} would leave the range", note.pitch),
                });
            }
        }
        for index in &indices {
            if let Some(note) = midi.notes.get_mut(*index as usize) {
                note.start = note.start.saturating_add(ticks);
                note.pitch = (i32::from(note.pitch) + semitones) as u8;
            }
        }
        midi.sort();
        payload.midi = Some(midi);
        let length = crate::midi_length(
            payload.length,
            payload.midi.as_ref().unwrap_or(&MidiPayload::default()),
            payload.start,
            &tempo_map(session.document()),
            session_of(session.document()).sample_rate,
        );
        payload.length = length;
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("moved notes of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "moved": indices.len(), "length": length })))
    }
}

/// **Add a controller** to a MIDI clip.
///
/// The **kind** says what it changes: a `control` change, a
/// `pitch_bend`, a `channel_pressure`, a `poly_pressure` or a
/// `program`. A kind that is not one of them is **refused**: a
/// controller that does nothing is worse than no controller at all.
///
/// The two kinds that change a **number** — the control change and the
/// pressure of one pitch — need one, and the others must not have one:
/// a number that nothing reads is not a fact.
///
/// The **value** is in the range of its kind: a control change, a
/// pressure and a program are from zero to a hundred and twenty-seven,
/// and the pitch bend is centred on zero, from minus eight thousand
/// one hundred and ninety-two to eight thousand one hundred and
/// ninety-one.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.ctrl.add",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Add Controller"),
    mutates,
    undoable
)]
pub struct AddMidiCtrl {
    /// The clip the controller goes in.
    pub clip: NodeRef,
    /// What it changes: `control`, `pitch_bend`, `channel_pressure`,
    /// `poly_pressure` or `program`.
    pub kind: Text,
    /// Where it happens, in ticks from the start of the clip.
    pub at: Number,
    /// The value it changes it to.
    pub value: Number,
    /// The number it changes, for the kinds that have one.
    pub number: Option<Number>,
    /// The channel it happens on: zero to fifteen.
    pub channel: Option<Number>,
}

impl AddMidiCtrl {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.ctrl.add".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let ctrl = ctrl_of(&id, self.kind.0.trim(), self.at, self.value, self.number, self.channel)?;
        midi.ctrls.push(ctrl);
        midi.sort();
        payload.midi = Some(midi);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("added a controller to the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "ctrls": payload.midi.as_ref().map(|midi| midi.ctrls.len()).unwrap_or(0) })))
    }
}

/// **Edit a controller** of a MIDI clip.
///
/// The controller is named by its **index** in the clip's controllers,
/// which are in order of their tick, and an index that names no
/// controller is **refused**.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.ctrl.edit",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Edit Controller"),
    mutates,
    undoable
)]
pub struct EditMidiCtrl {
    /// The clip the controller is in.
    pub clip: NodeRef,
    /// Which controller: its index in the clip's controllers.
    pub index: Number,
    /// What it changes.
    pub kind: Option<Text>,
    /// Where it happens, in ticks.
    pub at: Option<Number>,
    /// The value it changes it to.
    pub value: Option<Number>,
    /// The number it changes, for the kinds that have one.
    pub number: Option<Number>,
    /// The channel it happens on.
    pub channel: Option<Number>,
}

impl EditMidiCtrl {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.ctrl.edit".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let index = whole_of(&id, self.index, "which controller", 0.0, i64::MAX as f64)? as usize;
        let Some(ctrl) = midi.ctrls.get(index) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the clip `{clip_id}` has no controller `{index}`"),
            });
        };
        let kind = self.kind.as_ref().map(|kind| kind.0.trim().to_string()).unwrap_or(ctrl.kind.clone());
        let number = match self.number {
            Some(number) => Some(whole_of(&id, number, "the number", 0.0, 127.0)? as u8),
            None => ctrl.number,
        };
        // The **kind** decides whether the number means anything, and
        // the value is in the range of the kind.
        let edited = ctrl_of(
            &id,
            &kind,
            match self.at {
                Some(at) => Number(at.0),
                None => Number(ctrl.tick as f64),
            },
            match self.value {
                Some(value) => value,
                None => Number(ctrl.value as f64),
            },
            number.map(|number| Number(f64::from(number))),
            self.channel.or(Some(Number(ctrl.channel as f64))),
        )?;
        if let Some(slot) = midi.ctrls.get_mut(index) {
            *slot = edited;
        }
        midi.sort();
        payload.midi = Some(midi);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("edited a controller of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "index": index })))
    }
}

/// **Take controllers** off a MIDI clip.
///
/// The controllers are named by their **indices**, and an index that
/// names no controller is **refused**.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.ctrl.remove",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Remove Controller"),
    mutates,
    undoable
)]
pub struct RemoveMidiCtrls {
    /// The clip the controllers are in.
    pub clip: NodeRef,
    /// Which controllers: their indices in the clip's controllers.
    pub indices: serde_json::Value,
}

impl RemoveMidiCtrls {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.ctrl.remove".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let indices = crate::clip_ids_of(&self.indices, &id)?;
        for index in &indices {
            if *index as usize >= midi.ctrls.len() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` has no controller `{index}`"),
                });
            }
        }
        let before = midi.ctrls.len();
        let mut kept: Vec<CtrlPayload> = Vec::with_capacity(before);
        for (index, ctrl) in midi.ctrls.iter().enumerate() {
            if !indices.contains(&(index as u64)) {
                kept.push(ctrl.clone());
            }
        }
        midi.ctrls = kept;
        let left = midi.ctrls.len();
        payload.midi = Some(midi);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("removed controllers from the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "removed": before - left })))
    }
}

/// **Take the duplicate notes** out of a MIDI clip.
///
/// Two notes of the same channel, pitch and start are the same note,
/// and the **longer** one is kept: a note that was struck twice is one
/// note that was held. Two controllers of the same kind, channel and
/// tick are the same controller, and the **last** one is kept, because
/// that is the one a synthesiser would end up with.
#[derive(Command)]
#[command(
    id = "sound.clip.midi.notes.dedupe",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Remove Duplicate Notes"),
    mutates,
    undoable
)]
pub struct DedupeMidiNotes {
    /// The clip to tidy.
    pub clip: NodeRef,
}

impl DedupeMidiNotes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.midi.notes.dedupe".to_string();
        let clip_id = self.clip.0;
        let mut payload = clip_of(session, &id, clip_id)?;
        let mut midi = payload.midi.clone().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip_id}` holds no notes"),
        })?;
        let mut sequence = midi.to_sequence();
        let removed = sequence.remove_duplicates();
        let tidied = MidiPayload::from_sequence(&sequence);
        midi.notes = tidied.notes;
        midi.ctrls = tidied.ctrls;
        midi.sort();
        payload.midi = Some(midi);
        write_clip(session, clip_id, &payload)?;
        Ok(Outcome::changed(format!("took {removed} duplicates out of the clip `{clip_id}`"))
            .with_data(serde_json::json!({ "clip": clip_id, "removed": removed })))
    }
}

/// A controller, from the parameters of a command.
///
/// The kind says what it changes, the number is for the kinds that
/// have one, and the value is in the range of the kind. Everything
/// outside those ranges is **refused**.
fn ctrl_of(
    id: &str,
    kind: &str,
    at: Number,
    value: Number,
    number: Option<Number>,
    channel: Option<Number>,
) -> Result<CtrlPayload, EngineError> {
    if !CTRL_KINDS.contains(&kind) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no controller called `{kind}`"),
        });
    }
    let wants_number = matches!(kind, "control" | "poly_pressure");
    let number = match (wants_number, number) {
        (true, Some(number)) => Some(whole_of(id, number, "the number", 0.0, 127.0)? as u8),
        (true, None) => {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("a {kind} changes a number, and it needs one"),
            });
        }
        (false, Some(_)) => {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("a {kind} changes no number"),
            });
        }
        (false, None) => None,
    };
    let tick = whole_of(id, at, "where the controller happens", 0.0, i64::MAX as f64)?;
    let value = if kind == "pitch_bend" {
        whole_of(id, value, "the value", -8192.0, 8191.0)?
    } else {
        whole_of(id, value, "the value", 0.0, 127.0)?
    };
    let channel = match channel {
        Some(channel) => whole_of(id, channel, "the channel", 0.0, 15.0)? as u8,
        None => 0,
    };
    Ok(CtrlPayload { tick, channel, kind: kind.to_string(), number, value: value as i32 })
}

/// A whole number from a parameter, refused when it is not one.
fn whole_of(id: &str, value: Number, what: &str, from: f64, to: f64) -> Result<i64, EngineError> {
    let number = value.0;
    if !number.is_finite() || number.fract() != 0.0 || number < from || number > to {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is a whole number from {from} to {to}, and {number} is not one"),
        });
    }
    Ok(number as i64)
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
