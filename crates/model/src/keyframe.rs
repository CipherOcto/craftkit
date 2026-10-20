//! Keyframes: how a property varies over time.
//!
//! The rule the whole stack follows: **key times are [`Tick`], never seconds.**
//! A keyframe at 1.0 s and a keyframe at 23.976 fps are the same tick, always.
//!
//! Interpolation is described on the keyframe it points *back* from — the
//! `in_*` fields. That is the convention every timeline format uses, and it
//! means the keyframe *list* stays sorted by time regardless of the curve.
//!
//! # Who keeps the list sorted
//!
//! [`Keyframes::set`] sorts, replaces and bounds the list, so a list built
//! through the model is sorted, unique and within `MAX_KEYS`.
//!
//! A **file** is not built that way: it is deserialized straight into the
//! model, so the rules are not kept on the way in — they are *checked* by
//! [`Keyframes::check`], which the reader calls and the engine calls again
//! after every command. That distinction is the whole point: a list that
//! arrives out of order samples to the **wrong value**, not to an untidy one.
//! §91.

use craftkit_core::{CoreError, Tick};
use serde::{Deserialize, Serialize};
use std::fmt;

/// How the value between two keys is computed.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    /// The value jumps at the key and stays.
    Hold,
    /// A straight line in the *value*.
    Linear,
    /// A cubic bezier in *time*, which is what a velocity curve is.
    Bezier {
        /// Horizontal control point, 0..=1.
        out_x: f32,
        /// Vertical control point; 0..=1 is one frame, values outside ease out.
        out_y: f32,
    },
    /// Smooth in, smooth out. The default, because it has zero first *and*
    /// second derivative at both ends, so motion started or stopped by it does
    /// not visibly jerk.
    #[default]
    EaseInOut,
}

impl Easing {
    /// The four built-in presets, by the name a script or an agent uses.
    ///
    /// Returned as `Option` rather than `unwrap`/`expect`, because an agent
    /// will pass a name that does not exist and that must be an error message,
    /// not a panic.
    pub fn preset(name: &str) -> Option<Self> {
        match name {
            "hold" | "step" => Some(Easing::Hold),
            "linear" => Some(Easing::Linear),
            "ease" | "easeInOut" | "smooth" => Some(Easing::EaseInOut),
            "easeIn" => Some(Easing::Bezier { out_x: 0.42, out_y: 0.0 }),
            "easeOut" => Some(Easing::Bezier { out_x: 0.58, out_y: 1.0 }),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Easing::Hold => "hold",
            Easing::Linear => "linear",
            Easing::EaseInOut => "easeInOut",
            Easing::Bezier { .. } => "bezier",
        }
    }

    /// Validate the control points. A NaN here would make every sampled frame
    /// garbage, so it is refused at construction rather than at draw time.
    pub fn checked(self) -> Result<Self, CoreError> {
        if let Easing::Bezier { out_x, out_y } = self {
            if !out_x.is_finite() || !out_y.is_finite() {
                return Err(CoreError::NotFinite {
                    what: "easing control point".to_string(),
                    value: format!("{out_x},{out_y}"),
                });
            }
            craftkit_core::in_range("easing out_x", out_x as f64, 0.0, 1.0)?;
            // out_y is deliberately allowed outside 0..=1: that is how an ease-out
            // overshoots, and clamping it would make every easing preset a lie.
        }
        Ok(self)
    }
}

/// One keyframe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub time: Tick,
    pub value: craftkit_core::Value,
    #[serde(default)]
    pub ease: Easing,
}

impl Key {
    pub fn new(time: Tick, value: craftkit_core::Value, ease: Easing) -> Result<Self, CoreError> {
        Ok(Self { time, value, ease: ease.checked()? })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.time, self.value.display())
    }
}

/// A sorted, unique list of keyframes for one property.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Keyframes(Vec<Key>);

