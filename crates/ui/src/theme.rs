//! Colours, as named tokens.
//!
//! # One place
//!
//! No panel contains a colour literal. Every colour is a [`Tokens`] field, so
//! "make the selection colour match the accent" is one edit rather than a grep,
//! and a theme is a struct rather than a search.
//!
//! # Chosen for contrast, not taste
//!
//! The text/background pairs here are checked against the WCAG AA ratio in
//! [`Tokens::check_contrast`] rather than eyeballed. A theme whose contrast is
//! "probably fine" is a theme somebody will read at 3 a.m. and stop reading.

use egui::Color32;

/// The whole palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub background: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub selection: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub ok: Color32,
    /// The level bar: full is saturated, low is dim.
    pub meter_low: Color32,
    pub meter_high: Color32,
}

impl Default for Tokens {
    fn default() -> Self {
        Self::dark()
    }
}

impl Tokens {
    /// The dark theme.
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x14, 0x16, 0x1a),
            surface: Color32::from_rgb(0x1c, 0x1f, 0x24),
            surface_alt: Color32::from_rgb(0x24, 0x28, 0x2f),
            border: Color32::from_rgb(0x36, 0x3c, 0x45),
            text: Color32::from_rgb(0xe8, 0xea, 0xed),
            text_dim: Color32::from_rgb(0x9a, 0xa1, 0xab),
            accent: Color32::from_rgb(0x6a, 0xa8, 0xff),
            accent_text: Color32::from_rgb(0x0d, 0x10, 0x14),
            selection: Color32::from_rgb(0x2d, 0x44, 0x66),
            warning: Color32::from_rgb(0xe2, 0xa0, 0x3c),
            error: Color32::from_rgb(0xe2, 0x5c, 0x5c),
            ok: Color32::from_rgb(0x5c, 0xc0, 0x8a),
            meter_low: Color32::from_rgb(0x2b, 0x3a, 0x4a),
            meter_high: Color32::from_rgb(0x6a, 0xa8, 0xff),
        }
    }

    /// The light theme.
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xf7, 0xf8, 0xfa),
            surface: Color32::from_rgb(0xff, 0xff, 0xff),
            surface_alt: Color32::from_rgb(0xec, 0xef, 0xf3),
            border: Color32::from_rgb(0xc9, 0xcf, 0xd8),
            text: Color32::from_rgb(0x1a, 0x1d, 0x21),
            text_dim: Color32::from_rgb(0x5c, 0x64, 0x6e),
            accent: Color32::from_rgb(0x1f, 0x63, 0xc4),
            accent_text: Color32::from_rgb(0xff, 0xff, 0xff),
            selection: Color32::from_rgb(0xd6, 0xe4, 0xf7),
            warning: Color32::from_rgb(0x9a, 0x63, 0x00),
            error: Color32::from_rgb(0xb0, 0x28, 0x28),
            ok: Color32::from_rgb(0x1d, 0x7a, 0x46),
            meter_low: Color32::from_rgb(0xd5, 0xde, 0xe8),
            meter_high: Color32::from_rgb(0x1f, 0x63, 0xc4),
        }
    }

    /// A magnitude mapped onto 0..=1 for a ramp.
    ///
    /// `f64::clamp` does **not** sanitise `NaN` — it hands it straight back —
    /// so a NaN level would produce a NaN blend factor and a colour nobody
    /// asked for. This is the one place a level becomes a fraction, and it
    /// refuses nonsense explicitly rather than relying on the arithmetic.
    pub fn ramp_fraction(level: f64) -> f32 {
        if !level.is_finite() {
            return 0.0;
        }
        (level.abs() / craftkit_foundation::MAX_MAGNITUDE).clamp(0.0, 1.0) as f32
    }

    /// The meter colour for a level, low to high.
    ///
    /// A linear ramp rather than a binary "on/off", because a plan's whole point
    /// is that intermediate values mean something.
    pub fn meter(&self, level: f64) -> Color32 {
        blend(self.meter_low, self.meter_high, Self::ramp_fraction(level))
    }

    /// The pairs that must be readable, and the minimum WCAG AA ratio.
    ///
    /// Body text needs 4.5:1. Large text and non-text indicators need 3:1.
    pub const CONTRAST_PAIRS: [ContrastPair; 8] = [
        ContrastPair::new("text/background", |t| (t.text, t.background), 4.5),
        ContrastPair::new("text/surface", |t| (t.text, t.surface), 4.5),
        ContrastPair::new("text/surface_alt", |t| (t.text, t.surface_alt), 4.5),
        ContrastPair::new("dim/background", |t| (t.text_dim, t.background), 4.5),
        ContrastPair::new("accent/background", |t| (t.accent, t.background), 3.0),
        ContrastPair::new("warning/background", |t| (t.warning, t.background), 3.0),
        ContrastPair::new("error/background", |t| (t.error, t.background), 3.0),
        ContrastPair::new("accent_text/accent", |t| (t.accent_text, t.accent), 4.5),
    ];

    /// Check every pair. Returns the ones that fail, so a test can print them.
    pub fn contrast_failures(&self) -> Vec<(String, f64, f64)> {
        let mut out = Vec::new();
        for pair in Self::CONTRAST_PAIRS {
            let (fg, bg) = (pair.of)(self);
            let got = contrast(fg, bg);
            if got + 0.005 < pair.need {
                out.push((pair.name.to_string(), got, pair.need));
            }
        }
        out
    }
}

