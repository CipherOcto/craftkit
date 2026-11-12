//! The **Standard MIDI File**: the container the MIDI in a file is in.
//!
//! The format is the public **SMF 1.0 specification**: a header that
//! says the format and the division, and then a list of tracks, each
//! of them a chunk of events.

use std::collections::VecDeque;

use crate::{CtrlEvent, CtrlKind, Error, Note, PPQ, Sequence};

/// The most events a file may hold, across all of its tracks.
///
/// Five million events is more than a person plays in a lifetime, and
/// a file that claims more is not read: the reading would be the
/// problem, not the file.
pub const MAX_EVENTS: usize = 5_000_000;

/// The most tracks a file may hold.
pub const MAX_TRACKS: usize = 4096;

/// The release velocity of a note that ends with a note-on of velocity
/// zero.
///
/// The specification says that such a note-on is a note-off with a
/// velocity of sixty-four, and this is that sixty-four.
const DEFAULT_RELEASE: u8 = 64;

/// One track of a Standard MIDI File.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmfTrack {
    /// The name of the track, from the first name event in it.
    pub name: String,
    /// The notes and the controllers of the track.
    pub sequence: Sequence,
}

/// A Standard MIDI File in this crate's terms.
///
/// Every tick in it is at [`PPQ`](crate::PPQ) — nine hundred and sixty
/// a quarter note — whatever the file's own division said.
///
/// It is **not** an `Eq`: a tempo is a number of beats a minute, and a
/// number of beats a minute is a float.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Smf {
    /// The format of the file it was read from: zero, one or two.
    ///
    /// The writer always writes **format one**.
    pub format: u16,
    /// The tracks, in the order they were in the file.
    pub tracks: Vec<SmfTrack>,
    /// The tempos: a tick and a beats a minute.
    pub tempos: Vec<(i64, f64)>,
    /// The meters: a tick, a numerator and a denominator.
    pub meters: Vec<(i64, u8, u8)>,
    /// The markers: a tick and the text of the marker.
    pub markers: Vec<(i64, String)>,
    /// The key signatures: a tick, the sharps (or the flats) and
    /// whether the key is minor.
    pub key_sigs: Vec<(i64, i8, bool)>,
}

/// A byte at a time through the file.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
    /// The offset of `data[0]` in the file, for the messages.
    base: usize,
}

impl<'a> Cursor<'a> {
    /// The offset of the byte about to be read, in the whole file.
    fn at(&self) -> usize {
        self.base.saturating_add(self.pos)
    }

    /// Whether there is anything left to read.
    fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// One byte.
    fn u8(&mut self) -> Result<u8, Error> {
        let byte = self.data.get(self.pos).copied().ok_or(Error::Truncated { offset: self.at() })?;
        self.pos += 1;
        Ok(byte)
    }

    /// The byte about to be read, without taking it.
    fn peek(&self) -> Result<u8, Error> {
        self.data.get(self.pos).copied().ok_or(Error::Truncated { offset: self.at() })
    }

    /// `n` bytes.
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated { offset: self.at() })?;
        let slice = self.data.get(self.pos..end).ok_or(Error::Truncated { offset: self.at() })?;
        self.pos = end;
        Ok(slice)
    }

    /// Two bytes, as a number.
    fn u16(&mut self) -> Result<u16, Error> {
        let bytes = self.bytes(2)?;
        let first = bytes.first().copied().unwrap_or(0);
        let second = bytes.get(1).copied().unwrap_or(0);
        Ok(u16::from_be_bytes([first, second]))
    }

    /// Four bytes, as a number.
    fn u32(&mut self) -> Result<u32, Error> {
        let bytes = self.bytes(4)?;
        let mut out = [0u8; 4];
        out.copy_from_slice(bytes);
        Ok(u32::from_be_bytes(out))
    }

    /// A **variable-length quantity**: a number of at most four bytes.
    ///
    /// The specification allows four bytes and no more, and a longer
    /// one is refused: a number that does not fit is not a number the
    /// format has.
    fn vlq(&mut self) -> Result<u32, Error> {
        let start = self.at();
        let mut value: u32 = 0;
        for _ in 0..4 {
            let byte = self.u8()?;
            value = (value << 7) | u32::from(byte & 0x7F);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Error::NumberTooLong { offset: start })
    }
}