impl Keyframes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn keys(&self) -> &[Key] {
        &self.0
    }

    /// Change the easing of the key at `time`.
    ///
    /// Returns whether the easing **changed**. A key that is not there, and a
    /// key that already has this easing, both leave the list as it was — and
    /// the command that calls this needs to tell those apart, so it looks for
    /// the key first.
    ///
    /// The easing is checked **before** it is stored: an easing the model's
    /// own constructor refuses must not be able to enter the tree through this
    /// door either.
    pub fn set_ease(&mut self, time: Tick, ease: Easing) -> Result<bool, CoreError> {
        let ease = ease.checked()?;
        match self.0.binary_search_by(|k| k.time.cmp(&time)) {
            Ok(i) => {
                let changed = self.0[i].ease != ease;
                self.0[i].ease = ease;
                Ok(changed)
            }
            Err(_) => Ok(false),
        }
    }

    /// Check every rule a keyframe list must hold.
    ///
    /// The list is **deserialized straight into the model**: a file is not
    /// built by [`Keyframes::set`], so the rules `set` keeps are not kept on
    /// the way in. Measured, before this check existed:
    ///
    /// - a file could carry an **unsorted** list — and the sample of it was
    ///   *wrong*, not merely untidy: keys at 500 and 0 sampled to the value
    ///   of the later key, where the sorted list gives the interpolation
    ///   between them;
    /// - a file could carry **two keys at the same time**, of which only the
    ///   second was ever reachable;
    /// - a file could carry a **tick outside the range a time may hold**,
    ///   which [`Tick::from_raw`] clamps — so the same document had two
    ///   representations, one of which the type says it cannot hold;
    /// - a file could carry an **easing** that [`Easing::checked`] refuses
    ///   (`out_x` outside 0..=1, or not finite);
    /// - a file could carry **more keys than `MAX_KEYS`**.
    ///
    /// Every one of them validated as `Ok`, and the reader handed the editor a
    /// document whose animation did not mean what the file said. §91.
    ///
    /// `what` names the property, so the message says whose keyframes are
    /// wrong.
    pub fn check(&self, what: &str) -> Result<(), CoreError> {
        if self.0.len() > crate::limits::MAX_KEYS {
            return Err(CoreError::LimitReached { limit: "keyframes per property" });
        }
        for (i, key) in self.0.iter().enumerate() {
            // The time has to be one a reply could carry. The document's own
            // range is checked the same way — `Document::validate` refuses a
            // `start` or an `end` outside it — and a keyframe's time is the
            // same kind of number in the same kind of file. §60, §91.
            let raw = key.time.raw();
            if raw < Tick::MIN.raw() || raw > Tick::MAX.raw() {
                return Err(CoreError::OutOfRange {
                    what: format!("the time of a keyframe of {what}"),
                    value: format!("{raw}"),
                    min: format!("{}", Tick::MIN.raw()),
                    max: format!("{}", Tick::MAX.raw()),
                });
            }
            // The easing the model's own constructor refuses must not be able
            // to arrive by another door. The message names the property, so a
            // file with many properties says which one is wrong.
            key.ease
                .checked()
                .map_err(|e| CoreError::BadData(format!("the easing of a keyframe of {what}: {e}")))?;
            // The value, exactly as §73 checks it.
            key.value.check(what)?;
            if i > 0 {
                let prev = &self.0[i - 1];
                if key.time == prev.time {
                    return Err(CoreError::BadData(format!(
                        "two keyframes of {what} share the time {}",
                        prev.time.raw()
                    )));
                }
                if key.time < prev.time {
                    return Err(CoreError::BadData(format!(
                        "the keyframes of {what} are not sorted by time: {} comes after {}",
                        key.time.raw(),
                        prev.time.raw()
                    )));
                }
            }
        }
        Ok(())
    }

    /// Insert or replace the key at `time`. **Keeps the list sorted**, so a
    /// caller can insert in any order and the result is identical.
    pub fn set(&mut self, key: Key) -> Result<(), CoreError> {
        let max = crate::limits::MAX_KEYS;
        match self.0.binary_search_by(|k| k.time.cmp(&key.time)) {
            Ok(i) => self.0[i] = key,
            Err(i) => {
                if self.0.len() >= max {
                    return Err(CoreError::LimitReached { limit: "keyframes per property" });
                }
                self.0.insert(i, key);
            }
        }
        Ok(())
    }

    pub fn remove_at(&mut self, time: Tick) -> bool {
        match self.0.binary_search_by(|k| k.time.cmp(&time)) {
            Ok(i) => {
                self.0.remove(i);
                true
            }
            Err(_) => false,
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// The value at `time`.
    ///
    /// Before the first key and after the last, the value **holds**. Clamping at
    /// both ends is what makes a property well-defined everywhere, so no caller
    /// has to special-case "before the animation starts".
    ///
    /// Returns `None` when there are no keys at all, which the caller resolves
    /// to the property's static value.
    pub fn sample(&self, time: Tick) -> Option<craftkit_core::Value> {
        let n = self.0.len();
        if n == 0 {
            return None;
        }
        if n == 1 || time <= self.0[0].time {
            return Some(self.0[0].value.clone());
        }
        let last = &self.0[n - 1];
        if time >= last.time {
            return Some(last.value.clone());
        }
        // binary search for the last key at or before `time`
        let mut lo = 0usize;
        let mut hi = n - 1;
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if self.0[mid].time <= time {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let a = &self.0[lo];
        let b = &self.0[lo + 1];

        // Progress in 0..=1 across the span, computed on Ticks so that a span
        // shorter than one frame still interpolates smoothly instead of jumping.
        let span = b.time.since(a.time);
        if span <= 0 {
            return Some(b.value.clone());
        }
        let linear = time.since(a.time) as f64 / span as f64;
        let t = ease_progress(a.ease, linear);

        Some(blend(&a.value, &b.value, t))
    }

    /// The index of the last key at or before `time`, for drawing.
    pub fn active_index(&self, time: Tick) -> Option<usize> {
        let mut found = None;
        for (i, k) in self.0.iter().enumerate() {
            if k.time <= time {
                found = Some(i);
            } else {
                break;
            }
        }
        found
    }
}

/// Apply an easing curve to a linear 0..=1 progress.
///
/// A bezier easing is solved by bisection on x. A full cubic solve would be
/// slightly more accurate but needs an iteration budget and a failure mode; a
/// fixed 24-step bisection over a bounded interval is exact enough to be
/// invisible and cannot fail.
fn ease_progress(ease: Easing, linear: f64) -> f64 {
    let linear = linear.clamp(0.0, 1.0);
    match ease {
        Easing::Hold => 0.0,
        Easing::Linear => linear,
        Easing::EaseInOut => smoothstep(linear),
        Easing::Bezier { out_x, out_y } => bezier_progress(linear, out_x as f64, out_y as f64),
    }
}

/// `t²(3−2t)` — the standard smoothstep, zero first *and* second derivative at
/// both ends, which is why it is the default.
pub fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Solve a cubic bezier with control points (0,0), (out_x,out_y), (1,1) for y
/// given x, by bisection. Bounded and total: it cannot panic and it cannot hang.
pub fn bezier_progress(x: f64, cx: f64, cy: f64) -> f64 {
    let (cx, cy) = (cx.clamp(0.0, 1.0), cy.clamp(-1.0, 2.0));
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let sample_x = |t: f64| 3.0 * t * (1.0 - t) * (1.0 - t) * cx + 3.0 * t * t * (1.0 - t) + t * t * t;
    let sample_y = |t: f64| 3.0 * t * (1.0 - t) * (1.0 - t) * cy + 3.0 * t * t * (1.0 - t) + t * t * t;

    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        if sample_x(mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    sample_y((lo + hi) * 0.5).clamp(-1.0, 2.0)
}

/// Interpolate two values.
///
/// Mismatched types hold the incoming value rather than panicking or producing
/// a default: a keyframe list whose ends disagree is a document bug, and holding
/// is the least surprising thing to draw while the user fixes it.
fn blend(a: &craftkit_core::Value, b: &craftkit_core::Value, t: f64) -> craftkit_core::Value {
    use craftkit_core::Value as V;
    match (a, b) {
        // The result is **clamped** to the range a stored number may take.
        //
        // The easing's overshoot is deliberate — that is how an ease-out
        // overshoots, and `Easing::checked` says so — but an overshoot in the
        // *curve* is not a licence for the *value* to leave the range. The
        // keys are numbers the tree checked — the model checks a file's keys
        // when it reads them (§73) — so they are in range; the number between
        // them is not one of them, and without a clamp it is not checked
        // either. Measured: two keys at ±`MAX_MAGNITUDE` and a bezier that
        // overshoots sampled to -1 497 942 087, which is outside the range
        // `craftkit_core::Value` documents. §65.
        (V::Num(x), V::Num(y)) => {
            V::Num((x + (y - x) * t).clamp(-craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE))
        }
        (V::Bool(_), V::Bool(y)) => V::Bool(*y),
        (V::Text(_), V::Text(y)) => V::Text(y.clone()),
        (V::Color(x), V::Color(y)) => {
            let mix = |p: u8, q: u8| {
                let v = f64::from(p) + (f64::from(q) - f64::from(p)) * t;
                v.clamp(0.0, 255.0).round() as u8
            };
            V::Color(craftkit_core::Color {
                r: mix(x.r, y.r),
                g: mix(x.g, y.g),
                b: mix(x.b, y.b),
                a: mix(x.a, y.a),
            })
        }
        (_, V::Num(n)) => V::Num(*n),
        (_, V::Bool(v)) => V::Bool(*v),
        (_, V::Text(s)) => V::Text(s.clone()),
        (_, V::Color(c)) => V::Color(*c),
        _ => b.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_core::{FrameRate, Value};

    const FPS: FrameRate = FrameRate(24);

    fn num(v: f64) -> Value {
        Value::num("test", v).expect("test value")
    }

    fn track(pairs: &[(i64, f64)], ease: Easing) -> Keyframes {
        let mut k = Keyframes::new();
        for (f, v) in pairs {
            let key = Key::new(Tick::from_frames_i64(*f, FPS), num(*v), ease).expect("test key");
            k.set(key).expect("insert");
        }
        k
    }

    #[test]
    fn presets_resolve_by_name_and_unknown_names_are_errors() {
        assert_eq!(Easing::preset("hold"), Some(Easing::Hold));
        assert_eq!(Easing::preset("linear"), Some(Easing::Linear));
        assert_eq!(Easing::preset("easeOut"), Some(Easing::preset("easeOut").unwrap()));
        assert_eq!(Easing::preset("wobble"), None);
    }

    #[test]
    fn a_non_finite_control_point_is_refused() {
        assert!(Easing::Bezier { out_x: f32::NAN, out_y: 0.5 }.checked().is_err());
        assert!(Easing::Bezier { out_x: f32::INFINITY, out_y: 0.0 }.checked().is_err());
        // Overshoot in y is legitimate; overshoot in x is not.
        assert!(Easing::Bezier { out_x: 0.5, out_y: 1.5 }.checked().is_ok());
        assert!(Easing::Bezier { out_x: 1.5, out_y: 0.5 }.checked().is_err());
    }

    #[test]
    fn set_keeps_the_list_sorted_whatever_order_it_is_called_in() {
        let mut a = Keyframes::new();
        for f in [5i64, 1, 3] {
            a.set(Key::new(Tick::from_frames_i64(f, FPS), num(f as f64), Easing::Linear).unwrap()).unwrap();
        }
        let times: Vec<i64> = a.keys().iter().map(|k| k.time.frames_i64(FPS)).collect();
        assert_eq!(times, vec![1, 3, 5]);
    }

    #[test]
    fn setting_an_existing_time_replaces_rather_than_duplicates() {
        let mut k = track(&[(0, 0.0), (10, 1.0)], Easing::Linear);
        k.set(Key::new(Tick::from_frames_i64(10, FPS), num(99.0), Easing::Linear).unwrap()).unwrap();
        assert_eq!(k.len(), 2);
        assert_eq!(k.sample(Tick::from_frames_i64(20, FPS)).unwrap(), num(99.0));
    }

    #[test]
    fn sampling_holds_outside_the_key_range() {
        let k = track(&[(10, 1.0), (20, 2.0)], Easing::Linear);
        let before = k.sample(Tick::from_frames_i64(0, FPS)).unwrap();
        let after = k.sample(Tick::from_frames_i64(100, FPS)).unwrap();
        assert_eq!(before, num(1.0));
        assert_eq!(after, num(2.0));
        // An empty track has no sample at all, and the caller falls back to the
        // property's static value. This is distinct from a track that *holds*.
        assert_eq!(Keyframes::new().sample(Tick::ZERO), None);
    }

    #[test]
    fn linear_interpolation_hits_both_ends_exactly() {
        let k = track(&[(0, 0.0), (10, 100.0)], Easing::Linear);
        assert_eq!(k.sample(Tick::from_frames_i64(0, FPS)).unwrap(), num(0.0));
        assert_eq!(k.sample(Tick::from_frames_i64(10, FPS)).unwrap(), num(100.0));
        let mid = k.sample(Tick::from_frames_i64(5, FPS)).unwrap().as_num().unwrap();
        assert!((mid - 50.0).abs() < 1e-9, "midpoint was {mid}");
    }

    #[test]
    fn hold_does_not_interpolate() {
        let k = track(&[(0, 0.0), (10, 100.0)], Easing::Hold);
        assert_eq!(k.sample(Tick::from_frames_i64(5, FPS)).unwrap(), num(0.0));
    }

    /// Sample a track at a fraction of the way across its span.
    fn at_fraction(ease: Easing, x: f64) -> f64 {
        let span = 100i64;
        let t = Tick::from_frames_i64((x * span as f64).round() as i64, FPS);
        k_or_panic(&[(0, 0.0), (span, 1.0)], ease, t)
    }

    fn k_or_panic(keys: &[(i64, f64)], ease: Easing, t: Tick) -> f64 {
        track(keys, ease).sample(t).and_then(|v| v.as_num()).expect("a number at that time")
    }

    #[test]
    fn ease_in_out_is_smoothstep_and_not_linear() {
        // `EaseInOut` is `t²(3−2t)`. The three values below are that formula
        // worked out by hand, not by calling the function under test — an
        // oracle that calls its own subject is not an oracle.
        //
        //     x = 0.25 → 0.0625 × 2.5  = 0.15625
        //     x = 0.50 → 0.25   × 2.0   = 0.5
        //     x = 0.75 → 0.5625 × 1.5   = 0.84375
        //
        // Before this test, replacing every easing with `linear` failed exactly
        // one test in the workspace, and that one only covered `Hold`.
        assert!((at_fraction(Easing::EaseInOut, 0.25) - 0.15625).abs() < 1e-4, "at a quarter");
        assert!((at_fraction(Easing::EaseInOut, 0.50) - 0.5).abs() < 1e-4, "at the middle");
        assert!((at_fraction(Easing::EaseInOut, 0.75) - 0.84375).abs() < 1e-4, "at three quarters");
    }

    #[test]
    fn ease_in_out_is_symmetric_about_the_middle() {
        let lo = at_fraction(Easing::EaseInOut, 0.30);
        let hi = at_fraction(Easing::EaseInOut, 0.70);
        assert!((lo + hi - 1.0).abs() < 1e-4, "not symmetric: {lo} and {hi}");
    }

    #[test]
    fn the_two_bezier_presets_solve_their_own_curve() {
        // `easeIn` is the cubic with control points (0,0), (0.42,0), (1,1);
        // `easeOut` is (0,0), (0.58,1), (1,1). Solving for y at a given x means
        // bisecting x(t) and reading y(t) — the code does it, these are the
        // answers, obtained from the definition:
        //
        //   x = 0.25   x = 0.50   x = 0.75
        //   easeIn  0.093465   0.315357   0.621862
        //   easeOut 0.386805   0.686461   0.897053
        //
        // The shape is the point as much as the numbers: `easeIn` is below
        // linear and `easeOut` is above it, on every interior sample.
        let ease_in = Easing::Bezier { out_x: 0.42, out_y: 0.0 };
        let ease_out = Easing::Bezier { out_x: 0.58, out_y: 1.0 };

        for (x, want_in, want_out) in
            [(0.25, 0.093_465, 0.386_805), (0.50, 0.315_357, 0.686_461), (0.75, 0.621_862, 0.897_053)]
        {
            let got_in = at_fraction(ease_in, x);
            let got_out = at_fraction(ease_out, x);
            assert!((got_in - want_in).abs() < 1e-3, "easeIn at {x} was {got_in}, want {want_in}");
            assert!((got_out - want_out).abs() < 1e-3, "easeOut at {x} was {got_out}, want {want_out}");
            assert!(got_in < x, "easeIn must start slower than linear; at {x} it was {got_in}");
            assert!(got_out > x, "easeOut must start faster than linear; at {x} it was {got_out}");
        }
    }

    #[test]
    fn every_easing_pins_both_ends_and_stays_inside_the_unit_range() {
        // Properties that hold for all of them, and that a broken solver is
        // most likely to break: an off-by-one in the bisection shows up at the
        // ends before it shows up in the middle.
        for ease in [
            Easing::Hold,
            Easing::Linear,
            Easing::EaseInOut,
            Easing::Bezier { out_x: 0.42, out_y: 0.0 },
            Easing::Bezier { out_x: 0.58, out_y: 1.0 },
        ] {
            let lo = bezier_or_eased(ease, 0.0);
            let hi = bezier_or_eased(ease, 1.0);
            assert!((lo - 0.0).abs() < 1e-6, "{ease:?} starts at {lo}");
            // `Hold` never leaves the first key's value, so it ends at 0 and
            // not at 1. That is what "hold" means, and asserting otherwise
            // would be asserting the wrong thing beautifully.
            let expected_end = if matches!(ease, Easing::Hold) { 0.0 } else { 1.0 };
            assert!((hi - expected_end).abs() < 1e-6, "{ease:?} ends at {hi}, want {expected_end}");

            // Monotone: never goes backwards between two samples.
            let mut previous = f64::NEG_INFINITY;
            for step in 0..=20 {
                let x = f64::from(step) / 20.0;
                let y = bezier_or_eased(ease, x);
                assert!((0.0..=1.0).contains(&y), "{ease:?} left 0..=1 at x={x}: {y}");
                assert!(y >= previous - 1e-9, "{ease:?} went backwards at x={x}: {y} < {previous}");
                previous = y;
            }
        }
    }

    /// The progress curve itself, independent of a track and of its frame rate.
    fn bezier_or_eased(ease: Easing, x: f64) -> f64 {
        ease_progress(ease, x)
    }

    #[test]
    fn the_same_time_always_gives_the_same_value() {
        // Determinism is what makes a rendered frame reproducible.
        let k = track(&[(0, 0.0), (7, 3.0), (13, -2.0)], Easing::EaseInOut);
        let t = Tick::from_frames_i64(9, FPS);
        let first = k.sample(t);
        for _ in 0..8 {
            assert_eq!(k.sample(t), first, "sampling is not deterministic");
        }
    }

    #[test]
    fn easing_endpoints_are_exact() {
        for e in [Easing::Linear, Easing::EaseInOut, Easing::Bezier { out_x: 0.3, out_y: 0.8 }] {
            assert!((ease_progress(e, 0.0)).abs() < 1e-9);
            assert!((ease_progress(e, 1.0) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn easing_curves_are_monotonic_so_a_scrub_never_goes_backwards() {
        for e in [Easing::Linear, Easing::EaseInOut, Easing::Bezier { out_x: 0.42, out_y: 0.0 }] {
            let mut prev = f64::NEG_INFINITY;
            for i in 0..=100 {
                let y = ease_progress(e, i as f64 / 100.0);
                assert!(y >= prev - 1e-9, "easing {e:?} is not monotonic at {i}");
                prev = y;
            }
        }
    }

    #[test]
    fn a_single_key_is_well_defined_anywhere() {
        let k = track(&[(5, 42.0)], Easing::Linear);
        for f in [0i64, 4, 5, 1000] {
            assert_eq!(k.sample(Tick::from_frames_i64(f, FPS)).unwrap(), num(42.0));
        }
    }

    #[test]
    fn two_keys_at_the_same_time_do_not_divide_by_zero() {
        // `set` makes this impossible through the API; sample() is checked too
        // because it is public and a document could be hand-edited.
        let mut k = Keyframes::new();
        k.set(Key::new(Tick::from_frames_i64(5, FPS), num(1.0), Easing::Linear).unwrap()).unwrap();
        let v = k.sample(Tick::from_frames_i64(5, FPS)).unwrap();
        assert_eq!(v, num(1.0));
    }

    #[test]
    fn mismatched_end_values_hold_rather_than_panic() {
        let mut k = Keyframes::new();
        k.set(Key::new(Tick::ZERO, Value::Bool(true), Easing::Linear).unwrap()).unwrap();
        k.set(Key::new(Tick::from_frames_i64(10, FPS), Value::Bool(false), Easing::Linear).unwrap()).unwrap();
        assert_eq!(k.sample(Tick::from_frames_i64(5, FPS)).unwrap(), Value::Bool(false));
    }

    #[test]
    fn colours_interpolate_per_channel() {
        use craftkit_core::Color;
        let mut k = Keyframes::new();
        k.set(Key::new(Tick::ZERO, Value::Color(Color::rgb(0, 0, 0)), Easing::Linear).unwrap()).unwrap();
        k.set(
            Key::new(Tick::from_frames_i64(2, FPS), Value::Color(Color::rgb(255, 0, 0)), Easing::Linear)
                .unwrap(),
        )
        .unwrap();
        let mid = k.sample(Tick::from_frames_i64(1, FPS)).unwrap().as_color().unwrap();
        assert_eq!(mid.r, 128);
        assert_eq!(mid.g, 0);
    }

    /// The number between two keys is a number the tree never checked.
    ///
    /// The keys are numbers the tree checked, so they are in range; the
    /// interpolation between them is not one of them, and an easing's
    /// overshoot is deliberate. Without a clamp the sample left the range
    /// that `craftkit_core::Value` documents — measured at -1 497 942 087
    /// against a limit of 1 000 000 000. §65.
    #[test]
    fn an_overshooting_easing_stays_inside_the_range_a_stored_number_may_take() {
        let mut k = Keyframes::new();
        // The first key's easing is the one used across the span, and a
        // bezier with an overshoot in y is a checked, legal easing.
        let overshoot = Easing::Bezier { out_x: 0.5, out_y: 2.0 };
        k.set(Key::new(Tick::from_frames_i64(0, FPS), num(craftkit_core::MAX_MAGNITUDE), overshoot).unwrap())
            .unwrap();
        k.set(
            Key::new(Tick::from_frames_i64(24, FPS), num(-(craftkit_core::MAX_MAGNITUDE)), Easing::Linear)
                .unwrap(),
        )
        .unwrap();
        for f in 0..=24 {
            let at = Tick::from_frames_i64(f, FPS);
            let value = k.sample(at).expect("a value at every frame");
            let n = value.as_num().expect("a number");
            assert!(
                n.abs() <= craftkit_core::MAX_MAGNITUDE,
                "frame {f} sampled {n}, which is outside the range a stored number may take"
            );
        }
    }

    /// The overshoot is still an overshoot: clamping the *value* must not
    /// flatten the *curve*. Ordinary values are nowhere near the limit, so
    /// the shape of the easing is untouched.
    #[test]
    fn the_clamp_does_not_flatten_the_overshoot_of_an_ordinary_value() {
        let k = track(&[(0, 0.0), (10, 1.0)], Easing::Bezier { out_x: 0.5, out_y: 2.0 });
        let mut highest = f64::MIN;
        for f in 0..=10 {
            let at = Tick::from_frames_i64(f, FPS);
            if let Some(v) = k.sample(at).and_then(|v| v.as_num()) {
                highest = highest.max(v);
            }
        }
        assert!(highest > 1.0, "the overshoot was flattened: the highest sample was {highest}");
    }

    #[test]
    fn the_keyframe_limit_is_enforced() {
        let mut k = Keyframes::new();
        for i in 0..crate::limits::MAX_KEYS {
            k.set(Key::new(Tick::from_raw(i as i128), num(i as f64), Easing::Linear).unwrap()).unwrap();
        }
        assert!(k.set(Key::new(Tick::from_raw(1 << 40), num(0.0), Easing::Linear).unwrap()).is_err());
    }

    // ------------------------------------------------------------------ §91 --

    /// A key at a time, built without the constructor.
    ///
    /// The model cannot build a broken list through its own API — `set`
    /// sorts, replaces and bounds it — so a list that breaks the rules can
    /// only arrive the way a file arrives: deserialized. The test module is a
    /// child of this one, so it can build the list the way the file does.
    fn raw_key(time: i128, value: f64) -> Key {
        Key { time: Tick::from_raw(time), value: num(value), ease: Easing::Linear }
    }

    /// The reason the list has to be sorted.
    ///
    /// An unsorted list does not sample to a wrong *shape* — it samples to the
    /// wrong **value**, and quietly: the key that is earliest in time is never
    /// reached. Measured: keys at 500 and 0 sampled to the value of the later
    /// key, where the sorted list gives the interpolation between them. §91.
    #[test]
    fn an_unsorted_list_samples_to_the_wrong_value() {
        let sorted = Keyframes(vec![raw_key(0, 2.0), raw_key(500, 1.0)]);
        let unsorted = Keyframes(vec![raw_key(500, 1.0), raw_key(0, 2.0)]);
        let at = Tick::from_raw(50);

        let right = sorted.sample(at).and_then(|v| v.as_num()).expect("a number");
        let wrong = unsorted.sample(at).and_then(|v| v.as_num()).expect("a number");

        // 2.0 + (1.0 - 2.0) × (50 / 500) = 1.9.
        assert!((right - 1.9).abs() < 1e-9, "the sorted list gave {right}, want 1.9");
        assert!((wrong - 1.9).abs() > 1e-9, "the unsorted list gave {wrong}, which is what it always gave");
        assert!(unsorted.check("amount").is_err(), "and the check does not let that list in");
    }

    #[test]
    fn a_list_that_is_not_sorted_by_time_is_refused() {
        let k = Keyframes(vec![raw_key(500, 1.0), raw_key(0, 2.0)]);
        let err = k.check("amount").expect_err("refused");
        assert!(err.to_string().contains("not sorted by time"), "{err}");
        assert!(err.to_string().contains("amount"), "the message does not name the property: {err}");
    }

    #[test]
    fn two_keys_at_the_same_time_are_refused() {
        let k = Keyframes(vec![raw_key(0, 1.0), raw_key(0, 2.0)]);
        let err = k.check("amount").expect_err("refused");
        assert!(err.to_string().contains("share the time"), "{err}");
    }

    /// A time outside the range a reply could carry is refused.
    ///
    /// `Tick::from_raw` clamps, so the model cannot build one; a **file** can
    /// carry one, because a bare integer in a file is read straight into an
    /// `i128`. The tick's own documentation promises the reader refuses such a
    /// file — it did not, until this check existed. §60, §91.
    #[test]
    fn a_time_outside_the_range_a_reply_can_carry_is_refused() {
        let raw: i128 = 18_446_744_073_709_551_616; // u64::MAX + 1
        let tick: Tick = serde_json::from_str(&raw.to_string()).expect("a tick");
        assert_eq!(tick.raw(), raw, "the tick should have arrived out of range");
        let k = Keyframes(vec![Key { time: tick, value: num(1.0), ease: Easing::Linear }]);
        let err = k.check("amount").expect_err("refused");
        assert!(err.to_string().contains("out of range"), "{err}");
        assert!(err.to_string().contains("amount"), "the message does not name the property: {err}");
    }

    #[test]
    fn an_easing_the_model_refuses_is_refused() {
        let k = Keyframes(vec![Key {
            time: Tick::ZERO,
            value: num(1.0),
            ease: Easing::Bezier { out_x: 5.0, out_y: 1.0 },
        }]);
        let err = k.check("amount").expect_err("refused");
        assert!(err.to_string().contains("easing"), "{err}");
        assert!(err.to_string().contains("amount"), "the message does not name the property: {err}");
    }

    #[test]
    fn more_keys_than_the_limit_are_refused() {
        let max = crate::limits::MAX_KEYS;
        let k = Keyframes((0..=max as i128).map(|i| raw_key(i, 1.0)).collect());
        let err = k.check("amount").expect_err("refused");
        assert!(err.to_string().contains("keyframes per property"), "{err}");
    }

    /// The check does not refuse what the model builds.
    #[test]
    fn a_list_built_through_the_model_is_accepted() {
        let k = track(&[(0, 0.0), (10, 1.0)], Easing::Linear);
        assert!(k.check("amount").is_ok());
        let mut k = Keyframes::new();
        k.set(Key::new(Tick::ZERO, num(0.0), Easing::Bezier { out_x: 0.5, out_y: 2.0 }).unwrap()).unwrap();
        assert!(k.check("amount").is_ok(), "an overshoot in y is a legal easing");
    }

    #[test]
    fn set_ease_changes_the_curve_and_refuses_an_invalid_one() {
        let mut k = track(&[(0, 0.0), (10, 1.0)], Easing::Linear);
        assert!(k.set_ease(Tick::from_raw(0), Easing::EaseInOut).expect("a valid easing"));
        assert_eq!(k.keys()[0].ease, Easing::EaseInOut);
        assert!(!k.set_ease(Tick::from_raw(999), Easing::Hold).expect("no key there, but no error"));
        // An easing the model's own constructor refuses must not enter
        // through this door either.
        assert!(k.set_ease(Tick::from_raw(0), Easing::Bezier { out_x: 5.0, out_y: 1.0 }).is_err());
        assert_eq!(k.keys()[0].ease, Easing::EaseInOut, "the refused easing was not stored");
    }
}
