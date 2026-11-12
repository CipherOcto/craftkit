//! The **numbering** of a block.
//!
//! A list says what its items are numbered like, and the number of a
//! block is derived: it is the count of the blocks of the list before
//! it and the format of the level.

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

/// A **paragraph** in a document, and its id.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// A **list** in a document, and its id.
fn a_list(session: &mut Session, document: u64, kind: &str) -> u64 {
    let outcome = session
        .execute("prose.list.add", &json!({ "document": document, "kind": kind }))
        .expect("the list is added");
    outcome.data["id"].as_u64().expect("the list has an id")
}

/// Put a **block** in a list, at a level.
fn number(session: &mut Session, block: u64, list: u64, level: Option<u64>) {
    let mut params = json!({ "block": block, "list": list });
    if let Some(level) = level {
        params["level"] = json!(level);
    }
    session.execute("prose.block.numbering", &params).expect("the block is numbered");
}

/// The **label** of a block.
fn label_of(session: &mut Session, block: u64) -> Json {
    let outcome =
        session.execute("prose.block.number", &json!({ "block": block })).expect("the number is read");
    outcome.data["label"].clone()
}

/// A block in a list is **numbered**.
#[test]
fn a_block_in_a_list_is_numbered() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "numbered");
    let paragraph = a_paragraph(&mut session, document, "the first words");
    number(&mut session, paragraph, list, None);
    assert_eq!(label_of(&mut session, paragraph), Json::String("1.".to_string()), "the block is the first");
}

/// The number **counts** the blocks of the list before it.
#[test]
fn the_number_counts_the_blocks_before_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "numbered");
    let first = a_paragraph(&mut session, document, "the first words");
    let second = a_paragraph(&mut session, document, "the second words");
    number(&mut session, first, list, None);
    number(&mut session, second, list, None);
    assert_eq!(label_of(&mut session, first), Json::String("1.".to_string()), "the first is the first");
    assert_eq!(
        label_of(&mut session, second),
        Json::String("2.".to_string()),
        "and the second is the second"
    );
}

/// The **number** is derived: a block that is moved is renumbered.
#[test]
fn the_number_is_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "numbered");
    let first = a_paragraph(&mut session, document, "the first words");
    let second = a_paragraph(&mut session, document, "the second words");
    number(&mut session, first, list, None);
    number(&mut session, second, list, None);
    session.execute("prose.block.move", &json!({ "block": second, "to": 1 })).expect("the block is moved");
    assert_eq!(
        label_of(&mut session, second),
        Json::String("1.".to_string()),
        "the moved block is the first now"
    );
    assert_eq!(label_of(&mut session, first), Json::String("2.".to_string()), "and the other is the second");
}

/// A **level** counts in its own format: the tree's `numbered` list
/// counts `1.`, `a.` and `i.` as the levels go deeper.
#[test]
fn a_level_counts_in_its_own_format() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "numbered");
    let paragraph = a_paragraph(&mut session, document, "the first words");
    number(&mut session, paragraph, list, Some(1));
    assert_eq!(label_of(&mut session, paragraph), Json::String("a.".to_string()), "the level is letters");
}

/// A **bullet** list shows a bullet.
#[test]
fn a_bullet_list_shows_a_bullet() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "bullet");
    let paragraph = a_paragraph(&mut session, document, "the first words");
    number(&mut session, paragraph, list, None);
    assert_eq!(label_of(&mut session, paragraph), Json::String("•".to_string()), "the label is a bullet");
}

/// A list may be a **bullet of its own**: a single character.
#[test]
fn a_list_may_be_a_bullet_of_its_own() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "★");
    let paragraph = a_paragraph(&mut session, document, "the first words");
    number(&mut session, paragraph, list, None);
    assert_eq!(
        label_of(&mut session, paragraph),
        Json::String("★".to_string()),
        "the label is the character"
    );
}

/// A block that is in **no list** has no number.
#[test]
fn a_block_that_is_in_no_list_has_no_number() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    assert_eq!(label_of(&mut session, paragraph), Json::Null, "there is no number");
}

/// A kind that is **not a kind** is refused.
#[test]
fn a_kind_that_is_not_a_kind_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.list.add", &json!({ "document": document, "kind": "spirals" }))
        .expect_err("the kind is refused");
    assert!(error.to_string().contains("there is no list kind"), "the refusal says why: {error}");
}

/// A level that is **not a level** is refused.
#[test]
fn a_level_that_is_not_a_level_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let list = a_list(&mut session, document, "numbered");
    let paragraph = a_paragraph(&mut session, document, "the first words");
    let error = session
        .execute("prose.block.numbering", &json!({ "block": paragraph, "list": list, "level": 12 }))
        .expect_err("the level is refused");
    assert!(error.to_string().contains("a list has 9 levels"), "the refusal says why: {error}");
}

/// A list that is **not there** is refused, and so is a node that is
/// not a block.
#[test]
fn a_list_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    let error = session
        .execute("prose.block.numbering", &json!({ "block": paragraph, "list": 999 }))
        .expect_err("the list is refused");
    assert!(error.to_string().contains("there is no list"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.numbering", &json!({ "block": document, "list": 999 }))
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
}
