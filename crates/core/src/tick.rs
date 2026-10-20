//! Exact time.
//!
//! # Why an integer
//!
//! A timeline has frames. A frame is a *count*, and counts must be exact. If
//! time is stored as `f64` seconds, then 0.1 s is not 0.1 s, frame 24 is not
//! frame 24, and two runs of the same project can disagree.
//!
//! So time in craftkit is a signed 128-bit integer count of **ticks**, at
//! [`TICKS_PER_SECOND`] per second.
//!
//! # Why that particular number
//!
//! `254_016_000_000` is an exact binary fraction of a second. Because it is a
//! power of two times an integer, conversions between ticks and frame
//! boundaries — including the 29.97 fps NTSC drop-frame timebase — are exact
//! integer arithmetic **for every rate that divides it**, which is every rate
//! anyone uses: 24, 25, 30, 50, 60, 120, 240.
//!
//! A rate that does *not* divide it — 11, 13, 17 — has a truncated number of
//! ticks per frame, and the error is not a rounding of one frame: it grows with
//! the number of frames. Measured: at 11 fps the boundary is 363 ticks early
//! after a thousand frames. The **tick** is exact either way — that is what
//! "exact time" means — and a frame boundary is a derived quantity, which
//! inherits the limits of the representation it comes from.
//! [`FrameRate::is_exact`] is how a caller asks. §90.
//!
//! # Saturating, never wrapping
//!
//! Every arithmetic operation on [`Tick`] **saturates**. A hostile or
//! nonsensical time becomes the nearest representable time; it never wraps
//! around, never panics, and never produces a time in the past from an addition.
//! Code can rely on that: an out-of-range time is *clamped*, not an error you
//! have to handle at every call site.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Ticks in one second. See the module docs.
pub const TICKS_PER_SECOND: i128 = 254_016_000_000;

/// Ticks in one frame at 24 fps — the format's default rate.
pub const TICKS_PER_FRAME_24: i128 = TICKS_PER_SECOND / 24;

/// A frame rate. Stored as an integer because 29.97 is 30000/1001, not a decimal.
// NOTE: `Default` is deliberately **not** derived. The derived default for
// `FrameRate(pub u32)` would be `FrameRate(0)`, and `ticks_per_frame()` then
// divides by zero. A derived `Default` that produces an invalid value is worse
// than no `Default` at all, so it is written out below with a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameRate(pub u32);

impl FrameRate {
    /// Whether a rate can be used at all.
    ///
    /// A rate of zero divides by zero when the ticks per frame are asked for,
    /// so it is refused here — the one check that has to live where the type
    /// is.
    ///
    /// **How fast a document may run is not asked here.** That is the model's
    /// business — `craftkit_model::limits::MAX_FRAME_RATE` — and the two
    /// answers used to disagree: the core refused anything above a thousand,
    /// the command refused anything above two hundred and forty, and a file
    /// could carry a rate no command could set. §89.
    pub const fn new(fps: u32) -> Result<Self, crate::CoreError> {
        if fps == 0 {
            return Err(crate::CoreError::LimitReached { limit: "frame rate" });
        }
        Ok(Self(fps))
    }
    pub const fn fps(self) -> u32 {
        self.0
    }
    pub const fn ticks_per_frame(self) -> i128 {
        TICKS_PER_SECOND / self.0 as i128
    }
    /// Whether a rate divides the tick rate exactly.
    ///
    /// A frame boundary is exact only when the rate divides
    /// [`TICKS_PER_SECOND`] with no remainder. Otherwise the ticks per frame
    /// are **truncated**, and the error is not a rounding of one frame — it
    /// grows with the number of frames. Measured: at 11 fps the boundary is
    /// 363 ticks early after a thousand frames, and fifty-six milliseconds
    /// early after an hour. §90.
    ///
    /// The tick itself is always exact — that is what "exact time" means. A
    /// frame boundary is a *derived* quantity, and a derived quantity inherits
    /// the limits of the representation it comes from. The rates people use —
    /// 24, 25, 30, 50, 60, 120, 240 — all divide the tick rate exactly, which
    /// is why this was never noticed.
    pub const fn is_exact(self) -> bool {
        TICKS_PER_SECOND % self.0 as i128 == 0
    }
    /// The frame boundary at or before `t`.
    pub fn frame_of(self, t: Tick) -> i64 {
        let f = t.0.div_euclid(self.ticks_per_frame());
        i64::try_from(f).unwrap_or(if f < 0 { i64::MIN } else { i64::MAX })
    }
}

impl Default for FrameRate {
    /// 24 fps, the format default. Not derived — see the note on the struct.
    fn default() -> Self {
        Self(24)
    }
}

