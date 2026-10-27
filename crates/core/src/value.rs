//! The one value type that crosses every layer boundary.
//!
//! # Why a single enum
//!
//! A command parameter arrives as JSON. The document stores it. The evaluator
//! reads it. The UI shows it. The file format writes it. If each layer had its
//! own representation, every boundary would need a lossy conversion and every
//! layer would need to handle the cases the previous one dropped.
//!
//! So there is one [`Value`], and it is deliberately small: six variants cover
//! everything a command or a property needs, and anything richer is a different
//! type in a higher layer.

use serde::{Deserialize, Serialize};

/// An RGBA colour with 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };

    /// The shape of a colour, as a pattern.
    ///
    /// This is the **one** description of what a colour may look like, and it
    /// lives beside the parser that reads one. The tree publishes it to the
    /// MCP server's schema, so the schema a client reads is the rule the
    /// parser enforces — and cannot drift from it, because it is the same
    /// string. §254.
    pub const PATTERN: &'static str = "^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$";

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parse `#rrggbb` or `#rrggbbaa`. The leading `#` is required, so a bare
    /// token can never be mistaken for a colour.
    ///
    /// The digits are **hex digits and nothing else**. `u8::from_str_radix`
    /// accepts a leading `+`, so `"#+f+f+f"` parsed as the colour `#0f0f0f` —
    /// a token that is neither of the two shapes above, that the tree's own
    /// published pattern rejects, and that a person would not have meant.
    /// §254.
    pub fn parse(s: &str) -> Option<Self> {
        let h = s.strip_prefix('#')?;
        if h.len() != 6 && h.len() != 8 {
            return None;
        }
        if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
        let r = byte(0)?;
        let g = byte(2)?;
        let b = byte(4)?;
        let a = byte(6).unwrap_or(255);
        Some(Self { r, g, b, a })
    }

    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

/// A value in the document, in a command parameter, or in a property.
///
/// Invariants the constructors maintain:
///
/// - `Num` is always finite and within [`crate::MAX_MAGNITUDE`].
/// - `Text` is always within [`crate::MAX_TEXT`].
/// - `List` is always within `MAX_LIST` entries.
/// - a value never nests deeper than [`MAX_VALUE_DEPTH`] list levels.
///
/// The constructors are not the only way a value is made, and there are
/// **three** doors into one:
///
/// 1. a value can be *interpolated* between two of them, and an interpolation
///    is not one of the numbers the tree checked — so the interpolator clamps
///    to the same range rather than trusting the inputs. The model does it in
///    `craftkit_model::keyframe`, and §65 has the measurement of what happened
///    when it did not.
/// 2. a value can be **deserialized**: a file, a script's base, a crash
///    report. Nothing calls a constructor on the way in, so a deserialized
///    value breaks every invariant above unless somebody checks it — and the
///    somebody is [`Value::check`], which the model's validation calls for
///    every value in a document. §73.
/// 3. a value can be *built by hand*, in a test or in a command that means
///    to. That is the constructors' job, and it is the reason they exist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum Value {
    Bool(bool),
    Num(f64),
    Text(String),
    Color(Color),
    /// An ordered list, capped so a hostile payload cannot allocate.
    List(Vec<Value>),
    /// No value: the parameter is absent, or a property is not animated.
    None,
}

/// The most entries a [`Value::List`] may hold.
pub const MAX_LIST: usize = 4_096;

/// The most list levels a value may nest: a list inside a list inside a list.
///
/// The loader that brings values in — `serde_json` — refuses to parse a tagged
/// list nested more than 64 levels (measured: 63 parses, 64 is refused with
/// "recursion limit exceeded"), so the tree now says the same number itself.
/// Before this the bound was the loader's default and the tree's own walks
/// had none: `Value::check` on a value nested ten thousand deep overflowed the
/// stack, and a stack overflow is a fatal abort, not a catchable panic. A
/// bound nobody wrote down is a bound that can change under the tree's feet.
/// §251.
///
/// This is the tree's *own* number, not the model's: `craftkit_model::limits::MAX_DEPTH`
/// (32) bounds the node graph, and this bounds the values inside a property.
pub const MAX_VALUE_DEPTH: usize = 64;

/// How many list levels the values inside a list nest.
///
/// The walk is iterative on purpose: this function measures the depth of a
/// value that may be hostile, and a recursive walk over a hundred thousand
/// levels would overflow the stack while trying to say how deep it is.
fn nesting(items: &[Value]) -> usize {
    let mut deepest = 0usize;
    let mut stack: Vec<(&[Value], usize)> = vec![(items, 1)];
    while let Some((level, depth)) = stack.pop() {
        if depth > deepest {
            deepest = depth;
        }
        for item in level {
            if let Value::List(inner) = item {
                stack.push((inner, depth + 1));
            }
        }
    }
    deepest
}

