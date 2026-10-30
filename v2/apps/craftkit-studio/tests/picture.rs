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
