//! The **revisions** of a block.
//!
//! A run may be marked as a revision: it was inserted, it was deleted,
//! or its format was changed — and by whom.

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

/// A **paragraph** with **runs**, and its id.
fn a_paragraph(session: &mut Session, document: u64, text: &str, runs: Json) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.runs", &json!({ "block": paragraph, "runs": runs }))
        .expect("the runs are set");
    paragraph
}

/// The **revisions** of a block, as the answer holds them.
fn revisions_of(session: &mut Session, block: u64) -> Vec<Json> {
    let outcome =
        session.execute("prose.block.revisions", &json!({ "block": block })).expect("the revisions are read");
    outcome.data["revisions"].as_array().expect("the revisions are a list").clone()
}

/// A run may be marked as an **insertion**, and the words it covers are
/// derived.
#[test]
fn a_run_may_be_an_insertion() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(
        &mut session,
        document,
        "the first words",
        json!([
            { "len": 3 },
            { "len": 6, "revision": { "kind": "insert", "author": "Ada" } },
        ]),
    );
    let revisions = revisions_of(&mut session, paragraph);
    assert_eq!(revisions.len(), 1, "there is one revision");
    assert_eq!(revisions[0]["kind"], Json::String("insert".to_string()), "it is an insertion");
    assert_eq!(revisions[0]["author"], Json::String("Ada".to_string()), "by Ada");
    assert_eq!(revisions[0]["text"], Json::String(" first".to_string()), "and it covers those words");
}

/// The tree's **kinds** are the tree's own: an insertion, a deletion
/// and a format change.
#[test]
fn the_kinds_are_the_trees_own() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(
        &mut session,
        document,
        "the first words",
        json!([
            { "len": 3, "revision": { "kind": "delete", "author": "Grace" } },
            { "len": 6, "revision": { "kind": "format", "author": "Grace" } },
        ]),
    );
    let revisions = revisions_of(&mut session, paragraph);
    assert_eq!(revisions[0]["kind"], Json::String("delete".to_string()), "the first is a deletion");
    assert_eq!(revisions[1]["kind"], Json::String("format".to_string()), "and the second a format change");
}

/// A revision may carry a **date**.
#[test]
fn a_revision_may_carry_a_date() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(
        &mut session,
        document,
        "the first words",
        json!([{ "len": 3, "revision": { "kind": "insert", "author": "Ada", "date": "2026-10-26" } }]),
    );
    let revisions = revisions_of(&mut session, paragraph);
    assert_eq!(revisions[0]["date"], Json::String("2026-10-26".to_string()), "the date is said");
}

/// A revision with **no author** is refused, and so is a revision of a
/// kind that is not one.
#[test]
fn a_revision_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let error = session
        .execute(
            "prose.block.runs",
            &json!({ "block": paragraph, "runs": [{ "len": 3, "revision": { "kind": "insert" } }] }),
        )
        .expect_err("the revision is refused");
    assert!(error.to_string().contains("a revision has an author"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.block.runs",
            &json!({
                "block": paragraph,
                "runs": [{ "len": 3, "revision": { "kind": "spiral", "author": "Ada" } }],
            }),
        )
        .expect_err("the revision is refused");
    assert!(error.to_string().contains("there is no revision"), "the refusal says why: {error}");
}

/// A block that is **not there** is refused, and so is a node that is
/// not a block.
#[test]
fn a_block_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error =
        session.execute("prose.block.revisions", &json!({ "block": 999 })).expect_err("the block is refused");
    assert!(error.to_string().contains("there is no block"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.revisions", &json!({ "block": document }))
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
}
