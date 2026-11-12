//! The **facts** of a document and the **file** that carries them.
//!
//! The properties, the sources, the captions, the columns, the fields
//! and the borders of a document are facts of its nodes — they are in
//! the **payload** — and the payload is carried by the file. This is
//! the test that says so.

use craftkit_sdk::format::{read_document, write_document, write_document_oldest};
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
fn a_session() -> (Session, Ecosystem) {
    let first = ecosystem();
    let session =
        Session::with_formats(first.commands, first.kinds, first.formats, first.migrations, first.evaluators);
    (session, ecosystem())
}

/// A **document** with a **paragraph** and every fact this battery has
/// added to a document.
fn a_full_document(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute(
            "prose.block.paragraph.add",
            &json!({ "document": document, "text": "he said \"hello\" to her" }),
        )
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "author": "Ada", "custom": { "Project": "Atlas" } }),
        )
        .expect("the properties are set");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "title": "Notes on the Engine" }),
        )
        .expect("the source is said");
    session
        .execute("prose.block.caption", &json!({ "block": block, "label": "Figure" }))
        .expect("the caption is said");
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2 }))
        .expect("the columns are set");
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 8, "end": 13, "code": "GREETING" }),
        )
        .expect("the field is added");
    session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box", "style": "double" }))
        .expect("the borders are set");
    session
        .execute("prose.page.background", &json!({ "document": document, "colour": "#112233" }))
        .expect("the background is set");
    (document, block)
}

/// The **newest** format of the tree, and the tag it carries.
fn newest(ecosystem: &Ecosystem) -> &craftkit_sdk::format::FormatSpec {
    let mut best: Option<&craftkit_sdk::format::FormatSpec> = None;
    for format in ecosystem.formats.all() {
        if !format.tag.starts_with("craftkit/") {
            continue;
        }
        match best {
            Some(held) if held.version >= format.version => {}
            _ => best = Some(format),
        }
    }
    best.expect("there is a craftkit format")
}

/// The facts of a document, as they are read back from a file.
fn facts_of(
    document: &craftkit_sdk::Document,
    document_id: u64,
    block_id: u64,
) -> (String, String, String, String) {
    let node = document.node(document_id).expect("the document is in the document");
    let block = document.node(block_id).expect("the paragraph is in the document");
    (
        node.data.get("properties").map(|value| value.to_string()).unwrap_or_default(),
        node.data.get("custom_properties").map(|value| value.to_string()).unwrap_or_default(),
        node.data.get("sources").map(|value| value.to_string()).unwrap_or_default(),
        block.data.get("caption").map(|value| value.to_string()).unwrap_or_default(),
    )
}

/// The facts of a document **survive the file**: they are written and
/// they are read back.
#[test]
fn the_facts_survive_the_file() {
    let (mut session, ecosystem) = a_session();
    let (document_id, block_id) = a_full_document(&mut session);
    let before = facts_of(session.document(), document_id, block_id);
    let format = newest(&ecosystem);
    let written = write_document(session.document(), format);
    let read_back = read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the document is read back");
    let after = facts_of(&read_back, document_id, block_id);
    assert_eq!(after, before, "the facts are the same after the file");
    assert!(before.0.contains("Ada"), "the author is there: {before:?}");
    assert!(before.1.contains("Atlas"), "and the custom property: {before:?}");
    assert!(before.2.contains("Notes on the Engine"), "and the source: {before:?}");
    assert!(before.3.contains("Figure"), "and the caption: {before:?}");
}

/// The **oldest** format that loses nothing keeps the facts as well:
/// the payload is carried by every version of the file.
#[test]
fn the_oldest_format_that_loses_nothing_keeps_the_facts() {
    let (mut session, ecosystem) = a_session();
    let (document_id, block_id) = a_full_document(&mut session);
    let before = facts_of(session.document(), document_id, block_id);
    let (written, version) =
        write_document_oldest(session.document(), &ecosystem.formats, &ecosystem.migrations, "craftkit")
            .expect("some version keeps everything");
    assert!(version >= 1, "the version is one of the tree's: {version}");
    let read_back = read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the document is read back");
    let after = facts_of(&read_back, document_id, block_id);
    assert_eq!(after, before, "the facts are the same after the file of version {version}");
}
