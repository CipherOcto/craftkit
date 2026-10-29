//! The application writes a **bundle**: the document and the files it
//! needs, in one file.
//!
//! The stream of §25 needs a producer: without one, a node's stream is a
//! name with nothing behind it.

use craftkit_sdk::Session;
use craftkit_sdk::serde_json::json;

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-bundle-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A session with a node that has a stream, and a file for it.
fn with_a_stream(from: &str) -> Session {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session
        .execute("node.add", &json!({ "name": "page", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.stream.set", &json!({ "node": 1, "path": "streams/page-1.bin" }))
        .expect("the stream is set");
    std::fs::create_dir_all(std::path::Path::new(from).join("streams")).expect("the folder is made");
    std::fs::write(std::path::Path::new(from).join("streams/page-1.bin"), [0u8, 255, 16, 128])
        .expect("the file is written");
    session
}

/// The bundle carries the document and the file, and the file comes back
/// byte for byte.
#[test]
fn a_bundle_carries_the_document_and_its_files() {
    let from = directory("carry");
    let into = directory("carry-into");
    let mut session = with_a_stream(&from);
    let bundle = format!("{from}/song.craftkit-bundle");
    session
        .execute("doc.save.bundle", &json!({ "path": bundle, "from": from }))
        .expect("the bundle is written");

    let mut opened = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    opened
        .execute("doc.open.bundle", &json!({ "path": bundle, "into": into }))
        .expect("the bundle is opened");
    assert_eq!(opened.document().name, session.document().name);
    assert_eq!(
        opened.document().node(1).expect("the node is there").stream.as_deref(),
        Some("streams/page-1.bin")
    );
    let bytes =
        std::fs::read(std::path::Path::new(&into).join("streams/page-1.bin")).expect("the file is there");
    assert_eq!(bytes, vec![0u8, 255, 16, 128], "the bytes came back byte for byte");
}

/// A file that is not there is refused **by name**.
#[test]
fn a_file_that_is_not_there_is_refused_by_name() {
    let from = directory("missing");
    let mut session = with_a_stream(&from);
    std::fs::remove_file(std::path::Path::new(&from).join("streams/page-1.bin"))
        .expect("the file is removed");
    let error = session
        .execute("doc.save.bundle", &json!({ "path": format!("{from}/out.bundle"), "from": from }))
        .expect_err("the file is missing");
    let message = error.to_string();
    assert!(message.contains("streams/page-1.bin"), "the refusal names the file: {message}");
}

/// The assets travel too, not only the streams.
#[test]
fn the_assets_travel_as_well() {
    let from = directory("assets");
    let into = directory("assets-into");
    let mut session = with_a_stream(&from);
    std::fs::write(std::path::Path::new(&from).join("font.ttf"), b"not really a font")
        .expect("the font is written");
    session
        .execute("doc.asset.add", &json!({ "path": "font.ttf", "kind": "font" }))
        .expect("the asset is added");

    let bundle = format!("{from}/doc.bundle");
    session
        .execute("doc.save.bundle", &json!({ "path": bundle, "from": from }))
        .expect("the bundle is written");
    let mut opened = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    opened
        .execute("doc.open.bundle", &json!({ "path": bundle, "into": into }))
        .expect("the bundle is opened");
    assert_eq!(opened.document().assets.len(), 1, "the asset came back");
    let font = std::fs::read(std::path::Path::new(&into).join("font.ttf")).expect("the font is there");
    assert_eq!(font, b"not really a font", "the bytes came back byte for byte");
}
