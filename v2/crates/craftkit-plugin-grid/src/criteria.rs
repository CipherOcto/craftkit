//! The **criterion** of a counting function.
//!
//! `COUNTIF` and `SUMIF` take a **criterion**: a condition that a cell
//! may or may not meet. The criterion is a value, and a value is a
//! condition in the tree's own way:
//!
//! - a **number** is the number: the cell that holds it matches;
//! - a **boolean** is the boolean: the cell that holds it matches;
//! - an **empty** value is the number zero;
//! - **text** is a condition written out: it may begin with an
//!   operator — `=`, `<>`, `<`, `<=`, `>`, `>=` — and what follows the
//!   operator is what the cell is compared with.
//!
//! So `">2"` is *greater than two*, `"apple"` is *the text apple*, and
//! `"<>apple"` is *anything that is not the text apple*.
//!
//! # The wildcards
//!
//! A text criterion may hold the wildcards the tree has:
//!
//! - `*` is any **run** of characters, and it may be nothing at all;
//! - `?` is any **one** character;
//! - `~` is the escape: `~*` is a star and `~?` is a question mark.
//!
//! The comparison of text is **case-insensitive**, and so is the
//! wildcard: `APPLE` matches `apple`.
//!
//! # What the port found
//!
//! - **The tree's criterion has a length bound and this tree's has it
//!   too.** The tree refuses a text criterion of more than two hundred
//!   and fifty-five characters — the count is in the tree's own units,
//!   and the tree's note says it checked it on two versions of the
//!   tree's own application. Here the bound is the same two hundred
//!   and fifty-five characters, counted the same way, and a criterion
//!   over it is an error and not a truncated criterion.
//! - **The tree's criterion may be a date and this tree's is not.** The
//!   tree reads a criterion that looks like a date as the number of
//!   that date, so `">=1/1/2020"` is a date and not text; here a
//!   criterion is a number, a boolean or text, and a date is not one
//!   of them.
//! - **The tree's criterion may be an error and this tree's is too.**
//!   An error criterion is not raised: it is the condition *that the
//!   cell holds that error*, and a cell that holds it matches.

use std::cmp::Ordering;

use crate::formula::Value;

/// The most characters a text criterion may hold.
///
/// The tree's bound: two hundred and fifty-five, and the tree refuses
/// a criterion over it.
const MAX_CRITERION_CHARS: usize = 255;

/// The **comparison** a criterion makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

/// What a criterion is compared **with**.
#[derive(Clone, Debug)]
enum Operand {
    /// Nothing at all.
    Blank,
    /// A number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// An error, and what it says.
    Error(String),
    /// Text, and it may hold the wildcards.
    Text(String),
}

/// A **condition**: what a cell may or may not meet.
///
/// The tree calls this a criterion too, and so does the **filter's**
/// criterion in this crate — but the filter's criterion is an
/// operation and a value given **apart**, and this one is a single
/// value that says what it compares.
#[derive(Clone, Debug)]
pub struct Condition {
    /// The comparison.
    op: Op,
    /// What it is compared with.
    operand: Operand,
    /// The criterion was a **bare empty text** — `""` — and it matches
    /// the empty cells and the empty texts.
    bare_empty: bool,
}

impl Condition {
    /// The criterion of a **value**.
    ///
    /// A number is the number, a boolean is the boolean, an error is
    /// the error, an empty value is the number zero, and text is a
    /// condition written out.
    pub fn parse(value: &Value) -> Condition {
        match value {
            Value::Number(number) => {
                Condition { op: Op::Eq, operand: Operand::Number(*number), bare_empty: false }
            }
            Value::Bool(flag) => Condition { op: Op::Eq, operand: Operand::Bool(*flag), bare_empty: false },
            Value::Error(message) => {
                Condition { op: Op::Eq, operand: Operand::Error(message.clone()), bare_empty: false }
            }
            Value::Empty => Condition { op: Op::Eq, operand: Operand::Number(0.0), bare_empty: false },
            Value::Text(text) => Condition::parse_text(text),
        }
    }

    /// The criterion of an **argument**: a value, with the bound.
    ///
    /// A text criterion over the bound is an error: the tree refuses
    /// it, and a truncated criterion is a criterion that is not the
    /// one that was written.
    pub fn parse_argument(value: &Value) -> Result<Condition, String> {
        if let Value::Text(text) = value
            && text.chars().count() > MAX_CRITERION_CHARS
        {
            return Err(format!(
                "a criterion may be {MAX_CRITERION_CHARS} characters at most, and this one is {}",
                text.chars().count()
            ));
        }
        Ok(Condition::parse(value))
    }

    /// A criterion written out in **text**.
    fn parse_text(text: &str) -> Condition {
        if text.is_empty() {
            return Condition { op: Op::Eq, operand: Operand::Blank, bare_empty: true };
        }
        let (op, rest) = split_op(text);
        let operand = if rest.is_empty() {
            Operand::Blank
        } else if let Some(number) = number_text(rest) {
            Operand::Number(number)
        } else if rest.eq_ignore_ascii_case("TRUE") {
            Operand::Bool(true)
        } else if rest.eq_ignore_ascii_case("FALSE") {
            Operand::Bool(false)
        } else {
            Operand::Text(rest.to_string())
        };
        Condition { op, operand, bare_empty: false }
    }

