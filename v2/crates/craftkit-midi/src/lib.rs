//! `craftkit-midi` — the **MIDI library**: notes, controllers and the
//! Standard MIDI File.
//!
//! # What this is
//!
//! MIDI is a **sequence of events on a timeline**. The timeline is
//! counted in **ticks**, and this crate counts them at **960 a quarter
//! note** — the resolution the Standard MIDI File specification calls
//! the usual one, and the resolution every file is converted to when
//! it is read.
//!
//! There are two kinds of event:
//!
//! - a **note**: a pitch, a velocity, a channel, and where it starts
//!   and how long it is;
//! - a **controller**: a change of something that is not a pitch —
//!   volume, pan, pitch bend, program change, pressure.
//!
//! A **sequence** is the notes and the controllers of one thing, and a
//! **file** is a header and a list of tracks, each with a sequence of
//! its own.
//!
//! # Why it is a library and not a plugin
//!
//! Because a file is a file: the reader reads bytes and gives events,
//! and it knows nothing about a document, a canvas or the kit. A
//! battery that plays MIDI reads the file with this and puts the notes
//! in the document beside it.
//!
//! # The file is hostile
//!
//! A file came from somewhere else, so every length in it is checked,
//! every variable-length quantity is **bounded** — the specification
//! allows four bytes and no more — and the number of events and the
//! number of tracks are **capped**. A file that says it holds five
//! million events is not read, because the reading would be the
//! problem, not the file.
//!
//! # What was found
//!
//! **The tree's MIDI is a crate of a model, a file, an operations
//! module and a set of names.** The model and the file are here, and
//! the operations are here too — the quantise, the transpose, the
//! velocity and the length — and the **names** are not: a name is for
//! a person to read, and nothing here is read by a person yet.
//!
//! **The tree's transpose clamps and this one refuses.** A pitch of a
//! MIDI note is from zero to a hundred and twenty-seven, and the tree
//! pushes a note that would leave the range onto the end of it. Here
//! the transpose is refused and no note is moved: a note that was asked
//! for and is not there is a note the caller does not know about.
//!
//! **The tree's quantise is random and this one is not.** The tree's
//! quantise may move a note by a random fraction of a step, so that
//! the same quantise twice does not give the same notes. Here a note's
//! place is a fact, and a fact that changes each time it is asked for
//! is not a fact.
//!
//! **The tree's humanise is random and this one is not.** The tree's
//! humanise moves every note by a random amount, and the tree's
//! velocity therefore has a **humanise** of its own — which is not
//! here, for the same reason the random quantise is not: a note struck
//! harder by a different amount each time is not a note the caller
//! knows about. The tree's **limit** and **ramp** are here, because
//! both of them say the same thing every time they are asked.
//!
//! # What this does not do
//!
//! - **It does not make a sound.** There is no synthesiser here: the
//!   notes are numbers and the numbers do not sing.
//! - **It does not play.** The operations here move notes, change how
//!   hard they are struck and how long they last; they do not make a
//!   sound of them, and there is no synthesiser anywhere in the kit.
//! - **It does not keep the file's extras.** A MIDI file may carry a
//!   key signature, a marker, a tempo or a meter; the reader keeps
//!   those it understands, and the writer writes them back.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod ops;
mod smf;

pub use ops::{
    DurationOp, MAX_VELOCITY, MIN_VELOCITY, QuantizeOptions, TransposeError, VelocityError, VelocityOp,
    change_duration, change_velocity, quantize, quantize_sequence, split_by_pitch, transpose,
};
pub use smf::{MAX_EVENTS, MAX_TRACKS, Smf, SmfTrack, read_smf, write_smf};

/// Ticks a quarter note lasts, in this crate's ticks.
///
/// Nine hundred and sixty is the resolution the Standard MIDI File
/// specification calls the usual one, and every file is converted to
/// it when it is read — so a tick is a tick everywhere, whatever the
/// file said.
pub const PPQ: u32 = 960;

