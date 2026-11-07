//! The **quotes** of a paragraph.
//!
//! The straight quotes of a paragraph become the smart ones, and the
//! runs do not move.

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

/// A straight quote becomes a **smart** one: the character before it
/// decides which.
#[test]
fn a_straight_quote_becomes_a_smart_one() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said \"hello\" to her");
    let outcome =
        session.execute("prose.block.quotes", &json!({ "block": block })).expect("the quotes are made smart");
    assert_eq!(outcome.data["text"], json!("he said “hello” to her"), "the quotes are smart: {outcome:?}");
    assert_eq!(outcome.data["changed"], json!(2), "and both of them changed");
    assert_eq!(text_of(&session, block), "he said “hello” to her", "and the paragraph says it");
}

/// A quote at the **start** of a paragraph is an opening one, and a
/// quote after a **space** is an opening one too.
#[test]
fn a_quote_after_a_space_is_an_opening_one() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "\"Hello,\" she said. 'Yes,' he said.");
    session.execute("prose.block.quotes", &json!({ "block": block })).expect("the quotes are made smart");
    assert_eq!(text_of(&session, block), "“Hello,” she said. ‘Yes,’ he said.", "the quotes are smart");
}

/// The tree's own two marks: a `"` after a `‘` opens, and a `'` after
/// a `“` opens.
#[test]
fn the_trees_own_marks_are_here() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "‘Hello’ and “Hello”");
    session.execute("prose.block.quotes", &json!({ "block": block })).expect("the quotes are made smart");
    assert_eq!(text_of(&session, block), "‘Hello’ and “Hello”", "the quotes are already smart");
}

/// The **runs** do not move: a straight quote and a smart one are one
/// character each.
#[test]
fn the_runs_do_not_move() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said \"hello\"");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 8 }, { "len": 7 }] }))
        .expect("the runs are set");
    session.execute("prose.block.quotes", &json!({ "block": block })).expect("the quotes are made smart");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "prose.blocks").expect("the blocks are in the plan");
    let blocks = item.value["blocks"].as_array().expect("the blocks are a list");
    let runs = blocks[0]["runs"].as_array().expect("the runs are a list");
    assert_eq!(runs[0]["len"], json!(8), "the first run still covers its eight words");
    assert_eq!(runs[0]["text"], json!("he said "), "and it covers the same words");
    assert_eq!(runs[1]["text"], json!("\u{201c}hello\u{201d}"), "and so does the second");
}

/// A paragraph that holds a **revision** is not touched.
#[test]
fn a_paragraph_with_a_revision_is_not_touched() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said \"hello\"");
    session
        .execute(
            "prose.block.runs",
            &json!({
                "block": block,
                "runs": [{
                    "len": 15,
                    "revision": { "kind": "insert", "author": "Ada", "date": "2026-01-01" },
                }],
            }),
        )
        .expect("the run is set");
    let error = session
        .execute("prose.block.quotes", &json!({ "block": block }))
        .expect_err("the revision is not touched");
    assert!(error.to_string().contains("revision"), "the refusal says why: {error}");
    assert_eq!(text_of(&session, block), "he said \"hello\"", "and the paragraph is as it was");
}

/// A node that is **not a paragraph** may not have its quotes made
/// smart.
#[test]
fn a_node_that_is_not_a_paragraph_is_refused() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    let error = session
        .execute("prose.block.quotes", &json!({ "block": document }))
        .expect_err("the document is not a paragraph");
    assert!(error.to_string().contains("not a paragraph"), "the refusal says why: {error}");
}
