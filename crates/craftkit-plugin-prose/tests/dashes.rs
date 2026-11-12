//! The **dashes** of a paragraph.
//!
//! Two hyphens are a mark of the keyboard; a dash is the mark of the
//! book. The runs are rewritten with the text, because two hyphens
//! become one dash.

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

/// A pair of hyphens with **no space before it** is an em dash.
#[test]
fn a_pair_of_hyphens_inside_a_word_is_an_em_dash() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said--loudly--and meant it");
    let outcome =
        session.execute("prose.block.dashes", &json!({ "block": block })).expect("the dashes are made");
    assert_eq!(outcome.data["text"], json!("he said—loudly—and meant it"), "the dashes are em dashes");
    assert_eq!(outcome.data["dashes"], json!(2), "and there are two of them");
    assert_eq!(text_of(&session, block), "he said—loudly—and meant it", "and the paragraph says it");
}

/// A pair of hyphens with a **space on each side** is an en dash.
#[test]
fn a_pair_of_hyphens_between_words_is_an_en_dash() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said -- loudly -- and meant it");
    session.execute("prose.block.dashes", &json!({ "block": block })).expect("the dashes are made");
    assert_eq!(text_of(&session, block), "he said – loudly – and meant it", "the dashes are en dashes");
}

/// A run of **three or more** hyphens is a line and not a dash, and a
/// pair at the **end** of the text is left alone.
#[test]
fn a_line_of_hyphens_is_left_alone() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said --- and stopped---");
    session.execute("prose.block.dashes", &json!({ "block": block })).expect("the dashes are made");
    assert_eq!(text_of(&session, block), "he said --- and stopped---", "the text is as it was");
}

/// The **runs** are rewritten with the text: two hyphens become one
/// dash, and the runs are one character shorter.
#[test]
fn the_runs_are_rewritten_with_the_text() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said--loudly--and meant it");
    session
        .execute(
            "prose.block.runs",
            &json!({ "block": block, "runs": [{ "len": 8 }, { "len": 8 }, { "len": 12 }] }),
        )
        .expect("the runs are set");
    session.execute("prose.block.dashes", &json!({ "block": block })).expect("the dashes are made");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "prose.blocks").expect("the blocks are in the plan");
    let blocks = item.value["blocks"].as_array().expect("the blocks are a list");
    let runs = blocks[0]["runs"].as_array().expect("the runs are a list");
    assert_eq!(runs[0]["len"], json!(7), "the first run lost a character: {runs:?}");
    assert_eq!(runs[1]["len"], json!(7), "and so did the second");
    assert_eq!(runs[2]["len"], json!(12), "and the last one did not");
    assert_eq!(runs[0]["text"], json!("he said"), "and the first covers the same words");
    assert_eq!(runs[1]["text"], json!("—loudly"), "and the second covers its dash");
}

/// A paragraph that holds a **revision** is not touched.
#[test]
fn a_paragraph_with_a_revision_is_not_touched() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "he said--loudly");
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
        .execute("prose.block.dashes", &json!({ "block": block }))
        .expect_err("the revision is not touched");
    assert!(error.to_string().contains("revision"), "the refusal says why: {error}");
    assert_eq!(text_of(&session, block), "he said--loudly", "and the paragraph is as it was");
}

/// A node that is **not a paragraph** may not have its dashes made.
#[test]
fn a_node_that_is_not_a_paragraph_is_refused() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    let error = session
        .execute("prose.block.dashes", &json!({ "block": document }))
        .expect_err("the document is not a paragraph");
    assert!(error.to_string().contains("not a paragraph"), "the refusal says why: {error}");
}
