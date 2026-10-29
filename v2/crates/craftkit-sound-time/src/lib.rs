//! `craftkit-sound-time` — the **time model** of the sound battery.
//!
//! # Why this crate exists
//!
//! A DAW is mostly a disagreement about *where we are*. Two things have to
//! be true at once: a position is an absolute sample index (the audio
//! hardware counts samples and nothing else), and a position is also a
//! place in a bar (the musician reads a score, and a cursor has to be
//! where the beat is).
//!
//! Those two are not the same number and there is no constant between
//! them: the number of samples in a beat depends on the tempo, and the
//! tempo may change in the middle of the song. So the conversion is a
//! **map** — a timeline of tempo events and meter events — and the map is
//! the same object for every panel, every plugin and every file that
//! reads it.
//!
//! So it lives here: a **library**, not a plugin. It contributes nothing
//! to the registry, it is not wired in by the linker, and a plugin that
//! wants it depends on it the way it depends on the SDK. This is the
//! same rule the geometry library (`craftkit-vector-geom`) follows: the
//! *operations* are a library, the *thing they operate on* is the SDK's.
//!
//! # The model
//!
//! - A **position** is an absolute sample index ([`Samples`], an `i64`).
//! - A **tick** is the musical unit: [`TICKS_PER_QUARTER`] (960) ticks to
//!   a quarter note, whatever the tempo.
//! - The map is **tick-anchored**: a tempo event says "from this tick, a
//!   quarter note lasts `60 / bpm` seconds". Ticks are the independent
//!   variable; samples are derived from them.
//! - A **meter** event says how many beats are in a bar, and what a beat
//!   is worth, from a tick onwards.
//!
//! The conversions are exact inverses at tick boundaries: a tick is a
//! round number of ticks, and the sample index it maps to is the one a
//! plugin will read back.
//!
//! # What is deliberately not here
//!
//! There is no plugin, no command, no kind, no panel and no transport.
//! The transport is the application's; this crate only knows how to say
//! where the playhead is, in samples, in ticks, in bars and beats, or in
//! minutes and seconds.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

/// An absolute position in the audio stream, counted in samples.
pub type Samples = i64;

/// The furthest position the model will hold: `2^40` samples, about 265
/// days at 48 kHz.
///
/// Positions beyond this are refused rather than silently wrapped: a
/// number that large is a mistake, and a mistake in a position is a
/// mistake in every panel that shows it.
pub const MAX_POSITION: Samples = 1 << 40;

/// The ticks in a quarter note.
///
/// The tick is the musical unit of the map, and it is fixed: a tempo
/// event says how long a quarter note lasts, and the quarter note is
/// always this many ticks.
pub const TICKS_PER_QUARTER: i64 = 960;

/// A place in the bar.
///
/// `bar` and `beat` are **1-based** — the first bar is bar 1 and the
/// first beat is beat 1 — because that is how a musician counts. `tick`
/// is the tick **within the beat**, counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarBeat {
    /// The bar, counting from 1.
    pub bar: i64,
    /// The beat within the bar, counting from 1.
    pub beat: i64,
    /// The tick within the beat, counting from 0.
    pub tick: i64,
}

/// A tempo change: from `tick` onwards, a quarter note lasts `60 / bpm`
/// seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoEvent {
    /// The tick the change happens at.
    pub tick: i64,
    /// The tempo in beats per minute.
    pub bpm: f64,
}

/// A meter change: from `tick` onwards, a bar has `numerator` beats and
/// a beat is worth `1 / denominator` of a whole note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeterEvent {
    /// The tick the change happens at.
    pub tick: i64,
    /// How many beats are in a bar.
    pub numerator: u32,
    /// What a beat is worth, as the denominator of a whole note.
    pub denominator: u32,
}