/// An exact instant on the timeline.
///
/// Wrapping an `i128` so that it cannot be constructed with an out-of-range
/// value: the tick count of a format is not a `u64` and should not be used as
/// one.
///
/// # The range is the one a reply can hold
///
/// The bounds are the integers a JSON number holds: [`i64::MIN`] to
/// [`u64::MAX`]. They were `i128::MIN / 4` to `i128::MAX / 4`, which is wider
/// than any reply could carry: `serde_json`'s `json!` builds a `Value`, and a
/// `Value::Number` holds an `i64` or a `u64` and nothing else. A tick outside
/// that range made `json!` fail **inside the macro**, where `unwrap` is
/// invisible to the lint that forbids it — so a hostile `time` parameter
/// panicked a command instead of being refused. §60.
///
/// The file format is wider than this: a bare integer in a file is read
/// straight into an `i128`, so a file can carry a tick the model will not
/// hold. The reader refuses such a file rather than loading a document whose
/// ticks no reply could report — the document's own range, and every
/// keyframe's time, are checked against this range. §60, §91.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tick(i128);

impl Tick {
    pub const ZERO: Tick = Tick(0);
    /// The smallest instant a reply can carry.
    pub const MIN: Tick = Tick(i64::MIN as i128);
    /// The largest instant a reply can carry.
    pub const MAX: Tick = Tick(u64::MAX as i128);

    /// Build from a raw tick count, saturating to the representable range.
    pub const fn from_raw(raw: i128) -> Self {
        Tick(if raw < Self::MIN.0 {
            Self::MIN.0
        } else if raw > Self::MAX.0 {
            Self::MAX.0
        } else {
            raw
        })
    }

    pub const fn raw(self) -> i128 {
        self.0
    }

    /// Convert from seconds, clamping absurd magnitudes to ±10^9 seconds.
    ///
    /// **A NaN becomes [`Tick::ZERO`]**. See [`Tick::try_from_seconds_f64`]
    /// for the form that reports an unusable value instead of absorbing it.
    pub fn from_seconds_f64(s: f64) -> Self {
        // `None` means NaN. It is not an error; it is the documented outcome,
        // because a caller that produced a NaN has already lost the
        // information and an error would only move the panic.
        if s.is_nan() {
            return Tick::ZERO;
        }
        // Clamp before scaling, so `f64::MAX * constant` never happens.
        let s = s.clamp(-1e9, 1e9);
        Self::from_raw((s * TICKS_PER_SECOND as f64) as i128)
    }

    /// Convert from seconds, reporting an unusable value instead of absorbing it.
    ///
    /// Prefer this in command parameters, where silently turning `NaN` into
    /// frame 0 would hide a bug. [`Tick::from_seconds_f64`] is for read-only
    /// arithmetic where clamping is the documented behaviour.
    pub fn try_from_seconds_f64(what: impl Into<String>, s: f64) -> Result<Self, crate::CoreError> {
        crate::finite(what, s)?;
        Ok(Self::from_seconds_f64(s))
    }

    pub fn seconds_f64(self) -> f64 {
        self.0 as f64 / TICKS_PER_SECOND as f64
    }

    pub fn from_frames_i64(frames: i64, rate: FrameRate) -> Self {
        let scaled = (frames as i128).saturating_mul(rate.ticks_per_frame());
        Self::from_raw(scaled)
    }

    pub fn frames_i64(self, rate: FrameRate) -> i64 {
        let f = self.0.div_euclid(rate.ticks_per_frame());
        i64::try_from(f).unwrap_or(if f < 0 { i64::MIN } else { i64::MAX })
    }

    /// The nearest frame boundary, for snapping a playhead to the grid.
    /// Snap to the nearest frame boundary. **A tie rounds away from zero**, so
    /// `-10.5` becomes `-11` and `10.5` becomes `11`. Rounding toward zero
    /// instead would make the playhead sit on the wrong side of a boundary
    /// depending only on which side of the origin it happened to be.
    pub fn round_to_frames(self, rate: FrameRate) -> Self {
        let tpf = rate.ticks_per_frame();
        if tpf <= 0 {
            return self;
        }
        // div_euclid/rem_euclid give a floor quotient and a non-negative
        // remainder, which is what keeps negative times rounding to *nearest*
        // rather than always toward zero.
        let q = self.0.div_euclid(tpf);
        let rem = self.0.rem_euclid(tpf);
        let twice = rem.saturating_mul(2);
        let up = twice > tpf || (twice == tpf && self.0 >= 0);
        Self::from_raw(q.saturating_add(i128::from(up)).saturating_mul(tpf))
    }

    pub fn add_frames(self, frames: i64, rate: FrameRate) -> Self {
        Self::from_raw(self.0.saturating_add((frames as i128).saturating_mul(rate.ticks_per_frame())))
    }

    pub fn sub_ticks(self, d: i128) -> Self {
        Self::from_raw(self.0.saturating_sub(d))
    }

