//! The second payload field: a node has a dictionary **and** a stream.
//!
//! The bytes are not JSON and they are not in the document: they travel
//! beside it, in the bundle, and the node says which file is its stream.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with one node in it.
fn one_node() -> Session {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
}

/// A node can have a stream.
#[test]
fn a_node_can_have_a_stream() {
    let mut session = one_node();
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");
    assert_eq!(
        session.document().node(1).expect("the node is there").stream.as_deref(),
        Some("streams/page-1.bin")
    );
    let outcome = session.execute("node.stream", &json!({ "node": 1 })).expect("the stream is read");
    assert_eq!(outcome.data["stream"], json!("streams/page-1.bin"));
}

/// And it can lose it again.
#[test]
fn a_node_can_lose_its_stream() {
    let mut session = one_node();
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");
    session.execute("node.stream.clear", &json!({ "node": 1 })).expect("the stream is cleared");
    assert!(session.document().node(1).expect("the node is there").stream.is_none());
    // Clearing a stream that is not there is refused.
    let error =
        session.execute("node.stream.clear", &json!({ "node": 1 })).expect_err("there is no stream to clear");
    assert!(error.to_string().contains("no stream"), "{error}");
}

/// The stream survives a file.
#[test]
fn the_stream_survives_a_file() {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");

    let format =
        craftkit_sdk::format::newest_format(&ecosystem.formats, "craftkit").expect("there is a format");
    let written = craftkit_sdk::format::write_document(session.document(), format);
    assert_eq!(written["nodes"][0]["stream"], json!("streams/page-1.bin"), "the stream was written");
    let read = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    assert_eq!(read.nodes[0].stream.as_deref(), Some("streams/page-1.bin"), "the stream came back");
}

/// A file of the **sixth** version has no stream, and is not given one.
///
/// The step gives every node the field the seventh version writes, and
/// the field is empty: a node of the sixth version had no stream.
#[test]
fn a_file_of_the_sixth_version_has_no_stream() {
    let ecosystem = ecosystem();
    let old = json!({
        "format": "craftkit/6",
        "version": 6,
        "name": "old",
        "data": null,
        "nodes": [{
            "id": 1,
            "name": "page",
            "kind": "geometry.rect",
            "rect": null,
            "muted": false,
            "data": null,
            "children": [],
        }],
        "relations": [],
        "assets": [],
    });
    let read = craftkit_sdk::format::read_document(&old, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    assert!(read.nodes[0].stream.is_none(), "the node has no stream");
}

/// A file of the **sixth** version written by this program does not carry
/// the seventh version's field.
#[test]
fn a_file_of_the_sixth_version_does_not_carry_the_stream() {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");
    let old = ecosystem.formats.get("craftkit/6").expect("there is a sixth version");
    let written = craftkit_sdk::format::write_document(session.document(), old);
    assert!(written["nodes"][0].get("stream").is_none(), "the sixth version has no stream field");
}

/// The bytes travel **beside** the document, and come back byte for byte.
#[test]
fn the_bytes_come_back_byte_for_byte() {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");

    // The bytes are not JSON, and they are not in the document.
    let bytes: Vec<u8> = vec![0x00, 0xff, 0x10, 0x80, b'{', b'"'];
    let bundle = craftkit_sdk::bundle::write(
        session.document(),
        vec![craftkit_sdk::bundle::File::new("streams/page-1.bin", bytes.clone())],
        &ecosystem.formats,
    )
    .expect("the bundle is written");
    let read = craftkit_sdk::bundle::read(&bundle, &ecosystem.formats, &ecosystem.migrations)
        .expect("the bundle is read");
    assert_eq!(read.document.nodes[0].stream.as_deref(), Some("streams/page-1.bin"));
    let file = read.files.iter().find(|file| file.path == "streams/page-1.bin").expect("the file is there");
    assert_eq!(file.bytes, bytes, "the bytes came back byte for byte");
    assert!(read.missing().is_empty(), "the bundle is complete: {:?}", read.missing());
}

/// A document that names a file it has not got says so.
#[test]
fn a_document_says_what_it_is_missing() {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");

    let bundle = craftkit_sdk::bundle::write(session.document(), Vec::new(), &ecosystem.formats)
        .expect("the bundle is written");
    let read = craftkit_sdk::bundle::read(&bundle, &ecosystem.formats, &ecosystem.migrations)
        .expect("the bundle is read");
    assert_eq!(read.missing(), vec!["streams/page-1.bin".to_string()], "the stream is missing");
}

/// The document does not grow by the size of its bytes.
///
/// The whole point of the stream is that the JSON stays small: a
/// document that carries fifty megabytes of streams carries fifty
/// megabytes of *file*, and the text is only the names.
#[test]
fn the_document_does_not_grow_by_the_size_of_its_bytes() {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");
    let format =
        craftkit_sdk::format::newest_format(&ecosystem.formats, "craftkit").expect("there is a format");
    let written = craftkit_sdk::format::write_document(session.document(), format);
    let text = written.to_string();
    assert!(text.len() < 400, "the document is small: {} bytes", text.len());
    assert!(text.contains("streams/page-1.bin"), "the name of the stream is in it");
}
