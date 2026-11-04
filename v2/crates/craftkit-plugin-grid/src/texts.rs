//! The **shaping** of a text.
//!
//! A formula may shape a text: take a part of it, count it, fold it
//! or clean it. The tree's functions are here:
//!
//! - `LEFT(text, [count])` — the **first** characters of the text,
//!   and the first one when the count is not given;
//! - `RIGHT(text, [count])` — the **last** characters of the text;
//! - `MID(text, start, [count])` — the characters from the **start**,
//!   and the start is counted from one;
//! - `LEN(text)` — how many characters the text holds;
//! - `TRIM(text)` — the text without the spaces that are not between
//!   words;
//! - `UPPER(text)` and `LOWER(text)` — the text in one case or the
//!   other;
//! - `PROPER(text)` — the text with every **word** capitalised.
//!
//! The text of a value is the text it is **written** as: a number is
//! written as a number, a boolean as `TRUE` or `FALSE`, and an empty
//! value as nothing at all. And a value that is an **error** is that
//! error: the shaping of an error is the error.

use crate::formula::{Book, Expr, Value, truth_of};

/// Whether a name is one of the **text functions**.
pub fn is_text_function(name: &str) -> bool {
    matches!(
        name,
        "LEFT"
            | "RIGHT"
            | "MID"
            | "LEN"
            | "TRIM"
            | "UPPER"
            | "LOWER"
            | "PROPER"
            | "CONCATENATE"
            | "CONCAT"
            | "TEXTJOIN"
            | "FIND"
            | "SEARCH"
            | "SUBSTITUTE"
            | "REPLACE"
            | "T"
            | "CLEAN"
            | "LEFTB"
            | "RIGHTB"
            | "MIDB"
            | "LENB"
            | "FINDB"
            | "SEARCHB"
            | "REPLACEB"
    )
}

/// The most characters a text may hold.
///
/// The tree's bound: thirty-two thousand seven hundred and sixty-seven,
/// and the tree refuses a text that is longer.
const MAX_TEXT: usize = 32_767;

/// The text of a result, with the tree's bound on it.
fn text_val(text: String) -> Value {
    if text.len() > MAX_TEXT && text.chars().count() > MAX_TEXT {
        return Value::Error("#VALUE!".to_string());
    }
    Value::Text(text)
}

/// The **text** of a value: what it is written as.
///
/// An error is an error: the text of an error is not a text.
pub fn value_text(value: &Value) -> Result<String, String> {
    match value {
        Value::Error(message) => Err(message.clone()),
        other => Ok(display_text(other)),
    }
}

/// The **text** of a value, as a cell shows it.
fn display_text(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Error(message) => message.clone(),
    }
}

/// The **count** an argument holds: a whole number of characters.
///
/// A count that is negative is an error, and a count that is not a
/// number is an error. A count that is empty is zero — the tree's
/// rule: an argument that is there and holds nothing is zero.
pub fn count_argument(value: &Value) -> Result<usize, String> {
    let number = match value {
        Value::Empty => 0.0,
        other => other.as_number().ok_or_else(|| "#VALUE!".to_string())?,
    };
    if number.is_nan() || number < 0.0 {
        return Err("#VALUE!".to_string());
    }
    Ok(if number > 1e9 { 1_000_000_000 } else { number.trunc() as usize })
}

/// The **word** of a text: every word capitalised and the rest of it
/// in small letters.
///
/// A letter that is not in a word starts a new one, and so does
/// anything that is not a letter.
fn proper(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_word = false;
    for character in text.chars() {
        if character.is_alphabetic() {
            if in_word {
                out.extend(character.to_lowercase());
            } else {
                out.extend(character.to_uppercase());
            }
            in_word = true;
        } else {
            out.push(character);
            in_word = false;
        }
    }
    out
}

