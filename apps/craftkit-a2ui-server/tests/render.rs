//! The **headless** render — a picture of the document, and a click.
//!
//! The click is turned into a **command** — `a2ui.component.invoke` —
//! and not into a document edit: the house rule is that every change is
//! a command, and that a front end has no second path.

use std::sync::{Arc, Mutex};

use craftkit_a2ui_server::render::Headless;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Point, Session};

// The plugin's anchor, held by the test: without it the linker drops the
// plugin's items and the registry is empty.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_a2ui::CRAFTKIT_PLUGIN_ANCHOR;

/// A session over the plugin's ecosystem.
fn plain() -> Session {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    ecosystem.session()
}

/// The queue of the messages that are not replies.
fn queue() -> std::sync::mpsc::Sender<Option<craftkit_sdk::serde_json::Value>> {
    let (sender, _reader) = std::sync::mpsc::channel();
    sender
}

/// The same session, behind a lock.
fn session() -> Arc<Mutex<Session>> {
    Arc::new(Mutex::new(plain()))
}

/// A render over a session, and the queue of the messages that are not
/// replies.
fn render() -> Headless {
    Headless::new(session(), queue())
}

/// The **picture** is a PNG, and it is the tree's own.
#[test]
fn the_picture_is_a_png() {
    let mut render = render();
    let picture = render.render().expect("the picture is drawn");
    assert!(picture.len() > 8, "the picture holds bytes");
    // The PNG's own signature.
    assert_eq!(&picture[0..4], &[0x89, b'P', b'N', b'G'], "and it is a PNG");
}

/// A click on **nothing** says nothing.
#[test]
fn a_click_on_nothing_says_nothing() {
    let mut render = render();
    let answer = render.click(Point::new(-100.0, -100.0)).expect("the click is answered");
    assert!(answer["clicked"].is_null(), "{answer}");
}

/// A click on a **component** runs its action, through a command.
#[test]
fn a_click_on_a_component_runs_its_action() {
    let mut session = plain();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s1" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s1",
                "components": [
                    { "component": "Column", "id": "root", "children": ["b1"] },
                    {
                        "component": "Button",
                        "id": "b1",
                        "child": "t1",
                        "action": { "event": { "name": "required" } },
                    },
                    { "component": "Text", "id": "t1", "text": "press" },
                ],
            }),
        )
        .expect("the button is drawn");
    let mut render = Headless::new(Arc::new(Mutex::new(session)), queue());
    // The button is at the surface's place, so the click is inside it.
    let answer = render.click(Point::new(1.0, 1.0)).expect("the click is answered");
    assert!(!answer["clicked"].is_null(), "the button was clicked: {answer}");
    assert_eq!(answer["message"], json!("the action of the component b1 is the agent's"));
}

/// The click is a **command**, and the command's answer is the answer.
///
/// And a button whose action names a function in **neither** table is
/// refused when the tree is drawn: a button that does nothing is a
/// button the tree does not show.
#[test]
fn the_click_is_a_command() {
    let mut session = plain();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s2" })).expect("the surface is made");
    let error = session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s2",
                "components": [
                    { "component": "Column", "id": "root", "children": ["b1"] },
                    {
                        "component": "Button",
                        "id": "b1",
                        "child": "t1",
                        "action": { "functionCall": { "@call": "somethingElse" } },
                    },
                    { "component": "Text", "id": "t1", "text": "press" },
                ],
            }),
        )
        .expect_err("the button is refused");
    assert!(error.to_string().contains("neither"), "{error}");
}
