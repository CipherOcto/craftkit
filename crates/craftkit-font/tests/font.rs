//! The tests of the kit's typeface.
//!
//! A typeface is a table, and a table is only as good as the rules it
//! keeps. So the rules are: every letter and figure is there, every
//! glyph is wider than nothing, no two characters share an entry, the
//! letters are proportional, a string's width is the sum of its
//! letters, and a letter that is not there is left out rather than
//! drawn as a box.

use craftkit_font::{CAP_HEIGHT, DESCENT, Font, Metrics, entries, layout, width};

/// The typeface, once.
fn font() -> &'static Font {
    Font::builtin()
}

/// The metrics, once: a size of ten.
fn metrics() -> Metrics {
    Metrics::new(10.0).unwrap_or(Metrics { em: 10.0, x_height: 7.5, descent: 2.5, line_height: 12.5 })
}

/// Every letter, every figure and the marks a sentence needs.
#[test]
fn the_typeface_has_the_letters_and_the_figures() {
    let font = font();
    for character in 'A'..='Z' {
        assert!(font.glyph(character).is_some(), "the typeface has no capital {character}");
    }
    for character in 'a'..='z' {
        assert!(font.glyph(character).is_some(), "the typeface has no small {character}");
    }
    for character in '0'..='9' {
        assert!(font.glyph(character).is_some(), "the typeface has no figure {character}");
    }
    for character in ". , : ; ! ? ' \" - + = / ( ) [ ]".chars() {
        assert!(font.glyph(character).is_some(), "the typeface has no mark {character}");
    }
}

/// Every glyph is wider than nothing.
#[test]
fn every_glyph_is_wider_than_nothing() {
    for entry in entries() {
        assert!(entry.glyph.advance > 0.0, "the glyph of {} has no width", entry.character);
    }
}

/// No two characters share an entry.
///
/// The table is a list, and a list can say the same thing twice. The
/// typeface keeps the last one, so a duplicate is a line of the table
/// that is never read — and a line that is never read is a mistake.
#[test]
fn no_two_characters_share_an_entry() {
    let mut seen: Vec<char> = Vec::new();
    for entry in entries() {
        assert!(!seen.contains(&entry.character), "the table has {} twice", entry.character);
        seen.push(entry.character);
    }
    assert_eq!(seen.len(), font().characters().len(), "the typeface and the table disagree");
}

/// The space is nothing to draw and something to advance by.
#[test]
fn the_space_has_no_strokes_and_a_width() {
    let space = font().glyph(' ').expect("the typeface has a space");
    assert!(space.strokes.is_empty(), "the space has strokes");
    assert!(space.advance > 0.0, "the space has no width");
}

/// The letters are **proportional**: a narrow letter is narrower than a
/// wide one.
#[test]
fn the_letters_are_proportional() {
    let font = font();
    let narrow = font.advance('i').unwrap_or(0.0);
    let wide = font.advance('m').unwrap_or(0.0);
    assert!(narrow < wide, "the narrow letter is not narrower than the wide one");
}

/// A string is as wide as the sum of its letters.
#[test]
fn a_string_is_as_wide_as_its_letters() {
    let font = font();
    let metrics = metrics();
    let text = "Sphinx of black quartz";
    let mut expected = 0.0;
    for character in text.chars() {
        expected += font.advance(character).unwrap_or(0.0) * metrics.unit();
    }
    assert!((width(text, font, &metrics) - expected).abs() < 1e-9, "the width is not the sum");
}

/// The letters are placed from left to right, one after another.
#[test]
fn the_letters_are_placed_one_after_another() {
    let font = font();
    let metrics = metrics();
    let placed = layout("abc", font, &metrics);
    assert_eq!(placed.len(), 3, "three letters were placed");
    assert!(placed[0].x < placed[1].x, "the second letter is not to the right of the first");
    assert!(placed[1].x < placed[2].x, "the third letter is not to the right of the second");
    let last = placed[2].x + placed[2].advance;
    assert!((last - width("abc", font, &metrics)).abs() < 1e-9, "the last letter does not end the string");
}

/// A letter the typeface does not have is **left out**: there is nothing
/// to draw and nothing to advance by.
#[test]
fn a_letter_that_is_not_there_is_left_out() {
    let font = font();
    let metrics = metrics();
    assert!(font.glyph('\u{4e2d}').is_none(), "the typeface has a glyph it should not have");
    let placed = layout("a\u{4e2d}b", font, &metrics);
    assert_eq!(placed.len(), 2, "the unknown letter was placed");
    assert_eq!(placed[1].x, placed[0].x + placed[0].advance, "the unknown letter advanced the writing");
}

/// The outline of a letter stays inside the em: no letter is taller than
/// the cap height and none hangs further than the descender.
#[test]
fn the_outline_stays_inside_the_em() {
    let font = font();
    let metrics = metrics();
    let unit = metrics.unit();
    for character in font.characters() {
        let Some(glyph) = font.glyph(character) else { continue };
        let Some((lower, upper)) = glyph.box_of() else { continue };
        assert!(upper[1] <= CAP_HEIGHT + 1e-9, "the letter {character} is taller than the cap height");
        assert!(lower[1] >= DESCENT - 1e-9, "the letter {character} hangs below the descender");
        assert!(
            upper[0] - lower[0] <= glyph.advance + 1e-9,
            "the letter {character} is wider than its advance"
        );
    }
    // The unit is the em over the cap height: a cap letter is exactly
    // as tall as the em says.
    assert!((unit * CAP_HEIGHT - metrics.em).abs() < 1e-9, "the unit is not the em over the cap height");
}

/// A size that is not a number, or that is not positive, has no
/// metrics.
#[test]
fn a_size_that_is_not_a_size_is_refused() {
    assert!(Metrics::new(0.0).is_none(), "a size of nothing was accepted");
    assert!(Metrics::new(-4.0).is_none(), "a negative size was accepted");
    assert!(Metrics::new(f64::NAN).is_none(), "a size that is not a number was accepted");
    assert!(Metrics::new(12.0).is_some(), "a good size was refused");
}
