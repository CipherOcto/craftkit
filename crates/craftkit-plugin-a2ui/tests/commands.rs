//! The **five commands** — a surface is a command, and nothing else.
//!
//! The test says what the tree's rule is: every change is a command, a
//! component the layer does not know is **refused**, and a function name
//! in neither table is **refused** rather than forwarded.
//!
//! And it says what the **model** is: the surface's own, in the
//! document — so an undo takes it back, and two surfaces never share
//! one.

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

/// A surface, made.
fn surface(session: &mut Session, id: &str) {
    session.execute("a2ui.surface.create", &json!({ "surface_id": id })).expect("the surface is made");
}

/// A small **tree**, drawn onto a surface.
fn tree(session: &mut Session, id: &str) {
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": id,
                "components": [
                    { "component": "Column", "id": "root", "children": ["t1"] },
                    { "component": "Text", "id": "t1", "text": "hello" },
                ],
            }),
        )
        .expect("the tree is drawn");
}

/// The **model** of a surface, as the document holds it.
fn model_of(session: &Session, id: &str) -> craftkit_sdk::serde_json::Value {
    let document = session.document();
    let surface = document
        .walk()
        .find(|node| node.kind == "a2ui.surface" && node.name == id)
        .expect("the surface is there");
    surface.data.get("model").cloned().unwrap_or(json!({}))
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

/// A surface with **one name** is refused: the id is unique for the
/// renderer's lifetime.
#[test]
fn a_second_surface_with_the_same_name_is_refused() {
    let mut session = session();
    surface(&mut session, "s1");
    let error = session
        .execute("a2ui.surface.create", &json!({ "surface_id": "s1" }))
        .expect_err("the second surface is refused");
    assert!(error.to_string().contains("already"), "{error}");
}

/// A **component tree** is drawn onto a surface, and the nodes appear.
#[test]
fn a_component_tree_is_drawn() {
    let mut session = session();
    surface(&mut session, "s2");
    let outcome = session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s2",
                "components": [
                    { "component": "Column", "id": "root", "children": ["t1", "c1"] },
                    { "component": "Text", "id": "t1", "text": "hello" },
                    { "component": "Card", "id": "c1", "child": "t2" },
                    { "component": "Text", "id": "t2", "text": "inside" },
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

/// The children are **under** their parent: a card's child is in the
/// card, and not beside it.
#[test]
fn the_children_are_under_their_parent() {
    let mut session = session();
    surface(&mut session, "s2");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s2",
                "components": [
                    { "component": "Column", "id": "root", "children": ["c1"] },
                    { "component": "Card", "id": "c1", "child": "t2" },
                    { "component": "Text", "id": "t2", "text": "inside" },
                ],
            }),
        )
        .expect("the tree is drawn");
    let document = session.document();
    let card = document.walk().find(|node| node.name == "c1").expect("the card is there");
    assert_eq!(card.children.len(), 1, "the card holds one node");
    assert_eq!(card.children[0].name, "t2", "and it is the text");
}

