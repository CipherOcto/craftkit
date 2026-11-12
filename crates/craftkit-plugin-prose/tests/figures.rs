//! The **table of figures** of a document.
//!
//! A table of figures is the list of the captions that carry one
//! label, and it is derived from them.

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

/// A **document** with a **caption** in it, and the document's id.
///
/// The caption carries the label and says the text.
fn a_document(session: &mut Session, label: &str, text: &str) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the block" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.caption", &json!({ "block": block, "label": label, "text": text }))
        .expect("the caption is said");
    document
}

/// The **table of figures** of a document, and the entries in it.
fn figures(session: &mut Session, document: u64, label: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("prose.document.figures", &json!({ "document": document, "label": label }))
        .expect("the table is read");
    outcome.data["entries"].as_array().expect("the entries are a list").clone()
}

/// The table of figures is the **captions of one label**, in the
/// order of the document.
#[test]
fn the_table_of_figures_is_the_captions_of_one_label() {
    let mut session = session();
    let document = a_document(&mut session, "Figure", "the bridge");
    // A **second** caption, and a caption of another label.
    for (label, text) in [("Figure", "the mill"), ("Table", "the sales")] {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the block" }))
            .expect("the paragraph is added");
        let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
        session
            .execute("prose.block.caption", &json!({ "block": block, "label": label, "text": text }))
            .expect("the caption is said");
    }
    let entries = figures(&mut session, document, "Figure");
    assert_eq!(entries.len(), 2, "the table holds the two figures: {entries:?}");
    assert_eq!(entries[0]["entry"], json!("Figure 1 the bridge"), "the first: {entries:?}");
    assert_eq!(entries[1]["entry"], json!("Figure 2 the mill"), "the second: {entries:?}");
    let entries = figures(&mut session, document, "Table");
    assert_eq!(entries.len(), 1, "the table of the tables holds one: {entries:?}");
    assert_eq!(entries[0]["entry"], json!("Table 1 the sales"), "and it is this: {entries:?}");
}

/// The label of a table of figures is the tree's label, `Figure`,
/// when it is not given.
#[test]
fn the_label_of_a_table_is_figure_when_it_is_not_given() {
    let mut session = session();
    let document = a_document(&mut session, "Figure", "the bridge");
    let outcome = session
        .execute("prose.document.figures", &json!({ "document": document }))
        .expect("the table is read");
    assert_eq!(outcome.data["label"], json!("Figure"), "the label is the tree's: {outcome:?}");
    assert_eq!(
        outcome.data["entries"].as_array().map(|entries| entries.len()),
        Some(1),
        "and the table holds the caption: {outcome:?}"
    );
}

/// A label that says nothing is the tree's label too.
#[test]
fn a_label_that_says_nothing_is_figure() {
    let mut session = session();
    let document = a_document(&mut session, "Figure", "the bridge");
    let entries = figures(&mut session, document, "   ");
    assert_eq!(entries.len(), 1, "the table of nothing is the table of figures: {entries:?}");
}

/// The tree joins a multi-word caption label with underscores, so
/// `Photo Plate` and `Photo_Plate` are one label.
#[test]
fn a_multi_word_label_is_joined_with_underscores() {
    let mut session = session();
    let document = a_document(&mut session, "Photo Plate", "the portrait");
    let entries = figures(&mut session, document, "Photo Plate");
    assert_eq!(entries.len(), 1, "the table of the label: {entries:?}");
    let entries = figures(&mut session, document, "Photo_Plate");
    assert_eq!(entries.len(), 1, "and the table of the same label, underscored: {entries:?}");
}

/// A table asked **not to carry the label** shows the text of the
/// caption alone: the tree's own rule.
#[test]
fn a_table_asked_not_to_carry_the_label_shows_the_text_alone() {
    let mut session = session();
    let document = a_document(&mut session, "Figure", "the bridge");
    let outcome = session
        .execute(
            "prose.document.figures",
            &json!({ "document": document, "label": "Figure", "include_label": false }),
        )
        .expect("the table is read");
    let entries = outcome.data["entries"].as_array().expect("the entries are a list");
    assert_eq!(entries[0]["entry"], json!("the bridge"), "the entry is the text: {outcome:?}");
    assert_eq!(entries[0]["number"], json!("1"), "and the number is still there: {outcome:?}");
}

/// A label that **no caption carries** gives an empty table: the
/// tree's table of figures of a label that is not used is empty too.
#[test]
fn a_label_that_no_caption_carries_gives_an_empty_table() {
    let mut session = session();
    let document = a_document(&mut session, "Figure", "the bridge");
    let entries = figures(&mut session, document, "Equation");
    assert!(entries.is_empty(), "the table is empty: {entries:?}");
}
