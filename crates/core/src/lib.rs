//! # craftkit-core
//!
//! The foundation layer: exact time, typed values, identifiers, and the readers
//! that turn hostile input into typed values.
//!
//! **This crate has no workspace dependencies on purpose.** It is
//! `Class::Standalone(0)` in the layer table, which means it could be lifted out
//! and published on its own without dragging the application with it. That is a
//! design constraint from the family: a lower layer that cannot be reused is a
//! lower layer that will grow a dependency on something it should not know
//! about.
//!
//! Three things live here, and all three are things the rest of the stack must
//! not re-implement:
//!
//! 1. [`Tick`] — exact integer time. Never store seconds as `f64`.
//! 2. [`Value`] — the one value type that crosses every layer boundary.
//! 3. [`Params`] — the only way to read a command parameter. Never index.
//!
//! And one more thing, added because the same mistake was made in four
//! places at once: [`read_line_bounded`] — a reader that applies its limit
//! *before* the allocation, rather than after it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod ids;
pub mod params;
pub mod readers;
pub mod tick;
pub mod value;

pub use ids::{DocumentId, IdCounter, NodeId};
pub use params::{ParamError, Params};
pub use readers::{ReadError, read_file_bounded, read_line_bounded, read_to_string_bounded};
pub use tick::{FrameRate, TICKS_PER_FRAME_24, TICKS_PER_SECOND, Tick};
pub use value::{Color, Value};

/// The crate's own error type. Everything a caller can be told goes through it.
///
/// `what` is a `String`, not a `&'static str`, because the most useful thing an
/// error can name is usually a *parameter key* — which is only known at run
/// time. (The kernel's errors are the opposite: they name a call site, which
/// really is a literal, and there they stay `&'static str` so nothing
/// allocates on a path that only runs when something has already gone wrong.)
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// A value was outside the range the domain allows.
    #[error("{what} is out of range: {value} (allowed {min}..={max})")]
    OutOfRange { what: String, value: String, min: String, max: String },
    /// A value was not finite. `NaN` and `inf` are never valid stored data.
    #[error("{what} is not a finite number ({value})")]
    NotFinite { what: String, value: String },
    /// A limit protecting a loop, an allocation or a recursion was reached.
    #[error("limit reached: {limit}")]
    LimitReached { limit: &'static str },
    /// A string was longer than the domain allows.
    #[error("{what} is {len} characters, the limit is {max}")]
    TooLong { what: String, len: usize, max: usize },
    /// A path could not name what it was asked to name.
    ///
    /// The message says which path and why, because the caller is usually a
    /// command that was handed a path by a person or a script. §85.
    #[error("{0}")]
    BadPath(String),
    /// Stored data broke a rule the model keeps.
    ///
    /// The rules are the model's own — a keyframe list is sorted and unique, a
    /// property's label can be written as a path — and each message says which
    /// rule broke it and which value broke it. §85, §91.
    #[error("{0}")]
    BadData(String),
}

/// The largest magnitude any stored scalar may take.
///
/// A stored `f64` is written to a JSON file and read back by a future version of
/// the program. Past this magnitude, the arithmetic that follows stops being
/// meaningful long before it stops being finite, so the reader refuses it.
pub const MAX_MAGNITUDE: f64 = 1e9;

/// The largest string any free-text field may hold, in bytes.
pub const MAX_TEXT: usize = 1 << 16;

/// What one byte of JSON text costs in memory, measured.
///
/// The tree's input limits bound the **text** a reader keeps. The parser then
/// turns that text into a tree, and the tree is much bigger than the text: a
/// JSON number is two bytes of text and thirty-two bytes of memory — a value
/// has to carry its own type — and the vector holding a long list grows to
/// twice its length. Measured on the CLI's script path: a 32 000 063-byte
/// script cost 1 034 584 KiB of resident memory, a factor of thirty-two. §64.
///
/// So a limit of *n* bytes of text is a limit of about *32n* bytes of memory,
/// and a limit written in bytes of text has to be read as a limit in bytes of
/// memory thirty-two times larger.
pub const JSON_EXPANSION: usize = 32;

/// How much memory one parsed JSON input may hold.
///
/// The same budget the model gives one document (§52), so a hostile input
/// cannot cost more than a hostile document does.
pub const MAX_JSON_MEMORY_BYTES: usize = 32 * 1024 * 1024;