/// The conversion from the file's own ticks to this crate's ticks.
#[derive(Clone, Copy)]
struct TickConv {
    num: i128,
    den: i128,
}

impl TickConv {
    /// The conversion a division asks for.
    ///
    /// A division of ticks a quarter note is a plain ratio. A division
    /// of frames a second is **SMPTE**, and it is approximated at a
    /// hundred and twenty beats a minute — the file does not say how
    /// fast it is played, so the approximation is the only thing there
    /// is.
    fn from_division(division: u16, offset: usize) -> Result<Self, Error> {
        if division & 0x8000 == 0 {
            if division == 0 {
                return Err(Error::Invalid { offset, reason: "the division says zero ticks a quarter note" });
            }
            Ok(Self { num: i128::from(PPQ), den: i128::from(division) })
        } else {
            // SMPTE: the high byte is the frames a second, the low
            // byte the ticks a frame.
            let [high, low] = division.to_be_bytes();
            let fps = -i128::from(high as i8);
            let tpf = i128::from(low);
            if !matches!(fps, 24 | 25 | 29 | 30) || tpf == 0 {
                return Err(Error::Invalid { offset, reason: "the division is not a SMPTE one" });
            }
            let two_ppq = 2 * i128::from(PPQ);
            if fps == 29 {
                // 29.97 drop-frame: the ticks a second are 30000/1001
                // of the frame rate.
                Ok(Self { num: two_ppq * 1001, den: 30000 * tpf })
            } else {
                Ok(Self { num: two_ppq, den: fps * tpf })
            }
        }
    }

    /// The file's tick as this crate's tick.
    fn convert(&self, file_tick: u64) -> i64 {
        let value = (i128::from(file_tick) * self.num + self.den / 2) / self.den;
        i64::try_from(value).unwrap_or(i64::MAX)
    }
}

/// The things a file says about the whole of it.
struct Globals {
    tempos: Vec<(i64, f64)>,
    meters: Vec<(i64, u8, u8)>,
    markers: Vec<(i64, String)>,
    key_sigs: Vec<(i64, i8, bool)>,
}

