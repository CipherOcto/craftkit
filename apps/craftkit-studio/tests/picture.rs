//! The **picture**: the kit's painting seam, at work.
//!
//! Every kind of node draws itself, and the picture is what it drew. The
//! application knows nothing about rectangles: the canvas is given the
//! drawing, and the drawing comes from the kind.

use craftkit_sdk::serde_json::json;

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-picture-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A snapshot is a **PNG file**.
#[test]
fn a_snapshot_is_a_png_file() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session
        .execute("node.add", &json!({ "name": "box", "kind": "geometry.rect" }))
        .expect("the node is added");
    let path = format!("{}/shot.png", directory("png"));
    session
        .execute("doc.snapshot", &json!({ "path": path, "width": 64, "height": 64 }))
        .expect("the picture is written");
    let bytes = std::fs::read(&path).expect("the file is there");
    assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10], "the signature of a PNG");
}

/// A snapshot does not change the document.
///
/// It is a picture of the document, and not an edit of it: there is no
/// undo step for a photograph.
#[test]
fn a_snapshot_does_not_change_the_document() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session
        .execute("node.add", &json!({ "name": "box", "kind": "geometry.rect" }))
        .expect("the node is added");
    let before = session.document().nodes.len();
    let path = format!("{}/shot.png", directory("unchanged"));
    session.execute("doc.snapshot", &json!({ "path": path })).expect("the picture is written");
    assert_eq!(session.document().nodes.len(), before, "the document changed");
    assert_eq!(session.history().len(), 1, "the snapshot made an undo step");
}

/// A snapshot draws the pictures in the document.
///
/// The bytes are in the file and the pixels are in the decoder, and the
/// picture lands where the node says it does.
#[cfg(feature = "image")]
#[test]
fn a_snapshot_draws_the_pictures_in_the_document() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let directory = directory("drawn");
    let picture = format!("{directory}/grey.jpg");
    std::fs::write(&picture, a_grey_jpeg()).expect("the picture is written");
    session
        .execute("image.add", &json!({ "path": picture, "rect": { "x": -5, "y": -5, "w": 10, "h": 10 } }))
        .expect("the picture is added");
    let with_picture = format!("{directory}/with.png");
    let outcome = session
        .execute("doc.snapshot", &json!({ "path": with_picture, "width": 64, "height": 64 }))
        .expect("the picture is written");
    assert_eq!(outcome.data.get("pictures").and_then(|pictures| pictures.as_u64()), Some(1));

    // The picture is drawn: the file is not the same as a snapshot of an
    // empty document.
    let mut empty = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let without = format!("{directory}/without.png");
    empty
        .execute("doc.snapshot", &json!({ "path": without, "width": 64, "height": 64 }))
        .expect("the picture is written");
    assert_ne!(
        std::fs::read(&with_picture).expect("the file is there"),
        std::fs::read(&without).expect("the file is there"),
        "the picture was not drawn"
    );
}

/// A snapshot draws a **PNG** too.
///
/// The PNG is the kit's own: it is written by the kit's writer and read
/// by the kit's reader, and the picture lands where the node says.
#[cfg(feature = "image")]
#[test]
fn a_snapshot_draws_a_png() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let directory = directory("drawn-png");
    let picture = format!("{directory}/block.png");
    let image = craftkit_raster::Image::new(8, 8, craftkit_sdk::Colour::BLACK);
    std::fs::write(&picture, image.to_png()).expect("the picture is written");
    session
        .execute("image.add", &json!({ "path": picture, "rect": { "x": -5, "y": -5, "w": 10, "h": 10 } }))
        .expect("the picture is added");
    let with_picture = format!("{directory}/with.png");
    let outcome = session
        .execute("doc.snapshot", &json!({ "path": with_picture, "width": 64, "height": 64 }))
        .expect("the picture is written");
    assert_eq!(outcome.data.get("pictures").and_then(|pictures| pictures.as_u64()), Some(1));
    assert_eq!(
        outcome.data.get("skipped").and_then(|skipped| skipped.as_array()).map(|skipped| skipped.len()),
        Some(0)
    );
}

