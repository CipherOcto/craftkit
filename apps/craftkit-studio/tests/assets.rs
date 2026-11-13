//! What a document that points at files needs, and what travels with it.
//!
//! The assets are the file list of the document, and the bundle is the
//! container that carries the document and the files it names.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;

/// The ecosystem of this program.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

// ------------------------------------------------------------------ the assets --

/// The document says which files it needs.
///
/// A document that points at a file is not finished, and the place it
/// says so is one list and not every payload that happens to hold a path.
#[test]
fn the_document_says_what_it_needs() {
    let mut session = ecosystem().session();
    session
        .execute("doc.asset.add", &json!({ "path": "sounds/kick.wav", "kind": "audio" }))
        .expect("the asset is added");
    let assets = &session.document().assets;
    assert_eq!(assets.len(), 1, "the document needs the file: {assets:?}");
    assert_eq!(assets[0].path, "sounds/kick.wav");
    assert_eq!(assets[0].kind, "audio");
    assert_eq!(assets[0].length, None, "nobody has read the file");
}

/// The same file twice is the same file.
#[test]
fn the_same_file_twice_is_refused() {
    let mut session = ecosystem().session();
    session.execute("doc.asset.add", &json!({ "path": "sounds/kick.wav" })).expect("the asset is added");
    let error = session
        .execute("doc.asset.add", &json!({ "path": "sounds/kick.wav" }))
        .expect_err("the second one is refused");
    assert!(error.to_string().contains("already needs"), "{error}");
}

/// The assets survive the file.
#[test]
fn the_assets_survive_the_file() {
    let ecosystem = ecosystem();
    let mut document = craftkit_sdk::Document::new("with media");
    document.add_asset(craftkit_sdk::Asset::new("sounds/kick.wav", "audio", Some(1024)));
    let format =
        craftkit_sdk::format::newest_format(&ecosystem.formats, "craftkit").expect("there is a format");
    let value = craftkit_sdk::format::write_document(&document, format);
    let read = craftkit_sdk::format::read_document(&value, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read");
    assert_eq!(read.assets, document.assets, "the assets survive the file");
}

/// A document knows what it is missing.
#[test]
fn a_document_knows_what_it_is_missing() {
    let mut document = craftkit_sdk::Document::new("missing");
    document.add_asset(craftkit_sdk::Asset::new("sounds/kick.wav", "audio", None));
    document.add_asset(craftkit_sdk::Asset::new("sounds/snare.wav", "audio", None));
    assert_eq!(
        document.missing_assets(&["sounds/kick.wav".to_string()]),
        vec!["sounds/snare.wav".to_string()],
        "the one that is there is not missing"
    );
    assert!(
        document.missing_assets(&["sounds/kick.wav".to_string(), "sounds/snare.wav".to_string()]).is_empty()
    );
}

/// A bundle carries the document and the files it needs.
///
/// One file, so that a document and its media travel together, and a
/// checksum, so that a bundle that was damaged on the way is refused
/// rather than half-read.
#[test]
fn a_bundle_carries_the_document_and_its_files() {
    let ecosystem = ecosystem();
    let mut document = craftkit_sdk::Document::new("bundled");
    document.add_node("rect", "geometry.rect", None);
    document.add_asset(craftkit_sdk::Asset::new("sounds/kick.wav", "audio", Some(3)));
    let bytes = craftkit_sdk::bundle::write(
        &document,
        vec![craftkit_sdk::File::new("sounds/kick.wav", vec![1, 2, 3])],
        &ecosystem.formats,
        &ecosystem.migrations,
    )
    .expect("the bundle is written");
    let bundle =
        craftkit_sdk::bundle::read(&bytes, &ecosystem.formats, &ecosystem.migrations).expect("it is read");
    assert_eq!(bundle.document, document, "the document came back");
    assert_eq!(bundle.files[0].path, "sounds/kick.wav");
    assert_eq!(bundle.files[0].bytes, vec![1, 2, 3]);
    assert!(bundle.missing().is_empty(), "nothing is missing: {:?}", bundle.missing());
}

/// A bundle that was damaged is refused.
#[test]
fn a_damaged_bundle_is_refused() {
    let ecosystem = ecosystem();
    let document = craftkit_sdk::Document::new("bundled");
    let mut bytes = craftkit_sdk::bundle::write(&document, vec![], &ecosystem.formats, &ecosystem.migrations)
        .expect("the bundle is written");
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    let error = craftkit_sdk::bundle::read(&bytes, &ecosystem.formats, &ecosystem.migrations)
        .expect_err("it is refused");
    assert!(error.contains("damaged"), "{error}");
}

/// A **document file** that is bigger than the program allows is
/// refused — and it is refused **before** it is read, so the memory is
/// never spent.
#[test]
fn a_document_file_that_is_too_big_is_refused() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path = craftkit_scratch::file("craftkit-too-big.craftkit").expect("the scratch is made");
    let file = std::fs::File::create(&path).expect("the file is made");
    // Seventeen megabytes: more than this program's limit.
    let block = vec![b'x'; 1024 * 1024];
    let mut writer = std::io::BufWriter::new(file);
    for _ in 0..17 {
        std::io::Write::write_all(&mut writer, &block).expect("the bytes are written");
    }
    std::io::Write::flush(&mut writer).expect("the bytes are flushed");
    drop(writer);
    let error = session
        .execute("doc.open", &craftkit_sdk::serde_json::json!({ "path": path.to_string_lossy() }))
        .expect_err("that file is too big");
    assert!(error.to_string().contains("limit"), "the refusal says the limit: {error}");
    let _ = std::fs::remove_file(&path);
}