/// Reads a Standard MIDI File.
///
/// Formats **zero** and **one** are read fully; format **two** is read
/// as well as it can be — its tracks share one timeline. Every `MTrk`
/// chunk is one track, and in format one a leading track that carries
/// nothing but the tempos and the meters is folded into the file's own
/// lists.
///
/// The ticks are converted to [`PPQ`](crate::PPQ), and a SMPTE division
/// is approximated at a hundred and twenty beats a minute.
///
/// The reader is **patient** with the things the format allows: a
/// running status, a note-on of velocity zero as a note-off, a note-off
/// with no note-on (ignored), two notes of the same pitch on top of
/// each other (matched first in, first out) and a note that is never
/// let go (closed at the end of the track). A note of no length is
/// given a length of one tick, because a note of no length is not a
/// note.
///
/// The reader is not patient with the things the format does not allow:
/// a header that is not one, a chunk longer than the file, a number
/// longer than four bytes, a data byte with no status to run on — each
/// of them is a **refusal** with a name.
pub fn read_smf(bytes: &[u8]) -> Result<Smf, Error> {
    let mut cursor = Cursor { data: bytes, pos: 0, base: 0 };
    if cursor.bytes(4)? != b"MThd" {
        return Err(Error::NotSmf);
    }
    let header_len = cursor.u32()?;
    if header_len < 6 {
        return Err(Error::Invalid { offset: 4, reason: "the header is shorter than six bytes" });
    }
    let claimed = u64::from(header_len);
    let header_len =
        usize::try_from(header_len).map_err(|_| Error::ChunkTooLong { offset: 4, len: claimed })?;
    let header = cursor.bytes(header_len).map_err(|_| Error::ChunkTooLong { offset: 4, len: claimed })?;
    let mut header_cursor = Cursor { data: header, pos: 0, base: 8 };
    let format = header_cursor.u16()?;
    let _tracks = header_cursor.u16()?;
    let division = header_cursor.u16()?;
    if format > 2 {
        return Err(Error::Unsupported { reason: "the format is greater than two" });
    }
    let conversion = TickConv::from_division(division, 12)?;

    let mut globals =
        Globals { tempos: Vec::new(), meters: Vec::new(), markers: Vec::new(), key_sigs: Vec::new() };
    let mut tracks: Vec<SmfTrack> = Vec::new();
    let mut budget = MAX_EVENTS;

    while !cursor.is_empty() {
        let chunk_at = cursor.at();
        let id = cursor.bytes(4)?;
        let len = cursor.u32()?;
        let remaining = bytes.len().saturating_sub(cursor.pos);
        let len = usize::try_from(len).unwrap_or(usize::MAX);
        if len > remaining {
            return Err(Error::ChunkTooLong {
                offset: chunk_at,
                len: u64::try_from(len).unwrap_or(u64::MAX),
            });
        }
        let body_at = cursor.at();
        let body = cursor.bytes(len)?;
        if id != b"MTrk" {
            // A chunk this does not know is skipped: the format says
            // that a reader may meet one it does not know.
            continue;
        }
        if tracks.len() >= MAX_TRACKS {
            return Err(Error::TooManyTracks { limit: MAX_TRACKS });
        }
        let track = read_track(body, body_at, conversion, &mut globals, &mut budget)?;
        tracks.push(track);
    }

    // A lone track is only the conductor track when it carried the
    // file's globals and has no name of its own.
    let globals_present = !(globals.tempos.is_empty()
        && globals.meters.is_empty()
        && globals.markers.is_empty()
        && globals.key_sigs.is_empty());
    if format == 1
        && (tracks.len() > 1 || globals_present)
        && tracks.first().is_some_and(|track| {
            track.sequence.notes.is_empty()
                && track.sequence.ctrls.is_empty()
                && (tracks.len() > 1 || track.name.is_empty())
        })
    {
        tracks.remove(0);
    }

    // The lists are in the order of their ticks, and two things at one
    // tick keep the order they were in the file.
    globals.tempos.sort_by_key(|tempo| tempo.0);
    globals.meters.sort_by_key(|meter| meter.0);
    globals.markers.sort_by_key(|marker| marker.0);
    globals.key_sigs.sort_by_key(|key| key.0);

    Ok(Smf {
        format,
        tracks,
        tempos: globals.tempos,
        meters: globals.meters,
        markers: globals.markers,
        key_sigs: globals.key_sigs,
    })
}

/// The notes that are waiting for their end, by channel and pitch.
type OpenNotes = Vec<VecDeque<(i64, u8)>>;

/// Closes a note, if there is one to close.
fn close_note(open: &mut OpenNotes, notes: &mut Vec<Note>, channel: u8, pitch: u8, tick: i64, release: u8) {
    let key = usize::from(channel & 0x0F) * 128 + usize::from(pitch & 0x7F);
    let Some(stack) = open.get_mut(key) else { return };
    if stack.is_empty() {
        // A note-off with no note-on: ignored, as the format says.
        return;
    }
    let Some((start, velocity)) = stack.pop_front() else { return };
    let length = tick.saturating_sub(start).max(1);
    notes.push(Note { pitch, velocity, release_velocity: release, channel, start, length });
}

