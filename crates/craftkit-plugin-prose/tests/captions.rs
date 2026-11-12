//! The **captions** of a document.
//!
//! A caption names a thing: "Table 3 — the sales of the year". The
//! label is what the thing is called, and the number is the count of
//! the captions that carry that label — worked out every time.

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

/// A **document** with a **paragraph** that says the given words.
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

/// The **captions** of a document, as the command reads them.
fn captions_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.document.captions", &json!({ "document": document }))
        .expect("the captions are read");
    outcome.data["captions"].clone()
}

/// A caption names a thing, and its **number** is derived.
#[test]
fn a_caption_is_numbered() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "the sales of the year");
    session
        .execute(
            "prose.block.caption",
            &json!({ "block": block, "label": "Table", "text": "the sales of the year" }),
        )
        .expect("the caption is said");
    let captions = captions_of(&mut session, document);
    assert_eq!(captions.as_array().map(|captions| captions.len()), Some(1), "there is one caption");
    assert_eq!(captions[0]["label"], json!("Table"), "the label is the one that was given");
    assert_eq!(captions[0]["text"], json!("the sales of the year"), "and the text");
    assert_eq!(captions[0]["number"], json!("1"), "and the number is one");
}

/// The captions that carry the **same label** are numbered together,
/// and the ones with another label are numbered apart.
#[test]
fn the_labels_are_numbered_apart() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    for (label, text) in [("Table", "one"), ("Figure", "two"), ("Table", "three")] {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
            .expect("the paragraph is added");
        let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
        session
            .execute("prose.block.caption", &json!({ "block": block, "label": label, "text": text }))
            .expect("the caption is said");
    }
    let captions = captions_of(&mut session, document);
    assert_eq!(captions.as_array().map(|captions| captions.len()), Some(3), "there are three captions");
    assert_eq!(captions[0]["number"], json!("1"), "the first table is one");
    assert_eq!(captions[1]["number"], json!("1"), "the figure is one as well");
    assert_eq!(captions[2]["number"], json!("2"), "and the second table is two");
}

/// The **format** of the number is one of the tree's own, and the
/// number is written in it.
#[test]
fn the_number_is_written_in_its_format() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    for (format, text) in [("roman", "one"), ("alphabetic", "two"), ("ALPHABETIC", "three")] {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
            .expect("the paragraph is added");
        let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
        session
            .execute("prose.block.caption", &json!({ "block": block, "label": "Figure", "format": format }))
            .expect("the caption is said");
    }
    let captions = captions_of(&mut session, document);
    assert_eq!(captions[0]["number"], json!("i"), "the first is a roman one");
    assert_eq!(captions[1]["number"], json!("b"), "the second is a letter");
    assert_eq!(captions[2]["number"], json!("C"), "and the third is a capital one");
}

/// A caption that is **not one** is refused: a format that is not
/// the tree's, and a label with a quote in it.
#[test]
fn a_caption_that_is_not_one_is_refused() {
    let mut session = session();
    let (_document, block) = a_paragraph(&mut session, "hello");
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "label": "Table", "format": "greek" }))
        .expect_err("the format is refused");
    assert!(error.to_string().contains("no format"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "label": "Ta\"ble" }))
        .expect_err("the label with a quote is refused");
    assert!(error.to_string().contains("no quote"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "label": "T".repeat(50) }))
        .expect_err("the long label is refused");
    assert!(error.to_string().contains("at most"), "the refusal says why: {error}");
}

/// An **empty** label is the tree's label, and a block that is not a
/// paragraph may not be a caption.
#[test]
fn an_empty_label_is_the_trees_label() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "hello");
    session
        .execute("prose.block.caption", &json!({ "block": block, "label": "  " }))
        .expect("the caption is said");
    let captions = captions_of(&mut session, document);
    assert_eq!(captions[0]["label"], json!("Figure"), "the label is the tree's: {captions}");
    let error = session
        .execute("prose.block.caption", &json!({ "block": document }))
        .expect_err("the document is not a paragraph");
    assert!(error.to_string().contains("not a paragraph"), "the refusal says why: {error}");
}