impl Value {
    /// Build a number, refusing non-finite and out-of-range values.
    pub fn num(what: impl Into<String>, v: f64) -> Result<Self, crate::CoreError> {
        Ok(Value::Num(crate::finite(what, v)?))
    }

    /// Build text, refusing anything past the length cap.
    pub fn text(what: impl Into<String>, s: impl Into<String>) -> Result<Self, crate::CoreError> {
        let s = s.into();
        crate::text(what, &s)?;
        Ok(Value::Text(s))
    }

    /// Build a list, refusing over-long input **and** recursing only to a depth
    /// the loader could actually have produced.
    pub fn list(items: Vec<Value>) -> Result<Self, crate::CoreError> {
        if items.len() > MAX_LIST {
            return Err(crate::CoreError::LimitReached { limit: "list length" });
        }
        if nesting(&items) > MAX_VALUE_DEPTH {
            return Err(crate::CoreError::LimitReached { limit: "value depth" });
        }
        Ok(Value::List(items))
    }

    /// A cheap, conservative estimate of the memory this value holds.
    ///
    /// It is used for budgets, so it must never **under**-estimate: a budget a
    /// value can lie about is not a budget. String and list capacities are
    /// counted rather than their lengths, because the allocation is what the
    /// memory is.
    ///
    /// The walk stops at [`MAX_VALUE_DEPTH`], so a value too deep to be legal
    /// cannot make this walk overflow the stack on its way to an answer.
    pub fn estimated_bytes(&self) -> usize {
        self.estimated_bytes_at(0)
    }

    fn estimated_bytes_at(&self, depth: usize) -> usize {
        std::mem::size_of::<Value>()
            + match self {
                Value::Text(s) => s.capacity(),
                Value::List(items) => {
                    let own = items.capacity() * std::mem::size_of::<Value>();
                    if depth >= MAX_VALUE_DEPTH {
                        own
                    } else {
                        let children = items
                            .iter()
                            .map(|item| item.estimated_bytes_at(depth + 1))
                            .fold(0usize, usize::saturating_add);
                        own.saturating_add(children)
                    }
                }
                _ => 0,
            }
    }

    /// Check that this value is within the invariants the constructors keep.
    ///
    /// The constructors keep them on the way **in**; this keeps them for a
    /// value that arrived some other way — deserialized from a file, a
    /// script's base or a crash report, which is a door the constructors are
    /// not on. The model calls it for every value in a document, so a file
    /// that carries a number a hundred million times past the limit is
    /// refused rather than believed. §73.
    ///
    /// `what` names the value in the error message — the property's label,
    /// say — so the message says *which* value was wrong and not merely that
    /// one was. It costs nothing on the path where the value is fine: the
    /// message is only built when there is an error to build it for.
    pub fn check(&self, what: &str) -> Result<(), crate::CoreError> {
        self.check_at(what, 0)
    }

