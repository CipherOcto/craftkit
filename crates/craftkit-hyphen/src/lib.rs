//! `craftkit-hyphen` — **where a word may be broken**.
//!
//! # What this is
//!
//! A line breaker needs to know where a word may be broken, and there
//! are two ways to know: the caller says, or the kit works it out.
//! This is the kit working it out.
//!
//! - [`points`] is the answer: the places in a word where a hyphen may
//!   go, as the **character index** the hyphen goes before.
//! - The **patterns** are the bulk of it: a set of patterns says where
//!   the words of a language break, and they are read by
//!   [`patterns::Patterns`].
//! - The **limits** say how careful the answer is: how short a word
//!   must be before it may be broken at all, and how many characters
//!   must stay at each end.
//!
//! # The order of the answers
//!
//! A word is looked at in three ways, and the first that says something
//! is the answer:
//!
//! 1. the **patterns**, which know the words of the language;
//! 2. the **heuristic**, which splits a word into syllables when the
//!    patterns do not know it — a word with a letter the patterns have
//!    never seen.
//!
//! # Why it is a library and not a plugin
//!
//! Because it is arithmetic over words: it takes a word and gives back
//! the places it may be broken, and it knows nothing about a document,
//! a paragraph, a metric or the kit.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod english;
pub mod patterns;

mod heuristic;

use std::sync::OnceLock;

use patterns::Patterns;

/// The **limits** on where a word may be broken.
///
/// The limits are the paragraph's business — they are a setting, and a
/// setting that nobody asked for is a surprise — and the defaults are
/// the ones the family uses: a word of five letters or more, with two
/// characters left at each end of the break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The fewest letters a word may have before it may be broken.
    pub min_word: usize,
    /// How many letters must stay before the break.
    pub after_first: usize,
    /// How many letters must stay after the break.
    pub before_last: usize,
    /// Whether a word that **starts with a capital** may be broken.
    ///
    /// The default is yes: a capital says nothing about how a word
    /// breaks, and a line that ends in a capital is no rarer than any
    /// other.
    pub capitalized: bool,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { min_word: 5, after_first: 2, before_last: 2, capitalized: true }
    }
}

/// The patterns of **US English**.
///
/// The set is parsed once, when it is first asked for: parsing is a
/// walk over a hundred thousand characters, and a paragraph asks for
/// the points of every word it holds.
pub fn english_patterns() -> &'static Patterns {
    static PATTERNS: OnceLock<Patterns> = OnceLock::new();
    PATTERNS.get_or_init(|| Patterns::parse(english::PATTERNS))
}

/// The places a **word** may be broken.
///
/// The places are the **character indexes** the hyphen goes before, and
/// they are in the word as it was written: a word in capitals has the
/// same places as the same word in small letters, unless the limits say
/// otherwise.
///
/// The limits are applied to every run of letters in the word, and a
/// word is broken **run by run**: `well-known` is two words to a
/// hyphenator, and each is broken on its own.
pub fn points(word: &str, limits: &Limits) -> Vec<usize> {
    let characters: Vec<char> = word.chars().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if !is_word_char(characters[index]) {
            index += 1;
            continue;
        }
        let mut end = index;
        while end < characters.len() && is_word_char(characters[end]) {
            end += 1;
        }
        // A run does not begin or end with an apostrophe: the
        // apostrophe belongs to the word inside it.
        let mut start = index;
        let mut last = end;
        while start < last && is_apostrophe(characters[start]) {
            start += 1;
        }
        while last > start && is_apostrophe(characters[last - 1]) {
            last -= 1;
        }
        let core: Vec<char> = characters[start..last]
            .iter()
            .map(|character| {
                if is_apostrophe(*character) {
                    '\''
                } else {
                    character.to_lowercase().next().unwrap_or(*character)
                }
            })
            .collect();
        for point in breaks_of(&core) {
            // A break is never next to an apostrophe: a hyphen there
            // would hang off the quote.
            if point > 0 && point < core.len() && core[point - 1] != '\'' && core[point] != '\'' {
                out.push(start + point);
            }
        }
        index = end.max(index + 1);
    }
    // The limits: they are about the run, and a run is a word of its
    // own.
    let mut allowed = Vec::new();
    for point in out {
        if allowed_in(word, point, limits) {
            allowed.push(point);
        }
    }
    allowed
}

/// The breaks of a **run** of letters, without the limits.
///
/// The patterns come first, and the heuristic is what is left when the
/// patterns do not know the word.
fn breaks_of(core: &[char]) -> Vec<usize> {
    if core.len() < 2 {
        return Vec::new();
    }
    let patterns = english_patterns();
    if let Some(word) = patterns.exception(&core.iter().collect::<String>()) {
        return word.to_vec();
    }
    if let Some(points) = patterns.points(core) {
        return points;
    }
    heuristic::points(core)
}

/// Whether a break at that place is **allowed** by the limits.
fn allowed_in(word: &str, point: usize, limits: &Limits) -> bool {
    let characters: Vec<char> = word.chars().collect();
    if point == 0 || point >= characters.len() {
        return false;
    }
    // The run the break is in: the letters around it.
    let mut start = point;
    while start > 0 && is_word_char(characters[start - 1]) {
        start -= 1;
    }
    let mut end = point;
    while end < characters.len() && is_word_char(characters[end]) {
        end += 1;
    }
    let letters: Vec<char> =
        characters[start..end].iter().copied().filter(|character| character.is_alphabetic()).collect();
    if letters.len() < limits.min_word.max(2) {
        return false;
    }
    // A word in capitals is an **acronym**, and an acronym is not
    // broken: the letters are not a word.
    if letters.len() > 1 && letters.iter().all(|character| character.is_uppercase()) {
        return false;
    }
    // A word that begins with a capital may be broken only when the
    // limits say so.
    if !limits.capitalized && letters.first().is_some_and(|character| character.is_uppercase()) {
        return false;
    }
    // The letters before the break and the letters after it, inside the
    // run: the apostrophes are not letters and do not count.
    let before = characters[start..point].iter().filter(|character| character.is_alphabetic()).count();
    let after = characters[point..end].iter().filter(|character| character.is_alphabetic()).count();
    before >= limits.after_first.max(1) && after >= limits.before_last.max(1)
}

/// Whether a character is an **apostrophe**.
///
/// There are three of them in the world: the straight one, the curly
/// one and the one that looks like a turned comma.
fn is_apostrophe(character: char) -> bool {
    matches!(character, '\'' | '\u{2019}' | '\u{02bc}')
}

/// Whether a character is part of a **word**.
fn is_word_char(character: char) -> bool {
    character.is_alphabetic() || is_apostrophe(character)
}
