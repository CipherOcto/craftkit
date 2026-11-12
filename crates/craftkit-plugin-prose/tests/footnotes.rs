//! The **footnotes** of a document.
//!
//! A footnote is a note that belongs to the foot of the page, and the
//! number of the footnote is derived.

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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A **footnote** in a document, and its id.
fn a_footnote(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.footnote.add", &json!({ "document": document, "text": text }))
        .expect("the footnote is added");
    outcome.data["id"].as_u64().expect("the footnote has an id")
}

/// A **paragraph** that points at a footnote, and its id.
fn a_referencing_paragraph(session: &mut Session, document: u64, footnote: u64) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute(
            "prose.block.runs",
            &json!({ "block": paragraph, "runs": [{ "len": 3, "footnote": footnote }] }),
        )
        .expect("the runs are set");
    paragraph
}

/// The **footnotes** of a block, as the answer holds them.
fn footnotes_of(session: &mut Session, block: u64) -> Vec<Json> {
    let outcome =
        session.execute("prose.block.footnotes", &json!({ "block": block })).expect("the footnotes are read");
    outcome.data["footnotes"].as_array().expect("the footnotes are a list").clone()
}

/// A paragraph may point at a **footnote**, and what it says is said.
#[test]
fn a_paragraph_may_point_at_a_footnote() {
    let mut session = session();
    let document = a_document(&mut session);
    let footnote = a_footnote(&mut session, document, "the note");
    let paragraph = a_referencing_paragraph(&mut session, document, footnote);
    let footnotes = footnotes_of(&mut session, paragraph);
    assert_eq!(footnotes.len(), 1, "there is one footnote");
    assert_eq!(footnotes[0]["footnote"], json!(footnote), "the footnote is said");
    assert_eq!(footnotes[0]["text"], Json::String("the note".to_string()), "and what it says");
    assert_eq!(footnotes[0]["reference"], Json::String("the".to_string()), "and the words that point at it");
}

/// The **number** of a footnote is derived: it is where it is
/// referenced.
#[test]
fn the_number_of_a_footnote_is_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_footnote(&mut session, document, "the first note");
    let second = a_footnote(&mut session, document, "the second note");
    let paragraph = a_referencing_paragraph(&mut session, document, first);
    let other = a_referencing_paragraph(&mut session, document, second);
    let footnotes = footnotes_of(&mut session, paragraph);
    assert_eq!(footnotes[0]["number"], json!(1), "the first footnote is the first");
    let footnotes = footnotes_of(&mut session, other);
    assert_eq!(footnotes[0]["number"], json!(2), "and the second is the second");
}

/// A footnote that is **not referenced** has no number.
#[test]
fn a_footnote_that_is_not_referenced_has_no_number() {
    let mut session = session();
    let document = a_document(&mut session);
    let _footnote = a_footnote(&mut session, document, "the note");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    assert!(footnotes_of(&mut session, paragraph).is_empty(), "there is no footnote");
}

/// A reference to a footnote that is **not there** is refused, and so
/// is a reference to a node that is not a footnote.
#[test]
fn a_reference_to_a_footnote_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let error = session
        .execute("prose.block.runs", &json!({ "block": paragraph, "runs": [{ "len": 3, "footnote": 999 }] }))
        .expect_err("the reference is refused");
    assert!(error.to_string().contains("there is no footnote"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.block.runs",
            &json!({ "block": paragraph, "runs": [{ "len": 3, "footnote": document }] }),
        )
        .expect_err("the reference is refused");
    assert!(error.to_string().contains("not a footnote"), "the refusal says why: {error}");
}

/// A block that is **not there** is refused, and so is a node that is
/// not a block.
#[test]
fn a_block_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error =
        session.execute("prose.block.footnotes", &json!({ "block": 999 })).expect_err("the block is refused");
    assert!(error.to_string().contains("there is no block"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.footnotes", &json!({ "block": document }))
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
}