    /// The check, with the depth of the enclosing lists carried along.
    ///
    /// The depth is what stops the walk: past [`MAX_VALUE_DEPTH`] the value is
    /// refused rather than walked, so a value nested a hundred thousand deep
    /// is answered with an error instead of an overflowed stack.
    fn check_at(&self, what: &str, depth: usize) -> Result<(), crate::CoreError> {
        match self {
            Value::Num(v) => {
                if !v.is_finite() {
                    return Err(crate::CoreError::NotFinite {
                        what: format!("the value of {what}"),
                        value: format!("{v}"),
                    });
                }
                if v.abs() > crate::MAX_MAGNITUDE {
                    return Err(crate::CoreError::OutOfRange {
                        what: format!("the value of {what}"),
                        value: format!("{v}"),
                        min: format!("{}", -crate::MAX_MAGNITUDE),
                        max: format!("{}", crate::MAX_MAGNITUDE),
                    });
                }
                Ok(())
            }
            Value::Text(s) => {
                if s.len() > crate::MAX_TEXT {
                    return Err(crate::CoreError::TooLong {
                        what: format!("the value of {what}"),
                        len: s.len(),
                        max: crate::MAX_TEXT,
                    });
                }
                Ok(())
            }
            // A list is checked all the way down: a list of lists is still a
            // value, and the text inside it is still text.
            Value::List(items) => {
                if items.len() > MAX_LIST {
                    return Err(crate::CoreError::LimitReached { limit: "list length" });
                }
                if depth + 1 > MAX_VALUE_DEPTH {
                    return Err(crate::CoreError::LimitReached { limit: "value depth" });
                }
                for item in items {
                    item.check_at(what, depth + 1)?;
                }
                Ok(())
            }
            Value::Bool(_) | Value::Color(_) | Value::None => Ok(()),
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_color(&self) -> Option<Color> {
        match self {
            Value::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// The name used in error messages and in the UI, e.g. `"number"`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "boolean",
            Value::Num(_) => "number",
            Value::Text(_) => "text",
            Value::Color(_) => "colour",
            Value::List(_) => "list",
            Value::None => "none",
        }
    }

    /// A short human-readable form for a UI table or an error message.
    pub fn display(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    let s = format!("{n:.4}");
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
            Value::Text(s) => s.clone(),
            Value::Color(c) => c.to_hex(),
            Value::List(items) => format!("[{} items]", items.len()),
            Value::None => "—".to_string(),
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<Color> for Value {
    fn from(v: Color) -> Self {
        Value::Color(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A value nested `depth` list levels deep.
    fn nested(depth: usize) -> Value {
        let mut v = Value::Num(1.0);
        for _ in 0..depth {
            v = Value::List(vec![v]);
        }
        v
    }

    #[test]
    fn num_refuses_nan_and_overlarge_values() {
        assert!(Value::num("x", f64::NAN).is_err());
        assert!(Value::num("x", f64::INFINITY).is_err());
        assert!(Value::num("x", 1e12).is_err());
        assert!(Value::num("x", 12.5).is_ok());
    }

    #[test]
    fn list_is_capped() {
        assert!(Value::list(vec![Value::Bool(true); MAX_LIST]).is_ok());
        assert!(Value::list(vec![Value::Bool(true); MAX_LIST + 1]).is_err());
    }

    /// The depth of a value is the tree's own bound, not the loader's default.
    ///
    /// The loader (`serde_json`) refuses a tagged list nested more than 64
    /// levels, and the tree's own walk had no bound of its own: measured,
    /// `Value::check` on a value nested ten thousand deep overflowed the
    /// stack, and a stack overflow is a fatal abort. §251.
    #[test]
    fn a_value_nested_past_the_depth_limit_is_refused() {
        // Exactly at the limit the value is fine.
        let at_limit = nested(MAX_VALUE_DEPTH);
        assert!(at_limit.check("deep").is_ok(), "a value at the depth limit is legal");
        // One level past it is refused, by the check and by the constructor.
        let past = nested(MAX_VALUE_DEPTH + 1);
        let err = past.check("deep").expect_err("too deep");
        assert!(err.to_string().contains("depth"), "{err}");
        assert!(Value::list(vec![past]).is_err(), "the constructor refuses it too");
    }

    /// A value too deep to be legal must not overflow the stack on its way to
    /// an answer: the walk stops at the limit instead of following the value
    /// all the way down.
    ///
    /// Ten thousand is the depth at which the check used to overflow, measured
    /// on this machine (§251).
    #[test]
    fn a_very_deep_value_is_answered_rather_than_overflowing() {
        let deep = nested(10_000);
        assert!(deep.check("deep").is_err(), "refused, not overflowed");
        // The byte estimate walks too, and it answers instead of overflowing.
        let bytes = deep.estimated_bytes();
        assert!(bytes > 0, "the estimate is an answer, not a crash");
    }

    /// The estimate must never under-estimate: a budget a value can lie about
    /// is not a budget. A value at the depth limit is legal, so it must be
    /// counted all the way down.
    #[test]
    fn the_estimate_counts_a_legal_value_all_the_way_down() {
        let at_limit = nested(MAX_VALUE_DEPTH);
        let bytes = at_limit.estimated_bytes();
        let floor = MAX_VALUE_DEPTH * std::mem::size_of::<Value>();
        assert!(bytes >= floor, "the estimate {bytes} is below the floor {floor}");
    }

    #[test]
    fn text_is_capped() {
        assert!(Value::text("n", "ok").is_ok());
        assert!(Value::text("n", "x".repeat(crate::MAX_TEXT + 1)).is_err());
    }

    /// The check that holds a **deserialized** value to the constructors' rules.
    ///
    /// A file is not built by a constructor, so this is the only thing between
    /// a hand-written file and a number the tree cannot hold. §73.
    #[test]
    fn a_value_that_breaks_an_invariant_is_caught() {
        // The good ones pass.
        assert!(Value::Num(1.0).check("amount").is_ok());
        assert!(Value::Num(crate::MAX_MAGNITUDE).check("amount").is_ok());
        assert!(Value::Text("x".into()).check("label").is_ok());
        assert!(Value::Text("x".repeat(crate::MAX_TEXT)).check("label").is_ok());
        assert!(Value::List(vec![Value::Num(1.0); MAX_LIST]).check("list").is_ok());
        assert!(Value::Bool(true).check("flag").is_ok());
        assert!(Value::None.check("absent").is_ok());

        // The bad ones do not.
        let out_of_range = Value::Num(crate::MAX_MAGNITUDE * 1e6).check("amount").expect_err("too large");
        assert!(out_of_range.to_string().contains("amount"), "{out_of_range}");
        assert!(Value::Num(f64::NAN).check("amount").is_err());
        assert!(Value::Num(f64::INFINITY).check("amount").is_err());
        assert!(Value::Text("x".repeat(crate::MAX_TEXT + 1)).check("label").is_err());
        assert!(Value::List(vec![Value::Num(1.0); MAX_LIST + 1]).check("list").is_err());
        // A list is checked all the way down: a list of lists is still a value.
        let nested = Value::List(vec![Value::List(vec![Value::Num(1e300)])]);
        assert!(nested.check("list").is_err());
    }

    #[test]
    fn accessors_return_none_rather_than_a_wrong_type() {
        assert_eq!(Value::Bool(true).as_num(), None);
        assert_eq!(Value::Num(1.0).as_bool(), None);
        assert_eq!(Value::Num(1.0).as_text(), None);
    }

    /// A colour is hex digits and nothing else. §254.
    ///
    /// `u8::from_str_radix` takes a leading `+`, so `"#+f+f+f"` used to parse
    /// as the colour `#0f0f0f` — a token that is neither `#rrggbb` nor
    /// `#rrggbbaa`, and that the tree's own published pattern rejects.
    #[test]
    fn a_colour_is_hex_digits_and_nothing_else() {
        assert_eq!(Color::parse("#ff00aa"), Some(Color { r: 255, g: 0, b: 170, a: 255 }));
        assert_eq!(Color::parse("#ff00aa80"), Some(Color { r: 255, g: 0, b: 170, a: 128 }));
        assert_eq!(Color::parse("#FF00AA"), Some(Color { r: 255, g: 0, b: 170, a: 255 }));
        // A sign is not a digit.
        assert_eq!(Color::parse("#+f+f+f"), None, "a sign is not a hex digit");
        assert_eq!(Color::parse("#+1+2+3"), None, "a sign is not a hex digit");
        // Neither is anything else.
        assert_eq!(Color::parse("#gggggg"), None, "g is not a hex digit");
        assert_eq!(Color::parse("#ff00aazz"), None, "z is not a hex digit");
        // And the shapes are the two the tree says they are.
        assert_eq!(Color::parse("#ff"), None, "a colour is six or eight digits");
        assert_eq!(Color::parse("#fff"), None, "a colour is six or eight digits");
        assert_eq!(Color::parse("#fffffff"), None, "a colour is six or eight digits");
        assert_eq!(Color::parse("ff00aa"), None, "the leading # is required");
    }

    /// The published pattern and the parser are the same rule. §254.
    ///
    /// The pattern is a regular expression and the tree has no regular
    /// expression engine, so the test states the pattern's two shapes — six
    /// hex digits, or eight — and asks the parser the same question. The
    /// pattern **string** is pinned beside it, so a change to either is a
    /// change to one of the two.
    #[test]
    fn the_published_pattern_is_the_shape_the_parser_reads() {
        assert_eq!(Color::PATTERN, "^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$");
        let matches_pattern = |s: &str| {
            let Some(h) = s.strip_prefix('#') else { return false };
            (h.len() == 6 || h.len() == 8) && h.bytes().all(|b| b.is_ascii_hexdigit())
        };
        for s in [
            "#ff00aa",
            "#ff00aa80",
            "#FF00AA",
            "#000000",
            "#+f+f+f",
            "#+1+2+3",
            "#ff",
            "#fff",
            "#gggggg",
            "#ff00aazz",
            "#fffffff",
            "ff00aa",
        ] {
            assert_eq!(
                Color::parse(s).is_some(),
                matches_pattern(s),
                "the parser and the published pattern disagree about {s}"
            );
        }
    }

    #[test]
    fn colour_round_trips_through_hex() {
        let c = Color::rgb(0x12, 0xab, 0xef);
        assert_eq!(Color::parse(&c.to_hex()), Some(c));
        assert_eq!(Color::parse("#12abef80"), Some(Color { r: 0x12, g: 0xab, b: 0xef, a: 0x80 }));
        // The leading `#` is required, so a bare token is never a colour.
        assert_eq!(Color::parse("12abef"), None);
        assert_eq!(Color::parse("#zzzzzz"), None);
    }

    #[test]
    fn display_is_stable_for_round_trips() {
        assert_eq!(Value::Num(12.0).display(), "12");
        assert_eq!(Value::Num(12.5).display(), "12.5");
        assert_eq!(Value::None.display(), "—");
    }

    #[test]
    fn values_round_trip_through_json() {
        for v in [
            Value::Bool(true),
            Value::Num(1.5),
            Value::text("t", "hello").unwrap(),
            Value::Color(Color::rgb(1, 2, 3)),
            Value::list(vec![Value::Num(1.0), Value::Bool(false)]).unwrap(),
            Value::None,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            let back: Value = serde_json::from_str(&j).unwrap();
            assert_eq!(back, v, "round trip failed for {j}");
        }
    }
}
