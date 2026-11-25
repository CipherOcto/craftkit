//! The **limits of the typed value**: the numbers it will not hold.
//!
//! A length comes from outside — a command parameter, a file, an agent —
//! so it is checked before it is used, and a limit that no test names is
//! a limit nothing proves holds.

use craftkit_numeric::{Length, MAX_MILLIMETRES, NumericError};

/// A length **over** `MAX_MILLIMETRES` is refused.
///
/// The model holds a billion millimetres and no more: a number beyond it
/// is a mistake, and the mistake is named rather than clamped.
#[test]
fn a_length_over_the_limit_is_refused() {
    let value = MAX_MILLIMETRES + 1.0;
    let error = Length::millimetres(value).expect_err("that length is too long");
    match error {
        NumericError::OutOfRange { what, value: refused, limit } => {
            assert_eq!(what, "a length", "the error says what was measured");
            assert_eq!(refused, value, "the error says the number that was refused");
            assert_eq!(limit, MAX_MILLIMETRES, "the error says the limit it broke");
        }
        other => panic!("the error names the range: {other:?}"),
    }
}

/// A length **within** `MAX_MILLIMETRES` is a length.
///
/// This is the other half of the test above: a cap that refuses
/// everything is not a cap, it is a wall.
#[test]
fn a_length_within_the_limit_is_a_length() {
    let length = Length::millimetres(MAX_MILLIMETRES - 1.0).expect("the length is a length");
    assert_eq!(length.as_millimetres(), MAX_MILLIMETRES - 1.0, "the length is kept as it is");
}

/// The limit is **both ways**: a length is bounded by its size, not by
/// its sign.
#[test]
fn a_length_too_negative_is_refused() {
    let value = -(MAX_MILLIMETRES + 1.0);
    let error = Length::millimetres(value).expect_err("that length is too long");
    match error {
        NumericError::OutOfRange { limit, .. } => {
            assert_eq!(limit, MAX_MILLIMETRES, "the error says the limit it broke");
        }
        other => panic!("the error names the range: {other:?}"),
    }
}

/// A length **at** `MAX_MILLIMETRES` is the limit, and not past it.
#[test]
fn a_length_at_the_limit_is_kept() {
    let length = Length::millimetres(MAX_MILLIMETRES).expect("the length is a length");
    assert_eq!(length.as_millimetres(), MAX_MILLIMETRES);
    assert!(!length.is_zero(), "and it is a length");
}