/// The **places** are derived, and they are set — and no child overflows
/// the surface.
#[test]
fn the_places_are_derived_and_set() {
    let mut session = session();
    surface(&mut session, "s3");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s3",
                "components": [
                    { "component": "Row", "id": "root", "children": ["t1", "t2"] },
                    { "component": "Text", "id": "t1", "text": "one" },
                    { "component": "Text", "id": "t2", "text": "two" },
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
    let surface = document.walk().find(|node| node.kind == "a2ui.surface").expect("the surface is there");
    let box_ = surface.rect.expect("the surface has a place");
    assert!(second.x + second.w <= box_.x + box_.w + 0.001, "and the second fits the surface");
}

/// A tree that is sent **again** replaces the one that was there: the
/// surface holds one tree.
#[test]
fn a_tree_that_is_sent_again_replaces_the_one_that_was_there() {
    let mut session = session();
    surface(&mut session, "s3");
    let tree = json!([
        { "component": "Column", "id": "root", "children": ["t1"] },
        { "component": "Text", "id": "t1", "text": "one" },
    ]);
    session
        .execute("a2ui.surface.apply", &json!({ "surface_id": "s3", "components": tree }))
        .expect("the tree is drawn");
    session
        .execute("a2ui.surface.apply", &json!({ "surface_id": "s3", "components": tree }))
        .expect("the tree is drawn again");
    let document = session.document();
    let texts: Vec<&craftkit_sdk::Node> = document.walk().filter(|node| node.kind == "a2ui.text").collect();
    assert_eq!(texts.len(), 1, "and there is one text, not two");
}

/// A component the layer does not know is **refused**.
#[test]
fn an_unknown_component_is_refused() {
    let mut session = session();
    surface(&mut session, "s4");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s4",
            "components": [{ "component": "Video", "id": "root", "url": "https://example.com/v" }],
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

/// A value is **set**, and **mirrored** into craftkit — and it is in the
/// **document**, in the surface's own data.
#[test]
fn a_value_is_set_and_mirrored() {
    let mut session = session();
    surface(&mut session, "s5");
    let outcome = session
        .execute("a2ui.data.set", &json!({ "surface_id": "s5", "path": "/title", "value": "hello" }))
        .expect("the value is set");
    assert!(outcome.changed, "the document changed");
    let document = session.document();
    assert!(document.walk().any(|node| node.kind == "a2ui.data"), "the mirror is a node");
    assert_eq!(model_of(&session, "s5")["title"], json!("hello"), "and the model has it");
}

/// The model is the **surface's**: two surfaces never share one.
#[test]
fn the_model_is_the_surfaces_own() {
    let mut session = session();
    surface(&mut session, "s5");
    surface(&mut session, "s6");
    session
        .execute("a2ui.data.set", &json!({ "surface_id": "s5", "path": "/title", "value": "hello" }))
        .expect("the value is set");
    assert_eq!(model_of(&session, "s5")["title"], json!("hello"), "the first surface has it");
    assert!(model_of(&session, "s6").get("title").is_none(), "and the second has not");
}

/// A value that is set is **undone** with the document.
#[test]
fn a_value_that_is_set_is_undone() {
    let mut session = session();
    surface(&mut session, "s5");
    session
        .execute("a2ui.data.set", &json!({ "surface_id": "s5", "path": "/title", "value": "hello" }))
        .expect("the value is set");
    assert_eq!(model_of(&session, "s5")["title"], json!("hello"));
    session.undo().expect("the set is undone");
    assert!(model_of(&session, "s5").get("title").is_none(), "and the value is gone");
}

/// A path that is not a path is **refused**.
#[test]
fn a_path_that_is_not_a_path_is_refused() {
    let mut session = session();
    surface(&mut session, "s5");
    let error = session
        .execute("a2ui.data.set", &json!({ "surface_id": "s5", "path": "title", "value": "hello" }))
        .expect_err("the path is refused");
    assert!(error.to_string().contains("not a path"), "{error}");
}

/// An **unknown** function name is refused, not forwarded.
#[test]
fn an_unknown_function_name_is_refused() {
    let mut session = session();
    surface(&mut session, "s6");
    tree(&mut session, "s6");
    let outcome = session.execute(
        "a2ui.component.invoke",
        &json!({
            "surface_id": "s6",
            "component_id": "t1",
            "action": { "functionCall": { "@call": "somethingElse" } },
        }),
    );
    let error = outcome.expect_err("the action is refused");
    assert!(error.to_string().contains("neither"), "{error}");
}

/// An **agent-side** function is the agent's, and the layer says so.
#[test]
fn an_agent_side_function_is_the_agents() {
    let mut session = session();
    surface(&mut session, "s7");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s7",
                "components": [
                    { "component": "Column", "id": "root", "children": ["t1"] },
                    { "component": "Text", "id": "t1", "text": "hello" },
                ],
            }),
        )
        .expect("the tree is drawn");
    let outcome = session
        .execute(
            "a2ui.component.invoke",
            &json!({
                "surface_id": "s7",
                "component_id": "t1",
                "action": { "functionCall": { "@call": "required" } },
            }),
        )
        .expect("the action is the agent's");
    assert!(outcome.message.contains("the agent's"), "{}", outcome.message);
    assert!(
        outcome.data.get("callAgentFunction").is_some(),
        "and the door is told to call it: {:?}",
        outcome.data
    );
}

