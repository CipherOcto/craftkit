//! A **number** in a **format**.
//!
//! The formats are the tree's own: `decimal`, `lowerRoman`,
//! `upperRoman`, `lowerLetter` and `upperLetter` for a page number,
//! and `arabic`, `roman`, `alphabetic` and `ALPHABETIC` for a caption.
//!
//! The spelling of a format decides: `alphabetic` and `ALPHABETIC`
//! are two formats and not one, and `roman` and `upperRoman` are two
//! as well.

/// The **page** formats: the tree's own.
pub const PAGE_FORMATS: [&str; 5] = ["decimal", "lowerRoman", "upperRoman", "lowerLetter", "upperLetter"];

/// The number written in a format.
///
/// A format that is not one of ours is written as the number is: the
/// caller has refused it before it comes here.
pub fn of(value: u64, format: &str) -> String {
    match format {
        "lowerRoman" => roman(value).to_lowercase(),
        "upperRoman" => roman(value).to_uppercase(),
        "lowerLetter" => alphabetic(value, false),
        "upperLetter" => alphabetic(value, true),
        "roman" => roman(value).to_lowercase(),
        "ALPHABETIC" => alphabetic(value, true),
        "alphabetic" => alphabetic(value, false),
        _ => value.to_string(),
    }
}

/// A number in **roman** numerals.
pub fn roman(value: u64) -> String {
    let table = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut left = value;
    let mut out = String::new();
    for (value, letters) in table {
        while left >= value {
            out.push_str(letters);
            left -= value;
        }
    }
    out
}

/// A number in **letters**: `a`, `b`, … `z`, `aa`, `ab`.
pub fn alphabetic(value: u64, upper: bool) -> String {
    let mut left = value.max(1);
    let mut letters = Vec::new();
    while left > 0 {
        let rest = (left - 1) % 26;
        letters.push((b'a' + rest as u8) as char);
        left = (left - 1) / 26;
    }
    letters.reverse();
    let letters: String = letters.into_iter().collect();
    if upper { letters.to_uppercase() } else { letters }
}