impl Book {
    /// The **text functions**: the shaping of a text.
    pub(crate) fn shaping(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.is_empty() {
            return Value::Error("#VALUE!".to_string());
        }
        // The **byte** versions are the same functions: this tree's
        // text is of single bytes.
        let name = match name {
            "LEFTB" => "LEFT",
            "RIGHTB" => "RIGHT",
            "MIDB" => "MID",
            "LENB" => "LEN",
            "FINDB" => "FIND",
            "SEARCHB" => "SEARCH",
            "REPLACEB" => "REPLACE",
            other => other,
        };
        let first = self.evaluate(sheet, &arguments[0], being);
        let text = match value_text(&first) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        match name {
            "LEFT" => match self.count_at(sheet, arguments, 1, 1.0, being) {
                Ok(count) => Value::Text(text.chars().take(count).collect()),
                Err(message) => Value::Error(message),
            },
            "RIGHT" => match self.count_at(sheet, arguments, 1, 1.0, being) {
                Ok(count) => {
                    let characters: Vec<char> = text.chars().collect();
                    let start = characters.len().saturating_sub(count);
                    Value::Text(characters[start..].iter().collect())
                }
                Err(message) => Value::Error(message),
            },
            "MID" => {
                let start = match arguments.get(1) {
                    Some(expression) => self.evaluate(sheet, expression, being).as_number(),
                    None => None,
                };

                let Some(start) = start else {
                    return Value::Error("#VALUE!".to_string());
                };
                if start.is_nan() || start < 1.0 {
                    return Value::Error("#VALUE!".to_string());
                }
                let start = if start > 1e9 { 1_000_000_000 } else { start.trunc() as usize };
                match self.count_at(sheet, arguments, 2, 0.0, being) {
                    Ok(count) => Value::Text(text.chars().skip(start - 1).take(count).collect()),
                    Err(message) => Value::Error(message),
                }
            }
            "LEN" => Value::Number(text.chars().count() as f64),
            "TRIM" => {
                Value::Text(text.split(' ').filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" "))
            }
            "UPPER" => Value::Text(text.to_uppercase()),
            "LOWER" => Value::Text(text.to_lowercase()),
            "PROPER" => Value::Text(proper(&text)),
            "CONCATENATE" | "CONCAT" => self.concatenating(sheet, arguments, being),
            "TEXTJOIN" => self.textjoin(sheet, arguments, being),
            "FIND" | "SEARCH" => self.finding(sheet, name, arguments, being),
            "SUBSTITUTE" => self.substituting(sheet, arguments, being),
            "REPLACE" => self.replacing(sheet, arguments, being),
            "T" => self.t_of(sheet, arguments, being),
            "CLEAN" => Value::Text(text.chars().filter(|character| (*character as u32) >= 32).collect()),
            _ => Value::Error(format!("there is no function `{name}`")),
        }
    }

    /// The **count** an argument holds: how many characters.
    fn count_at(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        default: f64,
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Result<usize, String> {
        match arguments.get(index) {
            None => Ok(default.trunc() as usize),
            Some(expression) => {
                let value = self.evaluate(sheet, expression, being);
                count_argument(&value)
            }
        }
    }

    /// The tree's `CONCATENATE` and `CONCAT`: the texts of the
    /// arguments, **joined**.
    ///
    /// The tree's `CONCAT` takes the arrays apart and the tree's
    /// `CONCATENATE` does not; here there are no arrays, so the two
    /// are the same thing.
    fn concatenating(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        let mut out = String::new();
        for argument in arguments {
            let value = self.evaluate(sheet, argument, being);
            let text = match value_text(&value) {
                Ok(text) => text,
                Err(message) => return Value::Error(message),
            };
            out.push_str(&text);
        }
        text_val(out)
    }

    /// The tree's `TEXTJOIN`: the texts of the arguments, joined with
    /// a **delimiter**.
    ///
    /// The tree's rule: the empty texts may be left out, and the
    /// delimiter is used again and again when there are more texts
    /// than delimiters.
    fn textjoin(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.len() < 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        let delimiter = match value_text(&value) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        let value = self.evaluate(sheet, &arguments[1], being);
        let ignore_empty = match truth_of(&value) {
            Ok(truth) => truth,
            Err(message) => return Value::Error(message),
        };
        let mut parts: Vec<String> = Vec::new();
        for argument in arguments.iter().skip(2) {
            let value = self.evaluate(sheet, argument, being);
            let text = match value_text(&value) {
                Ok(text) => text,
                Err(message) => return Value::Error(message),
            };
            if ignore_empty && text.is_empty() {
                continue;
            }
            parts.push(text);
        }
        let mut out = String::new();
        for (index, part) in parts.iter().enumerate() {
            if index > 0 && !delimiter.is_empty() {
                out.push_str(&delimiter);
            }
            out.push_str(part);
        }
        text_val(out)
    }

    /// The tree's `FIND` and `SEARCH`: the **place** of a text in
    /// another.
    ///
    /// The tree's `FIND` is exact and the tree's `SEARCH` is not: the
    /// search folds the case and takes the tree's wildcards. The place
    /// is counted from one, and the place the search begins at may not
    /// be below one and may not be past the end of the text.
    fn finding(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        let needle = match value_text(&needle) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        let hay = self.evaluate(sheet, &arguments[1], being);
        let hay = match value_text(&hay) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        let start = match arguments.get(2) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                value.as_number().unwrap_or(f64::NAN)
            }
            None => 1.0,
        };
        let characters: Vec<char> = hay.chars().collect();
        if start.is_nan() || start < 1.0 || start > characters.len().max(1) as f64 {
            return Value::Error("#VALUE!".to_string());
        }
        let start = start as usize - 1;
        if needle.is_empty() {
            return Value::Number((start + 1) as f64);
        }
        if name == "SEARCH" {
            // The wildcards, and the case folded.
            let folded: String = hay.chars().flat_map(char::to_lowercase).collect();
            let pattern: String = needle.chars().flat_map(char::to_lowercase).collect();
            let mut at = start;
            while at <= folded.chars().count() {
                if crate::formula::wildcard_at(&pattern, &folded.chars().skip(at).collect::<String>()) {
                    // The wildcards may be shorter than the rest of
                    // the text, and the place is where the pattern
                    // begins — so the search is done on the tail and
                    // the place is the tail's own.
                    return Value::Number((at + 1) as f64);
                }
                at += 1;
            }
            return Value::Error("#VALUE!".to_string());
        }
        let needle: Vec<char> = needle.chars().collect();
        let found = (start..characters.len()).find(|index| {
            characters.get(*index..index + needle.len()).is_some_and(|window| window == needle.as_slice())
        });
        match found {
            Some(place) => Value::Number((place + 1) as f64),
            None => Value::Error("#VALUE!".to_string()),
        }
    }

    /// The tree's `SUBSTITUTE`: a text with a part of it **changed**.
    ///
    /// The tree's rule: the old text is changed for the new one, and
    /// with the instance the change is made to the *second* change and
    /// not to the first; and an old text that is nothing changes
    /// nothing.
    fn substituting(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.len() < 3 || arguments.len() > 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let mut texts: Vec<String> = Vec::new();
        for argument in arguments.iter().take(3) {
            let value = self.evaluate(sheet, argument, being);
            match value_text(&value) {
                Ok(text) => texts.push(text),
                Err(message) => return Value::Error(message),
            }
        }
        let (text, old, new) = (texts[0].clone(), texts[1].clone(), texts[2].clone());
        if old.is_empty() {
            return text_val(text);
        }
        let instance = match arguments.get(3) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                match value.as_number() {
                    Some(number) if number.is_finite() && number >= 1.0 => number.trunc() as usize,
                    _ => return Value::Error("#VALUE!".to_string()),
                }
            }
            None => 1,
        };
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        let mut done = 0usize;
        loop {
            let Some(place) = rest.find(&old) else {
                out.push_str(rest);
                break;
            };
            done += 1;
            out.push_str(&rest[..place]);
            if done == instance {
                out.push_str(&new);
            } else {
                out.push_str(&old);
            }
            rest = &rest[place + old.len()..];
            if done >= instance {
                out.push_str(rest);
                break;
            }
        }
        text_val(out)
    }

    /// The tree's `REPLACE`: a text with a part of it **changed**.
    ///
    /// The part is counted from one, and the count is how many
    /// characters it is — and a part that is past the end of the text
    /// is nothing at all.
    fn replacing(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.len() != 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        let text = match value_text(&value) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        let value = self.evaluate(sheet, &arguments[1], being);
        let Some(start) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if start.is_nan() || start < 1.0 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[2], being);
        let Some(count) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if count.is_nan() || count < 0.0 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[3], being);
        let new = match value_text(&value) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        let characters: Vec<char> = text.chars().collect();
        let start =
            ((if start > 1e9 { 1_000_000_000 } else { start.trunc() as usize }) - 1).min(characters.len());
        let end = start.saturating_add(count.trunc() as usize).min(characters.len());
        let mut out: String = characters[..start].iter().collect();
        out.push_str(&new);
        out.extend(characters[end..].iter());
        text_val(out)
    }

    /// The tree's `T`: the text of a value, when it is a text.
    ///
    /// A number is a number and not a text, and `T` gives it as
    /// nothing at all.
    fn t_of(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.len() != 1 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        match value {
            Value::Text(text) => Value::Text(text),
            Value::Error(message) => Value::Error(message),
            _ => Value::Text(String::new()),
        }
    }
}
