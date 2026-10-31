//! A reader that keeps what it does not understand.
//!
//! A file written by a newer program must not come back **smaller**. The
//! fields the reader knows are read; the fields it does not know are kept
//! and written back as they were.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The newest format of the craftkit family.
fn newest(ecosystem: &Ecosystem) -> craftkit_sdk::FormatSpec {
    *craftkit_sdk::format::newest_format(&ecosystem.formats, "craftkit").expect("there is a craftkit format")
}

/// A file with a field this program has never heard of.
fn a_file_with_an_unknown_field() -> craftkit_sdk::serde_json::Value {
    json!({
        "format": "craftkit/6",
        "version": 6,
        "name": "with a view",
        "data": null,
        "nodes": [],
        "relations": [],
        "assets": [],
        "views": ["one", "two"],
    })
}

/// A field the reader does not understand is kept.
#[test]
fn a_field_the_reader_does_not_understand_is_kept() {
    let ecosystem = ecosystem();
    let file = a_file_with_an_unknown_field();
    let document = craftkit_sdk::format::read_document(&file, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    assert_eq!(document.unknown["views"], json!(["one", "two"]), "the views were kept");
}

/// And it is written back.
#[test]
fn what_was_kept_is_written_back() {
    let ecosystem = ecosystem();
    let file = a_file_with_an_unknown_field();
    let document = craftkit_sdk::format::read_document(&file, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    let written = craftkit_sdk::format::write_document(&document, &newest(&ecosystem));
    assert_eq!(written["views"], json!(["one", "two"]), "the views were written back");
    // And the file is still a file of this program's version.
    assert_eq!(written["format"], "craftkit/9", "the newest version writes it");
}

/// A field of a **node** the reader does not understand is kept too.
#[test]
fn a_node_field_the_reader_does_not_understand_is_kept() {
    let ecosystem = ecosystem();
    let file = json!({
        "format": "craftkit/6",
        "version": 6,
        "name": "a node with a field",
        "data": null,
        "nodes": [{
            "id": 1,
            "name": "one",
            "kind": "geometry.rect",
            "rect": null,
            "muted": false,
            "data": null,
            "children": [],
            "opacity": 0.5,
        }],
        "relations": [],
        "assets": [],
    });
    let document = craftkit_sdk::format::read_document(&file, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    assert_eq!(document.nodes[0].unknown["opacity"], json!(0.5), "the opacity was kept");
    let written = craftkit_sdk::format::write_document(&document, &newest(&ecosystem));
    assert_eq!(written["nodes"][0]["opacity"], json!(0.5), "the opacity was written back");
}

/// A file from a **newer program** is read, and what is not understood is
/// kept.
#[test]
fn a_file_from_a_newer_program_is_read_and_kept() {
    let ecosystem = ecosystem();
    let file = json!({
        "format": "craftkit/10",
        "version": 10,
        "name": "from the future",
        "data": null,
        "nodes": [],
        "relations": [],
        "assets": [],
        "views": ["one"],
    });
    let (document, forward) =
        craftkit_sdk::format::read_document_noting(&file, &ecosystem.formats, &ecosystem.migrations)
            .expect("the file is read");
    assert_eq!(forward, Some(10), "the reader says the file was newer");
    assert_eq!(document.name, "from the future", "the name was read");
    assert_eq!(document.unknown["views"], json!(["one"]), "the views were kept");
    // The file this program writes keeps them.
    let written = craftkit_sdk::format::write_document(&document, &newest(&ecosystem));
    assert_eq!(written["views"], json!(["one"]), "the views were written back");
}

/// A file of a **family** this program does not know is still refused.
///
/// There is nothing to keep: the file is not a document of a shape this
/// program has ever seen.
#[test]
fn a_file_of_an_unknown_family_is_refused() {
    let ecosystem = ecosystem();
    let file = json!({ "format": "somethingelse/1", "version": 1, "name": "other" });
    let refused = craftkit_sdk::format::read_document(&file, &ecosystem.formats, &ecosystem.migrations);
    assert!(refused.is_err(), "the file of another family was read");
}

/// A **step** does not overwrite a field that is already there.
///
/// This is the rule that makes the kept fields safe: a program that reads
/// an old file and migrates it forward must not destroy a field that a
/// newer program put there.
#[test]
fn a_step_does_not_overwrite_a_field_that_is_already_there() {
    let ecosystem = ecosystem();
    // A file of the fifth version that already has the assets the sixth
    // version writes.
    let file = json!({
        "format": "craftkit/5",
        "version": 5,
        "name": "old",
        "data": null,
        "nodes": [],
        "relations": [],
        "assets": [{ "path": "sounds/kick.wav", "kind": "audio", "length": 4 }],
    });
    let document = craftkit_sdk::format::read_document(&file, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    assert_eq!(document.assets.len(), 1, "the assets were kept");
    assert_eq!(document.assets[0].path, "sounds/kick.wav");
}

/// The whole way round: a file with a field this program does not know
/// survives being read and written.
#[test]
fn an_unknown_field_survives_the_whole_way_round() {
    let ecosystem = ecosystem();
    let file = a_file_with_an_unknown_field();
    let document = craftkit_sdk::format::read_document(&file, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    let written = craftkit_sdk::format::write_document(&document, &newest(&ecosystem));
    let again = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read again");
    assert_eq!(again.unknown, document.unknown, "the kept fields are the same");
    assert_eq!(again.unknown["views"], json!(["one", "two"]));
}

/// The program **says** that the file was newer.
///
/// A program that reads a file from the future is doing something it
/// should say out loud.
#[test]
fn opening_a_file_from_the_future_says_so() {
    let ecosystem = ecosystem();
    let path = std::env::temp_dir().join("craftkit-v2-future-file.craftkit");
    let path = path.to_string_lossy().to_string();
    std::fs::write(
        &path,
        craftkit_sdk::serde_json::to_string(&json!({
            "format": "craftkit/10",
            "version": 10,
            "name": "from the future",
            "data": null,
            "nodes": [],
            "relations": [],
            "assets": [],
            "views": ["one"],
        }))
        .expect("the file is written"),
    )
    .expect("the file is written");

    let mut session: Session = ecosystem.session();
    let outcome = session.execute("doc.open", &json!({ "path": path })).expect("the file is opened");
    assert!(
        outcome.message.contains("newer"),
        "the program did not say that the file was newer: {}",
        outcome.message
    );
    assert_eq!(session.document().name, "from the future");
    assert_eq!(session.document().unknown["views"], json!(["one"]), "the views were kept");
}
