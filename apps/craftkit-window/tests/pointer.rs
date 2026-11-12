//! The **pointer**: what a hand is doing to the document.
//!
//! A drag that starts on a **handle** resizes the node, a drag that
//! starts on a **node** moves it, and a drag that starts on the
//! **background** moves the camera. What is under the pointer says
//! which of them it is.

use craftkit_sdk::serde_json::json;
use craftkit_window::Window;

/// A window over the kit.
fn window() -> Window {
    Window::new().expect("the ecosystem is built")
}

/// A window with one rectangle in it.
fn a_rectangle() -> Window {
    let mut window = window();
    window.open("geometry.rectangle");
    window.set("x", json!(0.0));
    window.set("y", json!(0.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.run().expect("the rectangle is added");
    window
}

/// A drag on the node **moves** it.
#[test]
fn a_drag_on_the_node_moves_it() {
    let mut window = a_rectangle();
    window.pointer_down(craftkit_sdk::Point::new(5.0, 5.0));
    window.pointer_up(craftkit_sdk::Point::new(15.0, 25.0));

    let session = window.session();
    let rect = session.document().node(1).and_then(|node| node.rect).expect("the node has a place");
    assert_eq!(rect.x, 10.0, "the node moved across");
    assert_eq!(rect.y, 20.0, "and up");
}

/// A drag on the **handle** resizes it.
///
/// The handle is the corner opposite the anchor, and the pointer is
/// within **half** of it: a tolerance of the whole handle would make
/// the handle reach twice as far as it is drawn.
#[test]
fn a_drag_on_the_handle_resizes_it() {
    let mut window = a_rectangle();
    window.pointer_down(craftkit_sdk::Point::new(10.0, 0.0));
    window.pointer_up(craftkit_sdk::Point::new(20.0, 5.0));

    let session = window.session();
    let rect = session.document().node(1).and_then(|node| node.rect).expect("the node has a place");
    assert_eq!(rect.x, 0.0, "the node did not move across");
    assert_eq!(rect.y, 0.0, "and it did not move up");
    assert_eq!(rect.w, 20.0, "the width followed the handle");
    assert_eq!(rect.h, 5.0, "and so did the height");
}

/// A drag on the **background** moves the camera.
///
/// The world slides under the hand, and the node stays where it is.
#[test]
fn a_drag_on_the_background_moves_the_camera() {
    let mut window = a_rectangle();
    window.pointer_down(craftkit_sdk::Point::new(100.0, 100.0));
    window.pointer_dragged(craftkit_sdk::Point::new(90.0, 95.0));
    window.pointer_up(craftkit_sdk::Point::new(90.0, 95.0));

    let session = window.session();
    let offset = session.camera().offset();
    assert_eq!(offset.x, 10.0, "the camera moved across");
    assert_eq!(offset.y, 5.0, "and up");
    let rect = session.document().node(1).and_then(|node| node.rect).expect("the node has a place");
    assert_eq!(rect.x, 0.0, "and the node did not move");
}

/// A drag on a node that is **selected** moves the selection.
#[test]
fn a_drag_on_a_selected_node_moves_the_selection() {
    let mut window = a_rectangle();
    window.open("geometry.rectangle");
    window.set("x", json!(0.0));
    window.set("y", json!(0.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.run().expect("the second rectangle is added");
    window.run_id("select.all", json!({})).expect("everything is selected");

    window.pointer_down(craftkit_sdk::Point::new(5.0, 5.0));
    window.pointer_up(craftkit_sdk::Point::new(15.0, 25.0));

    let session = window.session();
    for id in [1, 2] {
        let rect = session.document().node(id).and_then(|node| node.rect).expect("the node has a place");
        assert_eq!(rect.x, 10.0, "the node moved across");
        assert_eq!(rect.y, 20.0, "and up");
    }
}
