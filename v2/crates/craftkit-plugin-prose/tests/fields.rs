//! The **fields** of a block.
//!
//! A field covers words of a paragraph and says what it is: the date,
//! the title of the document, the name of the author. Its result is
//! the words it covers, and it is worked out every time.

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

/// The **fields** of a block, as the command reads them.
fn fields_of(session: &mut Session, block: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.block.field.read", &json!({ "block": block })).expect("the fields are read");
    outcome.data["fields"].clone()
}

/// A field is put on a paragraph, and its **result** is the words it
/// covers.
#[test]
fn a_field_covers_words() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    let outcome = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "DATE" }))
        .expect("the field is added");
    assert_eq!(outcome.data["field"]["code"], json!("DATE"), "the field says what it is");
    assert_eq!(outcome.data["field"]["result"], json!("2026-01-01"), "and it shows the words it covers");
    let fields = fields_of(&mut session, block);
    assert_eq!(fields.as_array().map(|fields| fields.len()), Some(1), "the block has one field: {fields}");
    assert_eq!(fields[0]["result"], json!("2026-01-01"), "and it is read back with its result");
}

/// The **result** of a field is **derived**: it follows the text, and
/// it is never written down.
#[test]
fn the_result_follows_the_text() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "DATE" }))
        .expect("the field is added");
    session
        .execute(
            "prose.block.paragraph.text",
            &json!({ "block": block, "text": "the date is 2027-02-03 today" }),
        )
        .expect("the text is rewritten");
    let fields = fields_of(&mut session, block);
    assert_eq!(fields[0]["result"], json!("2027-02-03"), "the result is what the text says now: {fields}");
}

/// A field is **named** by the character it begins at, and its code
/// may be changed.
#[test]
fn a_field_is_named_by_where_it_begins() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the title is craftkit here");
    session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 13, "end": 21, "code": "TITLE" }))
        .expect("the field is added");
    let outcome = session
        .execute("prose.block.field.set", &json!({ "block": block, "field": 13, "code": "FILENAME" }))
        .expect("the field is changed");
    assert_eq!(outcome.data["field"]["code"], json!("FILENAME"), "the code is the new one");
    assert_eq!(outcome.data["field"]["start"], json!(13), "and the field has not moved");
    let fields = fields_of(&mut session, block);
    assert_eq!(fields[0]["code"], json!("FILENAME"), "and it is read back with the new code");
}

/// Two fields may not cover the same words.
#[test]
fn two_fields_may_not_cover_the_same_words() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "DATE" }))
        .expect("the field is added");
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 15, "end": 25, "code": "TIME" }))
        .expect_err("the second field is refused");
    assert!(error.to_string().contains("another field"), "the refusal says why: {error}");
    let fields = fields_of(&mut session, block);
    assert_eq!(fields.as_array().map(|fields| fields.len()), Some(1), "and the block still has one field");
}

/// A field that is **locked** is not changed and not unlinked.
#[test]
fn a_locked_field_is_not_changed() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 12, "end": 22, "code": "DATE", "locked": true }),
        )
        .expect("the field is added");
    let error = session
        .execute("prose.block.field.set", &json!({ "block": block, "field": 12, "code": "TIME" }))
        .expect_err("the locked field is not changed");
    assert!(error.to_string().contains("locked"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.field.unlink", &json!({ "block": block, "field": 12 }))
        .expect_err("the locked field is not unlinked");
    assert!(error.to_string().contains("locked"), "the refusal says why: {error}");
    let fields = fields_of(&mut session, block);
    assert_eq!(fields[0]["code"], json!("DATE"), "and the field is as it was: {fields}");
}

/// **Unlinking** a field takes the field away and leaves its **words**
/// alone: they were ordinary words all along.
#[test]
fn unlinking_leaves_the_words_alone() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "DATE" }))
        .expect("the field is added");
    let outcome = session
        .execute("prose.block.field.unlink", &json!({ "block": block, "field": 12 }))
        .expect("the field is unlinked");
    assert_eq!(outcome.data["field"]["code"], json!("DATE"), "the field that was taken away is named");
    let fields = fields_of(&mut session, block);
    assert_eq!(
        fields.as_array().map(|fields| fields.len()),
        Some(0),
        "and the block has no fields: {fields}"
    );
    let node = session.document().node(block).expect("the block is in the document");
    assert_eq!(node.data["text"], json!("the date is 2026-01-01 today"), "and the words are still there");
}

/// A field that is **not one** is refused: an empty code, a code that
/// is too long, an interval that covers no words, an interval past
/// the end of the text, and a block that is not a block.
#[test]
fn a_field_that_is_not_one_is_refused() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "  " }))
        .expect_err("the empty code is refused");
    assert!(error.to_string().contains("says nothing"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 12, "end": 22, "code": "X".repeat(5000) }),
        )
        .expect_err("the long code is refused");
    assert!(error.to_string().contains("at most"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 12, "code": "DATE" }))
        .expect_err("the empty interval is refused");
    assert!(error.to_string().contains("at least one character"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 200, "code": "DATE" }))
        .expect_err("the interval past the text is refused");
    assert!(error.to_string().contains("characters"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.block.field.add",
            &json!({ "block": _document, "start": 0, "end": 3, "code": "DATE" }),
        )
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
    let fields = fields_of(&mut session, block);
    assert_eq!(fields.as_array().map(|fields| fields.len()), Some(0), "and the block has no fields");
}

/// A field that is **not there** is refused, and a change that says
/// nothing changes nothing.
#[test]
fn a_field_that_is_not_there_is_refused() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    let error = session
        .execute("prose.block.field.set", &json!({ "block": block, "field": 12, "code": "DATE" }))
        .expect_err("there is no field to change");
    assert!(error.to_string().contains("no field"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.field.unlink", &json!({ "block": block, "field": 12 }))
        .expect_err("there is no field to unlink");
    assert!(error.to_string().contains("no field"), "the refusal says why: {error}");
    session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "DATE" }))
        .expect("the field is added");
    let error = session
        .execute("prose.block.field.set", &json!({ "block": block, "field": 12 }))
        .expect_err("a change that says nothing is refused");
    assert!(error.to_string().contains("changed or it is not"), "the refusal says why: {error}");
}

/// The **fields** of a block are **derived**: they are read from the
/// block, and they are not a thing that is kept apart from it.
#[test]
fn the_fields_are_derived() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "the date is 2026-01-01 today");
    session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 12, "end": 22, "code": "DATE" }))
        .expect("the field is added");
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == document && item.kind == "prose.blocks")
        .expect("the blocks are in the plan");
    let blocks = item.value["blocks"].as_array().expect("the blocks are a list");
    let found = blocks.iter().find(|entry| entry["block"] == json!(block)).expect("the block is in the plan");
    assert_eq!(
        found["fields"].as_array().map(|fields| fields.len()),
        Some(1),
        "the block has its field: {found}"
    );
    assert_eq!(found["fields"][0]["result"], json!("2026-01-01"), "and the result is derived with it");
    assert!(document > 0, "and the document is the document");
}
