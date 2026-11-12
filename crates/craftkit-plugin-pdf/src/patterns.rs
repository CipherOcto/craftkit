//! The **patterns** of a redaction.
//!
//! A redaction may be made by **pattern**: the text that looks like a
//! phone number or an address or a card is found by the pattern, and
//! it is the pattern that says what it is looking for.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - The **patterns** are the tree's five: the phone numbers, the
//!   email addresses, the credit cards, the social security numbers
//!   and the dates. A pattern that is not one of them is refused by
//!   name.
//! - A **phone number** has ten or eleven digits and at least one
//!   separator, and a country code may come first.
//! - An **email address** has a local part, an `@`, and a domain with
//!   a dot and a top level domain of two letters or more.
//! - A **credit card** has thirteen to nineteen digits and it passes
//!   the Luhn check.
//! - A **social security number** has three digits, two and four,
//!   with a dash or a space between them, and its area is neither
//!   `000` nor `666` and does not begin with a nine.
//! - A **date** is a number, a separator and a number — `5/1/24`,
//!   `5-1-2024`, `5.1.2024`, `2024-01-05` — or a day and a month and
//!   a year, or a month and a day and a year.
//!
//! # What it does not do
//!
//! - **Nothing is redacted here.** The tree's patterns are found over
//!   the text of a page and the ranges are marked; here the ranges are
//!   read out of a text that is given.
//! - **The tree's patterns are matched glyph by glyph**, and here they
//!   are matched over characters.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Text};

/// The **patterns** of a redaction: the tree's own.
pub const PATTERNS: [&str; 5] = ["phone", "email", "credit-card", "ssn", "date"];

/// The **label** of a pattern: the tree's own.
pub fn label_of(pattern: &str) -> Option<&'static str> {
    match pattern {
        "phone" => Some("Phone Numbers"),
        "email" => Some("Email Addresses"),
        "credit-card" => Some("Credit Cards"),
        "ssn" => Some("Social Security Numbers"),
        "date" => Some("Dates"),
        _ => None,
    }
}

/// Whether a character is a **digit**.
fn digit(character: Option<&char>) -> bool {
    character.is_some_and(char::is_ascii_digit)
}

/// Whether a character is a **word** character.
fn word(character: Option<&char>) -> bool {
    character.is_some_and(|character| character.is_alphanumeric())
}

/// The **digits** and the separators between them, from `start`: the end
/// of the run and how many digits it holds.
fn digit_run(text: &[char], start: usize, separators: &[char], max_len: usize) -> (usize, usize) {
    let (mut at, mut count) = (start, 0);
    while at < text.len() && at - start < max_len {
        let character = text[at];
        if character.is_ascii_digit() {
            count += 1;
        } else if !(separators.contains(&character) && digit(text.get(at + 1))
            || (character == '(' || character == ')') && separators.contains(&character))
        {
            break;
        }
        at += 1;
    }
    // A run never ends on a separator.
    while at > start && !text[at - 1].is_ascii_digit() {
        at -= 1;
    }
    (at, count)
}

/// The length of a **phone number** at `at`, if there is one.
fn phone(text: &[char], at: usize) -> Option<usize> {
    let starts = text[at] == '+' && digit(text.get(at + 1))
        || text[at] == '(' && digit(text.get(at + 1))
        || text[at].is_ascii_digit();
    if !starts
        || word(at.checked_sub(1).and_then(|before| text.get(before)))
        || text.get(at.wrapping_sub(1)) == Some(&'+')
    {
        return None;
    }
    let first = if text[at] == '+' { at + 1 } else { at };
    let (end, count) = digit_run(text, first, &[' ', '-', '.', '(', ')'], 20);
    let separators = text[first..end].iter().filter(|character| !character.is_ascii_digit()).count();
    // Ten digits, or eleven with a country code, grouped by at least
    // one separator.
    if !(10..=11).contains(&count) || separators == 0 || digit(text.get(end)) {
        return None;
    }
    Some(end - at)
}