/// Reads one track.
fn read_track(
    body: &[u8],
    base: usize,
    conversion: TickConv,
    globals: &mut Globals,
    budget: &mut usize,
) -> Result<SmfTrack, Error> {
    let mut cursor = Cursor { data: body, pos: 0, base };
    let mut name: Option<String> = None;
    let mut sequence = Sequence::default();
    let mut open: OpenNotes = vec![VecDeque::new(); 16 * 128];
    let mut file_tick: u64 = 0;
    let mut tick: i64 = 0;
    let mut running: Option<u8> = None;

    while !cursor.is_empty() {
        if *budget == 0 {
            return Err(Error::TooManyEvents { limit: MAX_EVENTS });
        }
        *budget -= 1;

        let delta = cursor.vlq()?;
        file_tick = file_tick.saturating_add(u64::from(delta));
        tick = conversion.convert(file_tick);

        let event_at = cursor.at();
        let first = cursor.peek()?;
        let status = if first & 0x80 != 0 {
            cursor.pos += 1;
            first
        } else {
            running
                .ok_or(Error::Invalid { offset: event_at, reason: "a data byte with no status to run on" })?
        };

        match status {
            0xFF => {
                running = None;
                let kind = cursor.u8()?;
                let len = cursor.vlq()?;
                let data = cursor.bytes(usize::try_from(len).unwrap_or(usize::MAX))?;
                match kind {
                    0x2F => break, // the end of the track
                    0x03 if name.is_none() => {
                        name = Some(String::from_utf8_lossy(data).into_owned());
                    }
                    0x06 => globals.markers.push((tick, String::from_utf8_lossy(data).into_owned())),
                    0x51 => {
                        if let [a, b, c] = data {
                            let microseconds = (u32::from(*a) << 16) | (u32::from(*b) << 8) | u32::from(*c);
                            if microseconds > 0 {
                                globals.tempos.push((tick, 60_000_000.0 / f64::from(microseconds)));
                            }
                        }
                    }
                    0x58 => {
                        if let [numerator, denominator, ..] = data
                            && *numerator > 0
                            && *denominator <= 7
                        {
                            globals.meters.push((tick, *numerator, 1u8 << *denominator));
                        }
                    }
                    0x59 => {
                        if let [sharps, minor, ..] = data {
                            let sharps = *sharps as i8;
                            if (-7..=7).contains(&sharps) {
                                globals.key_sigs.push((tick, sharps, *minor == 1));
                            }
                        }
                    }
                    _ => {}
                }
            }
            0xF0 | 0xF7 => {
                running = None;
                let len = cursor.vlq()?;
                cursor.bytes(usize::try_from(len).unwrap_or(usize::MAX))?;
            }
            0x80..=0xEF => {
                running = Some(status);
                let channel = status & 0x0F;
                let first_data = cursor.u8()?;
                if first_data & 0x80 != 0 {
                    return Err(Error::Invalid {
                        offset: event_at,
                        reason: "a status byte where a data byte was expected",
                    });
                }
                // A program change and a channel pressure carry one
                // byte; everything else carries two.
                let needs_two = !matches!(status & 0xF0, 0xC0 | 0xD0);
                let second_data = if needs_two {
                    let data = cursor.u8()?;
                    if data & 0x80 != 0 {
                        return Err(Error::Invalid {
                            offset: event_at,
                            reason: "a status byte where a data byte was expected",
                        });
                    }
                    data
                } else {
                    0
                };
                match status & 0xF0 {
                    0x80 => {
                        close_note(&mut open, &mut sequence.notes, channel, first_data, tick, second_data)
                    }
                    0x90 if second_data == 0 => {
                        close_note(&mut open, &mut sequence.notes, channel, first_data, tick, DEFAULT_RELEASE)
                    }
                    0x90 => {
                        if let Some(stack) =
                            open.get_mut(usize::from(channel) * 128 + usize::from(first_data))
                        {
                            stack.push_back((tick, second_data));
                        }
                    }
                    0xA0 => sequence.ctrls.push(CtrlEvent {
                        tick,
                        channel,
                        kind: CtrlKind::PolyPressure(first_data),
                        value: i32::from(second_data),
                    }),
                    0xB0 => sequence.ctrls.push(CtrlEvent {
                        tick,
                        channel,
                        kind: CtrlKind::Control(first_data),
                        value: i32::from(second_data),
                    }),
                    0xC0 => sequence.ctrls.push(CtrlEvent {
                        tick,
                        channel,
                        kind: CtrlKind::Program,
                        value: i32::from(first_data),
                    }),
                    0xD0 => sequence.ctrls.push(CtrlEvent {
                        tick,
                        channel,
                        kind: CtrlKind::ChannelPressure,
                        value: i32::from(first_data),
                    }),
                    _ => {
                        // The pitch bend: the low byte and then the
                        // high one, centred on 8192.
                        let value = (i32::from(second_data) << 7 | i32::from(first_data)) - 8192;
                        sequence.ctrls.push(CtrlEvent { tick, channel, kind: CtrlKind::PitchBend, value });
                    }
                }
            }
            _ => {
                return Err(Error::Invalid { offset: event_at, reason: "a system message in a track" });
            }
        }
    }

    // A note that was never let go ends at the end of the track.
    for (key, stack) in open.iter_mut().enumerate() {
        let channel = u8::try_from(key / 128).unwrap_or(0);
        let pitch = u8::try_from(key % 128).unwrap_or(0);
        for (start, velocity) in stack.drain(..) {
            let length = tick.saturating_sub(start).max(1);
            sequence.notes.push(Note {
                pitch,
                velocity,
                release_velocity: DEFAULT_RELEASE,
                channel,
                start,
                length,
            });
        }
    }
    sequence.sort();
    Ok(SmfTrack { name: name.unwrap_or_default(), sequence })
}

