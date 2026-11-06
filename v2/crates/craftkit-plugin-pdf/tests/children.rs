//! The **children** of a field.

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

/// The **entries** of an object, as the document keeps them.
fn entries_of(session: &Session, object: u64) -> Json {
    let node = session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .expect("the object is there");
    node.data["entries"].clone()
}

/// The **kids** an object names, as the numbers of the objects.
fn kids_of(session: &Session, object: u64) -> Vec<u64> {
    let entries = entries_of(session, object);
    entries
        .as_array()
        .expect("the entries are a list")
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? != "Kids" {
                return None;
            }
            Some(pair.get(1)?.clone())
        })
        .and_then(|value| value.as_array().cloned())
        .map(|kids| {
            kids.iter()
                .filter_map(|kid| kid.get("ref").and_then(|pair| pair.as_array())?.first()?.as_u64())
                .collect()
        })
        .unwrap_or_default()
}

/// The **children** of a field, as the command reads them.
fn children_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.children", &json!({ "field": field })).expect("the children are read");
    outcome.data.clone()
}

/// A field put in a field becomes its **child**, and the file says it
/// on both sides.
#[test]
fn a_field_put_in_a_field_becomes_its_child_and_the_file_says_it_on_both_sides() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    assert_eq!(kids_of(&session, 2), vec![3], "the parent names the child");
    let children = children_of(&mut session, parent);
    let children = children["children"].as_array().expect("the children are a list");
    assert_eq!(children.len(), 1, "there is one child");
    assert_eq!(children[0]["object"], json!(3), "the child is the one");
    assert_eq!(children[0]["name"], json!("the child"), "and it is named");
    assert_eq!(children[0]["field"], json!("the parent.the child"), "and its name is qualified");
}

/// A field with **no children** has none.
#[test]
fn a_field_with_no_children_has_none() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }]]));
    let field = node_of(&session, 2);
    let children = children_of(&mut session, field);
    assert_eq!(children["children"].as_array().expect("the children are a list").len(), 0, "no children");
}

/// A kid with **no name** is a **widget** and not a child.
#[test]
fn a_kid_with_no_name_is_a_widget_and_not_a_child() {
    let mut session = session();
    add_object(&mut session, 3, json!([["Rect", [0, 0, 10, 10]]]));
    add_object(
        &mut session,
        2,
        json!([["FT", "Tx"], ["T", { "string": "the field" }], ["Kids", [{ "ref": [3, 0] }]]]),
    );
    let field = node_of(&session, 2);
    let children = children_of(&mut session, field);
    assert_eq!(children["children"].as_array().expect("the children are a list").len(), 0, "no children");
    assert_eq!(children["widgets"], json!([3]), "the widget is said");
}

/// The reading of the fields says the **children** and the
/// **widgets**.
#[test]
fn the_reading_of_the_fields_says_the_children_and_the_widgets() {
    let mut session = session();
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    add_object(&mut session, 4, json!([["Rect", [0, 0, 10, 10]]]));
    add_object(
        &mut session,
        2,
        json!([
            ["FT", "Tx"],
            ["T", { "string": "the parent" }],
            ["Kids", [{ "ref": [3, 0] }, { "ref": [4, 0] }]],
        ]),
    );
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let outcome = session.execute("pdf.field.read", &json!({})).expect("the fields are read");
    let fields = outcome.data["fields"].as_array().expect("the fields are a list");
    let parent = fields.iter().find(|field| field["object"] == json!(2)).expect("the parent is there");
    assert_eq!(parent["children"], json!([3]), "the child is said");
    assert_eq!(parent["widgets"], json!([4]), "the widget is said");
}

/// A field that **leaves** its parent leaves its kids too.
#[test]
fn a_field_that_leaves_its_parent_leaves_its_kids_too() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    session.execute("pdf.field.parent", &json!({ "field": child })).expect("the parent is taken away");
    assert!(kids_of(&session, 2).is_empty(), "the parent holds no kids now");
    let children = children_of(&mut session, parent);
    assert_eq!(children["children"].as_array().expect("the children are a list").len(), 0, "no children");
}

/// A field that moves to **another parent** leaves the one it was in.
#[test]
fn a_field_that_moves_to_another_parent_leaves_the_one_it_was_in() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the first" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the second" }]]));
    add_object(&mut session, 4, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let first = node_of(&session, 2);
    let second = node_of(&session, 3);
    let child = node_of(&session, 4);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": first }))
        .expect("the parent is put");
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": second }))
        .expect("the parent is put");
    assert!(kids_of(&session, 2).is_empty(), "the first parent holds no kids now");
    assert_eq!(kids_of(&session, 3), vec![4], "the second parent holds the child");
}

/// A node that is **not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    add_object(&mut session, 2, json!([["Rect", [0, 0, 10, 10]]]));
    let node = node_of(&session, 2);
    let refused =
        session.execute("pdf.field.children", &json!({ "field": node })).expect_err("the node is refused");
    assert!(refused.to_string().contains("is not a field"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused =
        session.execute("pdf.field.children", &json!({ "field": 999 })).expect_err("the node is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}