/// The timeline of tempo and meter changes.
///
/// The map is **tick-anchored**: it is defined on ticks, and samples are
/// derived from it. A tempo event at tick `T` holds from `T` until the
/// next tempo event; the same for meters.
///
/// The map always has a tempo and a meter at tick 0 — 120 bpm and 4/4 —
/// so a position before the first change is well defined.
#[derive(Debug, Clone, PartialEq)]
pub struct TempoMap {
    tempos: Vec<TempoEvent>,
    meters: Vec<MeterEvent>,
}

impl Default for TempoMap {
    fn default() -> Self {
        Self::new()
    }
}

impl TempoMap {
    /// A map with 120 bpm and 4/4, both from tick 0.
    pub fn new() -> Self {
        Self {
            tempos: vec![TempoEvent { tick: 0, bpm: 120.0 }],
            meters: vec![MeterEvent { tick: 0, numerator: 4, denominator: 4 }],
        }
    }

    /// The tempo events, in tick order.
    pub fn tempos(&self) -> &[TempoEvent] {
        &self.tempos
    }

    /// The meter events, in tick order.
    pub fn meters(&self) -> &[MeterEvent] {
        &self.meters
    }

    /// Set the tempo from `tick` onwards.
    ///
    /// The tempo must be finite and between 5 and 1000 bpm: below 5 the
    /// beat is longer than a bar of most music and the arithmetic turns
    /// silly, above 1000 it is faster than anything can be played. A
    /// tempo already set at that tick is replaced.
    pub fn set_tempo(&mut self, tick: i64, bpm: f64) -> Result<(), String> {
        if !bpm.is_finite() {
            return Err(format!("a tempo must be a finite number, not {bpm}"));
        }
        if !(5.0..=1000.0).contains(&bpm) {
            return Err(format!("a tempo must be between 5 and 1000 bpm, not {bpm}"));
        }
        if tick < 0 {
            return Err(format!("a tempo must be set at a tick of 0 or more, not {tick}"));
        }
        if tick > MAX_POSITION {
            return Err(format!("a tempo must be set at or before {MAX_POSITION}, not {tick}"));
        }
        let event = TempoEvent { tick, bpm };
        match self.tempos.binary_search_by_key(&tick, |existing| existing.tick) {
            Ok(index) => self.tempos[index] = event,
            Err(index) => self.tempos.insert(index, event),
        }
        Ok(())
    }

    /// Set the meter from `tick` onwards.
    ///
    /// The numerator is how many beats are in a bar (1 to 64) and the
    /// denominator is what a beat is worth: one of 1, 2, 4, 8, 16, 32 or
    /// 64. A meter already set at that tick is replaced.
    pub fn set_meter(&mut self, tick: i64, numerator: u32, denominator: u32) -> Result<(), String> {
        if !(1..=64).contains(&numerator) {
            return Err(format!("a bar must have between 1 and 64 beats, not {numerator}"));
        }
        if !matches!(denominator, 1 | 2 | 4 | 8 | 16 | 32 | 64) {
            return Err(format!(
                "a beat must be worth 1, 2, 4, 8, 16, 32 or 64 of a whole note, not {denominator}"
            ));
        }
        if tick < 0 {
            return Err(format!("a meter must be set at a tick of 0 or more, not {tick}"));
        }
        if tick > MAX_POSITION {
            return Err(format!("a meter must be set at or before {MAX_POSITION}, not {tick}"));
        }
        let event = MeterEvent { tick, numerator, denominator };
        match self.meters.binary_search_by_key(&tick, |existing| existing.tick) {
            Ok(index) => self.meters[index] = event,
            Err(index) => self.meters.insert(index, event),
        }
        Ok(())
    }

    /// The tempo in force at `tick`, in beats per minute.
    pub fn tempo_at_tick(&self, tick: i64) -> f64 {
        let index = self.tempos.partition_point(|event| event.tick <= tick);
        if index == 0 {
            self.tempos.first().map_or(120.0, |event| event.bpm)
        } else {
            self.tempos[index - 1].bpm
        }
    }

