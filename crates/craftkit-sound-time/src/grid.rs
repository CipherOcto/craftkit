//! The **grid**: the lines a thing lands on.
//!
//! A grid is a **division of the bar** — a quarter, an eighth, a
//! sixteenth — and the lines of that division are where a clip is put
//! when it is put on the grid. The lines are not a number of samples
//! fixed once and for all: they are **musical**, so they move with the
//! tempo and they follow the meter, and the same grid is a different
//! distance apart in a slow bar and a fast one.
//!
//! # The lines restart at every bar
//!
//! A grid of sixteenths in a bar of 4/4 has sixteen lines in it, and a
//! grid of eighths has eight. The lines are counted **from the bar
//! line**, and not from the start of the song, because a bar of 3/4
//! has no even number of eighths in it: lines counted from the start
//! would drift a quarter of a beat by the end of every odd bar, and a
//! musician would hear the grid slide. Counted from the bar line, every
//! bar is the same length and the grid never drifts.
//!
//! # The step may be dotted or a triplet
//!
//! A **dotted** note is one and a half of the note it is: a dotted
//! eighth is three sixteenths. A **triplet** is two thirds of it: a
//! triplet eighth is a twelfth of a bar. Both are ways of dividing the
//! beat further than the note value alone does, and the grid says which
//! of them is in force.

use crate::{Samples, TICKS_PER_QUARTER, TempoMap, ticks_per_beat};

/// A **note value**: what the grid divides the bar into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteValue {
    /// A whole bar.
    Bar,
    /// A half note.
    Half,
    /// A quarter note.
    Quarter,
    /// An eighth note.
    Eighth,
    /// A sixteenth note.
    Sixteenth,
    /// A thirty-second note.
    ThirtySecond,
    /// A sixty-fourth note.
    SixtyFourth,
}

impl NoteValue {
    /// Every note value, from the largest to the smallest.
    pub const ALL: [NoteValue; 7] = [
        NoteValue::Bar,
        NoteValue::Half,
        NoteValue::Quarter,
        NoteValue::Eighth,
        NoteValue::Sixteenth,
        NoteValue::ThirtySecond,
        NoteValue::SixtyFourth,
    ];

    /// The note value a name says.
    ///
    /// The names are the ones a person writes: `1/4`, `1/8`, `bar` and
    /// the rest. A name that is not a note value is **none** and not a
    /// guess: a grid asked for as `1/5` is not a grid.
    pub fn parse(text: &str) -> Option<NoteValue> {
        match text.trim().to_ascii_lowercase().as_str() {
            "bar" | "1 bar" | "1/1" => Some(NoteValue::Bar),
            "1/2" | "1/2 note" | "half" => Some(NoteValue::Half),
            "1/4" | "1/4 note" | "quarter" => Some(NoteValue::Quarter),
            "1/8" | "1/8 note" | "eighth" => Some(NoteValue::Eighth),
            "1/16" | "1/16 note" | "sixteenth" => Some(NoteValue::Sixteenth),
            "1/32" | "1/32 note" | "thirtysecond" | "thirty-second" => Some(NoteValue::ThirtySecond),
            "1/64" | "1/64 note" | "sixtyfourth" | "sixty-fourth" => Some(NoteValue::SixtyFourth),
            _ => None,
        }
    }

    /// The name of the note value, as a person writes it.
    pub fn name(self) -> &'static str {
        match self {
            NoteValue::Bar => "bar",
            NoteValue::Half => "1/2",
            NoteValue::Quarter => "1/4",
            NoteValue::Eighth => "1/8",
            NoteValue::Sixteenth => "1/16",
            NoteValue::ThirtySecond => "1/32",
            NoteValue::SixtyFourth => "1/64",
        }
    }

    /// The ticks in the note value.
    ///
    /// A bar is **not** a fixed number of ticks: it is as long as the
    /// meter in force says, so a bar is worked out where the grid is
    /// and not here.
    pub fn ticks(self) -> i64 {
        match self {
            NoteValue::Bar => TICKS_PER_QUARTER * 4,
            NoteValue::Half => TICKS_PER_QUARTER * 2,
            NoteValue::Quarter => TICKS_PER_QUARTER,
            NoteValue::Eighth => TICKS_PER_QUARTER / 2,
            NoteValue::Sixteenth => TICKS_PER_QUARTER / 4,
            NoteValue::ThirtySecond => TICKS_PER_QUARTER / 8,
            NoteValue::SixtyFourth => TICKS_PER_QUARTER / 16,
        }
    }
}

/// The **grid**: a division of the bar, and how it is divided.
///
/// A grid is a note value and two flags — **dotted** and **triplet** —
/// and nothing else. The step of the grid is worked out from the tempo
/// map every time it is asked for, because the same grid is a
/// different number of samples apart at a different tempo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// What the grid divides the bar into.
    pub value: NoteValue,
    /// Whether the step is **dotted**: one and a half of the note.
    pub dotted: bool,
    /// Whether the step is a **triplet**: two thirds of the note.
    pub triplet: bool,
}

