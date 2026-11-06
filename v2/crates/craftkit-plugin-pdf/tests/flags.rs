//! The **flags** of a field.

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

/// The **field** with the given object number, as the command reads it.
fn field_of(session: &mut Session, object: u64) -> Json {
    let outcome = session.execute("pdf.field.read", &json!({})).expect("the fields are read");
    outcome.data["fields"]
        .as_array()
        .expect("the fields are a list")
        .iter()
        .find(|field| field["object"] == json!(object))
        .cloned()
        .unwrap_or(Json::Null)
}

/// A field that is **read only** says so.
#[test]
fn a_field_that_is_read_only_says_so() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }], ["Ff", 1]]));
    let field = field_of(&mut session, 2);
    assert_eq!(field["flag_names"], json!(["read-only"]), "the flag is named");
}

/// The flags are read as a **number** and as **names**.
#[test]
fn the_flags_are_read_as_a_number_and_as_names() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }], ["Ff", 3]]));
    let field = field_of(&mut session, 2);
    assert_eq!(field["flags"], json!(3.0), "the number is read");
    assert_eq!(field["flag_names"], json!(["read-only", "required"]), "and the names are read");
}

/// A field that **must be filled in** says so.
#[test]
fn a_field_that_must_be_filled_in_says_so() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }], ["Ff", 2]]));
    let field = field_of(&mut session, 2);
    assert_eq!(field["flag_names"], json!(["required"]), "the flag is named");
}

/// A field is made **read only**.
#[test]
fn a_field_is_made_read_only() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }]]));
    let node = node_of(&session, 2);
    session
        .execute("pdf.field.flags", &json!({ "field": node, "read_only": true }))
        .expect("the flag is set");
    let field = field_of(&mut session, 2);
    assert_eq!(field["flags"], json!(1.0), "the number is the number");
    assert_eq!(field["flag_names"], json!(["read-only"]), "and the flag is named");
}

/// A field is made **required**.
#[test]
fn a_field_is_made_required() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }]]));
    let node = node_of(&session, 2);
    session.execute("pdf.field.flags", &json!({ "field": node, "required": true })).expect("the flag is set");
    let field = field_of(&mut session, 2);
    assert_eq!(field["flag_names"], json!(["required"]), "the flag is named");
}

/// A flag is **taken away** again.
#[test]
fn a_flag_is_taken_away_again() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }], ["Ff", 3]]));
    let node = node_of(&session, 2);
    session
        .execute("pdf.field.flags", &json!({ "field": node, "read_only": false }))
        .expect("the flag is taken away");
    let field = field_of(&mut session, 2);
    assert_eq!(field["flag_names"], json!(["required"]), "the other flag is left");
}

/// A flag that is **not said** is left as it was.
#[test]
fn a_flag_that_is_not_said_is_left_as_it_was() {
    let mut session = session();
    add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }], ["Ff", 1]]));
    let node = node_of(&session, 2);
    session.execute("pdf.field.flags", &json!({ "field": node, "required": true })).expect("the flag is set");
    let field = field_of(&mut session, 2);
    assert_eq!(field["flag_names"], json!(["read-only", "required"]), "the flag that was not named is left");
}

/// A node that is **not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    add_object(&mut session, 2, json!([["Type", "Page"]]));
    let node = node_of(&session, 2);
    let refused = session
        .execute("pdf.field.flags", &json!({ "field": node, "read_only": true }))
        .expect_err("the flags are refused");
    assert!(refused.to_string().contains("not a field"), "the refusal says why: {refused}");
}
