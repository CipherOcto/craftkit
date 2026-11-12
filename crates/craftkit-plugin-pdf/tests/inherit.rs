//! The **kind and the flags** a field inherits.

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

/// The **field** with the given object number.
fn field_of(session: &mut Session, object: u64) -> Json {
    let fields = fields_of(session);
    fields.iter().find(|field| field["object"] == json!(object)).expect("the field is there").clone()
}

/// What the field **inherits**.
fn inherited_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.inherit", &json!({ "field": field })).expect("the field is read");
    outcome.data.clone()
}

/// A field that says **nothing** takes the kind of its parent.
#[test]
fn a_field_that_says_nothing_takes_the_kind_of_its_parent() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let field = field_of(&mut session, 3);
    assert_eq!(field["kind"], json!("Tx"), "the kind comes from the parent");
    assert_eq!(field["own_kind"], Json::Null, "the field says nothing of its own");
}

/// A field that says **its own kind** keeps it.
#[test]
fn a_field_that_says_its_own_kind_keeps_it() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Ch"], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let field = field_of(&mut session, 3);
    assert_eq!(field["kind"], json!("Ch"), "the field says its own kind");
    assert_eq!(field["own_kind"], json!("Ch"), "and it is said so");
}

/// A field that says **nothing** takes the flags of its parent.
#[test]
fn a_field_that_says_nothing_takes_the_flags_of_its_parent() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["Ff", 3], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let field = field_of(&mut session, 3);
    assert_eq!(field["flags"], json!(3.0), "the flags come from the parent");
    assert_eq!(field["own_flags"], Json::Null, "the field says nothing of its own");
    let names = field["flag_names"].as_array().expect("the names are a list");
    assert!(names.contains(&json!("read-only")), "the field is read only");
    assert!(names.contains(&json!("required")), "and it must be filled in");
}

/// A field's own flags **replace** the ones it would inherit.
#[test]
fn a_fields_own_flags_replace_the_ones_it_would_inherit() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["Ff", 1], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["Ff", 2], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let field = field_of(&mut session, 3);
    assert_eq!(field["flags"], json!(2.0), "the field says its own flags");
    assert_eq!(field["own_flags"], json!(2.0), "and they are said so");
    let names = field["flag_names"].as_array().expect("the names are a list");
    assert!(!names.contains(&json!("read-only")), "the parent's flag does not come down");
    assert!(names.contains(&json!("required")), "the field's own flag is there");
}

/// What the field inherits is **said**, and **where** it comes from.
#[test]
fn what_the_field_inherits_is_said_and_where_it_comes_from() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["Ff", 1], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let inherited = inherited_of(&mut session, child);
    assert_eq!(inherited["kind"], json!("Tx"), "the kind is said");
    assert_eq!(inherited["kind_from"], json!(2), "and it comes from the parent");
    assert_eq!(inherited["flags"], json!(1.0), "the flags are said");
    assert_eq!(inherited["flags_from"], json!(2), "and they come from the parent");
    assert_eq!(inherited["chain"], json!([2]), "the chain is said");
}

/// A field that says **everything** inherits nothing.
#[test]
fn a_field_that_says_everything_inherits_nothing() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["Ff", 1], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["Ff", 2], ["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    let inherited = inherited_of(&mut session, child);
    assert_eq!(inherited["kind"], Json::Null, "the kind is inherited from nowhere");
    assert_eq!(inherited["flags"], Json::Null, "and the flags too");
}

/// The kind comes from the **nearest** ancestor that says one.
#[test]
fn the_kind_comes_from_the_nearest_ancestor_that_says_one() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Ch"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the middle" }]]));
    add_object(&mut session, 4, json!([["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let middle = node_of(&session, 3);
    let child = node_of(&session, 4);
    session
        .execute("pdf.field.parent", &json!({ "field": middle, "parent": parent }))
        .expect("the parent is put");
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": middle }))
        .expect("the parent is put");
    let field = field_of(&mut session, 4);
    assert_eq!(field["kind"], json!("Tx"), "the nearest ancestor that says one wins");
    let inherited = inherited_of(&mut session, child);
    assert_eq!(inherited["kind_from"], json!(3), "and it is said where it comes from");
    assert_eq!(inherited["chain"], json!([3, 2]), "the chain is the parents, nearest first");
}

/// A node that is **not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    add_object(&mut session, 2, json!([["T", { "string": "not a field" }]]));
    let node = node_of(&session, 2);
    let refused =
        session.execute("pdf.field.inherit", &json!({ "field": node })).expect_err("the node is refused");
    assert!(refused.to_string().contains("is not a field"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused =
        session.execute("pdf.field.inherit", &json!({ "field": 999 })).expect_err("the node is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}

/// A field that is **its own ancestor** still has an answer.
#[test]
fn a_field_that_is_its_own_ancestor_still_has_an_answer() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the parent" }]]));
    add_object(&mut session, 3, json!([["T", { "string": "the child" }]]));
    let parent = node_of(&session, 2);
    let child = node_of(&session, 3);
    session
        .execute("pdf.field.parent", &json!({ "field": child, "parent": parent }))
        .expect("the parent is put");
    // The loop is made by hand, because the command refuses to make one.
    session
        .execute("pdf.entry.set", &json!({ "object": parent, "key": "Parent", "value": { "ref": [3, 0] } }))
        .expect("the loop is made");
    let field = field_of(&mut session, 3);
    assert_eq!(field["kind"], json!("Tx"), "the kind is still found");
    let inherited = inherited_of(&mut session, child);
    let chain = inherited["chain"].as_array().expect("the chain is a list");
    assert_eq!(chain.len(), 2, "the chain ends: the parent and the field it comes back to");
    assert_eq!(chain[0], json!(2), "the parent is first");
}