impl Default for Grid {
    /// A grid of **sixteenths**: the tree's own default.
    fn default() -> Grid {
        Grid { value: NoteValue::Sixteenth, dotted: false, triplet: false }
    }
}

impl Grid {
    /// A grid of a note value.
    pub fn new(value: NoteValue) -> Grid {
        Grid { value, dotted: false, triplet: false }
    }

    /// The name of the grid, as a person writes it.
    pub fn label(&self) -> String {
        let mut label = String::from(self.value.name());
        if self.dotted {
            label.push_str(" dotted");
        }
        if self.triplet {
            label.push_str(" triplet");
        }
        label
    }

    /// The ticks in one step of the grid, at a tick.
    ///
    /// The meter decides how long a bar is, so the step of a bar grid
    /// depends on where the grid is asked for. A step is never less
    /// than a tick: a grid of sixty-fourths in a fast bar is still a
    /// grid of whole ticks.
    pub fn step_ticks(&self, map: &TempoMap, at_tick: i64) -> i64 {
        let base = match self.value {
            NoteValue::Bar => {
                let (numerator, denominator) = map.meter_at_tick(at_tick);
                ticks_per_beat(denominator) * numerator as i64
            }
            value => value.ticks(),
        };
        let mut ticks = base;
        if self.dotted {
            ticks = ticks * 3 / 2;
        }
        if self.triplet {
            ticks = ticks * 2 / 3;
        }
        ticks.max(1)
    }

    /// The samples in one step of the grid, at a position.
    ///
    /// The step is a distance and not a number of ticks, because the
    /// ticks are worth a different number of samples at a different
    /// tempo.
    pub fn step_samples(&self, at: Samples, map: &TempoMap, sample_rate: u32) -> Samples {
        let tick = map.samples_to_ticks(at, sample_rate);
        let step = self.step_ticks(map, tick);
        let here = map.ticks_to_samples(tick, sample_rate);
        let after = map.ticks_to_samples(tick + step, sample_rate);
        (after - here).max(1)
    }

    /// The grid line at or before a position.
    ///
    /// The lines are counted **from the bar line**, so that a bar of an
    /// odd number of beats does not make the grid drift.
    pub fn line_at_or_before(&self, pos: Samples, map: &TempoMap, sample_rate: u32) -> Samples {
        // The tick of the position, walked onto the tick whose sample
        // index is at or before it: a tick is worth a number of samples
        // that changes with the tempo, and the nearest tick is not
        // always the last one before the position.
        let mut tick = map.samples_to_ticks(pos, sample_rate);
        if map.ticks_to_samples(tick, sample_rate) > pos {
            tick = tick.saturating_sub(1);
        } else if map.ticks_to_samples(tick + 1, sample_rate) <= pos {
            tick += 1;
        }
        let bar_tick = map.bar_start_tick_at(tick);
        let step = self.step_ticks(map, bar_tick);
        let line = bar_tick + (tick - bar_tick).div_euclid(step) * step;
        map.ticks_to_samples(line, sample_rate)
    }

    /// The grid line after a line.
    pub fn next_line(&self, line: Samples, map: &TempoMap, sample_rate: u32) -> Samples {
        let step = self.step_samples(line, map, sample_rate);
        let candidate = self.line_at_or_before(line + step, map, sample_rate);
        if candidate > line { candidate } else { line + step }
    }

    /// The nearest grid line to a position.
    ///
    /// When the position is exactly between two lines it goes to the
    /// **earlier** one: a tie is broken the same way every time, and a
    /// thing that lands on a different line each time it is asked is a
    /// thing nobody can rely on.
    pub fn snap(&self, pos: Samples, map: &TempoMap, sample_rate: u32) -> Samples {
        let low = self.line_at_or_before(pos, map, sample_rate);
        let high = self.next_line(low, map, sample_rate);
        if (pos - low) <= (high - pos) { low } else { high }
    }

    /// The grid lines in a range, at most `max` of them.
    ///
    /// The lines are **bounded**: a range of a whole song on a grid of
    /// sixty-fourths is a great many lines, and a caller that asks for
    /// all of them is asking for a number that would fill the memory.
    pub fn lines(
        &self,
        start: Samples,
        end: Samples,
        map: &TempoMap,
        sample_rate: u32,
        max: usize,
    ) -> Vec<Samples> {
        let mut lines = Vec::new();
        let mut line = self.line_at_or_before(start, map, sample_rate);
        while line < end && lines.len() < max {
            if line >= start {
                lines.push(line);
            }
            let next = self.next_line(line, map, sample_rate);
            if next <= line {
                break;
            }
            line = next;
        }
        lines
    }
}
