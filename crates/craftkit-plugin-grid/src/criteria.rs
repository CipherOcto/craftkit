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
//! And what follows the operator is read as a **number** by the tree's
//! own rule of numbers — the rule `VALUE` reads a text by — so a
//! currency sign, thousands separators, a percent sign, a fraction, a
//! date and a time are all numbers here.
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
//!   over it is an error and not a truncated criterion — and §1011 says
//!   how the count is made: in the **code units** of the text, and not
//!   in its characters.
//! - **The tree's criterion may be a date and this tree's is not.** The
//!   tree reads a criterion that looks like a date as the number of
//!   that date, so `">=1/1/2020"` is a date and not text; here a
//!   criterion is a number, a boolean or text, and a date is not one
//!   of them — and §1010 says otherwise: a criterion is read by the
//!   tree's own rule of numbers, and a date is one of them.
//! - **The tree's criterion may be an error and this tree's is too.**
//!   An error criterion is not raised: it is the condition *that the
//!   cell holds that error*, and a cell that holds it matches.

use std::cmp::Ordering;

use crate::dates::System;
use crate::formula::Value;

/// The most **code units** a text criterion may hold.
///
/// The tree's bound: two hundred and fifty-five, and the tree refuses
/// a criterion over it. The count is in the **code units** of the
/// text — as the tree's own application counts them — and so a text of
/// astral characters is counted twice over.
const MAX_CRITERION_UNITS: usize = 255;

/// The **length** of a text in the tree's units: the code units of the
/// text.
fn units_of(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The **characters** of a text that fit in the bound, counted in code
/// units.
///
/// A character that does not fit is not half-kept: the text ends
/// before it.
fn cut_to_units(text: &str, units: usize) -> String {
    let mut count = 0;
    for (index, character) in text.char_indices() {
        let size = character.len_utf16();
        if count + size > units {
            return text[..index].to_string();
        }
        count += size;
    }
    text.to_string()
}

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
    /// The **date system** of the workbook: a criterion that holds a
    /// date is read in it.
    system: System,
}

impl Condition {
    /// The criterion of a **value**.
    ///
    /// A number is the number, a boolean is the boolean, an error is
    /// the error, an empty value is the number zero, and text is a
    /// condition written out.
    pub fn parse(value: &Value, system: System) -> Condition {
        match value {
            Value::Number(number) => {
                Condition { op: Op::Eq, operand: Operand::Number(*number), bare_empty: false, system }
            }
            Value::Bool(flag) => {
                Condition { op: Op::Eq, operand: Operand::Bool(*flag), bare_empty: false, system }
            }
            Value::Error(message) => {
                Condition { op: Op::Eq, operand: Operand::Error(message.clone()), bare_empty: false, system }
            }
            Value::Empty => {
                Condition { op: Op::Eq, operand: Operand::Number(0.0), bare_empty: false, system }
            }
            Value::Text(text) => Condition::parse_text(text, system),
        }
    }

    /// The criterion of an **argument**: a value, with the bound.
    ///
    /// A text criterion over the bound is an error: the tree refuses
    /// it, and a truncated criterion is a criterion that is not the
    /// one that was written.
    pub fn parse_argument(value: &Value, system: System) -> Result<Condition, String> {
        if let Value::Text(text) = value {
            let units = units_of(text);
            if units > MAX_CRITERION_UNITS {
                return Err(format!(
                    "a criterion may be {MAX_CRITERION_UNITS} code units at most, and this one is \
                     {units}"
                ));
            }
        }
        Ok(Condition::parse(value, system))
    }

    /// The criterion of an **argument** in a function that takes more
    /// than one condition.
    ///
    /// The tree does not refuse a long criterion there: it **cuts** it
    /// to its first two hundred and fifty-five characters and matches
    /// that. So a criterion over the bound is a shorter condition and
    /// not an error.
    pub fn parse_argument_truncated(value: &Value, system: System) -> Condition {
        match value {
            Value::Text(text) if units_of(text) > MAX_CRITERION_UNITS => {
                let cut = cut_to_units(text, MAX_CRITERION_UNITS);
                Condition::parse_text(&cut, system)
            }
            other => Condition::parse(other, system),
        }
    }

    /// A criterion written out in **text**.
    ///
    /// What follows the operator is read as a **number** by the tree's
    /// own rule — the rule `VALUE` reads a text by — so a sign, a
    /// currency sign, thousands separators, a percent sign, an
    /// exponent, parentheses, a date and a time are all numbers here.
    fn parse_text(text: &str, system: System) -> Condition {
        if text.is_empty() {
            return Condition { op: Op::Eq, operand: Operand::Blank, bare_empty: true, system };
        }
        let (op, rest) = split_op(text);
        let operand = if rest.is_empty() {
            Operand::Blank
        } else if let Some(number) = crate::textvalue::number_of(rest, system) {
            Operand::Number(number)
        } else if rest.eq_ignore_ascii_case("TRUE") {
            Operand::Bool(true)
        } else if rest.eq_ignore_ascii_case("FALSE") {
            Operand::Bool(false)
        } else {
            Operand::Text(rest.to_string())
        };
        Condition { op, operand, bare_empty: false, system }
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
            Value::Text(text) if matches!(self.op, Op::Eq | Op::Ne) => {
                crate::textvalue::number_of(text, self.system)
            }
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

/// Whether a **text** meets a **pattern**: the wildcards and all.
///
/// The matcher is the one the tree has — `*`, `?` and the escape `~` —
/// and it is the matcher the **filter's** criterion uses too, so there
/// is one wildcard in this crate and not two.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    crate::formula::wildcard(pattern, text)
}