    /// The meter in force at `tick`, as `(numerator, denominator)`.
    pub fn meter_at_tick(&self, tick: i64) -> (u32, u32) {
        let index = self.meters.partition_point(|event| event.tick <= tick);
        if index == 0 {
            self.meters.first().map_or((4, 4), |event| (event.numerator, event.denominator))
        } else {
            let event = self.meters[index - 1];
            (event.numerator, event.denominator)
        }
    }

    /// How many samples one tick lasts at `tick`, at `sample_rate`.
    ///
    /// A tick is a fixed number of ticks per quarter note, so a tick lasts
    /// `60 / bpm` seconds divided by [`TICKS_PER_QUARTER`].
    pub fn samples_per_tick(&self, tick: i64, sample_rate: u32) -> f64 {
        let bpm = self.tempo_at_tick(tick);
        sample_rate as f64 * 60.0 / (bpm * TICKS_PER_QUARTER as f64)
    }

    /// The sample index of a tick, rounded to the nearest sample.
    ///
    /// The walk goes over the tempo segments: each segment contributes its
    /// own ticks at its own tempo.
    pub fn ticks_to_samples(&self, ticks: i64, sample_rate: u32) -> Samples {
        round_samples(self.exact_samples(ticks, sample_rate))
    }

    /// The sample index of a tick, before it is rounded or bounded.
    fn exact_samples(&self, ticks: i64, sample_rate: u32) -> f64 {
        if ticks <= 0 {
            return 0.0;
        }
        let mut total = 0.0_f64;
        for (index, event) in self.tempos.iter().enumerate() {
            let start = event.tick;
            if start >= ticks {
                break;
            }
            let end = self.tempos.get(index + 1).map_or(i64::MAX, |next| next.tick);
            let span = end.min(ticks) - start;
            if span <= 0 {
                continue;
            }
            total += span as f64 * self.samples_per_tick(start, sample_rate);
        }
        total
    }

    /// The tick of a sample index.
    ///
    /// This is the inverse of [`TempoMap::ticks_to_samples`]: the walk
    /// accumulates the samples of each tempo segment until it reaches the
    /// requested one. At a tick boundary the answer is that exact tick,
    /// because the boundary is snapped to the tick whose sample index is
    /// the one asked for.
    pub fn samples_to_ticks(&self, samples: Samples, sample_rate: u32) -> i64 {
        if samples <= 0 {
            return 0;
        }
        let target = samples as f64;
        let mut cumulative = 0.0_f64;
        let mut estimate = 0.0_f64;
        for (index, event) in self.tempos.iter().enumerate() {
            let start = event.tick;
            let end = self.tempos.get(index + 1).map_or(i64::MAX, |next| next.tick);
            let per_tick = self.samples_per_tick(start, sample_rate);
            let segment = (end - start) as f64 * per_tick;
            if cumulative + segment >= target {
                estimate = start as f64 + (target - cumulative) / per_tick;
                break;
            }
            cumulative += segment;
            estimate = end as f64 + (target - cumulative) / per_tick;
        }
        let mut candidate = if estimate.is_finite() { estimate.round() as i64 } else { 0 };
        if candidate < 0 {
            candidate = 0;
        }
        // Snap to the tick whose sample index is the one asked for, so the
        // two conversions are inverses at a tick boundary.
        for distance in 0..=8_i64 {
            let low = candidate - distance;
            if low >= 0 && self.ticks_to_samples(low, sample_rate) == samples {
                return low;
            }
            let high = candidate + distance;
            if distance > 0 && self.ticks_to_samples(high, sample_rate) == samples {
                return high;
            }
        }
        candidate
    }

    /// The bar, beat and tick within the beat, at `tick`.
    ///
    /// The meter in force at the tick decides how long a bar is. A meter
    /// change starts a new bar: the bar count restarts from the tick the
    /// change happens at, because that is where the musician's foot is.
    pub fn bar_beat_at_tick(&self, tick: i64) -> BarBeat {
        let tick = tick.max(0);
        for (index, segment) in self.meter_segments().iter().enumerate() {
            let next_start = self.meters.get(index + 1).map_or(i64::MAX, |event| event.tick);
            if tick >= next_start {
                continue;
            }
            let per_beat = ticks_per_beat(segment.denominator);
            let per_bar = per_beat * segment.numerator as i64;
            let offset = tick - segment.start;
            let bar = segment.bars_before + offset / per_bar + 1;
            let within_bar = offset % per_bar;
            return BarBeat { bar, beat: within_bar / per_beat + 1, tick: within_bar % per_beat };
        }
        BarBeat { bar: 1, beat: 1, tick: 0 }
    }

