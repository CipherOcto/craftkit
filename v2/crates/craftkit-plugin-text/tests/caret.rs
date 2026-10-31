//! The **caret**: where the reader is in the text.
//!
//! The caret is a node under the paragraph, and its character is the
//! setting it carries. Its **place** is derived: the paragraph is
//! broken into lines and the character is wherever the break put it.
//! These are the tests of both: the setting and the place.

use craftkit_sdk::serde_json::{Value, json};
use craftkit_sdk::{Ecosystem, RecordingCanvas, Session};

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
///
/// The metric is two millimetres for every character and two for the
/// space, so a character is two millimetres and the text of "one two"
/// is fourteen millimetres long.
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

/// The paragraph's **words**, as the payload holds them.
fn words(session: &Session, paragraph: u64) -> Vec<String> {
    session
        .document()
        .node(paragraph)
        .and_then(|node| node.data.get("words"))
        .and_then(|words| words.as_array())
        .map(|words| words.iter().filter_map(|word| word.as_str().map(|word| word.to_string())).collect())
        .expect("the paragraph has words")
}

/// The caret of **that** paragraph: its id and the character it stands
/// before.
///
/// The caret is looked for under the paragraph and not anywhere in the
/// document: a document may hold many paragraphs, and each of them has
/// its own caret.
fn the_caret(session: &Session, paragraph: u64) -> Option<(u64, usize)> {
    let node = session.document().node(paragraph)?;
    let caret = node.children.iter().find(|child| child.kind == "text.caret")?;
    let character = caret.data.get("character")?.as_u64()?;
    Some((caret.id, character as usize))
}

/// The character the caret stands before.
fn character(session: &Session, paragraph: u64) -> usize {
    the_caret(session, paragraph).expect("there is a caret").1
}

/// The caret's **place**: where the character it stands before stands.
fn place(session: &Session, paragraph: u64) -> f64 {
    let (id, _) = the_caret(session, paragraph).expect("there is a caret");
    let node = session.document().node(id).expect("the caret is there");
    session.place_of(node).expect("the caret has a place").x
}

/// A caret is a **node** under the paragraph, and it carries the
/// character it stands before.
#[test]
fn the_caret_is_a_node_under_the_paragraph() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    let caret = session.document().walk().find(|node| node.kind == "text.caret").expect("there is a caret");
    let parent = session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == caret.id))
        .expect("the caret is held");
    assert_eq!(parent.id, paragraph, "the caret is in the paragraph");
    assert_eq!(caret.data["character"], json!(3), "and it says which character it stands before");
}

/// The caret is put where it is asked for, and putting it again moves
/// it: there is one caret in a paragraph and not two.
#[test]
fn the_caret_is_put_where_it_is_asked_for() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 1 }))
        .expect("the caret is put");
    assert_eq!(character(&session, paragraph), 1);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 5 }))
        .expect("the caret is put again");
    assert_eq!(character(&session, paragraph), 5, "the caret moved");
    let carets = session.document().walk().filter(|node| node.kind == "text.caret").count();
    assert_eq!(carets, 1, "and there is still one caret");
}

/// The caret moves a character at a time, and to the ends of the text.
#[test]
fn the_caret_moves_a_character_at_a_time() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "left" }))
        .expect("the caret moves left");
    assert_eq!(character(&session, paragraph), 2);
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "right" }))
        .expect("the caret moves right");
    assert_eq!(character(&session, paragraph), 3, "and it is back where it was");
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "start" }))
        .expect("the caret goes to the start");
    assert_eq!(character(&session, paragraph), 0);
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "end" }))
        .expect("the caret goes to the end");
    assert_eq!(character(&session, paragraph), 7, "the text is seven characters long");
}

/// The caret does not go past the ends of the text: it stops there.
#[test]
fn the_caret_stops_at_the_ends_of_the_text() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 0 }))
        .expect("the caret is put");
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "left" }))
        .expect("the caret moves left");
    assert_eq!(character(&session, paragraph), 0, "there is nothing before the first character");
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "end" }))
        .expect("the caret goes to the end");
    session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "right" }))
        .expect("the caret moves right");
    assert_eq!(character(&session, paragraph), 7, "and nothing after the last one");
}

/// A caret past the end of the text is refused, and a character that is
/// not a whole number of characters is refused.
#[test]
fn a_caret_past_the_end_of_the_text_is_refused() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    let error = session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 8 }))
        .expect_err("the caret is past the end of the text");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
    let error = session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 1.5 }))
        .expect_err("a character is a whole number of characters");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
    assert!(the_caret(&session, paragraph).is_none(), "and there is no caret");
}

/// A way that is not a way to move a caret is refused.
#[test]
fn a_way_that_is_not_a_way_is_refused() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    let error = session
        .execute("text.caret.move", &json!({ "paragraph": paragraph, "by": "sideways" }))
        .expect_err("`sideways` is not a way to move a caret");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
    assert_eq!(character(&session, paragraph), 3, "and the caret did not move");
}

