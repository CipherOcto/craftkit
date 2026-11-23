//! The **five commands** — a surface is a command, and nothing else.
//!
//! The test says what the tree's rule is: every change is a command, a
//! component the layer does not know is **refused**, and a function name
//! in neither table is **refused** rather than forwarded.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

// The plugin's anchor, held by the test: without it the linker drops the
// plugin's items and the registry is empty.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_a2ui::CRAFTKIT_PLUGIN_ANCHOR;

/// A session over the plugin's ecosystem.
fn session() -> Session {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    ecosystem.session()
}

/// The **five** commands are in the registry.
#[test]
fn the_five_commands_are_in_the_registry() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    for command in [
        "a2ui.surface.create",
        "a2ui.surface.apply",
        "a2ui.surface.delete",
        "a2ui.data.set",
        "a2ui.component.invoke",
    ] {
        assert!(ecosystem.commands.all().iter().any(|entry| entry.id == command), "{command} is a command");
    }
}

/// A surface is made, and it is a **node**.
#[test]
fn a_surface_is_made() {
    let mut session = session();
    let outcome =
        session.execute("a2ui.surface.create", &json!({ "surface_id": "s1" })).expect("the surface is made");
    assert!(outcome.changed, "the document changed");
    let document = session.document();
    assert!(
        document.nodes().any(|node| node.kind == "a2ui.surface" && node.name == "s1"),
        "and the surface is there"
    );
}

/// A **component tree** is drawn onto a surface, and the nodes appear.
#[test]
fn a_component_tree_is_drawn() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s2" })).expect("the surface is made");
    let outcome = session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s2",
                "components": [
                    { "component": "Text", "text": "hello" },
                    { "component": "Card", "child": "a2ui.text" },
                ],
            }),
        )
        .expect("the tree is drawn");
    assert!(outcome.changed, "the document changed");
    let document = session.document();
    let kinds: Vec<&str> = document.walk().map(|node| node.kind.as_str()).collect();
    assert!(kinds.contains(&"a2ui.text"), "the text is there: {kinds:?}");
    assert!(kinds.contains(&"a2ui.card"), "and the card: {kinds:?}");
}

/// The **places** are derived, and they are set.
#[test]
fn the_places_are_derived_and_set() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s3" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s3",
                "components": [
                    { "component": "Text", "text": "one" },
                    { "component": "Text", "text": "two" },
                ],
            }),
        )
        .expect("the tree is drawn");
    let document = session.document();
    let texts: Vec<&craftkit_sdk::Node> = document.walk().filter(|node| node.kind == "a2ui.text").collect();
    assert_eq!(texts.len(), 2, "there are two texts");
    let first = texts[0].rect.expect("the first has a place");
    let second = texts[1].rect.expect("the second has a place");
    assert!(second.x >= first.x, "and they are laid out in order");
}

/// A component the layer does not know is **refused**.
#[test]
fn an_unknown_component_is_refused() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s4" })).expect("the surface is made");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s4",
            "components": [{ "component": "Video", "url": "https://example.com/v" }],
        }),
    );
    let error = outcome.expect_err("the component is refused");
    assert!(error.to_string().contains("Video"), "{error}");
}

/// A surface that is not there is **refused**.
#[test]
fn a_surface_that_is_not_there_is_refused() {
    let mut session = session();
    let outcome = session
        .execute("a2ui.surface.apply", &json!({ "surface_id": "nope", "components": [] }))
        .expect_err("the surface is refused");
    assert!(outcome.to_string().contains("nope"), "{outcome}");
}

/// A value is **set**, and **mirrored** into craftkit.
#[test]
fn a_value_is_set_and_mirrored() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s5" })).expect("the surface is made");
    let outcome = session
        .execute("a2ui.data.set", &json!({ "surface_id": "s5", "path": "/title", "value": "hello" }))
        .expect("the value is set");
    assert!(outcome.changed, "the document changed");
    let document = session.document();
    assert!(document.nodes().any(|node| node.kind == "a2ui.data"), "the mirror is a node");
    // And the model holds it.
    let model = craftkit_plugin_a2ui::model::model();
    assert_eq!(model["title"], json!("hello"), "and the model has it");
}

/// An **unknown** function name is refused, not forwarded.
#[test]
fn an_unknown_function_name_is_refused() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s6" })).expect("the surface is made");
    let outcome = session.execute(
        "a2ui.component.invoke",
        &json!({
            "surface_id": "s6",
            "component_id": "a2ui.text",
            "action": { "name": "somethingElse" },
        }),
    );
    let error = outcome.expect_err("the action is refused");
    assert!(error.to_string().contains("neither"), "{error}");
}

/// An **agent-side** function is the agent's, and the layer says so.
#[test]
fn an_agent_side_function_is_the_agents() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s7" })).expect("the surface is made");
    let outcome = session
        .execute(
            "a2ui.component.invoke",
            &json!({
                "surface_id": "s7",
                "component_id": "a2ui.text",
                "action": { "name": "required" },
            }),
        )
        .expect("the action is the agent's");
    assert!(outcome.message.contains("the agent's"), "{}", outcome.message);
}

/// An action that names **nothing** is refused.
#[test]
fn an_action_that_names_nothing_is_refused() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s8" })).expect("the surface is made");
    let outcome = session.execute(
        "a2ui.component.invoke",
        &json!({
            "surface_id": "s8",
            "component_id": "a2ui.text",
            "action": { "userMessage": "hello" },
        }),
    );
    assert!(outcome.is_err(), "the action is refused");
}

/// A surface's nodes are **taken away**.
#[test]
fn a_surface_is_deleted() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s9" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s9",
                "components": [{ "component": "Text", "text": "hello" }],
            }),
        )
        .expect("the tree is drawn");
    session.execute("a2ui.surface.delete", &json!({ "surface_id": "s9" })).expect("the surface is deleted");
    let document = session.document();
    assert!(!document.walk().any(|node| node.kind == "a2ui.text"), "the text is gone");
}

/// A surface apply is **undoable**: the step it makes is a step a person
/// can take back.
#[test]
fn a_surface_apply_is_undoable() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s10" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s10",
                "components": [{ "component": "Text", "text": "hello" }],
            }),
        )
        .expect("the tree is drawn");
    assert!(session.document().walk().any(|node| node.kind == "a2ui.text"), "the text is there");
    session.undo().expect("the apply is undone");
    assert!(!session.document().walk().any(|node| node.kind == "a2ui.text"), "and the text is gone");
}

/// An apply that **fails** leaves the document as it was: the tree is
/// checked before it is changed, and a command that did not happen
/// changed nothing.
#[test]
fn a_failing_apply_leaves_the_document_as_it_was() {
    let mut session = session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s11" })).expect("the surface is made");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s11",
            "components": [
                { "component": "Text", "text": "hello" },
                { "component": "Video", "url": "https://example.com/v" },
            ],
        }),
    );
    assert!(outcome.is_err(), "the apply is refused");
    let kinds: Vec<&str> = session.document().walk().map(|node| node.kind.as_str()).collect();
    assert!(!kinds.contains(&"a2ui.text"), "and the text was never made: {kinds:?}");
}