/// A **local** function runs — and its arguments are **translated** from
/// a2ui's names to craftkit's.
#[test]
fn a_local_function_runs_with_translated_arguments() {
    let mut session = session();
    surface(&mut session, "s7");
    tree(&mut session, "s7");
    session
        .execute(
            "a2ui.component.invoke",
            &json!({
                "surface_id": "s7",
                "component_id": "t1",
                "action": {
                    "functionCall": {
                        "@call": "dataSet",
                        "args": { "surfaceId": "s7", "path": "/title", "value": "hello" },
                    },
                },
            }),
        )
        .expect("the function ran");
    assert_eq!(model_of(&session, "s7")["title"], json!("hello"), "and the value is set");
}

/// The action and the command it runs are **one** undo step.
#[test]
fn an_action_and_its_command_are_one_undo_step() {
    let mut session = session();
    surface(&mut session, "s7");
    tree(&mut session, "s7");
    session
        .execute(
            "a2ui.component.invoke",
            &json!({
                "surface_id": "s7",
                "component_id": "t1",
                "action": {
                    "functionCall": {
                        "@call": "dataSet",
                        "args": { "surfaceId": "s7", "path": "/title", "value": "hello" },
                    },
                },
            }),
        )
        .expect("the function ran");
    session.undo().expect("the action is undone");
    assert!(model_of(&session, "s7").get("title").is_none(), "and one undo takes the whole action back");
}

/// An argument the table does not name is **refused**.
#[test]
fn an_argument_the_table_does_not_name_is_refused() {
    let mut session = session();
    surface(&mut session, "s7");
    tree(&mut session, "s7");
    let error = session
        .execute(
            "a2ui.component.invoke",
            &json!({
                "surface_id": "s7",
                "component_id": "t1",
                "action": {
                    "functionCall": {
                        "@call": "dataSet",
                        "args": { "surfaceId": "s7", "path": "/title", "value": "hello", "nope": 1 },
                    },
                },
            }),
        )
        .expect_err("the argument is refused");
    assert!(error.to_string().contains("nope"), "{error}");
}

/// An action that names **nothing** is refused.
#[test]
fn an_action_that_names_nothing_is_refused() {
    let mut session = session();
    surface(&mut session, "s8");
    let outcome = session.execute(
        "a2ui.component.invoke",
        &json!({
            "surface_id": "s8",
            "component_id": "t1",
            "action": { "userMessage": "hello" },
        }),
    );
    assert!(outcome.is_err(), "the action is refused");
}

/// An action of a component that is **not on the surface** is refused.
#[test]
fn an_action_of_a_component_that_is_not_there_is_refused() {
    let mut session = session();
    surface(&mut session, "s8");
    let error = session
        .execute(
            "a2ui.component.invoke",
            &json!({
                "surface_id": "s8",
                "component_id": "nope",
                "action": { "functionCall": { "@call": "required" } },
            }),
        )
        .expect_err("the action is refused");
    assert!(error.to_string().contains("nope"), "{error}");
}

/// A surface's nodes are **taken away** — and so is the surface.
#[test]
fn a_surface_is_deleted() {
    let mut session = session();
    surface(&mut session, "s9");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s9",
                "components": [
                    { "component": "Column", "id": "root", "children": ["t1"] },
                    { "component": "Text", "id": "t1", "text": "hello" },
                ],
            }),
        )
        .expect("the tree is drawn");
    session.execute("a2ui.surface.delete", &json!({ "surface_id": "s9" })).expect("the surface is deleted");
    let document = session.document();
    assert!(!document.walk().any(|node| node.kind == "a2ui.text"), "the text is gone");
    assert!(!document.walk().any(|node| node.kind == "a2ui.surface"), "and the surface is gone");
}