/// Why a file was **refused**.
///
/// Every refusal has a name, because a refusal that says nothing is a
/// refusal the caller cannot do anything about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file does not begin with the header a MIDI file begins with.
    NotSmf,
    /// The file ends in the middle of something.
    Truncated {
        /// The byte the file ran out at.
        offset: usize,
    },
    /// A number in the file is longer than the specification allows.
    ///
    /// A variable-length quantity is at most four bytes — twenty-eight
    /// bits — and a longer one is not a number the format has.
    NumberTooLong {
        /// The byte the number began at.
        offset: usize,
    },
    /// A chunk says it is longer than the file.
    ChunkTooLong {
        /// The byte the chunk began at.
        offset: usize,
        /// The length the chunk claims.
        len: u64,
    },
    /// The file says something that cannot be true.
    Invalid {
        /// The byte the file was at.
        offset: usize,
        /// What was wrong with it.
        reason: &'static str,
    },
    /// The file is a MIDI file of a kind this does not read.
    Unsupported {
        /// What the file is.
        reason: &'static str,
    },
    /// The file holds more events than the limits allow.
    TooManyEvents {
        /// The limit.
        limit: usize,
    },
    /// The file holds more tracks than the limits allow.
    TooManyTracks {
        /// The limit.
        limit: usize,
    },
    /// The file cannot be written.
    ///
    /// A file is written from facts that may be wrong — a tempo of
    /// zero, a meter whose denominator is not a power of two — and the
    /// writer says so rather than writing a file that means something
    /// else.
    CannotWrite {
        /// What was wrong with the fact.
        reason: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotSmf => write!(f, "the file is not a Standard MIDI File"),
            Error::Truncated { offset } => {
                write!(f, "the file ends in the middle of something at byte {offset}")
            }
            Error::NumberTooLong { offset } => {
                write!(f, "a number at byte {offset} is longer than the format allows")
            }
            Error::ChunkTooLong { offset, len } => {
                write!(f, "a chunk at byte {offset} says it holds {len} bytes and the file does not")
            }
            Error::Invalid { offset, reason } => {
                write!(f, "the file is not a MIDI file at byte {offset}: {reason}")
            }
            Error::Unsupported { reason } => {
                write!(f, "the file is a MIDI file this does not read: {reason}")
            }
            Error::TooManyEvents { limit } => write!(f, "the file holds more than {limit} events"),
            Error::TooManyTracks { limit } => write!(f, "the file holds more than {limit} tracks"),
            Error::CannotWrite { reason } => write!(f, "the file cannot be written: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

/// A **note**.
///
/// A pitch from zero to a hundred and twenty-seven, a velocity, a
/// channel, and where it starts and how long it is. The times are in
/// ticks, and a note is always longer than nothing: a note of no
/// length is a note that cannot be heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    /// The pitch: zero is the lowest a MIDI note can be, and a hundred
    /// and twenty-seven the highest.
    pub pitch: u8,
    /// How hard the note is struck, from one to a hundred and
    /// twenty-seven.
    pub velocity: u8,
    /// How hard the note is let go, from zero to a hundred and
    /// twenty-seven.
    pub release_velocity: u8,
    /// The channel the note is on, from zero to fifteen.
    pub channel: u8,
    /// Where it starts, in ticks.
    pub start: i64,
    /// How long it is, in ticks. Never less than one.
    pub length: i64,
}

impl Note {
    /// The tick at which the note ends.
    pub fn end(&self) -> i64 {
        self.start.saturating_add(self.length)
    }
}

/// What a **controller** changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtrlKind {
    /// A control change: the number says which control, from zero to a
    /// hundred and twenty-seven.
    Control(u8),
    /// The pitch bend, which is centred on zero.
    PitchBend,
    /// The pressure of the whole channel.
    ChannelPressure,
    /// The pressure of one pitch.
    PolyPressure(u8),
    /// The program — the patch — the channel plays.
    Program,
}

/// A **controller** at a tick.
///
/// A controller is a change of something that is not a pitch: the
/// volume, the pan, the pitch bend, the program, the pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CtrlEvent {
    /// Where it happens, in ticks.
    pub tick: i64,
    /// The channel it happens on, from zero to fifteen.
    pub channel: u8,
    /// What it changes.
    pub kind: CtrlKind,
    /// The value it changes it to.
    ///
    /// A control change is from zero to a hundred and twenty-seven; the
    /// pitch bend is centred on zero and goes from minus eight thousand
    /// one hundred and ninety-two to eight thousand one hundred and
    /// ninety-one.
    pub value: i32,
}