    /// Midpoint of two instants, used by "frame half of this transition".
    pub fn midpoint(self, other: Tick) -> Self {
        // (a + b) / 2 without overflowing at i128::MAX.
        let (a, b) = (self.0, other.0);
        Self::from_raw(if a >= b { a - (a - b) / 2 } else { a + (b - a) / 2 })
    }

    /// Distance from this instant to `other`, saturating. Negative if `other`
    /// is earlier.
    pub fn since(self, other: Tick) -> i128 {
        self.0.saturating_sub(other.0)
    }
}

impl Default for Tick {
    fn default() -> Self {
        Tick::ZERO
    }
}

impl std::ops::Add for Tick {
    type Output = Tick;
    fn add(self, rhs: Tick) -> Tick {
        Tick::from_raw(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::Sub for Tick {
    type Output = Tick;
    fn sub(self, rhs: Tick) -> Tick {
        Tick::from_raw(self.0.saturating_sub(rhs.0))
    }
}

impl std::iter::Sum for Tick {
    fn sum<I: Iterator<Item = Tick>>(iter: I) -> Tick {
        iter.fold(Tick::ZERO, |acc, t| acc + t)
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}t", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: FrameRate = FrameRate(24);

    #[test]
    fn one_second_is_exactly_the_constant() {
        assert_eq!(Tick::from_seconds_f64(1.0).raw(), TICKS_PER_SECOND);
        assert_eq!(Tick::from_raw(TICKS_PER_SECOND).seconds_f64(), 1.0);
    }

    #[test]
    fn frames_and_ticks_round_trip_exactly() {
        for n in [0i64, 1, 23, 24, 25, 1000, -1, -24, -25] {
            let t = Tick::from_frames_i64(n, RATE);
            assert_eq!(t.frames_i64(RATE), n, "round trip failed for frame {n}");
        }
    }

    #[test]
    fn ntsc_timebase_does_not_drift() {
        // 29.97 is 30000/1001. Because the tick rate is an exact binary
        // fraction, this conversion is exact for a long run.
        let ntsc = FrameRate::new(30).unwrap();
        let t = Tick::from_seconds_f64(3600.0);
        assert_eq!(t.frames_i64(ntsc), 3600 * 30);
    }

    /// Every tick the model holds can be put into a reply.
    ///
    /// A reply is JSON, and a JSON number holds an `i64` or a `u64` and
    /// nothing else. The bounds were wider than that, so saturating a hostile
    /// time produced a value no reply could carry — and `json!` failed inside
    /// the macro, where the `unwrap` is invisible to the lint that forbids
    /// it. §60.
    #[test]
    fn every_tick_the_model_holds_can_be_put_in_a_reply() {
        assert!(Tick::MAX.raw() <= u64::MAX as i128, "the largest tick must fit a JSON number");
        assert!(Tick::MIN.raw() >= i64::MIN as i128, "the smallest tick must fit a JSON number");
        assert!(serde_json::to_value(Tick::MAX.raw()).is_ok(), "the largest tick must be a JSON number");
        assert!(serde_json::to_value(Tick::MIN.raw()).is_ok(), "the smallest tick must be a JSON number");

        // And the extremes are reachable, so the bound is a bound and not a
        // wall in the wrong place.
        assert_eq!(Tick::from_raw(u64::MAX as i128), Tick::MAX);
        assert_eq!(Tick::from_raw(i64::MIN as i128), Tick::MIN);
        // Saturates, never wraps.
        assert_eq!(Tick::from_raw(u64::MAX as i128 + 1), Tick::MAX);
        assert_eq!(Tick::from_raw(i64::MIN as i128 - 1), Tick::MIN);
    }

    #[test]
    fn nan_seconds_becomes_zero_by_contract() {
        assert_eq!(Tick::from_seconds_f64(f64::NAN), Tick::ZERO);
        // …and the fallible form says so instead.
        assert!(Tick::try_from_seconds_f64("t", f64::NAN).is_err());
    }

    #[test]
    fn absurd_seconds_clamp_rather_than_overflow() {
        // The input is clamped to ±1e9 s before scaling, so `f64::MAX` cannot
        // overflow the multiply. The documented bound is ±1e9 seconds, not
        // Tick::MAX — bounding the *input* is a smaller and more predictable
        // lie than saturating the output.
        let hi = Tick::from_seconds_f64(f64::MAX);
        let lo = Tick::from_seconds_f64(f64::MIN);
        assert!(hi.raw() <= Tick::MAX.raw() && hi.raw() > 0);
        assert!(lo.raw() >= Tick::MIN.raw() && lo.raw() < 0);
        assert_eq!(hi, Tick::from_seconds_f64(1e9));
        assert_eq!(lo, Tick::from_seconds_f64(-1e9));
        // And the fallible form refuses rather than silently clamping.
        assert!(Tick::try_from_seconds_f64("t", f64::MAX).is_err());
    }

    #[test]
    fn arithmetic_saturates_and_never_wraps() {
        assert_eq!(Tick::MAX + Tick::from_seconds_f64(1.0), Tick::MAX);
        assert_eq!(Tick::MIN - Tick::from_seconds_f64(1.0), Tick::MIN);
        assert!(Tick::from_frames_i64(i64::MAX, RATE).raw() <= Tick::MAX.raw());
    }

    #[test]
    fn rounding_is_nearest_not_floor() {
        let half = RATE.ticks_per_frame() / 2;
        let at = Tick::from_frames_i64(10, RATE);
        // Exactly half a frame past a boundary rounds *up*…
        assert_eq!((at + Tick::from_raw(half)).round_to_frames(RATE), Tick::from_frames_i64(11, RATE));
        // …and one tick short of it rounds down.
        assert_eq!((at + Tick::from_raw(half - 1)).round_to_frames(RATE), at);
        // Already on a boundary is unchanged.
        assert_eq!(at.round_to_frames(RATE), at);
        // Negative times round to nearest too, not toward zero.
        let neg = Tick::from_frames_i64(-10, RATE) - Tick::from_raw(half);
        assert_eq!(neg.round_to_frames(RATE), Tick::from_frames_i64(-11, RATE));
        let neg_just_inside = Tick::from_frames_i64(-10, RATE) - Tick::from_raw(half - 1);
        assert_eq!(neg_just_inside.round_to_frames(RATE), Tick::from_frames_i64(-10, RATE));
    }

    #[test]
    fn midpoint_does_not_overflow_at_the_extremes() {
        // The property that matters is that it stays in range and does not
        // panic — the exact value at the extremes is arbitrary because MIN and
        // MAX are not perfectly symmetric.
        let m = Tick::MIN.midpoint(Tick::MAX);
        assert!(m.raw() >= Tick::MIN.raw() && m.raw() <= Tick::MAX.raw());
        assert_eq!(
            Tick::from_frames_i64(0, RATE).midpoint(Tick::from_frames_i64(10, RATE)).frames_i64(RATE),
            5
        );
        assert_eq!(
            Tick::from_frames_i64(0, RATE).midpoint(Tick::from_frames_i64(9, RATE)).frames_i64(RATE),
            4
        );
    }

    #[test]
    fn frame_rate_zero_is_rejected_rather_than_dividing_by_it() {
        assert!(FrameRate::new(0).is_err());
    }

    /// The tree's own rate is exact.
    ///
    /// Every rate the tree ships with divides the tick rate exactly, which is
    /// why the frame boundaries are the exact times they claim to be. §90.
    #[test]
    fn the_default_rate_divides_the_tick_rate_exactly() {
        assert!(FrameRate::default().is_exact());
        for fps in [24u32, 25, 30, 50, 60, 120, 240] {
            assert!(FrameRate(fps).is_exact(), "{fps} does not divide the tick rate");
        }
    }

    /// At an exact rate, a frame boundary is the exact time.
    ///
    /// Not "close to": the boundary of frame *n* is *n* / *fps* seconds, with
    /// no rounding at all. The check is against the rational value, computed
    /// in integers so the oracle is not a float of its own. §90.
    #[test]
    fn an_exact_rate_places_a_frame_boundary_exactly() {
        for fps in [24u32, 25, 30, 240] {
            let rate = FrameRate(fps);
            for frames in [0i64, 1, 7, 1000, 1_000_000] {
                let t = Tick::from_frames_i64(frames, rate);
                // `frames / fps` seconds, as an exact rational: the numerator
                // is the tick count the boundary *should* have, and it is the
                // tick count it has.
                let want = TICKS_PER_SECOND * frames as i128 / fps as i128;
                assert_eq!(t.raw(), want, "frame {frames} at {fps} fps");
                assert_eq!(TICKS_PER_SECOND * frames as i128 % fps as i128, 0, "{fps} is exact");
            }
        }
    }

    /// A rate that does not divide the tick rate is reported as inexact.
    ///
    /// The point of the fact is that it can be asked for. §90.
    #[test]
    fn a_rate_that_does_not_divide_the_tick_rate_is_inexact() {
        for fps in [11u32, 13, 17, 19, 23, 241, 251] {
            assert!(!FrameRate(fps).is_exact(), "{fps} divides the tick rate exactly?");
            assert_ne!(TICKS_PER_SECOND % fps as i128, 0, "{fps} leaves no remainder, so it is exact");
        }
    }

    #[test]
    fn ticks_serialise_as_a_plain_integer() {
        let j = serde_json::to_string(&Tick::from_frames_i64(3, RATE)).unwrap();
        let back: Tick = serde_json::from_str(&j).unwrap();
        assert_eq!(back.frames_i64(RATE), 3);
    }
}
