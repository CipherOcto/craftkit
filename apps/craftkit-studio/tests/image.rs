//! The **picture**: a size and a file, and never the pixels.
//!
//! The family keeps a picture's bytes beside the document and writes
//! them into the package on save. So the document holds two numbers and
//! a name — and the file holds the rest.

use craftkit_sdk::serde_json::json;

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-image-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A PNG file of that size, written by the kit's own writer.
fn a_png(path: &str, width: u32, height: u32) -> String {
    let image = craftkit_raster::Image::new(width as usize, height as usize, craftkit_sdk::Colour::WHITE);
    std::fs::write(path, image.to_png()).expect("the picture is written");
    path.to_string()
}

/// A picture in the document holds its **size** and its **file**.
///
/// The pixels are in neither: the node's payload says how big the
/// picture is, and the node's stream says where the bytes are.
#[cfg(feature = "image")]
#[test]
fn a_picture_holds_its_size_and_its_file() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path = a_png(&format!("{}/shot.png", directory("size")), 64, 32);
    session.execute("image.add", &json!({ "path": path })).expect("the picture is added");

    let node = session.document().nodes.first().expect("there is a node");
    assert_eq!(node.kind, "image.picture");
    assert_eq!(node.data.get("width").and_then(|width| width.as_u64()), Some(64));
    assert_eq!(node.data.get("height").and_then(|height| height.as_u64()), Some(32));
    assert_eq!(node.data.get("format").and_then(|format| format.as_str()), Some("png"));
    assert!(node.stream.is_some(), "the picture has no file");
}

/// The aspect ratio and the pixel count are **derived**.
///
/// They are worked out from the two numbers, and they are never stored:
/// the payload says how big the picture is and nothing else.
#[cfg(feature = "image")]
#[test]
fn the_aspect_ratio_is_derived() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path = a_png(&format!("{}/shot.png", directory("aspect")), 64, 32);
    session.execute("image.add", &json!({ "path": path })).expect("the picture is added");
    let plan = session.evaluate();
    let item = plan.item(1).expect("the picture is in the plan");
    assert_eq!(item.value.get("aspect").and_then(|aspect| aspect.as_f64()), Some(2.0));
    assert_eq!(item.value.get("pixels").and_then(|pixels| pixels.as_u64()), Some(2048));
}

/// A file that is not a picture is refused.
#[cfg(feature = "image")]
#[test]
fn a_file_that_is_not_a_picture_is_refused() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path = format!("{}/notes.txt", directory("not-a-picture"));
    std::fs::write(&path, b"this is not a picture").expect("the file is written");
    let error = session.execute("image.add", &json!({ "path": path })).expect_err("that is not a picture");
    assert!(error.to_string().contains("picture"), "{error}");
    assert!(session.document().nodes.is_empty(), "the node was added anyway");
}

/// A measurement changes nothing.
///
/// It is not an undo step: a measurement is not an edit.
#[cfg(feature = "image")]
#[test]
fn a_measurement_changes_nothing() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path = a_png(&format!("{}/shot.png", directory("measure")), 8, 8);
    let outcome = session.execute("image.measure", &json!({ "path": path })).expect("it is measured");
    assert_eq!(outcome.data.get("width").and_then(|width| width.as_u64()), Some(8));
    assert!(session.document().nodes.is_empty(), "the measurement added a node");
    assert_eq!(session.history().len(), 0, "the measurement made an undo step");
}
