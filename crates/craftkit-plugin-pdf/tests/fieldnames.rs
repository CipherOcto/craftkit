//! The **name** of a field with a parent.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
}

/// The **node** of an object, by its number.
fn node_of(session: &Session, object: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| node.id)
        .expect("the object is there")
}

/// The **fields** of the document, as the command reads them.
fn fields_of(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("pdf.field.read", &json!({})).expect("the fields are read");
    outcome.data["fields"].as_array().expect("the fields are a list").clone()
}

/// The **name** of the field with the given object number.
fn name_of(session: &mut Session, object: u64) -> String {
    let fields = fields_of(session);
    fields
        .iter()
        .find(|field| field["object"] == json!(object))
        .and_then(|field| field["name"].as_str())
        .unwrap_or_default()
        .to_string()
}

/// A field with a **parent** is named with both names.
#[test]
fn a_field_with_a_parent_is_named_with_both_names() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    assert_eq!(name_of(&mut session, 3), "the parent.the child", "the name is qualified");
}

/// The **parent** of a field is read.
#[test]
fn the_parent_of_a_field_is_read() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let fields = fields_of(&mut session);
    let child_field = fields.iter().find(|field| field["object"] == json!(3)).expect("the child is there");
    assert_eq!(child_field["parent"], json!(2), "the parent is named");
}

/// The **parent** may be taken away.
#[test]
fn the_parent_may_be_taken_away() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    session.execute("pdf.field.parent", &json!({ "field": child })).expect("the parent is taken away");
    assert_eq!(name_of(&mut session, 3), "the child", "the name is its own again");
}

/// A field may not be **its own** parent.
#[test]
fn a_field_may_not_be_its_own_parent() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }]]));
    let field = node_of(&session, 2);
    let refused = session
        .execute("pdf.field.parent", &json!({ "field": field, "parent": field }))
        .expect_err("the parent is refused");
    assert!(refused.to_string().contains("its own parent"), "the refusal says why: {refused}");
}

/// A parent that is **below** the field is refused: that is a loop.
#[test]
fn a_parent_that_is_below_the_field_is_refused() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let refused = session
        .execute("pdf.field.parent", &json!({ "field": parent, "parent": child }))
        .expect_err("the parent is refused");
    assert!(refused.to_string().contains("loop"), "the refusal says why: {refused}");
}

/// A node that is **not a field** may not be a parent.
#[test]
fn a_node_that_is_not_a_field_may_not_be_a_parent() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }]]));
    add_object(&mut session, 3, json!([["Type", "Page"]]));
    let field = node_of(&session, 2);
    let page = node_of(&session, 3);
    let refused = session
        .execute("pdf.field.parent", &json!({ "field": field, "parent": page }))
        .expect_err("the parent is refused");
    assert!(refused.to_string().contains("not a field"), "the refusal says why: {refused}");
}

/// A **cycle** ends the name: the field still has a name.
#[test]
fn a_cycle_ends_the_name_and_the_field_still_has_a_name() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    // The other way round is a loop and is refused, so the chain
    // cannot be a cycle: the name is still whole.
    let name = name_of(&mut session, 3);
    assert_eq!(name, "the parent.the child", "the name is qualified");
    assert!(!name.is_empty(), "and the field has a name");
}

/// A field with **no name** is named after its object.
#[test]
fn a_field_with_no_name_is_named_after_its_object() {
    let mut session = session();
    add_object(&mut session, 5, json!([["FT", "Tx"]]));
    assert_eq!(name_of(&mut session, 5), "object 5", "the field is named after its object");
}
