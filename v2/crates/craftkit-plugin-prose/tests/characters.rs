//! The **character** of a run: the way a piece of text is written.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **paragraph** with a text and its runs.
fn a_paragraph(session: &mut Session, text: &str, runs: Json) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session.execute("prose.block.runs", &json!({ "block": block, "runs": runs })).expect("the runs are set");
    block
}

/// The **runs** of a block.
fn runs_of(session: &mut Session, block: u64) -> Vec<Json> {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("runs").and_then(|runs| runs.as_array()).cloned().expect("the runs are a list")
}

/// The **text** of a block.
fn text_of(session: &mut Session, block: u64) -> String {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// **The character is put on the range and on no other.**
#[test]
fn the_character_is_put_on_the_range() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    session
        .execute("prose.run.format", &json!({ "block": block, "start": 4, "length": 5, "bold": true }))
        .expect("the character is set");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 3, "the run is split at both ends of the range");
    assert_eq!(runs[0]["len"], 4);
    assert!(runs[0]["bold"].is_null(), "the run that was not touched is not bold");
    assert_eq!(runs[1]["len"], 5);
    assert_eq!(runs[1]["bold"], json!(true));
    assert_eq!(runs[2]["len"], 10);
    assert!(runs[2]["bold"].is_null(), "the run that was not touched is not bold");
}

/// **The text is not changed by the character.**
#[test]
fn the_text_is_not_changed() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    session
        .execute("prose.run.format", &json!({ "block": block, "start": 0, "length": 3, "italic": true }))
        .expect("the character is set");
    assert_eq!(text_of(&mut session, block), "the quick brown fox");
}

/// **Every character the tree has is set: bold, italic, underline, the
/// colour and the size.**
#[test]
fn every_character_is_set() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello", json!([{ "len": 5 }]));
    session
        .execute(
            "prose.run.format",
            &json!({
                "block": block,
                "start": 0,
                "length": 5,
                "bold": true,
                "italic": true,
                "underline": true,
                "colour": "#ff0000",
                "size": 14.0,
            }),
        )
        .expect("the character is set");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["bold"], json!(true));
    assert_eq!(runs[0]["italic"], json!(true));
    assert_eq!(runs[0]["underline"], json!(true));
    assert_eq!(runs[0]["colour"], json!("#ff0000"));
    assert_eq!(runs[0]["size"], json!(14.0));
}

/// **A range that reaches past the end of the text is refused.**
#[test]
fn a_range_past_the_end_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello", json!([{ "len": 5 }]));
    let failure = session
        .execute("prose.run.format", &json!({ "block": block, "start": 3, "length": 9, "bold": true }))
        .expect_err("the range is refused");
    let message = failure.to_string();
    assert!(message.contains("the text has 5 characters"), "the refusal says why: {message}");
}

/// **A command that changes nothing is refused.**
#[test]
fn a_command_that_changes_nothing_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello", json!([{ "len": 5 }]));
    let failure = session
        .execute("prose.run.format", &json!({ "block": block, "start": 0, "length": 2 }))
        .expect_err("the command is refused");
    let message = failure.to_string();
    assert!(message.contains("says nothing to change"), "the refusal says why: {message}");
}

/// **A colour that is not a colour is refused.**
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello", json!([{ "len": 5 }]));
    let failure = session
        .execute("prose.run.format", &json!({ "block": block, "start": 0, "length": 2, "colour": "red" }))
        .expect_err("the colour is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a colour"), "the refusal says why: {message}");
}

/// **A size that is not a size is refused.**
#[test]
fn a_size_that_is_not_a_size_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello", json!([{ "len": 5 }]));
    let failure = session
        .execute("prose.run.format", &json!({ "block": block, "start": 0, "length": 2, "size": 0.0 }))
        .expect_err("the size is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a size"), "the refusal says why: {message}");
}

/// **A tracked change is not formatted.**
#[test]
fn a_tracked_change_is_not_formatted() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        "hello world",
        json!([{ "len": 5, "revision": { "kind": "insert", "author": "ann" } }, { "len": 6 }]),
    );
    let failure = session
        .execute("prose.run.format", &json!({ "block": block, "start": 1, "length": 2, "bold": true }))
        .expect_err("the tracked change is refused");
    let message = failure.to_string();
    assert!(message.contains("a tracked change is not formatted"), "the refusal says why: {message}");
}

/// **The character of the text is read back with the blocks.**
#[test]
fn the_character_is_read_back() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello", json!([{ "len": 5 }]));
    session
        .execute("prose.run.format", &json!({ "block": block, "start": 0, "length": 5, "underline": true }))
        .expect("the character is set");
    let document = session
        .document()
        .walk()
        .find(|node| node.kind == "prose.document")
        .expect("the document is in the document")
        .id;
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == document && item.kind == "prose.blocks")
        .expect("the blocks are in the plan");
    let blocks = item.value["blocks"].as_array().expect("the blocks are a list");
    let runs = blocks[0]["runs"].as_array().expect("the runs are a list");
    assert_eq!(runs[0]["underline"], json!(true));
    assert_eq!(runs[0]["text"], json!("hello"));
}
