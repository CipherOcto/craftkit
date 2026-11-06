//! The **tab order** of a form.

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
fn add_widget(session: &mut Session, page: u64, object: u64, rect: Json) {
    session
        .execute(
            "pdf.annot.add",
            &json!({ "page": page, "object": object, "subtype": "Widget", "rect": rect }),
        )
        .expect("the widget is added");
}

/// An **entry** of an object, set.
fn set_entry(session: &mut Session, object: u64, key: &str, value: Json) {
    let node = node_of(session, object);
    session
        .execute("pdf.entry.set", &json!({ "object": node, "key": key, "value": value }))
        .expect("the entry is set");
}

/// A **field** with a widget under it.
fn add_field(session: &mut Session, object: u64, name: &str, widget: u64) -> u64 {
    add_object(
        session,
        object,
        json!([
            ["FT", "Tx"],
            ["T", { "string": name }],
            ["Kids", [{ "ref": [widget, 0] }]],
        ]),
    )
}

/// The **tab order** of the document, as the command reads it.
fn order_of(session: &mut Session) -> Vec<String> {
    let outcome = session.execute("pdf.tab.order", &json!({})).expect("the order is read");
    outcome.data["order"]
        .as_array()
        .expect("the order is a list")
        .iter()
        .map(|item| item["field"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The **order** the page says.
fn page_order_of(session: &mut Session) -> String {
    let outcome = session.execute("pdf.tab.order", &json!({})).expect("the order is read");
    outcome.data["pages"]
        .as_array()
        .expect("the pages are a list")
        .first()
        .and_then(|page| page["order"].as_str())
        .unwrap_or_default()
        .to_string()
}

/// A form with **three widgets** on one page.
///
/// The widgets are added in the order the last, the first and the
/// second — so the annotation order is not the order of the page.
fn a_form(session: &mut Session) -> u64 {
    let page = add_page(session, 5);
    // The last one first, so the annotation order is not the order of
    // the page.
    add_widget(session, page, 9, json!([0, 100, 100, 120]));
    add_widget(session, page, 7, json!([0, 700, 100, 720]));
    add_widget(session, page, 8, json!([200, 700, 300, 720]));
    add_field(session, 2, "the first", 7);
    add_field(session, 3, "the second", 8);
    add_field(session, 4, "the third", 9);
    page
}

/// A page that says nothing has the **annotation order**.
#[test]
fn a_page_that_says_nothing_has_the_annotation_order() {
    let mut session = session();
    a_form(&mut session);
    assert_eq!(page_order_of(&mut session), "annotations", "the page says nothing");
    assert_eq!(
        order_of(&mut session),
        vec!["the third", "the first", "the second"],
        "the order is the order the page lists them in"
    );
}

/// A page that says **rows** gives the rows of the page.
#[test]
fn a_page_that_says_rows_gives_the_rows_of_the_page() {
    let mut session = session();
    a_form(&mut session);
    set_entry(&mut session, 5, "Tabs", json!("R"));
    assert_eq!(page_order_of(&mut session), "rows", "the page says rows");
    assert_eq!(
        order_of(&mut session),
        vec!["the first", "the second", "the third"],
        "the order is the rows of the page"
    );
}

/// A page that says **columns** gives the columns of the page.
#[test]
fn a_page_that_says_columns_gives_the_columns_of_the_page() {
    let mut session = session();
    a_form(&mut session);
    set_entry(&mut session, 5, "Tabs", json!("C"));
    assert_eq!(page_order_of(&mut session), "columns", "the page says columns");
    assert_eq!(
        order_of(&mut session),
        vec!["the first", "the third", "the second"],
        "the order is the columns of the page"
    );
}

/// A page that says the **structure** gives the annotation order.
#[test]
fn a_page_that_says_the_structure_gives_the_annotation_order() {
    let mut session = session();
    a_form(&mut session);
    set_entry(&mut session, 5, "Tabs", json!("S"));
    assert_eq!(page_order_of(&mut session), "structure", "the page says the structure");
    assert_eq!(
        order_of(&mut session),
        vec!["the third", "the first", "the second"],
        "the order is the order the page lists them in"
    );
}

/// The **pages** come in the document's order.
#[test]
fn the_pages_come_in_the_documents_order() {
    let mut session = session();
    a_form(&mut session);
    let other = add_page(&mut session, 6);
    add_widget(&mut session, other, 10, json!([0, 0, 10, 10]));
    add_field(&mut session, 11, "the fourth", 10);
    let outcome = session.execute("pdf.tab.order", &json!({})).expect("the order is read");
    let pages = outcome.data["pages"].as_array().expect("the pages are a list");
    assert_eq!(pages.len(), 2, "there are two pages");
    assert_eq!(pages[0]["page"], json!(5), "the first page is the one that was made first");
    assert_eq!(pages[1]["page"], json!(6), "and the second is the other");
    assert_eq!(
        order_of(&mut session).last().map(|name| name.as_str()),
        Some("the fourth"),
        "the last page is last"
    );
}

/// A widget that is **on no page** has no place in the order.
#[test]
fn a_widget_that_is_on_no_page_has_no_place_in_the_order() {
    let mut session = session();
    a_form(&mut session);
    // A widget that no page holds.
    add_object(&mut session, 12, json!([["Rect", [0, 0, 10, 10]]]));
    add_field(&mut session, 13, "the fifth", 12);
    assert_eq!(order_of(&mut session).len(), 3, "the widget is not in the order");
}

/// The order of **one page** may be read on its own.
#[test]
fn the_order_of_one_page_may_be_read_on_its_own() {
    let mut session = session();
    let page = a_form(&mut session);
    let other = add_page(&mut session, 6);
    add_widget(&mut session, other, 10, json!([0, 0, 10, 10]));
    add_field(&mut session, 11, "the fourth", 10);
    let outcome = session.execute("pdf.tab.order", &json!({ "page": page })).expect("the order is read");
    let order = outcome.data["order"].as_array().expect("the order is a list");
    assert_eq!(order.len(), 3, "the order is of that page only");
    assert!(order.iter().all(|item| item["page"] == json!(5)), "and of no other");
}

/// A page that is **not there** is refused.
#[test]
fn a_page_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session.execute("pdf.tab.order", &json!({ "page": 999 })).expect_err("the page is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}
