//! The **operations** on the notes: moving them, changing how hard
//! they are struck and how long they last.
//!
//! An operation works **in place** on a list of notes and never
//! changes how many notes there are: a note is never dropped and never
//! duplicated, because a note that was there before is a note that is
//! there now.

use crate::{Note, Sequence};

/// A number between zero and one.
fn unit(value: f64) -> f64 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}

/// A number, rounded to a whole one.
fn round(value: f64) -> i64 {
    if value.is_finite() { value.round().clamp(i64::MIN as f64, i64::MAX as f64) as i64 } else { 0 }
}

/// How a quantise is done.
///
/// The defaults are the plainest quantise there is: every note moves
/// all the way to the nearest line, the notes keep their length, and
/// nothing is excluded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizeOptions {
    /// How far apart the lines are, in ticks.
    pub grid_ticks: i64,
    /// How far a note moves towards the line: one is all the way.
    pub strength: f64,
    /// The **swing**: every second line is later by this much of a
    /// third of the grid, and one is a triplet feel.
    pub swing: f64,
    /// How near the line a note must be to move at all.
    ///
    /// A note is at most half a step from its line, so one is every
    /// note and a half is none.
    pub include_within: f64,
    /// How near the line a note must be to **stay** where it is.
    pub exclude_within: f64,
    /// Whether the ends of the notes are quantised as well.
    pub quantize_ends: bool,
    /// Whether a note keeps its length when it moves.
    pub preserve_duration: bool,
}

impl Default for QuantizeOptions {
    fn default() -> Self {
        QuantizeOptions {
            grid_ticks: 240,
            strength: 1.0,
            swing: 0.0,
            include_within: 1.0,
            exclude_within: 0.0,
            quantize_ends: false,
            preserve_duration: true,
        }
    }
}

/// The swung line nearest to `tick`.
fn nearest_line(tick: i64, grid: i64, swing: f64) -> i64 {
    let swing_off = round(swing * grid as f64 / 3.0);
    let line = tick.saturating_add(grid / 2).div_euclid(grid);
    let mut best = i64::MAX;
    let mut best_distance = u64::MAX;
    for candidate in [line.saturating_sub(1), line, line.saturating_add(1)] {
        let at = candidate.saturating_mul(grid).saturating_add(if candidate.rem_euclid(2) == 1 {
            swing_off
        } else {
            0
        });
        let distance = at.abs_diff(tick);
        if distance < best_distance {
            best_distance = distance;
            best = at;
        }
    }
    best
}

/// Quantises the notes onto the grid, and says how many moved.
///
/// A grid of no ticks is **nothing to quantise to**, and the notes are
/// left alone.
///
/// The **random** component of the tree's quantise is not here: a
/// note's place in a document is a fact, and a random place is not the
/// same fact twice.
pub fn quantize(notes: &mut [Note], options: &QuantizeOptions) -> usize {
    let grid = options.grid_ticks;
    if grid <= 0 {
        return 0;
    }
    let strength = unit(options.strength);
    let swing = unit(options.swing);
    let include = unit(options.include_within);
    let exclude = unit(options.exclude_within);
    let half = grid as f64 / 2.0;
    let mut moved = 0;
    for note in notes.iter_mut() {
        let old_start = note.start;
        let old_end = note.end();
        let target = nearest_line(old_start, grid, swing);
        let distance = target.saturating_sub(old_start);
        let away = distance.unsigned_abs() as f64;
        if include < 1.0 && away > include * half {
            continue;
        }
        if away < exclude * half {
            continue;
        }
        let new_start = old_start.saturating_add(round(distance as f64 * strength));
        note.start = new_start;
        note.length = if options.quantize_ends {
            let end_target = nearest_line(old_end, grid, swing);
            let new_end = old_end.saturating_add(round(end_target.saturating_sub(old_end) as f64 * strength));
            new_end.saturating_sub(new_start)
        } else if options.preserve_duration {
            note.length
        } else {
            old_end.saturating_sub(new_start)
        }
        .max(1);
        moved += 1;
    }
    moved
}

/// Why a note could not be transposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransposeError {
    /// The pitch of the note that would have left the range.
    pub pitch: u8,
    /// How many semitones were asked for.
    pub semitones: i32,
}

/// Transposes the notes by so many semitones.
///
/// The pitch of a MIDI note is from zero to a hundred and twenty-seven,
/// and a note that would leave that range is **refused**: the notes
/// are not changed at all, and the error says which note and how far.
///
/// The tree **clamps** a pitch that leaves the range: a note pushed
/// past the top piles up on a hundred and twenty-seven. Here the
/// transpose is refused, because a note that was asked for and is not
/// there is a note the caller does not know about.
pub fn transpose(notes: &mut [Note], semitones: i32) -> Result<(), TransposeError> {
    for note in notes.iter() {
        let moved = i32::from(note.pitch).saturating_add(semitones);
        if !(0..=127).contains(&moved) {
            return Err(TransposeError { pitch: note.pitch, semitones });
        }
    }
    for note in notes.iter_mut() {
        note.pitch = (i32::from(note.pitch) + semitones) as u8;
    }
    Ok(())
}

