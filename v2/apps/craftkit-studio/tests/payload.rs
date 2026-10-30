//! A relation's **payload**: what the edge says, besides its kind.
//!
//! The payload is the eighth version's field, and these are the tests
//! that hold the two writers to it: the eighth version writes it, and
//! the seventh version — which does not know it — takes it off again.
//!
//! This file exists because the round trip in `version.rs` found the
//! bug: the seventh version wrote a field it did not know, and the
//! eighth version dropped the field it had, so a relation that said
//! something said it only until the document was saved.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, Ecosystem, Relation};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A document with a relation that says something.
fn a_document_that_says_something() -> Document {
    let mut document = Document::new("talking");
    let from = document.add_node("rect", "geometry.rect", None);
    let to = document.add_node("other", "geometry.rect", None);
    document.relations.push(Relation::with_payload(from, to, "test.link", json!({ "a": 1 })));
    document
}

/// The **eighth** version writes the payload: it is its field.
#[test]
fn the_eighth_version_writes_the_payload() {
    let ecosystem = ecosystem();
    let eighth = ecosystem
        .formats
        .all()
        .iter()
        .find(|format| format.version == 8)
        .expect("the eighth version is there");
    let written = craftkit_sdk::format::write_document(&a_document_that_says_something(), eighth);
    let relations = written["relations"].as_array().expect("there are relations");
    assert_eq!(relations[0]["payload"], json!({ "a": 1 }), "the payload is in the file");
}

/// The **seventh** version takes it off again: a file of a version must
/// not carry a field that version does not know.
#[test]
fn the_seventh_version_takes_the_payload_off() {
    let ecosystem = ecosystem();
    let seventh = ecosystem
        .formats
        .all()
        .iter()
        .find(|format| format.version == 7)
        .expect("the seventh version is there");
    let written = craftkit_sdk::format::write_document(&a_document_that_says_something(), seventh);
    let relations = written["relations"].as_array().expect("there are relations");
    assert!(relations[0].get("payload").is_none(), "the seventh version does not know the payload");
}

/// And the payload **comes back**: the document that is written and read
/// is the document that went in.
#[test]
fn the_payload_comes_back() {
    let ecosystem = ecosystem();
    let document = a_document_that_says_something();
    let eighth = ecosystem
        .formats
        .all()
        .iter()
        .find(|format| format.version == 8)
        .expect("the eighth version is there");
    let written = craftkit_sdk::format::write_document(&document, eighth);
    let read = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the document is read back");
    assert_eq!(read.relations[0].payload, json!({ "a": 1 }), "the payload came back");
    assert_eq!(read, document, "and the document came back whole");
}
