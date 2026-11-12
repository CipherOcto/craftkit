//! The **heuristic**: a word split into syllables, when the patterns do
//! not know it.
//!
//! # What this is
//!
//! A word the patterns have never seen — one with a letter they do not
//! have, most often — is split by **rule** instead: a syllable is a
//! vowel with its consonants around it, and the consonants that begin a
//! syllable stay with it.
//!
//! The rule is the family's rule, and it is a rule and not a
//! dictionary: it is right often enough to break a line with and wrong
//! often enough that the patterns are asked first.

/// The letters with a diacritic that are **consonants**.
///
/// Any other letter that is not ASCII is a **vowel**: `é` and `ø` and
/// `ą` are vowels, and letters of other scripts are vowels too, which
/// keeps the heuristic from breaking them.
const NON_ASCII_CONSONANTS: &str = "łńśćźżçčďđğģķļľĺňñņřŕšşșťţțžßŋðþ";

/// Whether a character is a **vowel**.
fn is_vowel(character: char) -> bool {
    let lower = character.to_lowercase().next().unwrap_or(character);
    if lower.is_ascii() {
        matches!(lower, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
    } else {
        lower.is_alphabetic() && !NON_ASCII_CONSONANTS.contains(lower)
    }
}

/// Whether two consonants **begin a syllable together**.
///
/// A pair like `ch`, `tr` or `st` is one sound and not two, and a
/// syllable that begins with one of them keeps it whole.
fn onset_pair(first: char, second: char) -> bool {
    let (first, second) = (first.to_ascii_lowercase(), second.to_ascii_lowercase());
    matches!(
        (first, second),
        ('c' | 's' | 't' | 'p' | 'w' | 'g', 'h')
            | ('b' | 'c' | 'd' | 'f' | 'g' | 'k' | 'p' | 't', 'r')
            | ('b' | 'c' | 'f' | 'g' | 'k' | 'p' | 's', 'l')
            | ('q', 'u')
            | ('s', 't' | 'p' | 'c' | 'k')
    )
}

/// The breaks of a word, by **rule**.
///
/// The word is split into syllables — a vowel with the consonants
/// around it — and the break is where one syllable ends and the next
/// begins. A word of fewer than four letters is not split, and a word
/// that holds something that is not a letter is not split at all.
pub fn points(word: &[char]) -> Vec<usize> {
    let length = word.len();
    if length < 4 || !word.iter().all(|character| character.is_alphabetic()) {
        return Vec::new();
    }
    let vowels: Vec<bool> = word.iter().map(|&character| is_vowel(character)).collect();
    // Both parts need a **sounded** vowel: a trailing `e`, `ed` or
    // `es` is silent, and the part before it has no vowel of its own.
    let end_of = |index: usize| {
        if length - index >= 2 && (word.ends_with(&['e', 'd']) || word.ends_with(&['e', 's'])) {
            length - 2
        } else if word.ends_with(&['e']) {
            length - 1
        } else {
            length
        }
    };
    // The bytes before each character, so that a length in bytes is
    // the length of the word the caller wrote.
    let mut bytes = Vec::with_capacity(length + 1);
    let mut total = 0_usize;
    for character in word {
        bytes.push(total);
        total += character.len_utf8();
    }
    bytes.push(total);
    // The first vowel at or after each character.
    let mut next_vowel = vec![length; length + 1];
    for index in (0..length).rev() {
        next_vowel[index] = if vowels[index] { index } else { next_vowel[index + 1] };
    }
    let mut out = Vec::new();
    let mut vowel_before = false;
    for index in 1..length {
        vowel_before |= vowels[index - 1];
        // The break is allowed where a syllable ends: after a vowel and
        // before a consonant, or between two consonants with a vowel
        // on each side and no pair that begins a syllable.
        let allowed = if vowels[index - 1] && !vowels[index] {
            (index + 1 < length && vowels[index + 1])
                || (index + 2 < length && onset_pair(word[index], word[index + 1]) && vowels[index + 2])
        } else if !vowels[index - 1] && !vowels[index] {
            (index >= 2
                && vowels[index - 2]
                && index + 1 < length
                && vowels[index + 1]
                && !onset_pair(word[index - 1], word[index])
                && word[index - 1] != word[index])
                || (word[index - 1] == word[index] && index >= 2 && vowels[index - 2])
        } else {
            false
        };
        let end = end_of(index);
        if allowed && next_vowel[index] < end && bytes[end] - bytes[index] > 1 && vowel_before {
            out.push(index);
        }
    }
    out
}