    /// The meter segments of the map, in tick order.
    ///
    /// Each segment carries the tick it starts at, its meter, and how many
    /// bars come before it.
    fn meter_segments(&self) -> Vec<MeterSegment> {
        let mut segments = Vec::with_capacity(self.meters.len());
        let mut bars_before = 0_i64;
        for (index, event) in self.meters.iter().enumerate() {
            let start = event.tick;
            let end = self.meters.get(index + 1).map_or(i64::MAX, |next| next.tick);
            let per_beat = ticks_per_beat(event.denominator);
            let per_bar = per_beat * event.numerator as i64;
            let bars = if end == i64::MAX {
                i64::MAX
            } else {
                let span = end - start;
                (span + per_bar - 1) / per_bar
            };
            segments.push(MeterSegment {
                start,
                numerator: event.numerator,
                denominator: event.denominator,
                bars_before,
            });
            if bars != i64::MAX {
                bars_before += bars;
            }
        }
        segments
    }

    /// The tick a bar starts at, if there is such a bar.
    fn bar_start_tick(&self, bar: i64) -> Option<i64> {
        if bar < 1 {
            return None;
        }
        for (index, segment) in self.meter_segments().iter().enumerate() {
            let next_start = self.meters.get(index + 1).map_or(i64::MAX, |event| event.tick);
            let per_beat = ticks_per_beat(segment.denominator);
            let per_bar = per_beat * segment.numerator as i64;
            let bars = if next_start == i64::MAX {
                i64::MAX
            } else {
                let span = next_start - segment.start;
                (span + per_bar - 1) / per_bar
            };
            let first = segment.bars_before + 1;
            if bar >= first && bar - first < bars {
                return Some(segment.start + (bar - first) * per_bar);
            }
        }
        None
    }
}

/// One run of the meter timeline.
#[derive(Debug, Clone, Copy)]
struct MeterSegment {
    /// The tick the segment starts at.
    start: i64,
    /// How many beats are in a bar.
    numerator: u32,
    /// What a beat is worth.
    denominator: u32,
    /// How many bars come before this segment.
    bars_before: i64,
}

/// How many ticks a beat is worth, for a meter denominator.
fn ticks_per_beat(denominator: u32) -> i64 {
    TICKS_PER_QUARTER * 4 / denominator as i64
}

/// A number of samples, rounded to the nearest whole sample.
fn round_samples(value: f64) -> Samples {
    if !value.is_finite() {
        return 0;
    }
    let rounded = value.round();
    if rounded > MAX_POSITION as f64 {
        MAX_POSITION
    } else if rounded < 0.0 {
        0
    } else {
        rounded as i64
    }
}

/// How a position is written and read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFormat {
    /// Bars, beats and ticks: `bar.beat.tick`, as `3.2.480`.
    BarsBeats,
    /// Minutes and seconds: `m:ss.mmm`, as `1:30.500`.
    MinSecs,
    /// A plain sample index, as `72000`.
    Samples,
}

impl TimeFormat {
    /// Write a position in this format.
    pub fn format_position(&self, samples: Samples, map: &TempoMap, sample_rate: u32) -> String {
        match self {
            TimeFormat::BarsBeats => {
                let tick = map.samples_to_ticks(samples, sample_rate);
                let place = map.bar_beat_at_tick(tick);
                format!("{}.{}.{}", place.bar, place.beat, place.tick)
            }
            TimeFormat::MinSecs => {
                let millis = if sample_rate == 0 {
                    0
                } else {
                    (samples as f64 * 1000.0 / sample_rate as f64).round() as i64
                };
                let millis = millis.max(0);
                let minutes = millis / 60_000;
                let seconds = (millis % 60_000) / 1000;
                let rest = millis % 1000;
                format!("{minutes}:{seconds:02}.{rest:03}")
            }
            TimeFormat::Samples => format!("{samples}"),
        }
    }