// ---------------------------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------------------------

/// The most a variable-length quantity can hold: twenty-eight bits.
const MAX_NUMBER: u64 = 0x0FFF_FFFF;

/// Writes a number as a variable-length quantity.
fn push_number(out: &mut Vec<u8>, value: u64) -> Result<(), Error> {
    if value > MAX_NUMBER {
        return Err(Error::CannotWrite {
            reason: format!("the number {value} does not fit in a MIDI variable-length quantity"),
        });
    }
    let mut buffer = [0u8; 4];
    let mut count = 0;
    let mut rest = value;
    loop {
        if let Some(slot) = buffer.get_mut(count) {
            *slot = (rest & 0x7F) as u8;
        }
        count += 1;
        rest >>= 7;
        if rest == 0 || count == 4 {
            break;
        }
    }
    for index in (0..count).rev() {
        let byte = buffer.get(index).copied().unwrap_or(0);
        out.push(if index > 0 { byte | 0x80 } else { byte });
    }
    Ok(())
}

/// An event on its way into the file: a tick, an order at that tick
/// and the bytes of the event.
struct WEvent {
    tick: u64,
    order: u8,
    bytes: Vec<u8>,
}

/// A tick as a number the file can hold: a negative tick is zero.
fn clamp_tick(tick: i64) -> u64 {
    u64::try_from(tick.max(0)).unwrap_or(0)
}

/// A meta event: its kind and its text.
fn meta(kind: u8, data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = vec![0xFF, kind];
    push_number(&mut out, u64::try_from(data.len()).unwrap_or(u64::MAX))?;
    out.extend_from_slice(data);
    Ok(out)
}