/// A surface apply is **undoable**: the step it makes is a step a person
/// can take back.
#[test]
fn a_surface_apply_is_undoable() {
    let mut session = session();
    surface(&mut session, "s10");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s10",
                "components": [
                    { "component": "Column", "id": "root", "children": ["t1"] },
                    { "component": "Text", "id": "t1", "text": "hello" },
                ],
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
    surface(&mut session, "s11");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s11",
            "components": [
                { "component": "Column", "id": "root", "children": ["t1"] },
                { "component": "Text", "id": "t1", "text": "hello" },
                { "component": "Video", "id": "v1", "url": "https://example.com/v" },
            ],
        }),
    );
    assert!(outcome.is_err(), "the apply is refused");
    let kinds: Vec<&str> = session.document().walk().map(|node| node.kind.as_str()).collect();
    assert!(!kinds.contains(&"a2ui.text"), "and the text was never made: {kinds:?}");
}

/// A component the layer **refuses** says why it is refused.
///
/// The refusal is not "craftkit cannot draw it" — a slider is two
/// rectangles. It is that the tree does not know what the component
/// **means**, and says so.
#[test]
fn a_refused_component_says_why() {
    let mut session = session();
    surface(&mut session, "s12");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s12",
            "components": [{ "component": "Slider", "id": "root", "value": 0.5 }],
        }),
    );
    let error = outcome.expect_err("the slider is refused");
    let message = error.to_string();
    assert!(message.contains("Slider"), "{message}");
    assert!(message.contains("refused"), "{message}");
    assert!(message.contains("widget"), "{message}");
}

/// A component with **no id** is refused: a container names its
/// children by id, and a component without one cannot be named.
#[test]
fn a_component_with_no_id_is_refused() {
    let mut session = session();
    surface(&mut session, "s13");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s13",
            "components": [{ "component": "Text", "text": "hello" }],
        }),
    );
    let error = outcome.expect_err("the component is refused");
    assert!(error.to_string().contains("no id"), "{error}");
}

/// A child that names **nothing** is refused.
#[test]
fn a_child_that_names_nothing_is_refused() {
    let mut session = session();
    surface(&mut session, "s13");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s13",
            "components": [{ "component": "Column", "id": "root", "children": ["nope"] }],
        }),
    );
    let error = outcome.expect_err("the child is refused");
    assert!(error.to_string().contains("nope"), "{error}");
}

/// A **malformed weight** is refused, and not ignored.
#[test]
fn a_malformed_weight_is_refused() {
    let mut session = session();
    surface(&mut session, "s13");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s13",
            "components": [
                { "component": "Column", "id": "root", "children": ["t1"] },
                { "component": "Text", "id": "t1", "text": "hello", "weight": "heavy" },
            ],
        }),
    );
    let error = outcome.expect_err("the weight is refused");
    assert!(error.to_string().contains("weight"), "{error}");
}

/// A **binding** to a path that is not a path is refused.
#[test]
fn a_binding_to_a_path_that_is_not_a_path_is_refused() {
    let mut session = session();
    surface(&mut session, "s13");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s13",
            "components": [
                { "component": "Column", "id": "root", "children": ["t1"] },
                { "component": "Text", "id": "t1", "text": { "@path": "title" } },
            ],
        }),
    );
    let error = outcome.expect_err("the binding is refused");
    assert!(error.to_string().contains("not a path"), "{error}");
}

/// A property a component does **not** have is refused: the published
/// schema says `additionalProperties: false`.
#[test]
fn a_property_the_component_does_not_have_is_refused() {
    let mut session = session();
    surface(&mut session, "s13");
    let outcome = session.execute(
        "a2ui.surface.apply",
        &json!({
            "surface_id": "s13",
            "components": [
                { "component": "Column", "id": "root", "children": ["t1"] },
                { "component": "Text", "id": "t1", "text": "hello", "nope": 1 },
            ],
        }),
    );
    let error = outcome.expect_err("the property is refused");
    assert!(error.to_string().contains("nope"), "{error}");
}