/// A baseline JPEG of one grey block, written by hand.
///
/// The decoder is the family's own, and this is the smallest picture it
/// will take: one block, one coefficient, and the average is zero.
#[cfg(feature = "image")]
fn a_grey_jpeg() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xFF, 0xD8]);
    bytes.extend_from_slice(&[0xFF, 0xDB]);
    bytes.extend_from_slice(&67u16.to_be_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(&[16u8; 64]);
    bytes.extend_from_slice(&[0xFF, 0xC0]);
    bytes.extend_from_slice(&11u16.to_be_bytes());
    bytes.push(8);
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.push(1);
    bytes.push(1);
    bytes.push(0x11);
    bytes.push(0);
    bytes.extend_from_slice(&[0xFF, 0xC4]);
    bytes.extend_from_slice(&20u16.to_be_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.push(0);
    bytes.extend_from_slice(&[0xFF, 0xC4]);
    bytes.extend_from_slice(&20u16.to_be_bytes());
    bytes.push(0x10);
    bytes.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.push(0x00);
    bytes.extend_from_slice(&[0xFF, 0xDA]);
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.push(1);
    bytes.push(1);
    bytes.push(0x00);
    bytes.push(0);
    bytes.push(63);
    bytes.push(0);
    bytes.push(0x0F);
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// A picture that is too big is refused, and nothing is written.
#[test]
fn a_picture_that_is_too_big_is_refused() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path = format!("{}/huge.png", directory("huge"));
    let error = session
        .execute("doc.snapshot", &json!({ "path": path, "width": 100_000, "height": 100_000 }))
        .expect_err("that picture is too big");
    assert!(error.to_string().contains("side"), "{error}");
    assert!(!std::path::Path::new(&path).exists(), "the picture was written anyway");
}

/// The snapshot shows **all** of the document.
///
/// The camera is worked out from the bounds of the document and the
/// size that was asked for: a document that is a thousand units from
/// the origin is not in a picture of the origin.
///
/// The rectangle is a **vector** one, because a white rectangle on a
/// white background is a rectangle nobody sees — and a picture that
/// shows nothing is a picture this test cannot tell from an empty one.
#[cfg(feature = "vector")]
#[test]
fn the_snapshot_shows_all_of_the_document() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session
        .execute(
            "vector.rectangle",
            &json!({ "x": 1000.0, "y": 1000.0, "width": 10.0, "height": 10.0, "fill": "#3366cc" }),
        )
        .expect("the rectangle is added");
    let path = format!("{}/far.png", directory("far"));
    session
        .execute("doc.snapshot", &json!({ "path": path, "width": 64, "height": 64 }))
        .expect("the picture is written");

    // The picture is not empty: the rectangle is in it, even though it
    // is far from where the camera was pointing.
    let bytes = std::fs::read(&path).expect("the file is there");
    let mut empty = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let without = format!("{}/empty.png", directory("far-empty"));
    empty
        .execute("doc.snapshot", &json!({ "path": without, "width": 64, "height": 64 }))
        .expect("the picture is written");
    assert_ne!(bytes, std::fs::read(&without).expect("the file is there"), "the rectangle was not drawn");
}

/// And the session's own camera is **not** moved.
///
/// A picture made for somebody else must not move what somebody else
/// is looking through.
#[test]
fn the_snapshot_does_not_move_the_camera() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session
        .execute("geometry.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session.camera_mut().set_zoom(3.0);
    let path = format!("{}/camera.png", directory("camera"));
    session
        .execute("doc.snapshot", &json!({ "path": path, "width": 64, "height": 64 }))
        .expect("the picture is written");
    assert_eq!(session.camera().zoom(), 3.0, "the camera did not move");
}

/// A **hidden** node is not in the snapshot.
///
/// The kit draws, and the kit knows what is hidden.
#[cfg(feature = "vector")]
#[test]
fn a_hidden_node_is_not_in_the_snapshot() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session
        .execute(
            "vector.rectangle",
            &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0, "fill": "#3366cc" }),
        )
        .expect("the rectangle is added");
    let shown = format!("{}/shown.png", directory("shown"));
    session
        .execute("doc.snapshot", &json!({ "path": shown, "width": 64, "height": 64 }))
        .expect("the picture is written");

    session.execute("node.hide", &json!({ "node": 1 })).expect("the node is hidden");
    let hidden = format!("{}/hidden.png", directory("hidden"));
    session
        .execute("doc.snapshot", &json!({ "path": hidden, "width": 64, "height": 64 }))
        .expect("the picture is written");

    assert_ne!(
        std::fs::read(&shown).expect("the file is there"),
        std::fs::read(&hidden).expect("the file is there"),
        "the hidden node was drawn"
    );
}
