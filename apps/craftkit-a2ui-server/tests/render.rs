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

/// The same session, behind a lock.
fn session() -> Arc<Mutex<Session>> {
    Arc::new(Mutex::new(plain()))
}

/// A render over a session, and the queue of the messages that are not
/// replies.
fn render() -> Headless {
    Headless::new(session(), Arc::new(Mutex::new(Vec::new())))
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
                "components": [{
                    "component": "Button",
                    "child": "press",
                    "action": { "name": "required" },
                }],
            }),
        )
        .expect("the button is drawn");
    let mut render = Headless::new(Arc::new(Mutex::new(session)), Arc::new(Mutex::new(Vec::new())));
    // The button is at the surface's place, so the click is inside it.
    let answer = render.click(Point::new(1.0, 1.0)).expect("the click is answered");
    assert!(!answer["clicked"].is_null(), "the button was clicked: {answer}");
    assert_eq!(answer["message"], json!("the action `required` of the component 2 is the agent's"));
}

/// The click is a **command**, and the command's answer is the answer.
#[test]
fn the_click_is_a_command() {
    let mut session = plain();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s2" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s2",
                "components": [{
                    "component": "Button",
                    "child": "press",
                    "action": { "name": "somethingElse" },
                }],
            }),
        )
        .expect("the button is drawn");
    let mut render = Headless::new(Arc::new(Mutex::new(session)), Arc::new(Mutex::new(Vec::new())));
    // The action names a function in neither table, so the command
    // **refuses** it — and the refusal is the answer.
    let error = render.click(Point::new(1.0, 1.0)).expect_err("the action is refused");
    assert!(error.contains("neither"), "{error}");
}
