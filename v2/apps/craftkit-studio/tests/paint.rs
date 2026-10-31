//! The **kit** draws the document.
//!
//! A front end that drew the document itself would have a second rule
//! for what is seen, and the second rule would drift from the first.
//! So the session draws: every node that is not hidden is offered to
//! the kind that says what it is, and the handle of the selected node
//! is drawn on top.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, RecordingCanvas, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with one rectangle in it.
fn a_rectangle() -> Session {
    let mut session = ecosystem().session();
    session
        .execute("geometry.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
}

/// The document is drawn, **through the kinds**.
#[test]
fn the_document_is_drawn_through_the_kinds() {
    let session = a_rectangle();
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert!(!canvas.drawn.is_empty(), "the rectangle was drawn");
}

/// The **handle** of the selected node is drawn.
///
/// A resize handle that nobody can see is a handle nobody can grab.
#[test]
fn the_handle_of_the_selected_node_is_drawn() {
    let mut session = a_rectangle();
    session.execute("select.node", &json!({ "node": 1 })).expect("the node is selected");

    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    // The handle is the corner opposite the anchor, and it is a screen
    // size: eight pixels across, whatever the zoom.
    assert!(
        canvas.drawn.iter().any(|drawn| drawn == "rect(6, -4, 8, 8)"),
        "the handle is drawn at the corner: {:?}",
        canvas.drawn
    );
}

/// And **no** handle is drawn when nothing is selected.
#[test]
fn no_handle_is_drawn_when_nothing_is_selected() {
    let session = a_rectangle();
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert!(
        !canvas.drawn.iter().any(|drawn| drawn == "rect(6, -4, 8, 8)"),
        "there is no handle: {:?}",
        canvas.drawn
    );
}

/// A **hidden** node is not drawn.
#[test]
fn a_hidden_node_is_not_drawn() {
    let mut session = a_rectangle();
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    let before = canvas.drawn.len();

    session.execute("node.hide", &json!({ "node": 1 })).expect("the node is hidden");
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert_eq!(canvas.drawn.len(), 0, "nothing is drawn: {:?}", canvas.drawn);
    assert!(before > 0, "and something was drawn before");
}
