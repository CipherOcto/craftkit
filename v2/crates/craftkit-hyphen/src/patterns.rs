//! The **patterns**: where a word may be broken, read from a set.
//!
//! # What this is
//!
//! A pattern is a word with **digits** in it, and a digit says how many
//! letters are on that side of a break: `a1b` is the pattern "a-b",
//! and `4re` is the pattern with four letters before it. Many patterns
//! match the same place in a word, and the **biggest** of their digits
//! wins: an odd value means a break is allowed there.
//!
//! This is the notation the family uses, and it is the notation the
//! pattern sets are written in.
//!
//! # Why it is a library and not a plugin
//!
//! Because it is arithmetic over words: it takes a word and gives back
//! the places it may be broken, and it knows nothing about a document,
//! a paragraph or the kit.

use std::collections::HashMap;

/// The most letters a pattern may have.
///
/// The packing below gives six bits to a letter, and the key holds the
/// length on top, so a pattern of more than twenty letters cannot be
/// told from another one.
const MAX_PATTERN_LEN: usize = 20;

/// The **alphabet**: the letters the patterns know.
///
/// A letter is given a code, and the code is what is packed into the
/// key: six bits a letter, so the key is a number and the lookup is a
/// hash. A letter that no pattern mentions has no code, and a word that
/// holds one cannot be read by the patterns at all.
#[derive(Clone, Debug)]
struct Alphabet {
    /// The codes of the ASCII letters, by their byte.
    ascii: [u8; 128],
    /// The codes of the letters that are not ASCII, in the order they
    /// were first seen.
    other: Vec<(char, u8)>,
    /// The next code to hand out.
    next: u8,
}

impl Default for Alphabet {
    fn default() -> Alphabet {
        // The code 1 is the **word boundary**: the dot of the notation.
        let mut ascii = [0_u8; 128];
        ascii['.' as usize] = 1;
        Alphabet { ascii, other: Vec::new(), next: 2 }
    }
}

impl Alphabet {
    /// The code of a letter, or zero when the alphabet does not have it.
    fn code(&self, character: char) -> u8 {
        if (character as u32) < 128 {
            self.ascii[character as usize]
        } else {
            self.other.iter().find(|(letter, _)| *letter == character).map_or(0, |(_, code)| *code)
        }
    }

    /// The code of a letter, **adding it** to the alphabet when it is
    /// new.
    ///
    /// Nothing is added when the alphabet is full: a set of patterns
    /// with more than sixty letters is not a set of patterns this
    /// engine reads.
    fn intern(&mut self, character: char) -> Option<u8> {
        let code = self.code(character);
        if code != 0 {
            return Some(code);
        }
        if self.next >= 63 {
            return None;
        }
        let code = self.next;
        self.next += 1;
        if (character as u32) < 128 {
            self.ascii[character as usize] = code;
        } else {
            self.other.push((character, code));
        }
        Some(code)
    }
}

/// The packed key of a run of codes.
///
/// The length goes on top, so that the keys of two runs of different
/// lengths are never the same.
fn pack(codes: &[u8]) -> u128 {
    let mut key = 0_u128;
    for &code in codes {
        key = (key << 6) | u128::from(code);
    }
    key | ((codes.len() as u128) << 120)
}

/// A **set of patterns**, and the exception words that go with them.
#[derive(Clone, Debug, Default)]
pub struct Patterns {
    /// The letters the patterns know.
    alphabet: Alphabet,
    /// The patterns: a packed run of letters, and the values between
    /// them. The values are one longer than the run, and the value at
    /// the end is the one after the last letter.
    map: HashMap<u128, Box<[u8]>>,
    /// The longest pattern in the set.
    longest: usize,
    /// The words that are broken in a way the patterns do not say: the
    /// word in lower case, and the places it breaks.
    exceptions: HashMap<String, Box<[usize]>>,
}

impl Patterns {
    /// Read a set of patterns from **text**.
    ///
    /// The text is whitespace-separated patterns, and a `%` starts a
    /// comment that runs to the end of the line. A pattern that holds
    /// a hyphen is an **exception word** written with its breaks —
    /// `ni-gdy` — and a word without a hyphen is never broken.
    pub fn parse(text: &str) -> Patterns {
        let mut patterns = Patterns::default();
        for line in text.lines() {
            let line = line.split('%').next().unwrap_or("");
            for token in line.split_whitespace() {
                if token.contains('-') {
                    patterns.insert_exception(token);
                } else {
                    patterns.insert(token);
                }
            }
        }
        patterns
    }

