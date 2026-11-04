//! The **words** of a number and the **letters** of a text.
//!
//! A formula may ask:
//!
//! - `REPT(text, times)` — a text **repeated**;
//! - `EXACT(text1, text2)` — whether two texts are the **same**,
//!   case and all;
//! - `BASE(number, radix, [length])` — a number written in **another
//!   base**, padded with zeros;
//! - `DECIMAL(text, radix)` — a text written in another base read as a
//!   number;
//! - `ROMAN(number, [form])` — a number written in **Roman numerals**;
//! - `ARABIC(text)` — Roman numerals read as a number;
//! - `CHAR(number)` and `CODE(text)` — the character of a code in the
//!   Windows-1252 set and the code of a character;
//! - `UNICHAR(number)` and `UNICODE(text)` — the same for **Unicode**;
//! - `ENCODEURL(text)` — a text written for a **URL**;
//! - `DBCS`, `JIS` and `PHONETIC` — the text itself, because this
//!   tree has no half-width characters and no furigana.
//!
//! The rules are the **tree's**: a number in a base is **whole** and
//! the base is from two to thirty-six, a Roman number is from zero to
//! three thousand nine hundred and ninety-nine, and a text longer
//! than the tree's bound is a value error.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **word functions**.
pub fn is_word_function(name: &str) -> bool {
    matches!(
        name,
        "REPT"
            | "EXACT"
            | "BASE"
            | "DECIMAL"
            | "ROMAN"
            | "ARABIC"
            | "CHAR"
            | "CODE"
            | "UNICHAR"
            | "UNICODE"
            | "ENCODEURL"
            | "DBCS"
            | "JIS"
            | "PHONETIC"
    )
}

/// The most characters a text may hold: the tree's bound.
const MAX_TEXT: usize = 32_767;

/// The numerals of Rome and what they are worth.
const NUMERALS: [(char, u32); 7] =
    [('I', 1), ('V', 5), ('X', 10), ('L', 50), ('C', 100), ('D', 500), ('M', 1000)];

/// The Windows-1252 code points of one hundred and twenty-eight to one
/// hundred and fifty-nine.
const CP1252: [u32; 32] = [
    0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152,
    0x8D, 0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161,
    0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
];

/// The tokens of a form of Roman numerals, largest value first.
///
/// The tree's rule: each higher form widens one of two limits on the
/// ratio of the subtracted numeral to the larger one, and the form
/// zero is the classic one.
fn roman_tokens(form: u32) -> Vec<(String, u32)> {
    let (pow10_max, five_max) = [(10, 0), (10, 20), (100, 20), (100, 200), (1000, 200)]
        .get(form as usize)
        .copied()
        .unwrap_or((10, 0));
    let mut tokens: Vec<(String, u32)> =
        NUMERALS.iter().map(|(character, value)| (character.to_string(), *value)).collect();
    for (index, (smaller, small_value)) in NUMERALS.iter().enumerate() {
        for (larger, large_value) in NUMERALS.iter().skip(index + 1) {
            let ratio = large_value / small_value;
            let is_power_of_ten = matches!(smaller, 'I' | 'X' | 'C');
            let allowed = ratio <= if is_power_of_ten { pow10_max } else { five_max };
            let value = large_value - small_value;
            // A pair that is worth a single numeral is never shorter.
            if allowed && !NUMERALS.iter().any(|(_, other)| *other == value) {
                tokens.push((format!("{smaller}{larger}"), value));
            }
        }
    }
    // The largest value first; on a tie the shorter, and so the
    // classic, token.
    tokens.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.len().cmp(&right.0.len())));
    tokens
}

/// A number written in **Roman numerals**.
fn roman_of(number: f64, form: u32) -> Result<String, String> {
    let number = number.trunc();
    if !(0.0..=3999.0).contains(&number) || !(0.0..=4.0).contains(&(form as f64)) {
        return Err("#VALUE!".to_string());
    }
    let tokens = roman_tokens(form);
    let mut rest = number as u32;
    let mut out = String::new();
    while rest > 0 {
        let Some((token, value)) = tokens.iter().find(|(_, value)| *value <= rest) else { break };
        out.push_str(token);
        rest -= value;
    }
    Ok(out)
}

/// Roman numerals read as a **number**.
fn arabic_of(text: &str) -> Result<f64, String> {
    let text = text.trim();
    if text.chars().count() > 255 {
        return Err("#VALUE!".to_string());
    }
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let mut values: Vec<i64> = Vec::new();
    for character in body.chars() {
        let upper = character.to_ascii_uppercase();
        let Some((_, value)) = NUMERALS.iter().find(|(numeral, _)| *numeral == upper) else {
            return Err("#VALUE!".to_string());
        };
        values.push(i64::from(*value));
    }
    let mut total = 0i64;
    for (index, value) in values.iter().enumerate() {
        if values.get(index + 1).is_some_and(|next| next > value) {
            total -= value;
        } else {
            total += value;
        }
    }
    Ok(if negative { -(total as f64) } else { total as f64 })
}