/// Writes one track.
fn write_track(out: &mut Vec<u8>, mut events: Vec<WEvent>) -> Result<(), Error> {
    events.sort_by_key(|event| (event.tick, event.order));
    let mut body: Vec<u8> = Vec::new();
    let mut last_tick: u64 = 0;
    let mut running: Option<u8> = None;
    for event in &events {
        push_number(&mut body, event.tick - last_tick)?;
        last_tick = event.tick;
        let status = event.bytes.first().copied().unwrap_or(0);
        if (0x80..=0xEF).contains(&status) {
            // The **running status**: the same status twice in a row
            // is written the second time without it.
            if running == Some(status) {
                body.extend_from_slice(event.bytes.get(1..).unwrap_or(&[]));
            } else {
                body.extend_from_slice(&event.bytes);
            }
            running = Some(status);
        } else {
            running = None;
            body.extend_from_slice(&event.bytes);
        }
    }
    // The end of the track.
    body.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    let len = u32::try_from(body.len())
        .map_err(|_| Error::CannotWrite { reason: "the track is larger than four gigabytes".to_string() })?;
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(())
}

/// The events of the **conductor** track: the tempos, the meters, the
/// key signatures and the markers.
fn conductor_events(smf: &Smf) -> Result<Vec<WEvent>, Error> {
    let mut events: Vec<WEvent> = Vec::new();
    for &(tick, bpm) in &smf.tempos {
        if !bpm.is_finite() || bpm <= 0.0 {
            return Err(Error::CannotWrite { reason: format!("the tempo {bpm} is not a tempo") });
        }
        let microseconds = (60_000_000.0 / bpm).round().clamp(1.0, f64::from(0x00FF_FFFFu32)) as u32;
        let [_, a, b, c] = microseconds.to_be_bytes();
        events.push(WEvent { tick: clamp_tick(tick), order: 0, bytes: meta(0x51, &[a, b, c])? });
    }
    for &(tick, numerator, denominator) in &smf.meters {
        if denominator == 0 || !denominator.is_power_of_two() {
            return Err(Error::CannotWrite {
                reason: format!("the denominator {denominator} of a meter is not a power of two"),
            });
        }
        let denominator = denominator.trailing_zeros() as u8;
        events.push(WEvent {
            tick: clamp_tick(tick),
            order: 1,
            bytes: meta(0x58, &[numerator, denominator, 24, 8])?,
        });
    }
    for &(tick, sharps, minor) in &smf.key_sigs {
        let sharps = sharps.clamp(-7, 7);
        events.push(WEvent {
            tick: clamp_tick(tick),
            order: 2,
            bytes: meta(0x59, &[sharps as u8, u8::from(minor)])?,
        });
    }
    for (tick, text) in &smf.markers {
        events.push(WEvent { tick: clamp_tick(*tick), order: 3, bytes: meta(0x06, text.as_bytes())? });
    }
    Ok(events)
}

/// Trims the notes that are still sounding when the next note of the
/// same pitch starts, and drops the duplicates.
///
/// The reader pairs a note-off with the note-on before it, so a note
/// still sounding when the next note of the same pitch starts would
/// **swap ends** with it: it is trimmed to that start. Two notes of
/// the same pitch that start on the same tick cannot be told apart, so
/// only the longer one is written.
fn trim_overlaps(notes: &[Note], ends: &mut [u64], dropped: &mut [bool]) {
    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by_key(|&index| {
        notes.get(index).map_or((0, 0, 0, std::cmp::Reverse(0)), |note| {
            (
                note.channel & 0x0F,
                note.pitch.min(127),
                clamp_tick(note.start),
                std::cmp::Reverse(ends.get(index).copied().unwrap_or(0)),
            )
        })
    });
    let mut previous: Option<usize> = None;
    for &index in &order {
        let Some(current) = notes.get(index) else { continue };
        if let Some(before) = previous
            && let Some(earlier) = notes.get(before)
            && earlier.channel & 0x0F == current.channel & 0x0F
            && earlier.pitch.min(127) == current.pitch.min(127)
        {
            let (earlier_start, current_start) = (clamp_tick(earlier.start), clamp_tick(current.start));
            if earlier_start == current_start {
                if let Some(slot) = dropped.get_mut(index) {
                    *slot = true;
                }
                continue;
            }
            if let Some(end) = ends.get_mut(before)
                && current_start < *end
            {
                *end = current_start;
            }
        }
        previous = Some(index);
    }
}

