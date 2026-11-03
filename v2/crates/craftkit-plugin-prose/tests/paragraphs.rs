//! The **properties** of a paragraph.
//!
//! How a paragraph stands, how far it is indented and how much space
//! is around it.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A **paragraph** in a document, and its id.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// The **facts** of a paragraph.
fn of(session: &Session, paragraph: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(paragraph).expect("the paragraph is there");
    node.data
        .as_object()
        .expect("the paragraph has a payload")
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// The value of one fact of a paragraph.
fn fact(session: &Session, paragraph: u64, key: &str) -> Option<craftkit_sdk::serde_json::Value> {
    of(session, paragraph).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// A paragraph may be told how it **stands**.
#[test]
fn a_paragraph_may_be_told_how_it_stands() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.align", &json!({ "block": paragraph, "align": "center" }))
        .expect("the alignment is set");
    assert_eq!(fact(&session, paragraph, "align"), Some(json!("center")), "the paragraph says so");
}

/// The tree's **logical** edges are the tree's own: `start` is `left`
/// and `end` is `right`.
#[test]
fn the_logical_edges_are_the_own_ones() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.align", &json!({ "block": paragraph, "align": "start" }))
        .expect("the alignment is set");
    assert_eq!(fact(&session, paragraph, "align"), Some(json!("left")), "start is left");
    session
        .execute("prose.block.paragraph.align", &json!({ "block": paragraph, "align": "end" }))
        .expect("the alignment is set");
    assert_eq!(fact(&session, paragraph, "align"), Some(json!("right")), "and end is right");
}

/// An alignment that is **not one of the tree's own** is refused, and
/// the refusal says what they are.
#[test]
fn an_alignment_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute("prose.block.paragraph.align", &json!({ "block": paragraph, "align": "middle" }))
        .expect_err("the alignment is refused");
    assert!(error.to_string().contains("there is no alignment `middle`"), "the error says so: {error}");
    assert!(error.to_string().contains("justify"), "and it says what they are");
}

/// A paragraph may be **indented**, and only what is named is changed.
#[test]
fn a_paragraph_may_be_indented() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.indent", &json!({ "block": paragraph, "left": 36 }))
        .expect("the indent is set");
    assert_eq!(fact(&session, paragraph, "indent_left"), Some(json!(36.0)), "the left indent is there");
    assert_eq!(fact(&session, paragraph, "indent_right"), None, "and the right is not");
    session
        .execute(
            "prose.block.paragraph.indent",
            &json!({ "block": paragraph, "right": 18, "first_line": -36 }),
        )
        .expect("the indent is set");
    assert_eq!(fact(&session, paragraph, "indent_right"), Some(json!(18.0)), "the right indent is there");
    assert_eq!(fact(&session, paragraph, "indent_first"), Some(json!(-36.0)), "and the first line hangs");
    assert_eq!(fact(&session, paragraph, "indent_left"), Some(json!(36.0)), "and the left is as it was");
}

/// An indent outside the tree's own range is **refused**: the tree
/// clamps it and says nothing.
#[test]
fn an_indent_out_of_range_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute("prose.block.paragraph.indent", &json!({ "block": paragraph, "left": 5000 }))
        .expect_err("the indent is refused");
    assert!(error.to_string().contains("an indent is between"), "the error says so: {error}");
    assert_eq!(fact(&session, paragraph, "indent_left"), None, "and nothing was changed");
}

/// A paragraph may have **space** around it.
#[test]
fn a_paragraph_may_have_space_around_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.spacing", &json!({ "block": paragraph, "before": 12 }))
        .expect("the space is set");
    assert_eq!(fact(&session, paragraph, "space_before"), Some(json!(12.0)), "the space before is there");
    assert_eq!(fact(&session, paragraph, "space_after"), None, "and the space after is not");
}

/// A space outside the tree's own range is refused, and a negative
/// space is refused too: the tree's space is never negative.
#[test]
fn a_space_out_of_range_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute("prose.block.paragraph.spacing", &json!({ "block": paragraph, "after": -5 }))
        .expect_err("the space is refused");
    assert!(error.to_string().contains("a space is between 0 and"), "the error says so: {error}");
    assert_eq!(fact(&session, paragraph, "space_after"), None, "and nothing was changed");
}

/// A **table** is not a paragraph, and it may not be given these.
#[test]
fn a_table_is_not_a_paragraph() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [[1, 2]] }))
        .expect("the table is added");
    let table = outcome.data["id"].as_u64().expect("the table has an id");
    let error = session
        .execute("prose.block.paragraph.align", &json!({ "block": table, "align": "center" }))
        .expect_err("the alignment is refused");
    assert!(error.to_string().contains("not a paragraph"), "the error says so: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("prose.block.paragraph.align", &json!({ "block": 999, "align": "center" }))
        .expect_err("the alignment is refused");
    assert!(error.to_string().contains("there is no node"), "the error says so: {error}");
}

/// The properties are **saved**: they are in the document after it is
/// done.
#[test]
fn the_properties_are_saved() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.align", &json!({ "block": paragraph, "align": "justify" }))
        .expect("the alignment is set");
    session
        .execute("prose.block.paragraph.indent", &json!({ "block": paragraph, "left": 36 }))
        .expect("the indent is set");
    session
        .execute("prose.block.paragraph.spacing", &json!({ "block": paragraph, "after": 12 }))
        .expect("the space is set");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(paragraph).expect("the paragraph is in the document");
    assert_eq!(node.data["align"].as_str(), Some("justify"), "the alignment is there");
    assert_eq!(node.data["indent_left"].as_f64(), Some(36.0), "and the indent");
    assert_eq!(node.data["space_after"].as_f64(), Some(12.0), "and the space");
}