    /// Add one pattern, and say whether it could be read.
    ///
    /// A pattern that is too long, or that holds a letter the alphabet
    /// is full of, is **not** added: a pattern that is not read is a
    /// pattern that says nothing, and saying nothing quietly is worse
    /// than saying so.
    pub fn insert(&mut self, pattern: &str) -> bool {
        let mut codes = Vec::with_capacity(pattern.len());
        let mut values = vec![0_u8];
        for character in pattern.chars() {
            if let Some(digit) = character.to_digit(10) {
                if let Some(last) = values.last_mut() {
                    *last = digit as u8;
                }
                continue;
            }
            let Some(code) = self.alphabet.intern(character) else { return false };
            codes.push(code);
            values.push(0);
        }
        if codes.is_empty() || codes.len() > MAX_PATTERN_LEN {
            return false;
        }
        self.longest = self.longest.max(codes.len());
        self.map.insert(pack(&codes), values.into_boxed_slice());
        true
    }

    /// Add an **exception word**, written with its breaks.
    ///
    /// The word is the exception's whole truth: `ni-gdy` breaks at one
    /// place and at no other, and a word written without a hyphen is
    /// never broken at all.
    pub fn insert_exception(&mut self, word: &str) {
        let mut plain = String::with_capacity(word.len());
        let mut points = Vec::new();
        let mut length = 0_usize;
        for character in word.chars() {
            if character == '-' {
                if length > 0 && points.last() != Some(&length) {
                    points.push(length);
                }
            } else {
                plain.extend(character.to_lowercase());
                length += 1;
            }
        }
        points.retain(|&point| point < length);
        if length > 0 {
            self.exceptions.insert(plain, points.into_boxed_slice());
        }
    }

    /// The breaks of an **exception word**, if it is one.
    pub fn exception(&self, word: &str) -> Option<&[usize]> {
        self.exceptions.get(word).map(|points| &points[..])
    }

    /// How many patterns the set holds.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the set holds no patterns at all.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The values **between** the letters of a word.
    ///
    /// The values are one longer than the word, and the value at `i` is
    /// the one **before** the letter at `i`. There is nothing to say
    /// about a word that holds a letter the alphabet does not have.
    pub fn values(&self, word: &[char]) -> Option<Vec<u8>> {
        let mut codes = Vec::with_capacity(word.len() + 2);
        codes.push(1);
        for &character in word {
            let code = self.alphabet.code(character);
            if code == 0 {
                return None;
            }
            codes.push(code);
        }
        codes.push(1);
        let mut values = vec![0_u8; codes.len() + 1];
        apply(&self.map, self.longest, &codes, &mut values);
        // The padded word holds one letter more at each end, so the
        // word's own values start one place in.
        values.get(1..codes.len()).map(<[u8]>::to_vec)
    }

    /// The **breaks** of a word, if the patterns know it.
    ///
    /// A break is a place where a hyphen goes **before** a letter, and
    /// an odd total means a break is allowed there.
    pub fn points(&self, word: &[char]) -> Option<Vec<usize>> {
        let values = self.values(word)?;
        Some((1..word.len()).filter(|&index| values.get(index).is_some_and(|value| value % 2 == 1)).collect())
    }
}

/// Add the patterns to the values of a padded word.
///
/// Every place in the word is looked at, and at each place the
/// patterns that end there are added up: the biggest wins.
fn apply(map: &HashMap<u128, Box<[u8]>>, longest: usize, codes: &[u8], values: &mut [u8]) {
    for start in 0..codes.len() {
        let mut key = 0_u128;
        for (length, &code) in codes.iter().skip(start).take(longest.min(MAX_PATTERN_LEN)).enumerate() {
            key = (key << 6) | u128::from(code);
            let Some(pattern) = map.get(&(key | (((length + 1) as u128) << 120))) else { continue };
            for (offset, &value) in pattern.iter().enumerate() {
                if let Some(slot) = values.get_mut(start + offset) {
                    *slot = (*slot).max(value);
                }
            }
        }
    }
}
