//! The **widgets** of a field.

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

/// The **node** of an object, by its number.
fn node_of(session: &Session, object: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| node.id)
        .expect("the object is there")
}

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) -> u64 {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
    node_of(session, object)
}

/// A **page** of the document.
fn add_page(session: &mut Session, object: u64) -> u64 {
    session
        .execute("pdf.page.add", &json!({ "object": object, "entries": [["Type", "Page"]] }))
        .expect("the page is added");
    node_of(session, object)
}

/// A **widget** on a page: an annotation of the kind `Widget`.
fn add_widget(session: &mut Session, page: u64, object: u64) -> u64 {
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({ "page": page, "object": object, "subtype": "Widget", "rect": [0, 0, 10, 10] }),
        )
        .expect("the widget is added");
    outcome.data["id"].as_u64().expect("the widget has an id")
}

/// An **entry** of an object, set.
fn set_entry(session: &mut Session, object: u64, key: &str, value: Json) {
    let node = node_of(session, object);
    session
        .execute("pdf.entry.set", &json!({ "object": node, "key": key, "value": value }))
        .expect("the entry is set");
}

/// A **field** with a widget under it.
fn a_field_with_a_widget(session: &mut Session) -> (u64, u64) {
    let page = add_page(session, 5);
    let widget = add_widget(session, page, 3);
    let field = add_object(
        session,
        2,
        json!([["FT", "Btn"], ["T", { "string": "the box" }], ["Kids", [{ "ref": [3, 0] }]]]),
    );
    (field, widget)
}

/// The **widgets** of a field, as the command reads them.
fn widgets_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.widgets", &json!({ "field": field })).expect("the widgets are read");
    outcome.data["widgets"].clone()
}

/// The **first** widget of a field.
fn first_widget(session: &mut Session, field: u64) -> Json {
    let widgets = widgets_of(session, field);
    widgets.as_array().expect("the widgets are a list")[0].clone()
}

/// The widgets of a field are **said**.
#[test]
fn the_widgets_of_a_field_are_said() {
    let mut session = session();
    let (field, widget) = a_field_with_a_widget(&mut session);
    let widgets = widgets_of(&mut session, field);
    let widgets = widgets.as_array().expect("the widgets are a list");
    assert_eq!(widgets.len(), 1, "there is one widget");
    assert_eq!(widgets[0]["object"], json!(3), "the widget is the one");
    assert_eq!(widgets[0]["id"], json!(widget), "and it is the node");
}

/// The **page** of a widget is the page whose annotations hold it.
#[test]
fn the_page_of_a_widget_is_the_page_whose_annotations_hold_it() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["page"], json!(5), "the page is said");
}

/// The **rectangle** of a widget is two corners in order.
#[test]
fn the_rectangle_of_a_widget_is_two_corners_in_order() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    set_entry(&mut session, 3, "Rect", json!([10, 20, 0, 0]));
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["rect"], json!([0.0, 0.0, 10.0, 20.0]), "the corners are in order");
}

/// A widget with **no rectangle** is at the **origin**.
#[test]
fn a_widget_with_no_rectangle_is_at_the_origin() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    set_entry(&mut session, 3, "Rect", json!([]));
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["rect"], json!([0.0, 0.0, 0.0, 0.0]), "the widget is at the origin");
}

/// The **state** a widget shows is said.
#[test]
fn the_state_a_widget_shows_is_said() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    set_entry(&mut session, 3, "AS", json!("Yes"));
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["state"], json!("Yes"), "the state is said");
}

/// The **on state** of a box is the appearance that is not the off
/// one.
#[test]
fn the_on_state_of_a_box_is_the_appearance_that_is_not_the_off_one() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    set_entry(
        &mut session,
        3,
        "AP",
        json!({ "entries": [["N", { "entries": [["Off", {}], ["Yes", {}]] }]] }),
    );
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["on_state"], json!("Yes"), "the on state is said");
}

/// The **on state** is not said for a field of another kind.
#[test]
fn the_on_state_is_not_said_for_a_field_of_another_kind() {
    let mut session = session();
    let page = add_page(&mut session, 5);
    let _widget = add_widget(&mut session, page, 3);
    let field = add_object(
        &mut session,
        2,
        json!([["FT", "Tx"], ["T", { "string": "the text" }], ["Kids", [{ "ref": [3, 0] }]]]),
    );
    set_entry(
        &mut session,
        3,
        "AP",
        json!({ "entries": [["N", { "entries": [["Off", {}], ["Yes", {}]] }]] }),
    );
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["on_state"], Json::Null, "the on state is not said");
}

/// A **locked** widget says so.
#[test]
fn a_locked_widget_says_so() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    set_entry(&mut session, 3, "F", json!(128));
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["locked"], json!(true), "the widget is locked");
    assert_eq!(widget["hidden"], json!(false), "and it is not hidden");
}

/// A **hidden** widget says so.
#[test]
fn a_hidden_widget_says_so() {
    let mut session = session();
    let (field, _) = a_field_with_a_widget(&mut session);
    set_entry(&mut session, 3, "F", json!(2));
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["hidden"], json!(true), "the widget is hidden");
    set_entry(&mut session, 3, "F", json!(32));
    let widget = first_widget(&mut session, field);
    assert_eq!(widget["hidden"], json!(true), "and a widget that cannot be seen is hidden too");
}

/// A field with **no widgets** has none.
#[test]
fn a_field_with_no_widgets_has_none() {
    let mut session = session();
    let field = add_object(&mut session, 2, json!([["FT", "Tx"], ["T", { "string": "the field" }]]));
    let widgets = widgets_of(&mut session, field);
    assert_eq!(widgets.as_array().expect("the widgets are a list").len(), 0, "no widgets");
}

/// A **child** is not a widget.
#[test]
fn a_child_is_not_a_widget() {
    let mut session = session();
    add_object(&mut session, 3, json!([["FT", "Tx"], ["T", { "string": "the child" }]]));
    let field = add_object(
        &mut session,
        2,
        json!([["FT", "Tx"], ["T", { "string": "the parent" }], ["Kids", [{ "ref": [3, 0] }]]]),
    );
    let widgets = widgets_of(&mut session, field);
    assert_eq!(widgets.as_array().expect("the widgets are a list").len(), 0, "no widgets");
}

/// A node that is **not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    let node = add_object(&mut session, 2, json!([["Rect", [0, 0, 10, 10]]]));
    let refused =
        session.execute("pdf.field.widgets", &json!({ "field": node })).expect_err("the node is refused");
    assert!(refused.to_string().contains("is not a field"), "the refusal says why: {refused}");
}