/// The **order** two controllers of the same kind are told apart by.
///
/// Two controllers of the same kind on the same channel at the same
/// tick are the same controller, and the last one is the one that is
/// heard — so a duplicate is replaced and not kept.
fn kind_key(kind: CtrlKind) -> (u8, u8) {
    match kind {
        CtrlKind::Control(number) => (0, number),
        CtrlKind::PitchBend => (1, 0),
        CtrlKind::ChannelPressure => (2, 0),
        CtrlKind::PolyPressure(pitch) => (3, pitch),
        CtrlKind::Program => (4, 0),
    }
}

/// The MIDI content of one thing: its notes and its controllers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sequence {
    /// The notes, sorted by their start and their pitch.
    pub notes: Vec<Note>,
    /// The controllers, sorted by their tick.
    pub ctrls: Vec<CtrlEvent>,
}

impl Sequence {
    /// Sorts the notes and the controllers.
    ///
    /// The notes go by their start, then their pitch, then their
    /// channel, their length and their velocity; the controllers go by
    /// their tick, and two controllers at one tick keep the order they
    /// came in.
    pub fn sort(&mut self) {
        self.notes.sort_by_key(|note| (note.start, note.pitch, note.channel, note.length, note.velocity));
        self.ctrls.sort_by_key(|ctrl| ctrl.tick);
    }

    /// The last tick anything reaches.
    ///
    /// It is the end of the last note and the tick of the last
    /// controller, whichever is later, and nothing when there is
    /// nothing.
    pub fn end_tick(&self) -> i64 {
        let note = self.notes.iter().map(Note::end).max();
        let ctrl = self.ctrls.iter().map(|ctrl| ctrl.tick).max();
        match (note, ctrl) {
            (Some(note), Some(ctrl)) => note.max(ctrl),
            (Some(note), None) | (None, Some(note)) => note,
            (None, None) => 0,
        }
    }

    /// The notes that **start** in `[start, end)`.
    pub fn notes_in(&self, start: i64, end: i64) -> impl Iterator<Item = &Note> + '_ {
        self.notes.iter().filter(move |note| note.start >= start && note.start < end)
    }

    /// The notes that sound **anywhere** in `[start, end)`.
    pub fn notes_overlapping(&self, start: i64, end: i64) -> impl Iterator<Item = &Note> + '_ {
        self.notes.iter().filter(move |note| note.start < end && note.end() > start)
    }

    /// Takes the **duplicates** out and says how many went.
    ///
    /// Two notes of the same channel, pitch and start are the same
    /// note, and the **longer** one is kept — a note that was struck
    /// twice is one note that was held. Two controllers of the same
    /// kind, channel and tick are the same controller, and the **last**
    /// one is kept, because that is the one a synthesiser would end
    /// up with.
    pub fn remove_duplicates(&mut self) -> usize {
        let before = self.notes.len() + self.ctrls.len();
        // The longest first within each group of notes, and then the
        // first of each group is the one kept.
        self.notes.sort_by(|later, kept| {
            (later.start, later.pitch, later.channel, kept.length).cmp(&(
                kept.start,
                kept.pitch,
                kept.channel,
                later.length,
            ))
        });
        self.notes.dedup_by(|later, kept| {
            later.start == kept.start && later.pitch == kept.pitch && later.channel == kept.channel
        });
        self.ctrls.sort_by_key(|ctrl| ctrl.tick);
        let mut kept: Vec<CtrlEvent> = Vec::with_capacity(self.ctrls.len());
        for ctrl in self.ctrls.drain(..) {
            // Only the controllers at this tick can be the same one.
            let slot =
                kept.iter_mut().rev().take_while(|other| other.tick == ctrl.tick).find(|other| {
                    other.channel == ctrl.channel && kind_key(other.kind) == kind_key(ctrl.kind)
                });
            match slot {
                Some(slot) => *slot = ctrl,
                None => kept.push(ctrl),
            }
        }
        self.ctrls = kept;
        self.sort();
        before - (self.notes.len() + self.ctrls.len())
    }
}
