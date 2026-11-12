//! The **endnotes** of a document.
//!
//! A note at the end of the document rather than at the foot of the
//! page, and a number that is counted apart from the footnotes'.

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

/// An **endnote** in a document.
fn an_endnote(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.endnote.add", &json!({ "document": document, "text": text }))
        .expect("the endnote is added");
    outcome.data["id"].as_u64().expect("the endnote has an id")
}

/// A **footnote** in a document.
fn a_footnote(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.footnote.add", &json!({ "document": document, "text": text }))
        .expect("the footnote is added");
    outcome.data["id"].as_u64().expect("the footnote has an id")
}

/// The **endnotes** of a block, as the command reads them.
fn endnotes_of(session: &mut Session, block: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.block.endnotes", &json!({ "block": block })).expect("the endnotes are read");
    outcome.data["endnotes"].clone()
}

/// The **footnotes** of a block, as the command reads them.
fn footnotes_of(session: &mut Session, block: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.block.footnotes", &json!({ "block": block })).expect("the footnotes are read");
    outcome.data["footnotes"].clone()
}

/// An endnote is a **note** that belongs to the end of the document,
/// and a run may point at it.
#[test]
fn a_run_may_point_at_an_endnote() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "he said it");
    let endnote = an_endnote(&mut session, document, "and meant it");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 10, "endnote": endnote }] }))
        .expect("the run is set");
    let endnotes = endnotes_of(&mut session, block);
    assert_eq!(endnotes.as_array().map(|endnotes| endnotes.len()), Some(1), "the block has an endnote");
    assert_eq!(endnotes[0]["number"], json!(1), "and it is numbered one");
    assert_eq!(endnotes[0]["text"], json!("and meant it"), "and it says what it says");
    assert_eq!(endnotes[0]["reference"], json!("he said it"), "and the run covers the words");
}

/// The endnotes are **counted apart** from the footnotes: an endnote
/// that is the first endnote is one, even when footnotes came before
/// it.
#[test]
fn the_endnotes_are_counted_apart_from_the_footnotes() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "he said it");
    let footnote = a_footnote(&mut session, document, "a note at the foot");
    let endnote = an_endnote(&mut session, document, "a note at the end");
    session
        .execute(
            "prose.block.runs",
            &json!({
                "block": block,
                "runs": [
                    { "len": 4, "footnote": footnote },
                    { "len": 6, "endnote": endnote },
                ],
            }),
        )
        .expect("the runs are set");
    let footnotes = footnotes_of(&mut session, block);
    assert_eq!(footnotes[0]["number"], json!(1), "the footnote is one");
    let endnotes = endnotes_of(&mut session, block);
    assert_eq!(endnotes[0]["number"], json!(1), "and the endnote is one as well");
}

/// A run that names a node that is **not an endnote** is refused.
#[test]
fn a_run_that_names_no_endnote_is_refused() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "he said it");
    let footnote = a_footnote(&mut session, document, "a note at the foot");
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 10, "endnote": footnote }] }))
        .expect_err("the footnote is not an endnote");
    assert!(error.to_string().contains("not an endnote"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 10, "endnote": 9999 }] }))
        .expect_err("there is no endnote");
    assert!(error.to_string().contains("no endnote"), "the refusal says why: {error}");
}

/// An endnote is a **node** of its own kind, and it is read in the
/// plan like every other node.
#[test]
fn an_endnote_is_a_node_of_its_own_kind() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "he said it");
    let endnote = an_endnote(&mut session, document, "and meant it");
    let node = session.document().node(endnote).expect("the endnote is in the document");
    assert_eq!(node.kind, json!("prose.endnote"), "the endnote is an endnote");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == endnote).expect("the endnote is in the plan");
    assert_eq!(item.value["text"], json!("and meant it"), "and it says what it says");
}