    /// Read a position in this format.
    ///
    /// Empty text, a number that is not finite, a negative position and a
    /// position past [`MAX_POSITION`] are all refused.
    pub fn parse_position(&self, text: &str, map: &TempoMap, sample_rate: u32) -> Result<Samples, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("a position cannot be empty".to_string());
        }
        match self {
            TimeFormat::BarsBeats => self.parse_bars_beats(text, map, sample_rate),
            TimeFormat::MinSecs => self.parse_min_secs(text, sample_rate),
            TimeFormat::Samples => {
                let value = parse_number(text)?;
                if value < 0.0 {
                    return Err("a position cannot be negative".to_string());
                }
                if value > MAX_POSITION as f64 {
                    return Err(format!("a position cannot be past {MAX_POSITION} samples"));
                }
                if value.fract() != 0.0 {
                    return Err(format!("a sample position is a whole number of samples, not {text}"));
                }
                Ok(value as i64)
            }
        }
    }

    /// Read a `bar.beat.tick` position.
    fn parse_bars_beats(&self, text: &str, map: &TempoMap, sample_rate: u32) -> Result<Samples, String> {
        let parts: Vec<&str> = text.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return Err(format!("'{text}' is not a bar.beat.tick position"));
        }
        let bar = parse_integer(parts[0], "bar")?;
        if bar < 1 {
            return Err(format!("a bar counts from 1, not {bar}"));
        }
        let beat = if parts.len() > 1 { parse_integer(parts[1], "beat")? } else { 1 };
        if beat < 1 {
            return Err(format!("a beat counts from 1, not {beat}"));
        }
        let tick = if parts.len() > 2 { parse_integer(parts[2], "tick")? } else { 0 };
        if tick < 0 {
            return Err(format!("a tick cannot be negative, not {tick}"));
        }
        let bar_start = map.bar_start_tick(bar).ok_or_else(|| format!("there is no bar {bar}"))?;
        let (numerator, denominator) = map.meter_at_tick(bar_start);
        if beat > numerator as i64 {
            return Err(format!("beat {beat} is past the end of a {numerator}/{denominator} bar"));
        }
        let per_beat = ticks_per_beat(denominator);
        if tick >= per_beat {
            return Err(format!("tick {tick} is past the end of a beat of {per_beat} ticks"));
        }
        let absolute = bar_start + (beat - 1) * per_beat + tick;
        if map.exact_samples(absolute, sample_rate) > MAX_POSITION as f64 {
            return Err(format!("a position cannot be past {MAX_POSITION} samples"));
        }
        Ok(map.ticks_to_samples(absolute, sample_rate))
    }

    /// Read a `m:ss.mmm` position.
    fn parse_min_secs(&self, text: &str, sample_rate: u32) -> Result<Samples, String> {
        let (minutes, seconds) = match text.split_once(':') {
            Some((minutes, seconds)) => {
                let minutes = parse_number(minutes)?;
                let seconds = parse_number(seconds)?;
                if !(0.0..60.0).contains(&seconds) {
                    return Err(format!("the seconds of a position are 0 to 59, not {seconds}"));
                }
                (minutes, seconds)
            }
            None => (0.0, parse_number(text)?),
        };
        if !minutes.is_finite() || !seconds.is_finite() {
            return Err("a position must be a finite number".to_string());
        }
        if minutes < 0.0 || seconds < 0.0 {
            return Err("a position cannot be negative".to_string());
        }
        let total = minutes * 60.0 + seconds;
        let exact = total * sample_rate as f64;
        if !exact.is_finite() || exact > MAX_POSITION as f64 {
            return Err(format!("a position cannot be past {MAX_POSITION} samples"));
        }
        Ok(round_samples(exact))
    }
}

