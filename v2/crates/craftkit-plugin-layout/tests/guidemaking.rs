//! The **guides of a page are created**.
//!
//! A page may be cut into rows and columns, and the guides of the cuts
//! are made. The rules are the tree's own.

use craftkit_sdk::serde_json::json;
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

/// A **document** with a **spread** and **two** pages of a hundred
/// millimetres in it, and their ids.
fn a_document(session: &mut Session) -> (u64, u64, u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut pages = Vec::new();
    for name in ["first", "second"] {
        let outcome = session
            .execute(
                "layout.page.add",
                &json!({ "spread": spread, "name": name, "width": 100, "height": 200 }),
            )
            .expect("the page is added");
        pages.push(outcome.data["id"].as_u64().expect("the page has an id"));
    }
    (document, spread, pages[0], pages[1])
}

/// The **guides** of a page, as the answer holds them.
fn guides_of(session: &mut Session, page: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("layout.page.guides", &json!({ "page": page })).expect("the guides are read");
    outcome.data["guides"].as_array().expect("the guides are a list").clone()
}

/// A page cut into **columns** makes a guide at every edge: `n`
/// divisions make `n - 1` guides.
#[test]
fn a_page_cut_into_columns_makes_guides() {
    let mut session = session();
    let (_document, spread, first, _second) = a_document(&mut session);
    let outcome = session
        .execute(
            "layout.guide.create",
            &json!({ "spread": spread, "page": first, "columns": 4, "column_gutter": 0 }),
        )
        .expect("the guides are made");
    assert_eq!(outcome.data["guides"], json!(3), "three divisions make three guides: {outcome:?}");
    let guides = guides_of(&mut session, first);
    assert_eq!(guides.len(), 3, "the page holds three: {guides:?}");
    assert_eq!(guides[0]["orientation"], json!("vertical"), "they are across: {guides:?}");
    // The page is a hundred wide and has no margins, so the guides
    // stand at a quarter, a half and three quarters of it.
    assert_eq!(guides[0]["position"], json!(25.0), "at the first edge: {guides:?}");
    assert_eq!(guides[1]["position"], json!(50.0), "at the second: {guides:?}");
    assert_eq!(guides[2]["position"], json!(75.0), "and at the third: {guides:?}");
}

/// A **gutter** doubles the guides: the edge of the cell and the edge
/// after the gutter.
#[test]
fn a_gutter_doubles_the_guides() {
    let mut session = session();
    let (_document, spread, first, _second) = a_document(&mut session);
    let outcome = session
        .execute(
            "layout.guide.create",
            &json!({ "spread": spread, "page": first, "columns": 2, "column_gutter": 10 }),
        )
        .expect("the guides are made");
    assert_eq!(outcome.data["guides"], json!(2), "one division and a gutter make two: {outcome:?}");
    let guides = guides_of(&mut session, first);
    // The cell is (100 - 10) / 2 = 45 wide, so the guides stand at 45
    // and at 55.
    assert_eq!(guides[0]["position"], json!(45.0), "at the edge of the cell: {guides:?}");
    assert_eq!(guides[1]["position"], json!(55.0), "and after the gutter: {guides:?}");
}

/// The guides stand across the **margins** of the page, and across the
/// page itself when they are told to.
#[test]
fn the_guides_stand_across_the_margins() {
    let mut session = session();
    let (document, spread, first, _second) = a_document(&mut session);
    let outcome = session
        .execute("layout.page.margin", &json!({ "page": first, "right": 50 }))
        .expect("the margins are set");
    assert!(outcome.data.is_object(), "the margins are set: {outcome:?}");
    let _ = document;
    session
        .execute(
            "layout.guide.create",
            &json!({ "spread": spread, "page": first, "columns": 2, "column_gutter": 0 }),
        )
        .expect("the guides are made");
    let guides = guides_of(&mut session, first);
    assert_eq!(guides.len(), 1, "there is one guide: {guides:?}");
    assert_eq!(guides[0]["position"], json!(25.0), "the margin box is fifty wide: {guides:?}");
}

/// The guides of a page that is not named are made on **every** page
/// of the spread, and the ones that are there may be cleared first.
#[test]
fn every_page_of_the_spread_may_be_cut() {
    let mut session = session();
    let (_document, spread, first, second) = a_document(&mut session);
    session
        .execute("layout.guide.create", &json!({ "spread": spread, "columns": 2, "column_gutter": 0 }))
        .expect("the guides are made");
    assert_eq!(guides_of(&mut session, first).len(), 1, "the first page is cut");
    assert_eq!(guides_of(&mut session, second).len(), 1, "and the second");
    // The guides that are there are cleared first.
    let outcome = session
        .execute(
            "layout.guide.create",
            &json!({ "spread": spread, "page": first, "columns": 3, "column_gutter": 0, "remove_existing": true }),
        )
        .expect("the guides are made");
    assert_eq!(outcome.data["guides"], json!(2), "the page is cut again: {outcome:?}");
    let guides = guides_of(&mut session, first);
    assert_eq!(guides.len(), 2, "and it holds two: {guides:?}");
}

/// A request that is not one is **refused**: too many divisions, a
/// gutter that is not one, and a fit that is not one.
#[test]
fn a_request_that_is_not_one_is_refused() {
    let mut session = session();
    let (_document, spread, first, _second) = a_document(&mut session);
    let error = session
        .execute("layout.guide.create", &json!({ "spread": spread, "page": first, "columns": 1001 }))
        .expect_err("the divisions are refused");
    assert!(error.to_string().contains("must not exceed 1000"), "it says why: {error}");
    let error = session
        .execute(
            "layout.guide.create",
            &json!({ "spread": spread, "page": first, "columns": 2, "column_gutter": -1 }),
        )
        .expect_err("the gutter is refused");
    assert!(error.to_string().contains("between 0 and 1000000"), "it says why: {error}");
    let error = session
        .execute(
            "layout.guide.create",
            &json!({ "spread": spread, "page": first, "columns": 2, "fit_to": "paper" }),
        )
        .expect_err("the fit is refused");
    assert!(error.to_string().contains("fitTo"), "it says why: {error}");
    let error = session
        .execute("layout.guide.create", &json!({ "spread": spread, "page": 9999, "columns": 2 }))
        .expect_err("the page is refused");
    assert!(error.to_string().contains("there is no page"), "it says why: {error}");
}