/// How the velocity of the notes is changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VelocityOp {
    /// Every note is struck this hard.
    Set(u8),
    /// Every note is struck this much harder.
    Add(i32),
    /// Every note is struck this many times as hard.
    Scale(f64),
}

/// Why a velocity could not be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VelocityError {
    /// The velocity that was asked for.
    pub velocity: i32,
}

/// The velocities a MIDI note may have: from one to a hundred and
/// twenty-seven.
///
/// A velocity of zero is not a velocity — it is the way a note is let
/// go — so the lowest a note may be struck is one.
pub const MIN_VELOCITY: i32 = 1;
/// The highest a note may be struck.
pub const MAX_VELOCITY: i32 = 127;

/// Changes how hard the notes are struck.
///
/// A velocity outside the range is **refused** and the notes are not
/// changed: the tree clamps such a velocity into the range, and a note
/// asked for as a hundred and fifty is not a note the caller knows
/// about.
pub fn change_velocity(notes: &mut [Note], op: &VelocityOp) -> Result<(), VelocityError> {
    let mut wanted: Vec<i32> = Vec::with_capacity(notes.len());
    for note in notes.iter() {
        let value = match op {
            VelocityOp::Set(value) => i32::from(*value),
            VelocityOp::Add(delta) => i32::from(note.velocity).saturating_add(*delta),
            VelocityOp::Scale(factor) => {
                if !factor.is_finite() {
                    return Err(VelocityError { velocity: i32::from(note.velocity) });
                }
                let scaled = round(f64::from(note.velocity) * factor);
                i32::try_from(scaled).unwrap_or(i32::MAX)
            }
        };
        if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&value) {
            return Err(VelocityError { velocity: value });
        }
        wanted.push(value);
    }
    for (note, value) in notes.iter_mut().zip(wanted) {
        note.velocity = value as u8;
    }
    Ok(())
}

/// How the length of the notes is changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DurationOp {
    /// Every note is this long.
    Set(i64),
    /// Every note is this much longer.
    Add(i64),
    /// Every note is this many times as long.
    Scale(f64),
    /// Every note ends this many ticks **before** the next note
    /// starts. The last note keeps its length.
    Legato {
        /// The gap before the next note, in ticks.
        gap: i64,
    },
    /// Every note ends no later than the next note of the same pitch
    /// and channel starts.
    RemoveOverlap,
}

/// Changes how long the notes are.
///
/// A note is always at least one tick long: a note of no length is a
/// note that cannot be heard.
pub fn change_duration(notes: &mut [Note], op: &DurationOp) {
    match op {
        DurationOp::Set(length) => notes.iter_mut().for_each(|note| note.length = (*length).max(1)),
        DurationOp::Add(delta) => {
            notes.iter_mut().for_each(|note| note.length = note.length.saturating_add(*delta).max(1));
        }
        DurationOp::Scale(factor) => {
            if factor.is_finite() {
                notes.iter_mut().for_each(|note| note.length = round(note.length as f64 * factor).max(1));
            }
        }
        DurationOp::Legato { gap } => {
            let mut starts: Vec<i64> = notes.iter().map(|note| note.start).collect();
            starts.sort_unstable();
            starts.dedup();
            for note in notes.iter_mut() {
                let at = starts.partition_point(|start| *start <= note.start);
                if let Some(next) = starts.get(at) {
                    note.length = next.saturating_sub(*gap).saturating_sub(note.start).max(1);
                }
            }
        }
        DurationOp::RemoveOverlap => {
            let mut order: Vec<usize> = (0..notes.len()).collect();
            order.sort_by_key(|index| notes.get(*index).map(|note| (note.channel, note.pitch, note.start)));
            for pair in order.windows(2) {
                let (Some(first), Some(second)) = (pair.first(), pair.get(1)) else { continue };
                let Some(next) = notes.get(*second).copied() else { continue };
                if let Some(note) = notes.get_mut(*first)
                    && note.channel == next.channel
                    && note.pitch == next.pitch
                    && note.end() > next.start
                {
                    note.length = next.start.saturating_sub(note.start).max(1);
                }
            }
        }
    }
}

/// Splits the notes by their **pitch**: the ones in the range and the
/// rest.
///
/// The bounds are swapped if they are the wrong way round.
pub fn split_by_pitch(notes: &[Note], from: u8, to: u8) -> (Vec<Note>, Vec<Note>) {
    let (low, high) = if from <= to { (from, to) } else { (to, from) };
    notes.iter().partition(|note| (low..=high).contains(&note.pitch))
}

/// The **notes** of a sequence, moved.
pub fn quantize_sequence(sequence: &mut Sequence, options: &QuantizeOptions) -> usize {
    quantize(&mut sequence.notes, options)
}