    /// Whether a **value** meets the criterion.
    pub fn matches(&self, value: &Value) -> bool {
        if self.bare_empty {
            return match value {
                Value::Empty => true,
                Value::Text(text) => text.is_empty(),
                _ => false,
            };
        }
        match &self.operand {
            Operand::Blank => self.matches_blank(value),
            Operand::Number(number) => self.matches_number(value, *number),
            Operand::Bool(flag) => match value {
                Value::Bool(other) => cmp_op(self.op, other.cmp(flag)),
                _ => self.op == Op::Ne,
            },
            Operand::Error(message) => match value {
                Value::Error(other) => cmp_op(self.op, other.as_str().cmp(message.as_str())),
                _ => self.op == Op::Ne,
            },
            Operand::Text(pattern) => self.matches_text(value, pattern),
        }
    }

    /// The criterion is **nothing**: the cell is empty or not.
    fn matches_blank(&self, value: &Value) -> bool {
        let blank = matches!(value, Value::Empty);
        match self.op {
            Op::Eq => blank,
            Op::Ne => !blank,
            _ => match value {
                Value::Text(text) if !text.is_empty() => cmp_op(self.op, compare_text(text, "")),
                _ => false,
            },
        }
    }

    /// The criterion is a **number**.
    fn matches_number(&self, value: &Value, number: f64) -> bool {
        let cell = match value {
            Value::Number(other) => Some(*other),
            // A number written as text matches **only** for equality:
            // `COUNTIF(range,"1")` counts the cell that holds the text
            // `1`, and `COUNTIF(range,">1")` does not.
            Value::Text(text) if matches!(self.op, Op::Eq | Op::Ne) => number_text(text),
            _ => None,
        };
        match cell {
            Some(cell) => cmp_op(self.op, compare_numbers(cell, number)),
            None => self.op == Op::Ne,
        }
    }

    /// The criterion is **text**, and it may hold the wildcards.
    fn matches_text(&self, value: &Value, pattern: &str) -> bool {
        match self.op {
            Op::Eq | Op::Ne => {
                // An empty cell holds no text: `*` skips it and
                // `<>*` counts it.
                let text = match value {
                    Value::Text(text) => Some(text.as_str()),
                    _ => None,
                };
                let matched = text.is_some_and(|text| wildcard_match(pattern, text));
                if self.op == Op::Eq { matched } else { !matched }
            }
            _ => match value {
                Value::Text(text) => cmp_op(self.op, compare_text(text, pattern)),
                _ => false,
            },
        }
    }
}

/// Split the **operator** off a text criterion.
fn split_op(text: &str) -> (Op, &str) {
    if let Some(rest) = text.strip_prefix(">=") {
        (Op::Ge, rest)
    } else if let Some(rest) = text.strip_prefix("<=") {
        (Op::Le, rest)
    } else if let Some(rest) = text.strip_prefix("<>") {
        (Op::Ne, rest)
    } else if let Some(rest) = text.strip_prefix('=') {
        (Op::Eq, rest)
    } else if let Some(rest) = text.strip_prefix('>') {
        (Op::Gt, rest)
    } else if let Some(rest) = text.strip_prefix('<') {
        (Op::Lt, rest)
    } else {
        (Op::Eq, text)
    }
}

/// The comparison a criterion makes, applied to an **order**.
fn cmp_op(op: Op, order: Ordering) -> bool {
    match op {
        Op::Eq => order == Ordering::Equal,
        Op::Ne => order != Ordering::Equal,
        Op::Lt => order == Ordering::Less,
        Op::Le => order != Ordering::Greater,
        Op::Gt => order == Ordering::Greater,
        Op::Ge => order != Ordering::Less,
    }
}

/// The **order** of two numbers.
///
/// The tree's order rounds to the fifteenth digit, so that two numbers
/// that are the same to fifteen digits are the same number.
pub fn compare_numbers(left: f64, right: f64) -> Ordering {
    fn round15(number: f64) -> f64 {
        if !number.is_finite() || number == 0.0 {
            return number;
        }
        format!("{number:.14e}").parse().unwrap_or(number)
    }
    if left == right {
        return Ordering::Equal;
    }
    if (left - right).abs() > left.abs().max(right.abs()) * 1e-13 {
        return left.partial_cmp(&right).unwrap_or(Ordering::Equal);
    }
    round15(left).partial_cmp(&round15(right)).unwrap_or(Ordering::Equal)
}

/// The **order** of two texts: case-insensitive.
pub fn compare_text(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().flat_map(char::to_lowercase);
    let mut right = right.chars().flat_map(char::to_lowercase);
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) if a != b => return a.cmp(&b),
            _ => {}
        }
    }
}

/// A **number written as text**: what it is, if it is one.
///
/// A number is a number, and a percentage is the number it is a
/// percentage of. A date is not a number here — see the module's
/// note.
fn number_text(text: &str) -> Option<f64> {
    let text = text.trim();
    if let Some(rest) = text.strip_suffix('%') {
        return number_text(rest).map(|number| number / 100.0);
    }
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

/// Whether a **text** meets a **pattern**: the wildcards and all.
///
/// The matcher is the one the tree has — `*`, `?` and the escape `~` —
/// and it is the matcher the **filter's** criterion uses too, so there
/// is one wildcard in this crate and not two.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    crate::formula::wildcard(pattern, text)
}