/// The length of an **email address** at `at`, if there is one.
fn email(text: &[char], at: usize) -> Option<usize> {
    let local = |character: &char| character.is_ascii_alphanumeric() || "._%+-".contains(*character);
    if !text[at].is_ascii_alphanumeric()
        || at.checked_sub(1).and_then(|before| text.get(before)).is_some_and(local)
    {
        return None;
    }
    let mut end = at;
    while end < text.len() && local(&text[end]) {
        end += 1;
    }
    if text.get(end) != Some(&'@') {
        return None;
    }
    let domain_start = end + 1;
    let mut domain_end = domain_start;
    while domain_end < text.len()
        && (text[domain_end].is_ascii_alphanumeric()
            || text[domain_end] == '-'
            || text[domain_end] == '.' && text.get(domain_end + 1).is_some_and(char::is_ascii_alphanumeric))
    {
        domain_end += 1;
    }
    let domain: String = text[domain_start..domain_end].iter().collect();
    let top = domain.rsplit('.').next().unwrap_or("");
    (domain.contains('.') && top.len() >= 2 && top.chars().all(|character| character.is_ascii_alphabetic()))
        .then_some(domain_end - at)
}

/// Whether the **digits** of a card pass the Luhn check.
fn luhn(digits: &[u32]) -> bool {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(at, digit)| {
            if !at.is_multiple_of(2) {
                let doubled = digit * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                *digit
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

/// The length of a **credit card** at `at`, if there is one.
fn credit_card(text: &[char], at: usize) -> Option<usize> {
    if !text[at].is_ascii_digit() || digit(at.checked_sub(1).and_then(|before| text.get(before))) {
        return None;
    }
    let (end, count) = digit_run(text, at, &[' ', '-'], 23);
    if !(13..=19).contains(&count) || digit(text.get(end)) {
        return None;
    }
    let digits: Vec<u32> = text[at..end].iter().filter_map(|character| character.to_digit(10)).collect();
    luhn(&digits).then_some(end - at)
}

/// The length of a **social security number** at `at`, if there is one.
fn ssn(text: &[char], at: usize) -> Option<usize> {
    if digit(at.checked_sub(1).and_then(|before| text.get(before))) {
        return None;
    }
    let group = |from: usize, count: usize| (from..from + count).all(|at| digit(text.get(at)));
    let separator = |at: usize| matches!(text.get(at), Some('-' | ' '));
    if !(group(at, 3) && separator(at + 3) && group(at + 4, 2) && separator(at + 6) && group(at + 7, 4))
        || digit(text.get(at + 11))
    {
        return None;
    }
    let area: String = text[at..at + 3].iter().collect();
    (area != "000" && area != "666" && !area.starts_with('9')).then_some(11)
}

/// The **months** of the tree's own.
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// The length of a **month** at `at`, if there is one: the name of it
/// or its three letters.
fn month(text: &[char], at: usize) -> Option<usize> {
    if word(at.checked_sub(1).and_then(|before| text.get(before))) {
        return None;
    }
    let rest: String = text[at..text.len().min(at + 10)].iter().collect::<String>().to_lowercase();
    for name in MONTHS {
        for candidate in [name, &name[..3]] {
            if rest.starts_with(candidate)
                && !rest[candidate.len()..].starts_with(|character: char| character.is_alphabetic())
            {
                let mut length = candidate.chars().count();
                if candidate.len() == 3 && text.get(at + length) == Some(&'.') {
                    length += 1;
                }
                return Some(length);
            }
        }
    }
    None
}

/// The length of a **number** at `at`, if there is one of the size.
fn number(text: &[char], at: usize, least: usize, most: usize) -> Option<usize> {
    let count = text[at.min(text.len())..]
        .iter()
        .take(most + 1)
        .take_while(|character| character.is_ascii_digit())
        .count();
    (count >= least && count <= most).then_some(count)
}

/// The length of a **date** at `at`, if there is one.
fn date(text: &[char], at: usize) -> Option<usize> {
    if word(at.checked_sub(1).and_then(|before| text.get(before))) {
        return None;
    }
    let spaces =
        |from: usize| text[from.min(text.len())..].iter().take_while(|character| **character == ' ').count();
    // A number, a separator and a number: `5/1/24`, `5-1-2024`,
    // `5.1.2024`, `2024-01-05`.
    if let Some(first) = number(text, at, 1, 4) {
        let separator = text.get(at + first).copied();
        if matches!(separator, Some('/' | '-' | '.'))
            && let Some(second) = number(text, at + first + 1, 1, 2)
            && text.get(at + first + 1 + second).copied() == separator
            && let Some(third) = number(
                text,
                at + first + second + 2,
                if first == 4 { 1 } else { 2 },
                if first == 4 { 2 } else { 4 },
            )
            && (first <= 2 || first == 4)
            && !digit(text.get(at + first + second + 2 + third))
        {
            return Some(first + second + third + 2);
        }
        // `5 January 2024`.
        if first <= 2 {
            let after = at + first + spaces(at + first);
            if after > at + first
                && let Some(length) = month(text, after)
            {
                let year_at = after + length + spaces(after + length);
                if let Some(year) = number(text, year_at, 4, 4) {
                    return Some(year_at + year - at);
                }
            }
        }
        return None;
    }
    // `January 5, 2024` and `Jan. 5 2024`.
    let length = month(text, at)?;
    let day_at = at + length + spaces(at + length);
    let day = number(text, day_at, 1, 2)?;
    let mut year_at = day_at + day;
    if text.get(year_at) == Some(&',') {
        year_at += 1;
    }
    year_at += spaces(year_at);
    let year = number(text, year_at, 4, 4)?;
    Some(year_at + year - at)
}

/// The **ranges** of the text that match the pattern: the tree's own.
pub fn find(pattern: &str, text: &[char]) -> Vec<std::ops::Range<usize>> {
    let matcher: fn(&[char], usize) -> Option<usize> = match pattern {
        "phone" => phone,
        "email" => email,
        "credit-card" => credit_card,
        "ssn" => ssn,
        "date" => date,
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        match matcher(text, at) {
            Some(length) if length > 0 => {
                out.push(at..at + length);
                at += length;
            }
            _ => at += 1,
        }
    }
    out
}

commands! {
    /// The **patterns** of a redaction: the tree's five, with their labels.
    pdf.redact.patterns {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
        },
        run: {

            let patterns: Vec<Json> = PATTERNS
                .iter()
                .filter_map(|pattern| label_of(pattern).map(|label| json!({ "id": pattern, "label": label })))
                .collect();
            Ok(Outcome::new(format!("there are {} pattern(s)", patterns.len())).with_data(json!({
                "patterns": patterns,
                "count": patterns.len(),
            })))

        }
    }
    /// The **text** that matches a pattern: the ranges of it, and the text
    /// of each range.
    pdf.redact.pattern.find {
        area: "pdf",
        layer: "pdf",
        icon: "find",
        params: {
            /// The text to look in.

            text: Text,
            /// The pattern: `phone`, `email`, `credit-card`, `ssn` or `date`.

            pattern: Text,
        },
        run: {

            let command = "pdf.redact.pattern.find";
            let pattern = self.pattern.0.trim().to_ascii_lowercase();
            if !PATTERNS.contains(&pattern.as_str()) {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "`{pattern}` is not a pattern of a redaction: the patterns are {}",
                        PATTERNS.join(", ")
                    ),
                });
            }
            let text: Vec<char> = self.text.0.chars().collect();
            let ranges: Vec<Json> = find(&pattern, &text)
                .into_iter()
                .map(|range| {
                    let found: String = text[range.clone()].iter().collect();
                    json!({ "start": range.start, "end": range.end, "text": found })
                })
                .collect();
            Ok(Outcome::new(format!("the text holds {} match(es)", ranges.len())).with_data(json!({
                "pattern": pattern,
                "matches": ranges,
                "count": ranges.len(),
            })))

        }
    }
}
