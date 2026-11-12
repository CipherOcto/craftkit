//! The **widgets** of a page that is gone.
//!
//! A widget is a field of the form, and a field that points at a page
//! that is not there would keep it, with its content, in the file that
//! is written. So the widgets of a page that is taken away leave the
//! form.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
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

/// An **object** of the file, with the id it was given.
fn an_object(session: &mut Session, object: u64, entries: craftkit_sdk::serde_json::Value) -> u64 {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
    outcome.data["id"].as_u64().expect("the object has an id")
}

/// A **page** of the file, with the id it was given.
fn a_page(session: &mut Session, object: u64, parent: u64) -> u64 {
    let outcome = session
        .execute("pdf.page.add", &json!({ "object": object, "parent": parent }))
        .expect("the page is added");
    outcome.data["id"].as_u64().expect("the page has an id")
}

/// The **entries** of a node, as the document holds them.
fn entries_of(session: &Session, id: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(id).expect("the node is there");
    node.data["entries"]
        .as_array()
        .expect("the node has a dictionary")
        .clone()
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            Some((key, pair.get(1)?.clone()))
        })
        .collect()
}

/// The **value** under a key of a node.
fn value_of(session: &Session, id: u64, key: &str) -> Option<craftkit_sdk::serde_json::Value> {
    entries_of(session, id).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **numbers** a list names.
fn numbers_in(value: Option<craftkit_sdk::serde_json::Value>) -> Vec<u64> {
    value
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|item| {
            item.get("ref")
                .and_then(|pair| pair.as_array())
                .and_then(|pair| pair.first())
                .and_then(|number| number.as_u64())
        })
        .collect()
}

/// Put an **entry** into a dictionary.
fn set_entry(session: &mut Session, id: u64, key: &str, value: craftkit_sdk::serde_json::Value) {
    session
        .execute("pdf.entry.set", &json!({ "object": id, "key": key, "value": value }))
        .expect("the entry is set");
}

/// A file with a **catalog**, a **form**, a tree of pages and two
/// pages.
fn a_file(session: &mut Session) -> (u64, u64, u64) {
    let root = an_object(session, 1, json!([["Type", "Pages"]]));
    let first = a_page(session, 3, root);
    let second = a_page(session, 4, root);
    (root, first, second)
}

/// The **widget** of a page: an annotation whose subtype is `Widget`.
fn a_widget(session: &mut Session, object: u64, page: u64) -> u64 {
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": object,
                "subtype": "Widget",
                "rect": [0.0, 0.0, 10.0, 10.0],
            }),
        )
        .expect("the widget is added");
    outcome.data["id"].as_u64().expect("the widget has an id")
}

/// The widget of a page that is gone **leaves the form**.
#[test]
fn the_widget_of_a_page_that_is_gone_leaves_the_form() {
    let mut session = session();
    let (_, first, _) = a_file(&mut session);
    // The **form**: the catalog names it, and it holds the field.
    let form = an_object(&mut session, 6, json!([["Type", "AcroForm"], ["Fields", [{ "ref": [7, 0] }]]]));
    an_object(
        &mut session,
        2,
        json!([["Type", "Catalog"], ["Pages", { "ref": [1, 0] }], ["AcroForm", { "ref": [6, 0] }]]),
    );
    let widget = a_widget(&mut session, 7, first);
    assert_eq!(numbers_in(value_of(&session, form, "Fields")), vec![7], "the form holds the field");
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    assert!(
        numbers_in(value_of(&session, form, "Fields")).is_empty(),
        "and the form does not hold it any more"
    );
    assert!(session.document().node(widget).is_some(), "the widget itself stays");
}

/// A field with **children** leaves them, and a field left with no
/// children leaves the form as well.
#[test]
fn a_field_left_with_no_children_leaves_the_form() {
    let mut session = session();
    let (_, first, _) = a_file(&mut session);
    // The **field**: it holds the widget, and the form holds the field.
    let field = an_object(&mut session, 6, json!([["Type", "Annot"], ["Kids", [{ "ref": [7, 0] }]]]));
    let form = an_object(&mut session, 8, json!([["Type", "AcroForm"], ["Fields", [{ "ref": [6, 0] }]]]));
    an_object(
        &mut session,
        2,
        json!([["Type", "Catalog"], ["Pages", { "ref": [1, 0] }], ["AcroForm", { "ref": [8, 0] }]]),
    );
    let widget = a_widget(&mut session, 7, first);
    set_entry(&mut session, widget, "Parent", json!({ "ref": [6, 0] }));
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    assert!(numbers_in(value_of(&session, field, "Kids")).is_empty(), "the field has no children");
    assert!(
        numbers_in(value_of(&session, form, "Fields")).is_empty(),
        "and the form does not hold the field"
    );
    assert!(session.document().node(widget).is_some(), "the widget itself stays");
}

/// The widgets of a page that **stays** are not touched.
#[test]
fn the_widgets_of_a_page_that_stays_are_not_touched() {
    let mut session = session();
    let (_, first, second) = a_file(&mut session);
    let form = an_object(&mut session, 6, json!([["Type", "AcroForm"], ["Fields", [{ "ref": [7, 0] }]]]));
    an_object(
        &mut session,
        2,
        json!([["Type", "Catalog"], ["Pages", { "ref": [1, 0] }], ["AcroForm", { "ref": [6, 0] }]]),
    );
    let _widget = a_widget(&mut session, 7, second);
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    assert_eq!(numbers_in(value_of(&session, form, "Fields")), vec![7], "the form still holds the field");
}

/// A file with **no form** is not troubled by the removal of a page.
#[test]
fn a_file_with_no_form_is_not_troubled() {
    let mut session = session();
    let (_, first, _) = a_file(&mut session);
    let outcome = session.execute("pdf.page.remove", &json!({ "page": first }));
    assert!(outcome.is_ok(), "the page goes");
}
