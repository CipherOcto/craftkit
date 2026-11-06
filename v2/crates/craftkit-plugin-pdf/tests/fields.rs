//! The **fields** of a form.

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

/// A **text field** with a name and a value.
fn a_text_field(session: &mut Session) {
    add_object(
        session,
        2,
        json!([
            ["FT", "Tx"],
            ["T", { "string": "the name" }],
            ["V", { "string": "the words" }],
            ["Ff", 0],
        ]),
    );
}

/// A **choice field** with two options.
fn a_choice_field(session: &mut Session) {
    add_object(
        session,
        3,
        json!([
            ["FT", "Ch"],
            ["T", { "string": "the choice" }],
            ["V", { "string": "the second" }],
            ["Opt", [{ "string": "the first" }, { "string": "the second" }]],
        ]),
    );
}

/// A **button** with two states.
fn a_button(session: &mut Session) {
    add_object(
        session,
        4,
        json!([
            ["FT", "Btn"],
            ["T", { "string": "the button" }],
            ["AP", { "entries": [["N", { "entries": [["Off", {}], ["Yes", {}]] }]] }],
        ]),
    );
}

/// The **fields** of a form are read.
#[test]
fn the_fields_of_a_form_are_read() {
    let mut session = session();
    a_text_field(&mut session);
    let fields = fields_of(&mut session);
    assert_eq!(fields.len(), 1, "there is one field");
    assert_eq!(fields[0]["name"], json!("the name"), "the field is named");
    assert_eq!(fields[0]["kind"], json!("Tx"), "and it is a text field");
    assert_eq!(fields[0]["value"], json!("the words"), "and it holds its value");
}

/// The three kinds of field are told apart.
#[test]
fn the_three_kinds_of_field_are_told_apart() {
    let mut session = session();
    a_text_field(&mut session);
    a_choice_field(&mut session);
    a_button(&mut session);
    let fields = fields_of(&mut session);
    let kinds: Vec<String> =
        fields.iter().map(|field| field["kind"].as_str().unwrap_or_default().to_string()).collect();
    assert!(kinds.contains(&"Tx".to_string()), "there is a text field: {kinds:?}");
    assert!(kinds.contains(&"Ch".to_string()), "there is a choice field: {kinds:?}");
    assert!(kinds.contains(&"Btn".to_string()), "there is a button: {kinds:?}");
}

/// The **options** of a choice are read.
#[test]
fn the_options_of_a_choice_are_read() {
    let mut session = session();
    a_choice_field(&mut session);
    let fields = fields_of(&mut session);
    assert_eq!(fields[0]["options"], json!(["the first", "the second"]), "the options are read");
}

/// A field **without a name** is named by its object number.
#[test]
fn a_field_without_a_name_is_named_by_its_object_number() {
    let mut session = session();
    add_object(&mut session, 5, json!([["FT", "Tx"]]));
    let fields = fields_of(&mut session);
    assert_eq!(fields[0]["name"], json!("object 5"), "the field is named after its object");
}

/// A **text field** takes any text.
#[test]
fn a_text_field_takes_any_text() {
    let mut session = session();
    a_text_field(&mut session);
    let field = node_of(&session, 2);
    session
        .execute("pdf.field.set", &json!({ "field": field, "value": "other words" }))
        .expect("the value is put");
    let fields = fields_of(&mut session);
    assert_eq!(fields[0]["value"], json!("other words"), "the field holds it");
}

/// A **choice** takes one of its options, and nothing else.
#[test]
fn a_choice_takes_one_of_its_options_and_nothing_else() {
    let mut session = session();
    a_choice_field(&mut session);
    let field = node_of(&session, 3);
    session
        .execute("pdf.field.set", &json!({ "field": field, "value": "the first" }))
        .expect("the value is put");
    let fields = fields_of(&mut session);
    assert_eq!(fields[0]["value"], json!("the first"), "the field holds it");
    let refused = session
        .execute("pdf.field.set", &json!({ "field": field, "value": "the third" }))
        .expect_err("the value is refused");
    assert!(refused.to_string().contains("not one of the options"), "the refusal says why: {refused}");
}

/// A **button** takes one of its states, and nothing else.
#[test]
fn a_button_takes_one_of_its_states_and_nothing_else() {
    let mut session = session();
    a_button(&mut session);
    let field = node_of(&session, 4);
    session.execute("pdf.field.set", &json!({ "field": field, "value": "Yes" })).expect("the value is put");
    let fields = fields_of(&mut session);
    assert_eq!(fields[0]["value"], json!("Yes"), "the field holds it");
    let refused = session
        .execute("pdf.field.set", &json!({ "field": field, "value": "Maybe" }))
        .expect_err("the value is refused");
    assert!(refused.to_string().contains("not one of the states"), "the refusal says why: {refused}");
}

/// An **empty** value takes the value away.
#[test]
fn an_empty_value_takes_the_value_away() {
    let mut session = session();
    a_text_field(&mut session);
    let field = node_of(&session, 2);
    session
        .execute("pdf.field.set", &json!({ "field": field, "value": "" }))
        .expect("the value is taken away");
    let fields = fields_of(&mut session);
    assert_eq!(fields[0]["value"], Json::Null, "the field holds nothing");
}

/// A node that is **not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    add_object(&mut session, 6, json!([["Type", "Page"]]));
    let node = node_of(&session, 6);
    let refused = session
        .execute("pdf.field.set", &json!({ "field": node, "value": "the words" }))
        .expect_err("the value is refused");
    assert!(refused.to_string().contains("not a field"), "the refusal says why: {refused}");
}
