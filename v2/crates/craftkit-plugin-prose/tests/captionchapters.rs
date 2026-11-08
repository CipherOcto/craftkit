//! The **chapter** of a caption.
//!
//! A caption that names a chapter is counted again at each chapter,
//! and says the chapter it is in.

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

/// A **document** with the given **paragraphs**: a `true` says the
/// paragraph is a **heading** of the first level.
fn a_document(session: &mut Session, texts: &[(&str, bool)]) -> (u64, Vec<u64>) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    // A **style** that is a heading of the first level.
    let outcome = session
        .execute(
            "prose.style.add",
            &json!({ "document": document, "name": "the heading", "style_kind": "paragraph" }),
        )
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    session.execute("prose.style.level", &json!({ "style": style, "level": 1 })).expect("the level is set");
    let mut blocks: Vec<u64> = Vec::new();
    for (text, heading) in texts {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
            .expect("the paragraph is added");
        let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
        if *heading {
            session
                .execute("prose.block.style", &json!({ "block": block, "style": style }))
                .expect("the style is set");
        }
        blocks.push(block);
    }
    (document, blocks)
}

/// The **captions** of a document, each with the number it has.
fn captions_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("prose.document.captions", &json!({ "document": document }))
        .expect("the captions are read");
    outcome.data["captions"].as_array().expect("the captions are a list").clone()
}

/// A caption that names a **chapter** says the chapter it is in.
#[test]
fn a_caption_says_the_chapter_it_is_in() {
    let mut session = session();
    let (document, blocks) = a_document(
        &mut session,
        &[("the first chapter", true), ("the second chapter", true), ("a picture", false)],
    );
    session
        .execute("prose.block.caption", &json!({ "block": blocks[2], "label": "Figure", "chapter": 1 }))
        .expect("the caption is made");
    let captions = captions_of(&mut session, document);
    assert_eq!(captions.len(), 1, "the document has one caption: {captions:?}");
    assert_eq!(captions[0]["chapter"], json!(2), "and it is in the second chapter: {captions:?}");
    assert_eq!(captions[0]["full"], json!("2-1"), "and it says so: {captions:?}");
}

/// The number of a caption is counted **again at each chapter**: the
/// tree's own rule.
#[test]
fn the_number_is_counted_again_at_each_chapter() {
    let mut session = session();
    let (document, blocks) = a_document(
        &mut session,
        &[
            ("the first chapter", true),
            ("a picture", false),
            ("another picture", false),
            ("the second chapter", true),
            ("a third picture", false),
        ],
    );
    for block in [&blocks[1], &blocks[2], &blocks[4]] {
        session
            .execute("prose.block.caption", &json!({ "block": block, "label": "Figure", "chapter": 1 }))
            .expect("the caption is made");
    }
    let captions = captions_of(&mut session, document);
    assert_eq!(captions.len(), 3, "the document has three captions: {captions:?}");
    assert_eq!(captions[0]["number"], json!("1"), "the first is the first: {captions:?}");
    assert_eq!(captions[0]["full"], json!("1-1"), "and it is in the first chapter: {captions:?}");
    assert_eq!(captions[1]["number"], json!("2"), "the second is the second: {captions:?}");
    assert_eq!(captions[1]["full"], json!("1-2"), "and it is in the same chapter: {captions:?}");
    assert_eq!(captions[2]["number"], json!("1"), "and the third is counted again: {captions:?}");
    assert_eq!(captions[2]["full"], json!("2-1"), "in the second chapter: {captions:?}");
}

/// The **separator** between the chapter and the number is one of the
/// tree's five.
#[test]
fn the_separator_is_one_of_the_trees_five() {
    let mut session = session();
    let (document, blocks) = a_document(&mut session, &[("a chapter", true), ("a picture", false)]);
    session
        .execute(
            "prose.block.caption",
            &json!({ "block": blocks[1], "label": "Table", "chapter": 1, "separator": ":" }),
        )
        .expect("the caption is made");
    let captions = captions_of(&mut session, document);
    assert_eq!(captions[0]["full"], json!("1:1"), "the separator says so: {captions:?}");
    let error = session
        .execute(
            "prose.block.caption",
            &json!({ "block": blocks[1], "label": "Table", "chapter": 1, "separator": "/" }),
        )
        .expect_err("a slash is not one of the tree's separators");
    assert!(error.to_string().contains("separator"), "the refusal says why: {error}");
}

/// A chapter that is **not one of the tree's nine levels** is refused.
#[test]
fn a_chapter_that_is_not_a_level_is_refused() {
    let mut session = session();
    let (document, blocks) = a_document(&mut session, &[("a chapter", true), ("a picture", false)]);
    let error = session
        .execute("prose.block.caption", &json!({ "block": blocks[1], "label": "Figure", "chapter": 12 }))
        .expect_err("twelve is not a level");
    assert!(error.to_string().contains("levels"), "the refusal says why: {error}");
    let captions = captions_of(&mut session, document);
    assert!(captions.is_empty(), "and no caption was made: {captions:?}");
}