/// Writes a **format one** Standard MIDI File at [`PPQ`](crate::PPQ)
/// ticks a quarter note.
///
/// The first track is the conductor track and holds the tempos, the
/// meters, the key signatures and the markers; every track of the file
/// follows as its own track.
///
/// A negative tick is **zero**, a value outside its MIDI range is
/// brought into it, and a note-on of velocity zero is written as one —
/// a zero would be a note-off. At one tick the note-offs are written
/// first, then the controllers, then the note-ons.
///
/// The writer **refuses** a fact it cannot write: a tempo of zero, a
/// meter whose denominator is not a power of two, more than sixty-five
/// thousand tracks, or a gap between two events too long for the
/// twenty-eight bits the format allows.
pub fn write_smf(smf: &Smf) -> Result<Vec<u8>, Error> {
    // The conductor track is left out when there is nothing to put in
    // it and no other track, so an empty file is a file with no tracks
    // and reads back as no tracks.
    let has_conductor = !(smf.tempos.is_empty()
        && smf.meters.is_empty()
        && smf.key_sigs.is_empty()
        && smf.markers.is_empty()
        && smf.tracks.is_empty());
    let tracks = smf.tracks.len().saturating_add(usize::from(has_conductor));
    let tracks = u16::try_from(tracks).map_err(|_| Error::CannotWrite {
        reason: "the file has more than sixty-five thousand tracks".to_string(),
    })?;
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&tracks.to_be_bytes());
    out.extend_from_slice(&u16::try_from(PPQ).unwrap_or(960).to_be_bytes());

    let events = conductor_events(smf)?;
    if has_conductor {
        write_track(&mut out, events)?;
    }

    for track in &smf.tracks {
        let mut events: Vec<WEvent> = Vec::new();
        if !track.name.is_empty() {
            events.push(WEvent { tick: 0, order: 0, bytes: meta(0x03, track.name.as_bytes())? });
        }
        let notes = &track.sequence.notes;
        let mut ends: Vec<u64> = notes
            .iter()
            .map(|note| clamp_tick(note.end()).max(clamp_tick(note.start).saturating_add(1)))
            .collect();
        let mut dropped = vec![false; notes.len()];
        trim_overlaps(notes, &mut ends, &mut dropped);
        for (index, note) in notes.iter().enumerate() {
            if dropped.get(index).copied().unwrap_or(false) {
                continue;
            }
            let channel = note.channel & 0x0F;
            let pitch = note.pitch.min(127);
            let start = clamp_tick(note.start);
            let end = ends.get(index).copied().unwrap_or(start.saturating_add(1));
            events.push(WEvent {
                tick: start,
                order: 3,
                bytes: vec![0x90 | channel, pitch, note.velocity.clamp(1, 127)],
            });
            events.push(WEvent {
                tick: end,
                order: 1,
                bytes: vec![0x80 | channel, pitch, note.release_velocity.min(127)],
            });
        }
        for ctrl in &track.sequence.ctrls {
            let channel = ctrl.channel & 0x0F;
            let value = ctrl.value.clamp(0, 127) as u8;
            let bytes = match ctrl.kind {
                CtrlKind::Control(number) => vec![0xB0 | channel, number.min(127), value],
                CtrlKind::Program => vec![0xC0 | channel, value],
                CtrlKind::ChannelPressure => vec![0xD0 | channel, value],
                CtrlKind::PolyPressure(pitch) => vec![0xA0 | channel, pitch.min(127), value],
                CtrlKind::PitchBend => {
                    let bent = (ctrl.value.saturating_add(8192)).clamp(0, 16383);
                    vec![0xE0 | channel, (bent & 0x7F) as u8, ((bent >> 7) & 0x7F) as u8]
                }
            };
            events.push(WEvent { tick: clamp_tick(ctrl.tick), order: 2, bytes });
        }
        write_track(&mut out, events)?;
    }
    Ok(out)
}
