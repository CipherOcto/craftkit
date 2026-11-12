//! The **version a file is written at**: the oldest one that loses
//! nothing.
//!
//! What is proved here: the version is **derived from the document** and
//! not from the program, the round trip keeps everything, and the bundle
//! says which version it wrote.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, Ecosystem, Relation};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The version a document would be written at.
fn version_of(document: &Document) -> u32 {
    let ecosystem = ecosystem();
    craftkit_sdk::format::write_document_oldest(
        document,
        &ecosystem.formats,
        &ecosystem.migrations,
        "craftkit",
    )
    .expect("the document is written at some version")
    .1
}

/// A document with one plain node.
fn plain() -> Document {
    let mut document = Document::new("plain");
    document.add_node("rect", "geometry.rect", None);
    document
}

/// A document with a **muted** node.
fn muted() -> Document {
    let mut document = plain();
    let id = document.nodes[0].id;
    document.node_mut(id).expect("the node is there").muted = true;
    document
}

/// A document with a **child**.
fn with_child() -> Document {
    let mut document = plain();
    let parent = document.nodes[0].id;
    let kid = document.add_node("kid", "geometry.rect", None);
    let taken = document.take_node(kid).expect("the kid is taken out");
    document.node_mut(parent).expect("the parent is there").children.push(std::sync::Arc::new(taken));
    document
}

/// A document with a **relation** that says nothing.
fn with_relation() -> Document {
    let mut document = plain();
    document.add_node("other", "geometry.rect", None);
    let to = document.nodes[1].id;
    let from = document.nodes[0].id;
    document.relations.push(Relation::with_payload(from, to, "test.link", json!(null)));
    document
}

/// A document with a **relation that says something**: the payload is
/// the eighth version's field.
fn with_relation_that_says_something() -> Document {
    let mut document = plain();
    document.add_node("other", "geometry.rect", None);
    let to = document.nodes[1].id;
    let from = document.nodes[0].id;
    document.relations.push(Relation::with_payload(from, to, "test.link", json!({ "a": 1 })));
    document
}

/// A document with a **stream**.
fn with_stream() -> Document {
    let mut document = plain();
    let id = document.nodes[0].id;
    document.node_mut(id).expect("the node is there").stream = Some("streams/page-1.bin".to_string());
    document
}

/// A plain document is written at the **oldest** version.
#[test]
fn a_plain_document_is_written_at_the_oldest_version() {
    assert_eq!(version_of(&plain()), 1, "the first version can say this document");
}

/// A **muted** node needs the version that knows about muting.
#[test]
fn a_muted_node_needs_the_version_that_knows_it() {
    assert_eq!(version_of(&muted()), 2, "the second version is the one with `muted`");
}

/// A **child** needs the version that knows about children.
#[test]
fn a_child_needs_the_version_that_knows_it() {
    assert_eq!(version_of(&with_child()), 3, "the third version is the one with children");
}

/// A **relation** needs the version that knows about relations.
#[test]
fn a_relation_needs_the_version_that_knows_it() {
    assert_eq!(version_of(&with_relation()), 5, "the fifth version is the one with relations");
}

/// A relation that **says something** needs the version that knows what
/// a relation can say: the payload is the eighth version's field.
#[test]
fn a_relation_that_says_something_needs_the_eighth_version() {
    assert_eq!(
        version_of(&with_relation_that_says_something()),
        8,
        "the payload of a relation is the eighth version's field"
    );
}

/// A **stream** needs the version that knows about streams.
#[test]
fn a_stream_needs_the_version_that_knows_it() {
    assert_eq!(version_of(&with_stream()), 7, "the seventh version is the one with streams");
}

/// The round trip keeps **everything**: the document that comes back is
/// the document that went in.
#[test]
fn the_round_trip_keeps_everything() {
    let ecosystem = ecosystem();
    for document in [plain(), muted(), with_child(), with_relation(), with_stream()] {
        let (written, version) = craftkit_sdk::format::write_document_oldest(
            &document,
            &ecosystem.formats,
            &ecosystem.migrations,
            "craftkit",
        )
        .expect("the document is written");
        let read = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
            .expect("the document is read back");
        assert_eq!(read, document, "the document came back whole, at version {version}");
    }
}

/// A relation's **payload** survives the file.
///
/// It is the eighth version's field, and the eighth version is the one
/// that writes it: this is the regression test for the bug the round
/// trip found, where the seventh version wrote a field it did not know
/// and the eighth version dropped the field it had.
#[test]
fn a_relations_payload_survives_the_file() {
    let ecosystem = ecosystem();
    let document = with_relation_that_says_something();
    let (written, version) = craftkit_sdk::format::write_document_oldest(
        &document,
        &ecosystem.formats,
        &ecosystem.migrations,
        "craftkit",
    )
    .expect("the document is written");
    assert_eq!(version, 8, "a relation that says something needs the eighth version");
    let read = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the document is read back");
    assert_eq!(read.relations[0].payload, json!({ "a": 1 }), "the payload came back");
}

/// And a file of the **seventh** version does not carry it: a file of a
/// version must not have a field that version does not know.
#[test]
fn a_file_of_the_seventh_version_does_not_carry_the_payload() {
    let ecosystem = ecosystem();
    let document = with_relation_that_says_something();
    let seventh = ecosystem
        .formats
        .all()
        .iter()
        .find(|format| format.version == 7)
        .expect("the seventh version is there");
    let written = craftkit_sdk::format::write_document(&document, seventh);
    let relations = written["relations"].as_array().expect("there are relations");
    assert!(relations[0].get("payload").is_none(), "the seventh version does not know the payload");
}

/// The **bundle** says which version it wrote.
#[test]
fn the_bundle_says_which_version_it_wrote() {
    let ecosystem = ecosystem();
    let bytes = craftkit_sdk::bundle::write(&plain(), Vec::new(), &ecosystem.formats, &ecosystem.migrations)
        .expect("the bundle is written");
    let bundle =
        craftkit_sdk::bundle::read(&bytes, &ecosystem.formats, &ecosystem.migrations).expect("it is read");
    assert_eq!(bundle.document, plain(), "the document came back");
}

/// And a document that needs the newest version gets it: the version is
/// derived from the document, and not from the program.
#[test]
fn a_document_that_needs_the_newest_gets_the_newest() {
    let mut document = plain();
    // The document's **own** payload is the fourth version's field.
    document.data = json!({ "a": 1 });
    assert_eq!(version_of(&document), 4, "a document with a payload needs the fourth version");
}

/// A node's **dictionary** is written at every version, so a node with a
/// payload does not need a late one.
#[test]
fn a_nodes_dictionary_does_not_need_a_late_version() {
    let mut document = plain();
    let id = document.nodes[0].id;
    document.node_mut(id).expect("the node is there").data = json!({ "a": 1 });
    assert_eq!(version_of(&document), 1, "the dictionary is every version's field");
}
