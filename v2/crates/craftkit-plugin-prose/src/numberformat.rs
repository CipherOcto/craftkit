//! The **numbering format** of a number.
//!
//! A number may be written in a **named form**: a paragraph may be
//! the fourth one, a chapter may be the eighth, and a list may count
//! in letters.
//!
//! The forms this tree knows:
//!
//! - **roman** — `i`, `ii`, `iii`, `iv`: the letters of the Romans,
//!   and the number above three thousand nine hundred and
//!   ninety-nine has no roman form here;
//! - **letters** — `a`, `b`, `c`: the letters of the alphabet, and the
//!   twenty-seventh is `aa`;
//! - **ordinal** — `first`, `second`, `third`: the number in words
//!   with its place;
//! - **cardinal** — `one`, `two`, `three`: the number in words.
//!
//! And the rules:
//!
//! - a number that is **not a whole one** has no form here, and a
//!   number that is **negative** has none either: there is no
//!   negative letter and no negative roman numeral in this tree;
//! - a number that is **too large** for the form is **refused**, and
//!   not written as a long string of letters: a form that runs out is
//!   a form that is not there;
//! - a **name** this tree does not know is refused, and not guessed
//!   at.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Number, Outcome, Session, Text};

/// The largest number that has a **roman** form here.
const ROMAN_LIMIT: i64 = 3_999;

/// The largest number that has a **letter** form here.
const LETTER_LIMIT: i64 = 702;

/// The largest number that has a **word** form here.
const WORD_LIMIT: i64 = 999;

/// The **roman** form of a number.
pub fn roman_of(number: i64) -> String {
    const NUMERALS: [(i64, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut left = number;
    let mut out = String::new();
    for (value, numeral) in NUMERALS {
        while left >= value {
            out.push_str(numeral);
            left -= value;
        }
    }
    out
}

/// The **letter** form of a number: `a` is one and `aa` is the
/// twenty-seventh.
pub fn letter_of(number: i64) -> String {
    let mut left = number;
    let mut out = String::new();
    while left > 0 {
        let rest = (left - 1) % 26;
        out.insert(0, (b'a' + rest as u8) as char);
        left = (left - 1) / 26;
    }
    out
}

/// The **words** of a number below a thousand: the ones, the tens and
/// the hundreds.
fn words_of(number: i64) -> String {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] =
        ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    if number < 20 {
        return ONES[number as usize].to_string();
    }
    if number < 100 {
        let tens = TENS[(number / 10) as usize];
        let rest = number % 10;
        if rest == 0 {
            return tens.to_string();
        }
        return format!("{tens}-{}", ONES[rest as usize]);
    }
    if number < 1_000 {
        let hundreds = ONES[(number / 100) as usize];
        let rest = number % 100;
        if rest == 0 {
            return format!("{hundreds} hundred");
        }
        return format!("{hundreds} hundred and {}", words_of(rest));
    }
    String::new()
}

/// The **ordinal** form of a number: `first`, `second`, `twenty-first`.
pub fn ordinal_of(number: i64) -> String {
    const ORDINALS: [&str; 20] = [
        "zeroth",
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
    ];
    if number < 20 {
        return ORDINALS[number as usize].to_string();
    }
    if number < 100 {
        let tens = number / 10;
        let rest = number % 10;
        if rest == 0 {
            return match tens {
                2 => "twentieth".to_string(),
                3 => "thirtieth".to_string(),
                4 => "fortieth".to_string(),
                5 => "fiftieth".to_string(),
                6 => "sixtieth".to_string(),
                7 => "seventieth".to_string(),
                8 => "eightieth".to_string(),
                9 => "ninetieth".to_string(),
                _ => words_of(number),
            };
        }
        let tens_word = words_of(tens * 10);
        return format!("{tens_word}-{}", ORDINALS[rest as usize]);
    }
    if number < 1_000 {
        let hundreds = number / 100;
        let rest = number % 100;
        let word = match hundreds {
            1 => "one hundred".to_string(),
            2 => "two hundred".to_string(),
            3 => "three hundred".to_string(),
            4 => "four hundred".to_string(),
            5 => "five hundred".to_string(),
            6 => "six hundred".to_string(),
            7 => "seven hundred".to_string(),
            8 => "eight hundred".to_string(),
            _ => "nine hundred".to_string(),
        };
        if rest == 0 {
            return format!("{word}th");
        }
        return format!("{word} {}", ordinal_of(rest));
    }
    words_of(number)
}

/// The **form** a number is written in.
pub fn format_of(number: i64, format: &str) -> Option<String> {
    match format.trim() {
        "roman" => Some(roman_of(number)),
        "letters" => Some(letter_of(number)),
        "ordinal" => Some(ordinal_of(number)),
        "cardinal" => Some(words_of(number)),
        _ => None,
    }
}

/// **Read** the **numbering format** of a number.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "prose.number.format", area = "prose", layer = "prose", icon = "list")]
pub struct NumberFormat {
    /// The number to write.
    pub value: Number,
    /// The form: `roman`, `letters`, `ordinal` or `cardinal`.
    pub format: Text,
}

impl NumberFormat {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.number.format".to_string();
        let number = self.value.0;
        if !number.is_finite() || number.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{number} is not a whole number"),
            });
        }
        let whole = number as i64;
        if whole < 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{whole} is negative, and a negative number has no form here"),
            });
        }
        let format = self.format.0.trim();
        let Some(text) = format_of(whole, format) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{format}` is not a form this tree knows: the forms are roman, letters, ordinal and cardinal"
                ),
            });
        };
        // The **bounds** of the form: a number outside them has no
        // form here, and a form that runs out is a form that is not
        // there.
        let (low, high) = match format {
            "roman" => (1, ROMAN_LIMIT),
            "letters" => (1, LETTER_LIMIT),
            "ordinal" | "cardinal" => (0, WORD_LIMIT),
            _ => (0, 0),
        };
        if whole < low || whole > high {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{whole} has no {format} form: the form runs from {low} to {high}"),
            });
        }
        Ok(Outcome::new(text.clone()).with_data(json!({
            "value": whole,
            "format": format,
            "text": text,
        })))
    }
}