/// The largest JSON text whose parse fits in [`MAX_JSON_MEMORY_BYTES`].
///
/// This is the number every reader of untrusted JSON in the tree uses: the
/// control channel's line limit, the MCP server's message limit, and the
/// CLI's script limit. They used to be three unrelated numbers — 1 MiB,
/// 4 MiB and 32 MiB — and the largest of them bought a gigabyte of memory.
/// §64.
pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / JSON_EXPANSION;

/// Check a number that is about to be stored.
///
/// Rejects NaN and infinities, and rejects magnitudes past [`MAX_MAGNITUDE`].
/// Every numeric setter in this crate and in every crate above it should go
/// through here.
pub fn finite(what: impl Into<String>, v: f64) -> Result<f64, CoreError> {
    let what = what.into();
    if !v.is_finite() {
        return Err(CoreError::NotFinite { what, value: format!("{v}") });
    }
    if v.abs() > MAX_MAGNITUDE {
        return Err(CoreError::OutOfRange {
            what,
            value: format!("{v}"),
            min: format!("-{MAX_MAGNITUDE}"),
            max: MAX_MAGNITUDE.to_string(),
        });
    }
    Ok(v)
}

/// Check a number against an explicit inclusive range.
pub fn in_range(what: impl Into<String>, v: f64, min: f64, max: f64) -> Result<f64, CoreError> {
    let what = what.into();
    let v = finite(what.clone(), v)?;
    // `!(v >= min && v <= max)` rather than `v < min || v > max` so NaN fails
    // the comparison. `finite` above already rejected it; this is belt and
    // braces for callers that pass a computed bound.
    if !(v >= min && v <= max) {
        return Err(CoreError::OutOfRange {
            what,
            value: format!("{v}"),
            min: format!("{min}"),
            max: format!("{max}"),
        });
    }
    Ok(v)
}

/// Check a free-text field.
pub fn text(what: impl Into<String>, s: &str) -> Result<(), CoreError> {
    let what = what.into();
    if s.len() > MAX_TEXT {
        return Err(CoreError::TooLong { what, len: s.len(), max: MAX_TEXT });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text limit has to fit the memory budget once the parse has
    /// expanded it.
    ///
    /// This is the property that was missing: three readers of untrusted JSON
    /// had three unrelated limits — 1 MiB, 4 MiB and 32 MiB — and the largest
    /// of them bought a gigabyte of memory, because none of them was written
    /// down as what it was for. §64.
    #[test]
    fn the_json_text_limit_fits_the_memory_budget() {
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same trick the model's limit test uses.
        let (text, expansion, budget) = (MAX_JSON_TEXT_BYTES, JSON_EXPANSION, MAX_JSON_MEMORY_BYTES);
        assert!(
            text * expansion <= budget,
            "a text of {text} bytes can cost {} bytes of memory, and the budget is {budget}",
            text * expansion
        );
        assert_eq!(text, budget / expansion);
    }

    /// The expansion is a measurement, and it is tied to something real.
    ///
    /// A JSON number is two bytes of text — `1,` — and a parsed value is
    /// `size_of::<Value>()` bytes, because a value has to carry its own type.
    /// The vector holding a long list can be twice its length, so the worst
    /// case is twice the value's size for every two bytes of text: the size
    /// of the value itself.
    ///
    /// If `serde_json`'s `Value` ever grows, this fails, and the text limits
    /// derived from [`JSON_EXPANSION`] have to be re-derived with it.
    #[test]
    fn the_expansion_covers_the_size_of_a_parsed_value() {
        let worst = std::mem::size_of::<serde_json::Value>();
        assert!(
            JSON_EXPANSION >= worst,
            "a parsed JSON value is {worst} bytes and two bytes of text, so the expansion is at least {worst}"
        );
    }

    #[test]
    fn finite_rejects_nan_and_infinity() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(finite("x", bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn finite_rejects_overlarge_magnitude() {
        assert!(finite("x", MAX_MAGNITUDE * 2.0).is_err());
        assert!(finite("x", -MAX_MAGNITUDE * 2.0).is_err());
        assert!(finite("x", 0.0).is_ok());
    }

    #[test]
    fn in_range_is_inclusive_and_nan_safe() {
        assert!(in_range("x", 0.0, 0.0, 1.0).is_ok());
        assert!(in_range("x", 1.0, 0.0, 1.0).is_ok());
        assert!(in_range("x", 1.0001, 0.0, 1.0).is_err());
        assert!(in_range("x", f64::NAN, 0.0, 1.0).is_err());
    }

    #[test]
    fn text_enforces_the_length_limit() {
        assert!(text("name", "ok").is_ok());
        let long = "x".repeat(MAX_TEXT + 1);
        assert!(text("name", &long).is_err());
    }
}