/// Typing puts the text into the paragraph, and the caret stands after
/// what was typed.
#[test]
fn typing_puts_the_text_into_the_paragraph() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 0 }))
        .expect("the caret is put");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": "ab" }))
        .expect("the text is typed");
    assert_eq!(words(&session, paragraph), vec!["abone".to_string(), "two".to_string()]);
    assert_eq!(character(&session, paragraph), 2, "the caret stands after what was typed");
}

/// Typing in the middle of a word makes the word two words, and typing
/// a space ends the word.
#[test]
fn typing_in_the_middle_of_a_word_makes_two_words() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 2 }))
        .expect("the caret is put");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": "X" }))
        .expect("the text is typed");
    assert_eq!(words(&session, paragraph), vec!["onXe".to_string(), "two".to_string()]);
    assert_eq!(character(&session, paragraph), 3);

    // And a space ends the word: the text is "one X two" and the words
    // are three of them.
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": " " }))
        .expect("the text is typed");
    assert_eq!(words(&session, paragraph), vec!["one".to_string(), "two".to_string()]);
    assert_eq!(character(&session, paragraph), 4, "the caret is after the space");
}

/// The backspace takes the character before the caret away.
///
/// A character in a word makes the word shorter, and the **space**
/// between two words makes them one word: the text is the words with a
/// space between them, and taking the space away takes the words'
/// boundary away with it.
#[test]
fn deleting_takes_the_character_back() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 2 }))
        .expect("the caret is put");
    session
        .execute("text.caret.delete", &json!({ "paragraph": paragraph }))
        .expect("the character is deleted");
    assert_eq!(words(&session, paragraph), vec!["oe".to_string(), "two".to_string()]);
    assert_eq!(character(&session, paragraph), 1, "the caret stands where the character was");

    // And the space: taking it away makes the two words one word. The
    // text is "oe two" now, and the caret at 3 stands before the "t".
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    session
        .execute("text.caret.delete", &json!({ "paragraph": paragraph }))
        .expect("the character is deleted");
    assert_eq!(words(&session, paragraph), vec!["oetwo".to_string()]);
    assert_eq!(character(&session, paragraph), 2);
}

/// There is nothing before the caret at the start of the text, and
/// there is nothing to delete.
#[test]
fn there_is_nothing_before_the_caret_to_delete() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 0 }))
        .expect("the caret is put");
    let error = session
        .execute("text.caret.delete", &json!({ "paragraph": paragraph }))
        .expect_err("there is nothing before the caret");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
    assert_eq!(
        words(&session, paragraph),
        vec!["one".to_string(), "two".to_string()],
        "and the text is whole"
    );
}

/// The caret's **place** is the place of the character it stands
/// before: two millimetres for every character, and the space is a
/// character too.
#[test]
fn the_caret_stands_where_its_character_stands() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 0 }))
        .expect("the caret is put");
    assert!((place(&session, paragraph) - 0.0).abs() < 1e-9, "the first character is at the margin");
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 2 }))
        .expect("the caret is put");
    assert!((place(&session, paragraph) - 4.0).abs() < 1e-9, "two characters in is four millimetres");
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    assert!((place(&session, paragraph) - 6.0).abs() < 1e-9, "and the space is a character as well");
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 7 }))
        .expect("the caret is put");
    assert!((place(&session, paragraph) - 14.0).abs() < 1e-9, "the end of the text is fourteen millimetres");
}

/// The caret is **drawn**: it is a line of the text's own drawing.
#[test]
fn the_caret_is_drawn() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    let without = canvas.drawn.len();
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 3 }))
        .expect("the caret is put");
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert_eq!(canvas.drawn.len(), without + 1, "the caret is one more line of the drawing");
}

/// The paragraph **says** where its caret is, and the caret says which
/// character it stands before.
#[test]
fn the_paragraph_says_where_its_caret_is() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 5 }))
        .expect("the caret is put");
    let kind = session.kinds().get("text.paragraph").expect("the kind is there");
    let node = session.document().node(paragraph).expect("the paragraph is there");
    let value = kind.evaluate.expect("the kind evaluates")(node, session.document());
    let value = value.expect("the paragraph says what it means");
    assert_eq!(value["caret"], json!(5));

    let (id, _) = the_caret(&session, paragraph).expect("there is a caret");
    let kind = session.kinds().get("text.caret").expect("the kind is there");
    let node = session.document().node(id).expect("the caret is there");
    let value = kind.evaluate.expect("the kind evaluates")(node, session.document());
    let value: Value = value.expect("the caret says what it means");
    assert_eq!(value["character"], json!(5));
}

/// The undo takes the typing back, and the caret with it: the caret is
/// in the document, and the document is what the history keeps.
#[test]
fn undoing_typing_takes_the_text_back() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 0 }))
        .expect("the caret is put");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": "ab" }))
        .expect("the text is typed");
    assert_eq!(words(&session, paragraph), vec!["abone".to_string(), "two".to_string()]);
    session.execute("doc.undo", &json!({})).expect("the undo runs");
    assert_eq!(
        words(&session, paragraph),
        vec!["one".to_string(), "two".to_string()],
        "the text is as it was"
    );
    assert_eq!(character(&session, paragraph), 0, "and the caret is where it was");
}
