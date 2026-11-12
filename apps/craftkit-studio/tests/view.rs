//! The **view state**: the camera's place, written down or not.
//!
//! The camera is the session's state and the undo does not move it. A
//! document may still remember where the view was, and two commands do
//! the remembering: `view.save` writes it down and `view.restore` moves
//! the camera back to it.

use craftkit_sdk::serde_json::json;

/// The camera is the **session's** state: the undo does not move it.
#[test]
fn the_undo_does_not_move_the_camera() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.camera_mut().set_zoom(2.0);
    assert_eq!(session.camera().zoom(), 2.0);
    // Nothing changed the document, so there is nothing to undo, and the
    // camera is where it was put.
    let _ = session.undo();
    assert_eq!(session.camera().zoom(), 2.0, "the camera did not move");
}

/// The view is written into the document, under the SDK's own key.
#[test]
fn the_view_is_written_down() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.camera_mut().set_zoom(4.0);
    session.camera_mut().look_at(craftkit_sdk::Point::new(10.0, -5.0));
    session.execute("view.save", &json!({})).expect("the view is saved");
    let view = &session.document().data["craftkit/view"];
    assert_eq!(view["zoom"], json!(4.0));
    assert_eq!(view["offset"]["x"], json!(10.0));
    assert_eq!(view["offset"]["y"], json!(-5.0));
}

/// And it comes back: a document that says where the view was opens
/// where it was left.
#[test]
fn the_view_comes_back() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.camera_mut().set_zoom(4.0);
    session.camera_mut().look_at(craftkit_sdk::Point::new(10.0, -5.0));
    session.execute("view.save", &json!({})).expect("the view is saved");
    // The camera moves away, and then back.
    session.camera_mut().set_zoom(1.0);
    session.camera_mut().look_at(craftkit_sdk::Point::new(0.0, 0.0));
    session.execute("view.restore", &json!({})).expect("the view is restored");
    assert_eq!(session.camera().zoom(), 4.0, "the zoom came back");
    assert_eq!(session.camera().offset().x, 10.0, "the view came back");
    assert_eq!(session.camera().offset().y, -5.0);
}

/// A document that says nothing about the view says so when asked.
#[test]
fn a_document_that_says_nothing_about_the_view_says_so() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let error = session.execute("view.restore", &json!({})).expect_err("there is no view");
    assert!(error.to_string().contains("does not say where the view was"), "{error}");
}

/// The undo takes the **saved** view back, because saving changes the
/// document — and it does not take the **camera** back, because the
/// camera is not the document's.
#[test]
fn the_undo_takes_the_saved_view_back_and_not_the_camera() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("view.save", &json!({})).expect("the view is saved");
    assert!(session.document().data["craftkit/view"].is_object(), "the view is in the document");
    session.undo().expect("the undo works");
    assert!(session.document().data["craftkit/view"].is_null(), "the view is gone from the document");
}

/// The view is written under a name of the SDK's own, so that a plugin
/// that keeps a view of its own does not collide with it.
#[test]
fn the_view_is_written_under_a_name_of_the_sdks_own() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("view.save", &json!({})).expect("the view is saved");
    let keys: Vec<&String> =
        session.document().data.as_object().map(|data| data.keys().collect()).unwrap_or_default();
    assert_eq!(keys, vec!["craftkit/view"], "the SDK writes one key");
}
