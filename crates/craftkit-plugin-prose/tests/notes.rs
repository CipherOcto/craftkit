//! The **notes** of a document are converted.
//!
//! A footnote becomes an endnote, an endnote becomes a footnote, and
//! the two swap.

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

/// A **document** with a **paragraph** that points at a note of that
/// kind, and the note itself.
fn a_document(session: &mut Session, kind: &str) -> (u64, u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a word" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let outcome = session
        .execute(
            if kind == "footnote" { "prose.footnote.add" } else { "prose.endnote.add" },
            &json!({ "document": document, "text": "the note" }),
        )
        .expect("the note is added");
    let note = outcome.data["id"].as_u64().expect("the note has an id");
    // The **run** that points at the note.
    let key = if kind == "footnote" { "footnote" } else { "endnote" };
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 5, key: note }] }))
        .expect("the runs are set");
    (document, block, note)
}

/// The **kind** of a node, as it is kept.
fn kind_of(session: &Session, id: u64) -> String {
    session.document().node(id).map(|node| node.kind.clone()).unwrap_or_default()
}

/// The **references** a block holds: the keys of its runs.
fn references_of(session: &Session, block: u64) -> Vec<String> {
    session
        .document()
        .node(block)
        .and_then(|node| node.data.get("runs"))
        .and_then(|runs| runs.as_array())
        .map(|runs| {
            runs.iter()
                .flat_map(|run| {
                    ["footnote", "endnote"]
                        .iter()
                        .filter(|key| run.get(*key).is_some())
                        .map(|key| key.to_string())
                        .collect::<Vec<String>>()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **footnotes** become endnotes.
#[test]
fn the_footnotes_become_endnotes() {
    let mut session = session();
    let (document, block, note) = a_document(&mut session, "footnote");
    let outcome = session
        .execute("prose.notes.convert", &json!({ "document": document, "to": "endnote" }))
        .expect("the notes are converted");
    assert_eq!(outcome.data["notes"], json!(1), "the note is converted: {outcome:?}");
    assert_eq!(kind_of(&session, note), "prose.endnote", "and it is an endnote now");
    assert_eq!(references_of(&session, block), vec!["endnote"], "and the reference says so too");
}

/// The **endnotes** become footnotes.
#[test]
fn the_endnotes_become_footnotes() {
    let mut session = session();
    let (document, block, note) = a_document(&mut session, "endnote");
    session
        .execute("prose.notes.convert", &json!({ "document": document, "to": "footnote" }))
        .expect("the notes are converted");
    assert_eq!(kind_of(&session, note), "prose.footnote", "it is a footnote now");
    assert_eq!(references_of(&session, block), vec!["footnote"], "and the reference says so too");
}

/// The two **swap**: a footnote becomes an endnote and an endnote
/// becomes a footnote.
#[test]
fn the_two_swap() {
    let mut session = session();
    let (document, _block, note) = a_document(&mut session, "footnote");
    let outcome = session
        .execute("prose.notes.convert", &json!({ "document": document, "to": "swap" }))
        .expect("the notes are converted");
    assert_eq!(outcome.data["notes"], json!(1), "the one note is converted: {outcome:?}");
    assert_eq!(kind_of(&session, note), "prose.endnote", "and it is an endnote now");
}

/// A way that is **not one of the tree's three** is refused.
#[test]
fn a_way_that_is_not_one_of_the_trees_three_is_refused() {
    let mut session = session();
    let (document, _block, _note) = a_document(&mut session, "footnote");
    let error = session
        .execute("prose.notes.convert", &json!({ "document": document, "to": "sideways" }))
        .expect_err("there is no such way");
    assert!(error.to_string().contains("sideways"), "the refusal says why: {error}");
}
