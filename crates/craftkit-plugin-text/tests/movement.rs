//! The **caret moves**: a character at a time, a word at a time, and a
//! line at a time.
//!
//! §151 said that there was no word-wise movement and that the caret
//! did not move a line at a time. These are the tests of both.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_text::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the text battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A **monospaced** paragraph of that text, measured at that width.
fn a_paragraph(session: &mut Session, text: &str, measure_mm: f64) -> u64 {
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": text, "metric": 1 }))
        .expect("the paragraph is added");
    let paragraph = session
        .document()
        .walk()
        .filter(|node| node.kind == "text.paragraph")
        .last()
        .map(|node| node.id)
        .expect("the paragraph is there");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": paragraph, "measure_mm": measure_mm }))
        .expect("the paragraph is measured");
    paragraph
}

/// The character the caret stands before.
fn character(session: &Session, paragraph: u64) -> usize {
    let node = session.document().node(paragraph).expect("the paragraph is there");
    let caret = node.children.iter().find(|child| child.kind == "text.caret");
    let Some(caret) = caret else { return usize::MAX };
    caret.data.get("character").and_then(|character| character.as_u64()).expect("the caret has a character")
        as usize
}

/// Put the caret at that character and move it that way.
fn move_the_caret(session: &mut Session, paragraph: u64, character: usize, by: &str) {
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": character }))
        .expect("the caret is put");
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": by }))
        .expect("the caret moves");
}

/// The caret moves a **word** at a time: to the end of the word it is
/// in, and then to the start of the next one.
#[test]
fn the_caret_moves_a_word_at_a_time() {
    let mut session = session();
    // "one two three": one is 0 to 2, the space is 3, two is 4 to 6, the
    // space is 7 and three is 8 onwards.
    let paragraph = a_paragraph(&mut session, "one two three", 40.0);
    move_the_caret(&mut session, paragraph, 0, "word_right");
    assert_eq!(character(&session, paragraph), 3, "the end of the first word");
    move_the_caret(&mut session, paragraph, 3, "word_right");
    assert_eq!(character(&session, paragraph), 4, "and the start of the next");
    move_the_caret(&mut session, paragraph, 5, "word_left");
    assert_eq!(character(&session, paragraph), 4, "the start of the word it is in");
    move_the_caret(&mut session, paragraph, 4, "word_left");
    assert_eq!(character(&session, paragraph), 0, "and the start of the one before");
}

/// The caret moves a **line** at a time, and it keeps its place across
/// the line.
#[test]
fn the_caret_moves_a_line_at_a_time() {
    let mut session = session();
    // "one two three four" at thirty millimetres is two lines: the
    // first holds "one two three" and the second holds "four".
    let paragraph = a_paragraph(&mut session, "one two three four", 30.0);
    move_the_caret(&mut session, paragraph, 0, "down");
    assert_eq!(character(&session, paragraph), 14, "the first character of the line below");
    move_the_caret(&mut session, paragraph, 14, "up");
    assert_eq!(character(&session, paragraph), 0, "and back to the line above");
}

/// The caret keeps its **place** across the line: it goes to the
/// character of the line below whose place is nearest its own.
#[test]
fn the_caret_keeps_its_place_across_a_line() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two three four", 30.0);
    // The third character of the first line is four millimetres in.
    move_the_caret(&mut session, paragraph, 2, "down");
    assert_eq!(character(&session, paragraph), 16, "the character nearest the same place");
}

/// There is no line above the first line, and none below the last: the
/// caret stays where it is.
#[test]
fn the_caret_stays_where_there_is_no_line() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two three four", 30.0);
    move_the_caret(&mut session, paragraph, 0, "up");
    assert_eq!(character(&session, paragraph), 0, "there is no line above");
    move_the_caret(&mut session, paragraph, 17, "down");
    assert_eq!(character(&session, paragraph), 17, "and none below");
}

/// A paragraph with **no measure** has no lines to move between.
#[test]
fn a_paragraph_with_no_measure_has_no_lines() {
    let mut session = session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two", "metric": 1 }))
        .expect("the paragraph is added");
    let paragraph = session
        .document()
        .walk()
        .filter(|node| node.kind == "text.paragraph")
        .last()
        .map(|node| node.id)
        .expect("the paragraph is there");
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 1 }))
        .expect("the caret is put");
    let error = session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "down" }))
        .expect_err("there are no lines");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
    assert_eq!(character(&session, paragraph), 1, "and the caret did not move");
}
