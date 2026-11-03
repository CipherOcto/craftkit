//! The **editing** of a document.
//!
//! What a paragraph says may be changed, a block may be taken away, and
//! a block may be moved to another place in the document.

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

/// The **blocks** of a document, in order, and what each of them says.
fn blocks(session: &Session, document: u64) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    let node = session.document().node(document).expect("the document is there");
    for child in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        let text = child.data["text"].as_str().unwrap_or("").to_string();
        out.push((child.id, text));
    }
    out
}

/// What a paragraph **says** may be changed.
#[test]
fn what_a_paragraph_says_may_be_changed() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.block.paragraph.text", &json!({ "block": paragraph, "text": "other words" }))
        .expect("the text is set");
    assert_eq!(blocks(&session, document)[0].1, "other words", "the paragraph says something else now");
}

/// A paragraph may be **emptied**: an empty paragraph is a paragraph.
#[test]
fn a_paragraph_may_be_emptied() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.block.paragraph.text", &json!({ "block": paragraph, "text": "" }))
        .expect("the text is set");
    assert_eq!(blocks(&session, document)[0].1, "", "the paragraph says nothing now");
}

/// The **runs** stay when the text changes: a run is an interval over
/// the text, and what it covers is derived.
#[test]
fn the_runs_stay_when_the_text_changes() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.block.runs", &json!({ "block": paragraph, "runs": [{ "len": 3, "bold": true }] }))
        .expect("the runs are set");
    session
        .execute("prose.block.paragraph.text", &json!({ "block": paragraph, "text": "ab" }))
        .expect("the text is set");
    let node = session.document().node(paragraph).expect("the paragraph is there");
    let runs = node.data["runs"].as_array().expect("the runs are a list");
    assert_eq!(runs.len(), 1, "the run is still there");
    assert_eq!(runs[0]["len"], 3, "and it still says how many characters it covers");
}

/// A **table** is not a paragraph, and its text may not be set.
#[test]
fn a_table_is_not_a_paragraph() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [[1, 2]] }))
        .expect("the table is added");
    let table = outcome.data["id"].as_u64().expect("the table has an id");
    let error = session
        .execute("prose.block.paragraph.text", &json!({ "block": table, "text": "hello" }))
        .expect_err("the text is refused");
    assert!(error.to_string().contains("not a paragraph"), "the error says so: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("prose.block.paragraph.text", &json!({ "block": 999, "text": "hello" }))
        .expect_err("the text is refused");
    assert!(error.to_string().contains("there is no node"), "the error says so: {error}");
}

/// A block may be **taken away**, and the others keep their order.
#[test]
fn a_block_may_be_taken_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let second = a_paragraph(&mut session, document, "two");
    let third = a_paragraph(&mut session, document, "three");
    session.execute("prose.block.remove", &json!({ "block": second })).expect("the block is taken away");
    assert!(session.document().node(second).is_none(), "the block is gone");
    let blocks = blocks(&session, document);
    assert_eq!(blocks.len(), 2, "and there are two blocks");
    assert_eq!(blocks[0].0, first, "the first is the first");
    assert_eq!(blocks[1].0, third, "and the third is the second");
}

/// A block's **style** goes with it: a block that was in a style is not
/// in it any more.
#[test]
fn the_style_of_a_block_goes_with_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the style" }))
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    let paragraph = a_paragraph(&mut session, document, "one");
    session
        .execute("prose.block.style", &json!({ "block": paragraph, "style": style }))
        .expect("the style is put on");
    session.execute("prose.block.remove", &json!({ "block": paragraph })).expect("the block is taken away");
    let edges = session.document().relations.len();
    assert_eq!(edges, 0, "the edge went with it");
}

/// A **document** is not a block, and it may not be taken away by this
/// command.
#[test]
fn a_document_is_not_a_block() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.block.remove", &json!({ "block": document }))
        .expect_err("the block is refused");
    assert!(error.to_string().contains("not a block"), "the error says so: {error}");
    assert!(session.document().node(document).is_some(), "and the document is there");
}

/// A block may be **moved** to another place in its document.
#[test]
fn a_block_may_be_moved() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let second = a_paragraph(&mut session, document, "two");
    let third = a_paragraph(&mut session, document, "three");
    session.execute("prose.block.move", &json!({ "block": third, "to": 1 })).expect("the block is moved");
    let blocks = blocks(&session, document);
    assert_eq!(blocks[0].0, third, "the third is the first now");
    assert_eq!(blocks[1].0, first, "the first is the second");
    assert_eq!(blocks[2].0, second, "and the second is the third");
}

/// A block may be moved **down** as well as up.
#[test]
fn a_block_may_be_moved_down() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let second = a_paragraph(&mut session, document, "two");
    let third = a_paragraph(&mut session, document, "three");
    session.execute("prose.block.move", &json!({ "block": first, "to": 3 })).expect("the block is moved");
    let blocks = blocks(&session, document);
    assert_eq!(blocks[0].0, second, "the second is the first now");
    assert_eq!(blocks[1].0, third, "the third is the second");
    assert_eq!(blocks[2].0, first, "and the first is the third");
}

/// A block that is **already** in that place is refused: a command that
/// does nothing did not do anything.
#[test]
fn a_block_that_is_already_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let second = a_paragraph(&mut session, document, "two");
    let error = session
        .execute("prose.block.move", &json!({ "block": first, "to": 1 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("already the 1"), "the error says so: {error}");
    let error = session
        .execute("prose.block.move", &json!({ "block": second, "to": 2 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("already the 2"), "the error says so: {error}");
}

/// A place that is **not there** is refused, and the refusal says how
/// many blocks there are.
#[test]
fn a_place_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let error = session
        .execute("prose.block.move", &json!({ "block": first, "to": 4 }))
        .expect_err("the move is refused");
    assert!(
        error.to_string().contains("has 1 blocks and the 4 is not one of them"),
        "the error says so: {error}"
    );
}

/// A place that is **not a whole number** is refused.
#[test]
fn a_place_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let error = session
        .execute("prose.block.move", &json!({ "block": first, "to": 1.5 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("counted from one"), "the error says so: {error}");
}

/// A **style** is not a block, and it may not be moved.
#[test]
fn a_style_is_not_a_block() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the style" }))
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    let error = session
        .execute("prose.block.move", &json!({ "block": style, "to": 1 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("not a block"), "the error says so: {error}");
}

/// The edit is **saved**: it is in the document after it is done.
#[test]
fn the_edit_is_saved() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "one");
    let second = a_paragraph(&mut session, document, "two");
    session
        .execute("prose.block.paragraph.text", &json!({ "block": first, "text": "other" }))
        .expect("the text is set");
    session.execute("prose.block.move", &json!({ "block": second, "to": 1 })).expect("the block is moved");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(document).expect("the document is in the document");
    let blocks: Vec<u64> = node
        .children
        .iter()
        .filter(|child| child.kind.starts_with("prose.block."))
        .map(|child| child.id)
        .collect();
    assert_eq!(blocks[0], second, "the second block is the first now");
    let node = saved.node(first).expect("the paragraph is in the document");
    assert_eq!(node.data["text"].as_str(), Some("other"), "and the paragraph says something else");
}