/// A number read out of a position string.
fn parse_number(text: &str) -> Result<f64, String> {
    let value: f64 = text.trim().parse().map_err(|_| format!("'{text}' is not a number"))?;
    if !value.is_finite() {
        return Err(format!("'{text}' is not a finite number"));
    }
    Ok(value)
}

/// A whole number read out of a position string.
fn parse_integer(text: &str, what: &str) -> Result<i64, String> {
    let value: i64 = text.trim().parse().map_err(|_| format!("'{text}' is not a {what}"))?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn map_with_two_tempos() -> TempoMap {
        let mut map = TempoMap::new();
        map.set_tempo(0, 120.0).expect("120 bpm is allowed");
        map.set_tempo(960, 240.0).expect("240 bpm is allowed");
        map
    }

    #[test]
    fn a_new_map_is_120_bpm_and_4_4() {
        let map = TempoMap::new();
        assert_eq!(map.tempos().len(), 1);
        assert_eq!(map.meters().len(), 1);
        assert_eq!(map.tempo_at_tick(0), 120.0);
        assert_eq!(map.meter_at_tick(0), (4, 4));
        assert_eq!(map.tempo_at_tick(9_999), 120.0);
        assert_eq!(map.meter_at_tick(9_999), (4, 4));
    }

    #[test]
    fn a_tempo_holds_until_the_next_one() {
        let map = map_with_two_tempos();
        assert_eq!(map.tempo_at_tick(0), 120.0);
        assert_eq!(map.tempo_at_tick(959), 120.0);
        assert_eq!(map.tempo_at_tick(960), 240.0);
        assert_eq!(map.tempo_at_tick(100_000), 240.0);
    }

    #[test]
    fn a_tempo_at_the_same_tick_replaces_it() {
        let mut map = TempoMap::new();
        map.set_tempo(0, 90.0).expect("90 bpm is allowed");
        assert_eq!(map.tempos().len(), 1);
        assert_eq!(map.tempo_at_tick(0), 90.0);
        map.set_tempo(480, 100.0).expect("100 bpm is allowed");
        map.set_tempo(480, 110.0).expect("110 bpm is allowed");
        assert_eq!(map.tempos().len(), 2);
        assert_eq!(map.tempo_at_tick(480), 110.0);
    }

    #[test]
    fn tempo_events_stay_sorted() {
        let mut map = TempoMap::new();
        map.set_tempo(960, 100.0).expect("100 bpm is allowed");
        map.set_tempo(0, 130.0).expect("130 bpm is allowed");
        map.set_tempo(480, 140.0).expect("140 bpm is allowed");
        let ticks: Vec<i64> = map.tempos().iter().map(|event| event.tick).collect();
        assert_eq!(ticks, vec![0, 480, 960]);
    }

    #[test]
    fn a_bad_tempo_is_refused() {
        let mut map = TempoMap::new();
        assert!(map.set_tempo(0, f64::NAN).is_err());
        assert!(map.set_tempo(0, f64::INFINITY).is_err());
        assert!(map.set_tempo(0, 4.999).is_err());
        assert!(map.set_tempo(0, 1000.1).is_err());
        assert!(map.set_tempo(-1, 120.0).is_err());
        assert!(map.set_tempo(MAX_POSITION + 1, 120.0).is_err());
        assert!(map.set_tempo(0, 5.0).is_ok());
        assert!(map.set_tempo(0, 1000.0).is_ok());
    }

    #[test]
    fn a_bad_meter_is_refused() {
        let mut map = TempoMap::new();
        assert!(map.set_meter(0, 0, 4).is_err());
        assert!(map.set_meter(0, 65, 4).is_err());
        assert!(map.set_meter(0, 4, 3).is_err());
        assert!(map.set_meter(0, 4, 0).is_err());
        assert!(map.set_meter(-1, 4, 4).is_err());
        assert!(map.set_meter(0, 64, 64).is_ok());
        assert!(map.set_meter(0, 4, 4).is_ok());
    }

    #[test]
    fn a_meter_at_the_same_tick_replaces_it() {
        let mut map = TempoMap::new();
        map.set_meter(0, 3, 4).expect("3/4 is allowed");
        assert_eq!(map.meters().len(), 1);
        assert_eq!(map.meter_at_tick(0), (3, 4));
        map.set_meter(1920, 7, 8).expect("7/8 is allowed");
        map.set_meter(1920, 5, 8).expect("5/8 is allowed");
        assert_eq!(map.meters().len(), 2);
        assert_eq!(map.meter_at_tick(1920), (5, 8));
    }

    #[test]
    fn a_tick_is_a_fixed_number_of_ticks_per_quarter() {
        let map = TempoMap::new();
        // 120 bpm at 48 kHz: a quarter note is half a second, so a tick is
        // a fiftieth of a second.
        assert!((map.samples_per_tick(0, SR) - 25.0).abs() < 1e-9);
        let mut faster = TempoMap::new();
        faster.set_tempo(0, 240.0).expect("240 bpm is allowed");
        assert!((faster.samples_per_tick(0, SR) - 12.5).abs() < 1e-9);
    }

    #[test]
    fn the_walk_over_tempo_segments_adds_up() {
        let map = map_with_two_tempos();
        // The first 960 ticks are at 120 bpm: 25 samples a tick.
        assert_eq!(map.ticks_to_samples(960, SR), 24_000);
        // The next 960 ticks are at 240 bpm: 12.5 samples a tick.
        assert_eq!(map.ticks_to_samples(1920, SR), 36_000);
        assert_eq!(map.ticks_to_samples(0, SR), 0);
        assert_eq!(map.ticks_to_samples(-10, SR), 0);
    }

    #[test]
    fn the_two_conversions_are_inverses_at_tick_boundaries() {
        let map = map_with_two_tempos();
        for tick in [0_i64, 1, 959, 960, 961, 1920, 3840, 9_999] {
            let samples = map.ticks_to_samples(tick, SR);
            assert_eq!(map.samples_to_ticks(samples, SR), tick, "tick {tick}");
        }
    }

    #[test]
    fn a_bar_holds_four_quarter_notes_by_default() {
        let map = TempoMap::new();
        assert_eq!(map.bar_beat_at_tick(0), BarBeat { bar: 1, beat: 1, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(959), BarBeat { bar: 1, beat: 1, tick: 959 });
        assert_eq!(map.bar_beat_at_tick(960), BarBeat { bar: 1, beat: 2, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(1920), BarBeat { bar: 1, beat: 3, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(2880), BarBeat { bar: 1, beat: 4, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(3840), BarBeat { bar: 2, beat: 1, tick: 0 });
    }

    #[test]
    fn a_meter_change_starts_a_new_bar() {
        let mut map = TempoMap::new();
        map.set_meter(3840, 3, 4).expect("3/4 is allowed");
        // The last beat of the old bar.
        assert_eq!(map.bar_beat_at_tick(3839), BarBeat { bar: 1, beat: 4, tick: 959 });
        // The new meter starts a new bar of three beats.
        assert_eq!(map.bar_beat_at_tick(3840), BarBeat { bar: 2, beat: 1, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(4800), BarBeat { bar: 2, beat: 2, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(6720), BarBeat { bar: 3, beat: 1, tick: 0 });
    }

    #[test]
    fn a_different_denominator_changes_the_beat() {
        let mut map = TempoMap::new();
        map.set_meter(0, 4, 8).expect("4/8 is allowed");
        // A beat of an eighth is 480 ticks.
        assert_eq!(map.bar_beat_at_tick(480), BarBeat { bar: 1, beat: 2, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(960), BarBeat { bar: 1, beat: 3, tick: 0 });
        assert_eq!(map.bar_beat_at_tick(1920), BarBeat { bar: 2, beat: 1, tick: 0 });
    }

    #[test]
    fn bars_and_beats_round_trip() {
        let map = TempoMap::new();
        let text = TimeFormat::BarsBeats.format_position(24_000, &map, SR);
        assert_eq!(text, "1.2.0");
        assert_eq!(TimeFormat::BarsBeats.parse_position("1.2.0", &map, SR), Ok(24_000));
        assert_eq!(TimeFormat::BarsBeats.parse_position("1.2", &map, SR), Ok(24_000));
        assert_eq!(TimeFormat::BarsBeats.parse_position("1", &map, SR), Ok(0));
        assert_eq!(TimeFormat::BarsBeats.parse_position("2.1.0", &map, SR), Ok(96_000));
    }

    #[test]
    fn bars_and_beats_use_the_tempo_map() {
        let mut map = TempoMap::new();
        map.set_tempo(0, 240.0).expect("240 bpm is allowed");
        // At 240 bpm a quarter note is a quarter of a second: 12 000
        // samples.
        let text = TimeFormat::BarsBeats.format_position(12_000, &map, SR);
        assert_eq!(text, "1.2.0");
    }

    #[test]
    fn a_beat_past_the_meter_is_refused() {
        let map = TempoMap::new();
        assert!(TimeFormat::BarsBeats.parse_position("1.5.0", &map, SR).is_err());
        assert!(TimeFormat::BarsBeats.parse_position("0.1.0", &map, SR).is_err());
        assert!(TimeFormat::BarsBeats.parse_position("1.0.0", &map, SR).is_err());
        assert!(TimeFormat::BarsBeats.parse_position("1.1.960", &map, SR).is_err());
    }

    #[test]
    fn minutes_and_seconds_round_trip() {
        let map = TempoMap::new();
        // 90.5 seconds at 48 kHz.
        let samples: Samples = 90 * SR as i64 + SR as i64 / 2;
        let text = TimeFormat::MinSecs.format_position(samples, &map, SR);
        assert_eq!(text, "1:30.500");
        assert_eq!(TimeFormat::MinSecs.parse_position("1:30.500", &map, SR), Ok(samples));
        assert_eq!(TimeFormat::MinSecs.parse_position("1:30", &map, SR), Ok(90 * SR as i64));
        assert_eq!(TimeFormat::MinSecs.parse_position("1.5", &map, SR), Ok(72_000));
        assert_eq!(TimeFormat::MinSecs.parse_position("1.500", &map, SR), Ok(72_000));
        assert_eq!(TimeFormat::MinSecs.format_position(0, &map, SR), "0:00.000");
    }

    #[test]
    fn samples_round_trip() {
        let map = TempoMap::new();
        assert_eq!(TimeFormat::Samples.format_position(72_000, &map, SR), "72000");
        assert_eq!(TimeFormat::Samples.parse_position("72000", &map, SR), Ok(72_000));
        assert_eq!(TimeFormat::Samples.parse_position("0", &map, SR), Ok(0));
    }

    #[test]
    fn a_bad_position_is_refused() {
        let map = TempoMap::new();
        for format in [TimeFormat::BarsBeats, TimeFormat::MinSecs, TimeFormat::Samples] {
            assert!(format.parse_position("", &map, SR).is_err());
            assert!(format.parse_position("   ", &map, SR).is_err());
            assert!(format.parse_position("-1", &map, SR).is_err());
            assert!(format.parse_position("nonsense", &map, SR).is_err());
            assert!(format.parse_position("inf", &map, SR).is_err());
            assert!(format.parse_position("nan", &map, SR).is_err());
            let past = (MAX_POSITION + 1).to_string();
            assert!(format.parse_position(&past, &map, SR).is_err());
        }
    }

    #[test]
    fn a_position_past_the_maximum_is_refused() {
        let map = TempoMap::new();
        assert!(TimeFormat::Samples.parse_position(&MAX_POSITION.to_string(), &map, SR).is_ok());
        assert!(TimeFormat::Samples.parse_position(&(MAX_POSITION + 1).to_string(), &map, SR).is_err());
    }
}
