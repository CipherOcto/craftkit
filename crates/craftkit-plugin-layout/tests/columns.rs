//! The **columns** of a page.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the layout battery.
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

/// A **document** with a **spread** and a **page** in it.
fn a_page(session: &mut Session) -> u64 {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let outcome = session
        .execute(
            "layout.page.add",
            &json!({ "spread": spread, "name": "the page", "width": 100.0, "height": 200.0 }),
        )
        .expect("the page is added");
    outcome.data["id"].as_u64().expect("the page has an id")
}

/// The **facts** of a page, as the plan reports them.
fn facts_of(session: &mut Session, page: u64) -> Json {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == page).expect("the page is in the plan");
    entry.value.clone()
}

/// **A page may be set in columns.**
#[test]
fn a_page_may_be_set_in_columns() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.columns", &json!({ "page": page, "count": 2, "gutter": 10.0 }))
        .expect("the page is set in columns");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["columns"]["count"], json!(2), "there are two columns");
    assert_eq!(facts["columns"]["gutter"], json!(10.0), "and a gutter of ten between them");
}

/// The **column boxes** are derived.
#[test]
fn the_column_boxes_are_derived() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.columns", &json!({ "page": page, "count": 2, "gutter": 10.0 }))
        .expect("the page is set in columns");
    let facts = facts_of(&mut session, page);
    let boxes = facts["column_boxes"].as_array().expect("the boxes are a list");
    assert_eq!(boxes.len(), 2, "there are two boxes");
    assert_eq!(boxes[0]["x"], json!(0.0), "the first starts at the edge");
    assert_eq!(boxes[0]["width"], json!(45.0), "and is as wide as its share of the page");
    assert_eq!(boxes[1]["x"], json!(55.0), "the second starts after the gutter");
}

/// The columns follow the **margins** when they change.
#[test]
fn the_columns_follow_the_margins_when_they_change() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.columns", &json!({ "page": page, "count": 2, "gutter": 10.0 }))
        .expect("the page is set in columns");
    session
        .execute("layout.page.margin", &json!({ "page": page, "left": 10.0, "right": 10.0 }))
        .expect("the page has margins");
    let facts = facts_of(&mut session, page);
    let boxes = facts["column_boxes"].as_array().expect("the boxes are a list");
    assert_eq!(boxes[0]["x"], json!(10.0), "the first starts at the margin");
    assert_eq!(boxes[0]["width"], json!(35.0), "and is as wide as its share of the margin box");
}

/// A page with **one column** has the page's box as its column box.
#[test]
fn a_page_with_one_column_has_the_pages_box_as_its_column_box() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.columns", &json!({ "page": page, "count": 1 }))
        .expect("the page is set in columns");
    let facts = facts_of(&mut session, page);
    let boxes = facts["column_boxes"].as_array().expect("the boxes are a list");
    assert_eq!(boxes.len(), 1, "there is one box");
    assert_eq!(boxes[0]["width"], json!(100.0), "and it is as wide as the page");
}

/// A number of columns that is **not a whole number** is refused.
#[test]
fn a_number_of_columns_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let refused = session
        .execute("layout.page.columns", &json!({ "page": page, "count": 2.5 }))
        .expect_err("the columns are refused");
    assert!(refused.to_string().contains("counted from one"), "the refusal says why");
}

/// A number of columns beyond the tree's **own** bound is refused.
#[test]
fn a_number_of_columns_beyond_the_trees_own_bound_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let refused = session
        .execute("layout.page.columns", &json!({ "page": page, "count": 217 }))
        .expect_err("the columns are refused");
    assert!(refused.to_string().contains("216"), "the refusal says how many there may be");
}

/// A gutter so wide that the columns **do not fit** is refused.
#[test]
fn a_gutter_so_wide_that_the_columns_do_not_fit_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let refused = session
        .execute("layout.page.columns", &json!({ "page": page, "count": 3, "gutter": 60.0 }))
        .expect_err("the columns are refused");
    assert!(refused.to_string().contains("do not fit"), "the refusal says why");
}
