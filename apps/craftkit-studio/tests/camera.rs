//! The camera: one set of controls, and its own caps.
//!
//! Every application with a canvas has a zoom and a scroll. The tree
//! this replaces has a rule about them — one module that owns them, with
//! its own caps — and v2 had no camera at all.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The session owns a camera.
#[test]
fn the_session_owns_a_camera() {
    let session = ecosystem().session();
    assert_eq!(session.camera().zoom(), 1.0, "the camera starts at the start");
    assert_eq!(session.camera().offset().x, 0.0);
}

/// The camera is the session's state, and a command does not move it.
#[test]
fn a_command_does_not_move_the_camera() {
    let mut session = ecosystem().session();
    session.camera_mut().set_zoom(2.0);
    session
        .execute("node.add", &json!({ "name": "rect", "kind": "geometry.rect" }))
        .expect("the node is added");
    assert_eq!(session.camera().zoom(), 2.0, "the camera did not move");
}

/// The undo does not move the camera either.
///
/// Scrolling is not an edit, and an undo that scrolls the view is a
/// surprise.
#[test]
fn the_undo_does_not_move_the_camera() {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "rect", "kind": "geometry.rect" }))
        .expect("the node is added");
    session.camera_mut().set_zoom(4.0);
    session.camera_mut().pan(10.0, 0.0);
    session.undo().expect("the node is undone");
    assert_eq!(session.camera().zoom(), 4.0, "the camera did not move");
    assert_eq!(session.camera().offset().x, 10.0, "the camera did not move");
}

/// The caps hold, whatever the wheel says.
#[test]
fn the_caps_hold() {
    let mut session = ecosystem().session();
    for _ in 0..1000 {
        session.camera_mut().zoom_by(10.0);
    }
    assert_eq!(session.camera().zoom(), craftkit_sdk::Camera::MAX_ZOOM, "the zoom is capped");
    for _ in 0..1000 {
        session.camera_mut().zoom_by(0.1);
    }
    assert_eq!(session.camera().zoom(), craftkit_sdk::Camera::MIN_ZOOM, "the zoom is capped");
}