/// A number written in another **base**.
fn base_of(number: f64, radix: f64, length: f64) -> Result<String, String> {
    let number = number.trunc();
    let radix = radix.trunc();
    let length = length.trunc();
    if !(0.0..9.007_199_254_740_992e15).contains(&number)
        || !(2.0..=36.0).contains(&radix)
        || !(0.0..=255.0).contains(&length)
    {
        return Err("#NUM!".to_string());
    }
    let radix = radix as u64;
    let mut rest = number as u64;
    let mut digits: Vec<char> = Vec::new();
    loop {
        let digit = (rest % radix) as u32;
        digits.push(char::from_digit(digit, radix as u32).unwrap_or('0').to_ascii_uppercase());
        rest /= radix;
        if rest == 0 {
            break;
        }
    }
    while digits.len() < length as usize {
        digits.push('0');
    }
    Ok(digits.iter().rev().collect())
}

/// A text written in another **base**, read as a number.
fn decimal_of(text: &str, radix: f64) -> Result<f64, String> {
    let radix = radix.trunc();
    if !(2.0..=36.0).contains(&radix) || text.chars().count() > 255 {
        return Err("#NUM!".to_string());
    }
    let radix = radix as u32;
    let mut total = 0f64;
    for character in text.trim().chars() {
        let Some(digit) = character.to_digit(radix) else { return Err("#NUM!".to_string()) };
        total = total * f64::from(radix) + f64::from(digit);
    }
    if total >= 9.007_199_254_740_992e15 {
        return Err("#NUM!".to_string());
    }
    Ok(total)
}

impl Book {
    /// The **word functions**.
    pub(crate) fn words(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "REPT" => {
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let times = match self.number_at(sheet, arguments, 1, being) {
                    Ok(times) => times,
                    Err(message) => return Value::Error(message),
                };
                if times.is_nan() || times < 0.0 {
                    return Value::Error("#VALUE!".to_string());
                }
                let times = times.trunc();
                let total = text.chars().count() as f64 * times;
                if total > MAX_TEXT as f64 {
                    return Value::Error("#VALUE!".to_string());
                }
                Value::Text(text.repeat(times as usize))
            }
            "EXACT" => {
                let first = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let second = match self.text_at(sheet, arguments, 1, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                Value::Bool(first == second)
            }
            "BASE" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let radix = match self.number_at(sheet, arguments, 1, being) {
                    Ok(radix) => radix,
                    Err(message) => return Value::Error(message),
                };
                let length = match self.optional_number(sheet, arguments, 2, being) {
                    Ok(Some(length)) => length,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                match base_of(number, radix, length) {
                    Ok(text) => Value::Text(text),
                    Err(message) => Value::Error(message),
                }
            }
            "DECIMAL" => {
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let radix = match self.number_at(sheet, arguments, 1, being) {
                    Ok(radix) => radix,
                    Err(message) => return Value::Error(message),
                };
                match decimal_of(&text, radix) {
                    Ok(number) => Value::Number(number),
                    Err(message) => Value::Error(message),
                }
            }
            "ROMAN" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let form = match self.optional_number(sheet, arguments, 1, being) {
                    Ok(Some(form)) => form.trunc(),
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                match roman_of(number, form as u32) {
                    Ok(text) => Value::Text(text),
                    Err(message) => Value::Error(message),
                }
            }
            "ARABIC" => {
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                match arabic_of(&text) {
                    Ok(number) => Value::Number(number),
                    Err(message) => Value::Error(message),
                }
            }
            "CHAR" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let number = number.trunc();
                if !(1.0..=255.0).contains(&number) {
                    return Value::Error("#VALUE!".to_string());
                }
                let number = number as u32;
                let point =
                    if (128..160).contains(&number) { CP1252[(number - 128) as usize] } else { number };
                Value::Text(char::from_u32(point).map(String::from).unwrap_or_default())
            }
            "CODE" => {
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let Some(character) = text.chars().next() else {
                    return Value::Error("#VALUE!".to_string());
                };
                let point = character as u32;
                let code = if point < 128 || (160..256).contains(&point) {
                    point
                } else if let Some(index) = CP1252.iter().position(|other| *other == point) {
                    128 + index as u32
                } else {
                    63
                };
                Value::Number(f64::from(code))
            }
            "UNICHAR" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let number = number.trunc();
                if !(1.0..=1_114_111.0).contains(&number) {
                    return Value::Error("#VALUE!".to_string());
                }
                match char::from_u32(number as u32) {
                    Some(character) => Value::Text(character.to_string()),
                    None => Value::Error("#N/A".to_string()),
                }
            }
            "UNICODE" => {
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let Some(character) = text.chars().next() else {
                    return Value::Error("#VALUE!".to_string());
                };
                Value::Number(f64::from(character as u32))
            }
            "ENCODEURL" => {
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let mut out = String::with_capacity(text.len());
                for byte in text.bytes() {
                    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                        out.push(char::from(byte));
                    } else {
                        out.push_str(&format!("%{byte:02X}"));
                    }
                }
                Value::Text(out)
            }
            _ => {
                // `DBCS`, `JIS` and `PHONETIC`: the text itself.
                let text = match self.text_at(sheet, arguments, 0, being) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                Value::Text(text)
            }
        }
    }
}
