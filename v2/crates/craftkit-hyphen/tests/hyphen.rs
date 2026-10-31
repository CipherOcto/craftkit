//! Where a **word may be broken**.
//!
//! The kit works it out: the patterns say where the words of the
//! language break, and the limits say how careful the answer is.

use craftkit_hyphen::{Limits, english_patterns, points};

/// The classic example of the **patterns**.
///
/// The pattern set of Liang's own thesis breaks "hyphenation" in two
/// places, and the engine reads that set as it is written.
#[test]
fn the_patterns_break_a_word_where_the_thesis_says() {
    let word: Vec<char> = "hyphenation".chars().collect();
    let patterns =
        craftkit_hyphen::patterns::Patterns::parse("hy3ph he2n hena4 hen5at 1na n2at 1tio 2io o2n");
    assert_eq!(patterns.points(&word), Some(vec![2, 6]), "the points of the thesis");
}

/// The set that ships with the kit is a **trained** one, and it breaks
/// the same word in its own way.
///
/// The set was trained from a word list and not written by hand, so it
/// is not the thesis's set: it says "hy-phenat-ion" where the thesis
/// says "hy-phen-ation", and both are English.
#[test]
fn the_trained_set_breaks_a_word_in_its_own_way() {
    let word: Vec<char> = "hyphenation".chars().collect();
    let shipped = english_patterns();
    assert_eq!(shipped.points(&word), Some(vec![2, 7]), "the points of the kit");
    assert!(shipped.len() > 1000, "and the set is a real one: {}", shipped.len());
}

/// A word is broken by the **patterns**.
#[test]
fn a_word_is_broken_by_the_patterns() {
    let breaks = points("hyphenation", &Limits::default());
    assert!(breaks.contains(&2), "the word breaks after the second letter: {breaks:?}");
    assert!(breaks.contains(&7), "and after the seventh: {breaks:?}");
}

/// A **short** word is not broken.
///
/// The limits say a word of five letters or more, and a word of four
/// has nothing to break.
#[test]
fn a_short_word_is_not_broken() {
    assert!(points("word", &Limits::default()).is_empty(), "a short word is not broken");
    assert!(points("cat", &Limits::default()).is_empty(), "and a shorter one is not");
}

/// A break leaves **two letters** at each end.
///
/// The limits are the family's: a word of five letters breaks in one
/// place only, and that place is in the middle.
#[test]
fn a_break_leaves_two_letters_at_each_end() {
    let breaks = points("hyphenation", &Limits::default());
    for break_at in breaks {
        assert!(break_at >= 2, "the break is not at the head: {break_at}");
        assert!("hyphenation".chars().count() - break_at >= 2, "nor at the tail: {break_at}");
    }
}

/// The limits are **the caller's**.
#[test]
fn the_limits_are_the_callers() {
    let strict = Limits { min_word: 8, after_first: 3, before_last: 3, capitalized: true };
    let loose = points("hyphenation", &Limits::default());
    let careful = points("hyphenation", &strict);
    assert!(
        careful.len() <= loose.len(),
        "the careful limits do not give more breaks: {careful:?} and {loose:?}"
    );
}

/// A word in **capitals** is not broken.
///
/// The letters of an acronym are not a word, and a break in the middle
/// of one is a mistake that looks like a mistake.
#[test]
fn a_word_in_capitals_is_not_broken() {
    assert!(points("HYPHENATION", &Limits::default()).is_empty(), "an acronym is not broken");
}

/// A word that **begins with a capital** may be broken.
///
/// The default says yes: a capital says nothing about how a word
/// breaks.
#[test]
fn a_capital_word_may_be_broken() {
    assert!(!points("Hyphenation", &Limits::default()).is_empty(), "the word is broken");
}

/// A capital word may be **kept whole**.
#[test]
fn a_capital_word_may_be_kept_whole() {
    let limits = Limits { min_word: 5, after_first: 2, before_last: 2, capitalized: false };
    assert!(points("Hyphenation", &limits).is_empty(), "the word is whole");
}

/// An **apostrophe** is not a break.
///
/// A hyphen next to a quote hangs off it, and the word inside the
/// apostrophe is broken on its own.
#[test]
fn an_apostrophe_is_not_a_break() {
    let breaks = points("don't", &Limits::default());
    for break_at in &breaks {
        assert_ne!(*break_at, 4, "the break is not at the apostrophe: {breaks:?}");
    }
}

/// A word with an **unknown letter** is split by the heuristic.
///
/// The patterns do not know a letter they have never seen, and the
/// heuristic is what is left.
#[test]
fn a_word_with_an_unknown_letter_is_split_by_the_heuristic() {
    // The patterns know no `é`, so this word is the heuristic's.
    let breaks = points("café", &Limits { min_word: 2, after_first: 1, before_last: 1, capitalized: true });
    assert!(!breaks.is_empty(), "the word is split: {breaks:?}");
}
