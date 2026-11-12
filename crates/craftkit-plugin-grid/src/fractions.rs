//! The **fraction**: a number written as a fraction.
//!
//! The tree's fraction format is a code with **placeholders**: a whole
//! part, a numerator, a bar and a denominator — `# ?/?`, `# ??/??`,
//! `?/?` or `# ?/4`.
//!
//! The number is written as the **nearest** fraction, and the tree
//! finds that fraction with **continued fractions**: the best rational
//! number whose denominator is no bigger than the limit. The limit is
//! what the placeholders say: one digit of denominator is nine, and
//! two are ninety-nine.
//!
//! A code may also name the **denominator**: `# ?/4` is a quarter, and
//! the numerator is the number of quarters. That denominator is fixed,
//! and the fraction is written with it whatever the nearest one is.
//!
//! # What it does not do
//!
//! - **The zero padding is not here.** The tree's `0` placeholders in
//!   the numerator and the denominator pad them with zeros — `01/02` —
//!   and the tree's `?` placeholders leave blanks where a part is
//!   hidden. Here the numerator and the denominator are written as
//!   they are.
//! - **The tree clamps a long denominator and here it is refused.** The
//!   tree takes the last seven digits of a denominator that has more
//!   than seven; here a code with more than seven digits of
//!   denominator is not a format this tree has.

/// The **shape** of a fraction format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fraction {
    /// Whether the **whole** part is written beside the fraction:
    /// `2 1/2` and not `5/2`.
    pub whole: bool,
    /// The **limit** on the denominator: the largest denominator the
    /// fraction may have. A code that names the denominator has that
    /// one, and a code with placeholders has the largest the
    /// placeholders allow — nine for one digit, ninety-nine for two.
    pub limit: u64,
    /// Whether the denominator is **fixed**: a fixed denominator is the
    /// one the fraction is written with, and a free one is the nearest
    /// that fits.
    pub fixed: bool,
}

/// The most digits a **denominator** may have.
///
/// The tree's own bound: seven digits, and the tree clamps to it.
pub const MAX_DENOMINATOR_DIGITS: usize = 7;

/// The largest **fixed denominator** a code may name.
///
/// The tree's own bound: a billion, and the tree clamps to it.
pub const MAX_FIXED_DENOMINATOR: u64 = 1_000_000_000;

/// The **fraction** a code says, if it says one.
///
/// The code is a whole part, a space, a numerator, a bar and a
/// denominator — and the whole part may be left out. Without the space
/// the numerator is made of `?` placeholders only, because a `0` or a
/// `#` there would be a whole part that was not written down.
pub fn parse(code: &str) -> Option<Fraction> {
    let code = code.trim();
    let (whole_text, rest) = match code.split_once(' ') {
        Some((whole, rest)) => (Some(whole), rest.trim()),
        None => (None, code),
    };
    let (numerator, denominator) = rest.split_once('/')?;
    // The **whole** part: placeholders and nothing else.
    let whole = match whole_text {
        Some(whole) => {
            if whole.is_empty() || !whole.chars().all(|place| place == '#' || place == '0') {
                return None;
            }
            true
        }
        None => false,
    };
    // The **numerator**: placeholders and nothing else. A `#` there is
    // a whole part that was not written down, so it is refused when
    // there is no whole part.
    if numerator.is_empty() {
        return None;
    }
    if !numerator.chars().all(|place| place == '#' || place == '0' || place == '?') {
        return None;
    }
    if !whole && numerator.contains('#') {
        return None;
    }
    // The **denominator**: a number of its own, or placeholders.
    let (limit, fixed) = if !denominator.is_empty() && denominator.chars().all(|digit| digit.is_ascii_digit())
    {
        let number: u64 = denominator.parse().ok()?;
        if number == 0 || number > MAX_FIXED_DENOMINATOR {
            return None;
        }
        (number, true)
    } else {
        if denominator.is_empty()
            || !denominator.chars().all(|place| place == '#' || place == '0' || place == '?')
        {
            return None;
        }
        if denominator.chars().count() > MAX_DENOMINATOR_DIGITS {
            return None;
        }
        (10u64.pow(denominator.chars().count() as u32) - 1, false)
    };
    Some(Fraction { whole, limit, fixed })
}

/// The **text** a number is written with, as a fraction.
///
/// The sign is in front, and a number that rounds to zero has no sign:
/// the sign of a number that is not there is not written.
pub fn text(number: f64, fraction: &Fraction) -> String {
    if !number.is_finite() {
        return crate::format::general(number);
    }
    let value = number.abs();
    let mut whole = if fraction.whole { value.floor() } else { 0.0 };
    let rest = if fraction.whole { value - whole } else { value };
    let (mut numerator, denominator) = if fraction.fixed {
        let denominator = fraction.limit;
        ((rest * denominator as f64).round() as u64, denominator)
    } else {
        approximate(rest, fraction.limit)
    };
    // The tree's rule: a numerator that is not smaller than the
    // denominator is carried into the **whole** part, because the
    // fraction is below one and the whole part takes the rest.
    if fraction.whole && denominator > 0 && numerator >= denominator {
        whole += (numerator / denominator) as f64;
        numerator %= denominator;
    }
    // The sign is written only when there is something to be negative.
    let nothing = whole == 0.0 && numerator == 0;
    let sign = if number < 0.0 && !nothing { "-" } else { "" };
    if numerator == 0 && fraction.whole {
        return format!("{sign}{whole}");
    }
    if fraction.whole && whole > 0.0 {
        return format!("{sign}{whole} {numerator}/{denominator}");
    }
    format!("{sign}{numerator}/{denominator}")
}

/// The **nearest fraction** of a number, with a denominator no bigger
/// than the limit.
///
/// The tree's rule: the fraction is found with **continued fractions**,
/// and the last step takes the **semiconvergent** — the best of the
/// two ends of the last step — because the best fraction with a
/// denominator no bigger than the limit is often one step short of the
/// next one.
pub fn approximate(number: f64, limit: u64) -> (u64, u64) {
    if number.is_nan() || number <= 0.0 || !number.is_finite() {
        return (0, 1);
    }
    if number >= 1e15 {
        return (number.round() as u64, 1);
    }
    let (mut p0, mut q0, mut p1, mut q1) = (0u64, 1u64, 1u64, 0u64);
    let mut rest = number;
    for _ in 0..64 {
        let whole = rest.floor();
        if whole > 1e15 {
            break;
        }
        let whole = whole as u64;
        let Some(q2) = whole.checked_mul(q1).and_then(|value| value.checked_add(q0)) else { break };
        if q2 > limit {
            break;
        }
        let Some(p2) = whole.checked_mul(p1).and_then(|value| value.checked_add(p0)) else { break };
        (p0, q0, p1, q1) = (p1, q1, p2, q2);
        let fraction = rest - whole as f64;
        if fraction < 1e-12 {
            break;
        }
        rest = 1.0 / fraction;
    }
    if q1 == 0 {
        return (number.round() as u64, 1);
    }
    // The **semiconvergent**: the best of the two ends of the last
    // step.
    let step = (limit - q0) / q1;
    let (candidate, candidate_denominator) =
        (p0.saturating_add(step.saturating_mul(p1)), q0.saturating_add(step.saturating_mul(q1)));
    let error = (p1 as f64 / q1 as f64 - number).abs();
    let candidate_error = if candidate_denominator > 0 {
        (candidate as f64 / candidate_denominator as f64 - number).abs()
    } else {
        f64::INFINITY
    };
    if candidate_error < error { (candidate, candidate_denominator) } else { (p1, q1) }
}
