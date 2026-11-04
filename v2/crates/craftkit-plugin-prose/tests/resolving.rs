//! The **resolution** of a revision: accepting it, or rejecting it.

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

/// A **paragraph** whose runs carry one revision.
///
/// The runs cover the whole text, so that every word belongs to a run
/// and the words of a revision are the words of its run.
fn a_paragraph(session: &mut Session, runs: Json) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the quick brown fox" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session.execute("prose.block.runs", &json!({ "block": block, "runs": runs })).expect("the runs are set");
    block
}

/// The **text** of a block.
fn text_of(session: &mut Session, block: u64) -> String {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The **runs** of a block.
fn runs_of(session: &mut Session, block: u64) -> Vec<Json> {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default()
}

/// An accepted **insertion** is ordinary text: the words stay and the
/// mark goes.
#[test]
fn an_accepted_insertion_is_ordinary_text() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4 },
            { "len": 6, "revision": { "kind": "insert", "author": "a reader" } },
            { "len": 9 },
        ]),
    );
    session
        .execute("prose.revision.accept", &json!({ "block": block, "run": 1 }))
        .expect("the revision is accepted");
    assert_eq!(text_of(&mut session, block), "the quick brown fox", "the words stay");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 3, "there are still three runs");
    assert!(runs[1].get("revision").is_none(), "and the mark is gone");
}

/// A rejected **insertion** is taken away: the words go and the run
/// goes with them.
#[test]
fn a_rejected_insertion_is_taken_away() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4 },
            { "len": 6, "revision": { "kind": "insert", "author": "a reader" } },
            { "len": 9 },
        ]),
    );
    session
        .execute("prose.revision.reject", &json!({ "block": block, "run": 1 }))
        .expect("the revision is rejected");
    assert_eq!(text_of(&mut session, block), "the brown fox", "the words are gone");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 2, "and the run is gone with them");
    assert_eq!(runs[1]["len"], json!(9), "and the run after it covers the rest");
}

/// An accepted **deletion** is really deleted.
#[test]
fn an_accepted_deletion_is_really_deleted() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4 },
            { "len": 6, "revision": { "kind": "delete", "author": "a reader" } },
            { "len": 9 },
        ]),
    );
    session
        .execute("prose.revision.accept", &json!({ "block": block, "run": 1 }))
        .expect("the revision is accepted");
    assert_eq!(text_of(&mut session, block), "the brown fox", "the words are gone");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 2, "and the run is gone with them");
    assert_eq!(runs[1]["len"], json!(9), "and the run after it covers the rest");
}

/// A rejected **deletion** is restored: the words stay and the mark
/// goes.
#[test]
fn a_rejected_deletion_is_restored() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4 },
            { "len": 6, "revision": { "kind": "delete", "author": "a reader" } },
            { "len": 9 },
        ]),
    );
    session
        .execute("prose.revision.reject", &json!({ "block": block, "run": 1 }))
        .expect("the revision is rejected");
    assert_eq!(text_of(&mut session, block), "the quick brown fox", "the words stay");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 3, "there are still three runs");
    assert!(runs[1].get("revision").is_none(), "and the mark is gone");
}

/// A rejected **format** is refused: the format the run had before the
/// change is not kept, and so it cannot be put back.
#[test]
fn a_rejected_format_is_refused() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4 },
            { "len": 6, "revision": { "kind": "format", "author": "a reader" } },
            { "len": 9 },
        ]),
    );
    let value = session
        .execute("prose.revision.reject", &json!({ "block": block, "run": 1 }))
        .expect_err("the rejection is refused");
    assert!(value.to_string().contains("not supported yet"), "the refusal says why, and it says {}", value);
}

/// Every revision of a block may be **accepted** at once.
#[test]
fn every_revision_of_a_block_may_be_accepted_at_once() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4, "revision": { "kind": "insert", "author": "a reader" } },
            { "len": 6, "revision": { "kind": "delete", "author": "another" } },
            { "len": 9 },
        ]),
    );
    let outcome = session
        .execute("prose.revision.accept.all", &json!({ "block": block }))
        .expect("the revisions are accepted");
    assert_eq!(outcome.data["count"], json!(2), "two revisions were accepted");
    assert_eq!(text_of(&mut session, block), "the brown fox", "the deletion is done and the insertion stays");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 2, "and the run of the deletion is gone");
    assert!(runs[0].get("revision").is_none(), "and the mark of the insertion is gone");
}

/// Every revision of a block may be **rejected** at once.
#[test]
fn every_revision_of_a_block_may_be_rejected_at_once() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        json!([
            { "len": 4, "revision": { "kind": "insert", "author": "a reader" } },
            { "len": 6, "revision": { "kind": "delete", "author": "another" } },
            { "len": 9 },
        ]),
    );
    let outcome = session
        .execute("prose.revision.reject.all", &json!({ "block": block }))
        .expect("the revisions are rejected");
    assert_eq!(outcome.data["count"], json!(2), "two revisions were rejected");
    assert_eq!(
        text_of(&mut session, block),
        "quick brown fox",
        "the insertion is undone and the deletion stays"
    );
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 2, "and the run of the insertion is gone");
    assert!(runs[0].get("revision").is_none(), "and the mark of the deletion is gone");
}

/// A run with **no revision** is refused: there is nothing to accept.
#[test]
fn a_run_with_no_revision_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, json!([{ "len": 4 }]));
    let value = session
        .execute("prose.revision.accept", &json!({ "block": block, "run": 0 }))
        .expect_err("the acceptance is refused");
    assert!(value.to_string().contains("no revision"), "the refusal says why, and it says {}", value);
}

/// A block with **no revisions** has nothing to accept, and accepting
/// nothing is not a failure.
#[test]
fn a_block_with_no_revisions_has_nothing_to_accept() {
    let mut session = session();
    let block = a_paragraph(&mut session, json!([{ "len": 4 }]));
    let outcome = session
        .execute("prose.revision.accept.all", &json!({ "block": block }))
        .expect("there is nothing to accept");
    assert_eq!(outcome.data["count"], json!(0), "nothing was accepted");
}