/// One pair of colours that must be readable, and the ratio it must reach.
pub struct ContrastPair {
    pub name: &'static str,
    pub of: fn(&Tokens) -> (Color32, Color32),
    /// The WCAG AA minimum: 4.5 for body text, 3 for large text and indicators.
    pub need: f64,
}

impl ContrastPair {
    const fn new(name: &'static str, of: fn(&Tokens) -> (Color32, Color32), need: f64) -> Self {
        Self { name, of, need }
    }
}

/// Linear blend between two colours.
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (f32::from(x) * (1.0 - t) + f32::from(y) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// The WCAG relative-luminance contrast ratio between two opaque colours.
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let la = luminance(a);
    let lb = luminance(b);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn luminance(c: Color32) -> f64 {
    let channel = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.039_28 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_themes_meet_their_contrast_requirements() {
        for (name, tokens) in [("dark", Tokens::dark()), ("light", Tokens::light())] {
            let failures = tokens.contrast_failures();
            assert!(failures.is_empty(), "{name} theme fails contrast: {failures:?}");
        }
    }

    #[test]
    fn the_contrast_ratio_matches_a_known_value() {
        // Black on white is the definition: 21:1.
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        // A colour against itself is 1:1, which is the floor.
        assert!((contrast(Color32::from_rgb(20, 20, 20), Color32::from_rgb(20, 20, 20)) - 1.0).abs() < 0.01);
    }

    #[test]
    fn the_meter_ramps_from_low_to_high() {
        let t = Tokens::dark();
        assert_eq!(t.meter(0.0), t.meter_low);
        assert_eq!(t.meter(craftkit_foundation::MAX_MAGNITUDE), t.meter_high);
        let mid = t.meter(craftkit_foundation::MAX_MAGNITUDE / 2.0);
        assert!(mid.r() > t.meter_low.r(), "the ramp should move");
    }

    #[test]
    fn a_negative_or_absurd_level_still_maps_inside_the_ramp() {
        let t = Tokens::dark();
        assert_eq!(t.meter(-1.0), t.meter(1.0), "magnitude decides, sign does not");
        assert_eq!(t.meter(f64::NAN), t.meter_low, "NaN must not become a colour of its own");
        assert_eq!(t.meter(f64::NEG_INFINITY), t.meter_low);
        // An infinite level is a bug upstream, and the evaluator saturates long
        // before it can happen. If one arrives anyway it reads as "nothing",
        // because a bar pegged to the top says "maximum", which is the opposite
        // of the truth.
        assert_eq!(t.meter(f64::INFINITY), t.meter_low);
        assert_eq!(t.meter(1e300), t.meter_high, "a merely huge level is still large");
    }

    #[test]
    fn blending_stays_inside_the_range() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);
        assert_eq!(blend(a, b, 0.0), a);
        assert_eq!(blend(a, b, 1.0), b);
        assert_eq!(blend(a, b, -5.0), a);
        assert_eq!(blend(a, b, 5.0), b);
        let mid = blend(a, b, 0.5);
        assert_eq!(mid, Color32::from_rgb(128, 128, 128));
    }

    #[test]
    fn ramp_fraction_refuses_every_shape_of_nonsense() {
        assert_eq!(Tokens::ramp_fraction(0.0), 0.0);
        assert_eq!(Tokens::ramp_fraction(craftkit_foundation::MAX_MAGNITUDE), 1.0);
        assert_eq!(Tokens::ramp_fraction(1e300), 1.0);
        assert_eq!(Tokens::ramp_fraction(f64::NAN), 0.0);
        assert_eq!(Tokens::ramp_fraction(f64::INFINITY), 0.0);
        assert_eq!(Tokens::ramp_fraction(f64::NEG_INFINITY), 0.0);
    }

    #[test]
    fn the_default_theme_is_the_dark_one() {
        assert_eq!(Tokens::default(), Tokens::dark());
    }
}
