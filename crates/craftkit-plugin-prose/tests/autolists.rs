//! The **list** a paragraph begins.
//!
//! A paragraph that begins with `1. ` or `* ` is an item of a list,
//! and the marker is not part of its words.

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

/// A **document** with one **paragraph** that says the given words.
fn a_paragraph(session: &mut Session, text: &str) -> (u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    (document, block)
}

/// The **text** of a paragraph, as it is kept.
fn text_of(session: &Session, block: u64) -> String {
    session
        .document()
        .node(block)
        .and_then(|node| node.data.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or_default()
        .to_string()
}

/// The **number** of a paragraph, as the command reads it.
fn number_of(session: &mut Session, block: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.block.number", &json!({ "block": block })).expect("the number is read");
    outcome.data["label"].clone()
}

/// A paragraph that begins with a **numbered marker** becomes an item
/// of a numbered list, and the marker is taken away.
#[test]
fn a_numbered_marker_makes_an_item() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "1. the first thing");
    let outcome = session.execute("prose.block.list", &json!({ "block": block })).expect("the list is made");
    assert_eq!(outcome.data["kind"], json!("numbered"), "the kind is the tree's: {outcome:?}");
    assert_eq!(outcome.data["text"], json!("the first thing"), "and the marker is gone");
    assert_eq!(text_of(&session, block), "the first thing", "and the paragraph says it");
    assert_eq!(number_of(&mut session, block), json!("1."), "and the label is derived");
}

/// The **markers** are the tree's own, and each of them makes its own
/// kind of list.
#[test]
fn the_markers_are_the_trees_own() {
    let mut session = session();
    for (text, kind) in [
        ("* a bullet", "bullet"),
        ("- a bullet of a dash", "-"),
        ("> a bullet of an arrow", ">"),
        ("1) a number in brackets", "numberedParen"),
        ("a. a letter", "lowerLetterDot"),
        ("a) a letter in brackets", "lowerLetter"),
        ("A. a capital", "upperLetter"),
        ("i. a roman", "lowerRoman"),
        ("I. a capital roman", "outline"),
    ] {
        let (_document, block) = a_paragraph(&mut session, text);
        let outcome =
            session.execute("prose.block.list", &json!({ "block": block })).expect("the list is made");
        assert_eq!(outcome.data["kind"], json!(kind), "the marker of {text:?} makes a {kind}");
    }
}

/// A **bullet** joins the bullet list the document already has, and a
/// **numbered** list always starts a new one.
#[test]
fn a_bullet_joins_the_list_that_is_there() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "* one");
    let mut lists = Vec::new();
    for text in ["* one", "* two", "1. three", "1. four"] {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
            .expect("the paragraph is added");
        let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
        let outcome =
            session.execute("prose.block.list", &json!({ "block": block })).expect("the list is made");
        lists.push(outcome.data["list"].as_u64().expect("the list has an id"));
    }
    assert_eq!(lists[0], lists[1], "the two bullets are in one list");
    assert_ne!(lists[2], lists[3], "and the two numbered paragraphs are in lists of their own");
    assert_ne!(lists[0], lists[2], "and a bullet is not in the numbered list");
}

/// The **runs** are rewritten with the text: the marker is gone and
/// the runs cover the words that are left.
#[test]
fn the_runs_are_rewritten_with_the_text() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "1. the first thing");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 3 }, { "len": 15 }] }))
        .expect("the runs are set");
    session.execute("prose.block.list", &json!({ "block": block })).expect("the list is made");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "prose.blocks").expect("the blocks are in the plan");
    let blocks = item.value["blocks"].as_array().expect("the blocks are a list");
    let runs = blocks[0]["runs"].as_array().expect("the runs are a list");
    assert_eq!(runs.len(), 1, "the run that covered the marker is gone: {runs:?}");
    assert_eq!(runs[0]["len"], json!(15), "and the other run covers the words that are left");
    assert_eq!(runs[0]["text"], json!("the first thing"), "and it covers them");
}

/// A paragraph that does **not begin** with a marker is refused, and
/// so is one that is **already in a list**.
#[test]
fn a_paragraph_that_begins_with_nothing_is_refused() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the first thing");
    let error = session
        .execute("prose.block.list", &json!({ "block": block }))
        .expect_err("the paragraph is refused");
    assert!(error.to_string().contains("does not begin with a marker"), "the refusal says why: {error}");
    session
        .execute("prose.block.paragraph.text", &json!({ "block": block, "text": "1. the first thing" }))
        .expect("the text is rewritten");
    session.execute("prose.block.list", &json!({ "block": block })).expect("the list is made");
    let error = session
        .execute("prose.block.list", &json!({ "block": block }))
        .expect_err("the paragraph is already in a list");
    assert!(error.to_string().contains("already in a list"), "the refusal says why: {error}");
}

/// A node that is **not a paragraph** may not become an item.
#[test]
fn a_node_that_is_not_a_paragraph_is_refused() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    let error = session
        .execute("prose.block.list", &json!({ "block": document }))
        .expect_err("the document is not a paragraph");
    assert!(error.to_string().contains("not a paragraph"), "the refusal says why: {error}");
}
