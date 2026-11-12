//! The **selection is replaced**: what a reader picks out is what a
//! reader writes over.
//!
//! The caret says where the reader is (§151) and the selection says
//! what is picked out (§152), and they are two things. Typing and
//! deleting read both: what is picked out goes first, and what is not
//! is the character before the caret.

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
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 0 }))
        .expect("the caret is put");
    paragraph
}

/// The **words** of the paragraph, as the payload holds them.
fn words(session: &Session, paragraph: u64) -> Vec<String> {
    session
        .document()
        .node(paragraph)
        .and_then(|node| node.data.get("words"))
        .and_then(|words| words.as_array())
        .map(|words| words.iter().filter_map(|word| word.as_str().map(|word| word.to_string())).collect())
        .expect("the paragraph has words")
}

/// The character the caret stands before.
fn character(session: &Session, paragraph: u64) -> usize {
    let node = session.document().node(paragraph).expect("the paragraph is there");
    let caret = node.children.iter().find(|child| child.kind == "text.caret").expect("there is a caret");
    caret.data.get("character").and_then(|character| character.as_u64()).expect("the caret has a character")
        as usize
}

/// Typing writes over what is picked out.
#[test]
fn typing_writes_over_what_is_picked_out() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("select.span", &json!({ "node": paragraph, "from": 0, "to": 3 }))
        .expect("the characters are picked out");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": "X" }))
        .expect("the text is typed");
    assert_eq!(
        words(&session, paragraph),
        vec!["X".to_string(), "two".to_string()],
        "the picked-out word is gone"
    );
    assert_eq!(character(&session, paragraph), 1, "the caret stands after what was typed");
    assert!(session.selection().is_empty(), "and nothing is picked out any more");
}

/// Typing with nothing picked out types at the caret.
#[test]
fn typing_with_nothing_picked_out_types_at_the_caret() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("text.caret.set", &json!({ "paragraph": paragraph, "character": 4 }))
        .expect("the caret is put");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": "X" }))
        .expect("the text is typed");
    assert_eq!(words(&session, paragraph), vec!["one".to_string(), "Xtwo".to_string()]);
    assert_eq!(character(&session, paragraph), 5);
}

/// An **empty** range picks nothing out, so the typing goes at the
/// caret.
#[test]
fn an_empty_range_picks_nothing_out() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("select.span", &json!({ "node": paragraph, "from": 3, "to": 3 }))
        .expect("an empty range is picked out");
    session
        .execute("text.caret.type", &json!({ "paragraph": paragraph, "text": "X" }))
        .expect("the text is typed");
    assert_eq!(words(&session, paragraph), vec!["Xone".to_string(), "two".to_string()]);
    assert_eq!(character(&session, paragraph), 1, "the caret was at the start");
}

/// Deleting takes away what is picked out, and the caret stands where
/// the range began.
#[test]
fn deleting_takes_away_what_is_picked_out() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("select.span", &json!({ "node": paragraph, "from": 0, "to": 4 }))
        .expect("the characters are picked out");
    session
        .execute("text.caret.delete", &json!({ "paragraph": paragraph }))
        .expect("the characters are deleted");
    assert_eq!(words(&session, paragraph), vec!["two".to_string()]);
    assert_eq!(character(&session, paragraph), 0, "the caret stands where the range began");
    assert!(session.selection().is_empty(), "and nothing is picked out any more");
}

/// The selection of **another** paragraph is not this paragraph's, so
/// the typing goes at the caret.
#[test]
fn the_selection_of_another_paragraph_is_not_this_one() {
    let mut session = session();
    let first = a_paragraph(&mut session, "one two", 14.0);
    let second = a_paragraph(&mut session, "three four", 14.0);
    session
        .execute("select.span", &json!({ "node": first, "from": 0, "to": 3 }))
        .expect("the characters are picked out");
    session
        .execute("text.caret.type", &json!({ "paragraph": second, "text": "X" }))
        .expect("the text is typed");
    assert_eq!(words(&session, second), vec!["Xthree".to_string(), "four".to_string()]);
    assert_eq!(
        words(&session, first),
        vec!["one".to_string(), "two".to_string()],
        "the other paragraph is whole"
    );
}
