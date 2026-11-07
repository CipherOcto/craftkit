//! The **full stop** of a paragraph.
//!
//! Two spaces at the end of a paragraph after a word become a full
//! stop and a space, and the runs do not move.

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

/// Two spaces at the end of a paragraph become a **full stop** and a
/// space.
#[test]
fn two_spaces_become_a_full_stop() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said it  ");
    let outcome =
        session.execute("prose.block.stops", &json!({ "block": block })).expect("the full stop is made");
    assert_eq!(outcome.data["text"], json!("he said it. "), "the full stop is there: {outcome:?}");
    assert_eq!(text_of(&session, block), "he said it. ", "and the paragraph says it");
}

/// The word before the two spaces may end with a **letter or a
/// digit**, or with a **closing mark**.
#[test]
fn the_word_before_may_end_with_a_closing_mark() {
    let mut session = session();
    for text in ["he said it  ", "he said it 2  ", "he said (it)  ", "he said “it”  "] {
        let (_document, block) = a_paragraph(&mut session, text);
        session.execute("prose.block.stops", &json!({ "block": block })).expect("the full stop is made");
        let characters: Vec<char> = text.chars().collect();
        let expected: String = characters[..characters.len() - 2].iter().collect::<String>() + ". ";
        assert_eq!(text_of(&session, block), expected, "the full stop is made in {text:?}");
    }
}

/// Two spaces that are **not** after a word are refused: at the
/// start of a paragraph, after other punctuation, and after a third
/// space.
#[test]
fn two_spaces_that_are_not_after_a_word_are_refused() {
    let mut session = session();
    for text in ["  ", "he said it.  ", "he said it   "] {
        let (_document, block) = a_paragraph(&mut session, text);
        let error = session
            .execute("prose.block.stops", &json!({ "block": block }))
            .expect_err("the two spaces are refused");
        assert!(error.to_string().contains("two spaces"), "the refusal says why: {error}");
        assert_eq!(text_of(&session, block), text, "and the paragraph is as it was");
    }
}

/// The **runs** do not move: a space and a full stop are one
/// character each.
#[test]
fn the_runs_do_not_move() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said it  ");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 10 }, { "len": 2 }] }))
        .expect("the runs are set");
    session.execute("prose.block.stops", &json!({ "block": block })).expect("the full stop is made");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "prose.blocks").expect("the blocks are in the plan");
    let blocks = item.value["blocks"].as_array().expect("the blocks are a list");
    let runs = blocks[0]["runs"].as_array().expect("the runs are a list");
    assert_eq!(runs[0]["len"], json!(10), "the first run still covers its ten words");
    assert_eq!(runs[1]["len"], json!(2), "and the second its two");
    assert_eq!(runs[1]["text"], json!(". "), "and the spaces are the full stop and a space");
}

/// A paragraph that holds a **revision** is not touched, and a node
/// that is **not a paragraph** is refused.
#[test]
fn a_revision_is_not_touched_and_a_document_is_refused() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "he said it  ");
    session
        .execute(
            "prose.block.runs",
            &json!({
                "block": block,
                "runs": [{
                    "len": 12,
                    "revision": { "kind": "insert", "author": "Ada", "date": "2026-01-01" },
                }],
            }),
        )
        .expect("the run is set");
    let error = session
        .execute("prose.block.stops", &json!({ "block": block }))
        .expect_err("the revision is not touched");
    assert!(error.to_string().contains("revision"), "the refusal says why: {error}");
    assert_eq!(text_of(&session, block), "he said it  ", "and the paragraph is as it was");
    let error = session
        .execute("prose.block.stops", &json!({ "block": document }))
        .expect_err("the document is not a paragraph");
    assert!(error.to_string().contains("not a paragraph"), "the refusal says why: {error}");
}
