//! The **columns** of a page.
//!
//! How many columns the page of a document has, the space between
//! them, the line that may be drawn down the middle, and the width of
//! a column — which is worked out from the page and the space.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// The **page** of a document, as the command reads it.
fn page_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.page.get", &json!({ "document": document })).expect("the page is read");
    outcome.data.clone()
}

/// A page has **one column** until it is told otherwise, and the
/// space between the columns is half an inch.
#[test]
fn a_page_has_one_column_until_it_is_told_otherwise() {
    let mut session = session();
    let document = a_document(&mut session);
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["count"], json!(1), "the page has one column: {page}");
    assert_eq!(page["columns"]["space"], json!(36.0), "and the space is half an inch");
    assert_eq!(page["columns"]["separator"], json!(false), "and there is no line between them");
}

/// The **count** of the columns is from one to twelve.
#[test]
fn the_count_is_from_one_to_twelve() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 3 }))
        .expect("the columns are set");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["count"], json!(3), "the page has three columns: {page}");
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 13 }))
        .expect_err("thirteen columns are refused");
    assert!(error.to_string().contains("one to 12"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 0 }))
        .expect_err("no columns at all are refused");
    assert!(error.to_string().contains("one to 12"), "the refusal says why: {error}");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["count"], json!(3), "and the page is as it was: {page}");
}

/// The **space** between the columns is in points, and the tree
/// clamps it while this tree refuses it.
#[test]
fn a_space_outside_the_range_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2, "space": 24.0 }))
        .expect("the columns are set");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["space"], json!(24.0), "the space is what was given: {page}");
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2, "space": 400.0 }))
        .expect_err("the wide space is refused");
    assert!(error.to_string().contains("zero to 300"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2, "space": -5.0 }))
        .expect_err("the negative space is refused");
    assert!(error.to_string().contains("zero to 300"), "the refusal says why: {error}");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["space"], json!(24.0), "and the space is as it was: {page}");
}

/// The **width of a column** is derived: the room the text has, less
/// the spaces between the columns, shared out.
#[test]
fn the_width_of_a_column_is_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    // US Letter, an inch of margin: 468 points of text.
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2, "space": 36.0 }))
        .expect("the columns are set");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["width"], json!(216.0), "the column is half the room less the gap: {page}");
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 3, "space": 36.0 }))
        .expect("the columns are set");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["width"], json!(132.0), "and with three it is a third less the gaps");
}

/// The **separator** is a line drawn between the columns.
#[test]
fn the_separator_is_a_line_between_the_columns() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2, "separator": true }))
        .expect("the columns are set");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["separator"], json!(true), "the line is there: {page}");
}

/// Columns that **do not fit** on the page are refused, and the page
/// is left as it was.
#[test]
fn columns_that_do_not_fit_are_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2, "space": 36.0 }))
        .expect("the columns are set");
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 12, "space": 300.0 }))
        .expect_err("the columns do not fit");
    assert!(error.to_string().contains("do not fit"), "the refusal says why: {error}");
    let page = page_of(&mut session, document);
    assert_eq!(page["columns"]["count"], json!(2), "and the page is as it was: {page}");
}

/// A node that is **not a document** may not have columns.
#[test]
fn a_node_that_is_not_a_document_may_not_have_columns() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "hello" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let error = session
        .execute("prose.page.columns", &json!({ "document": block, "count": 2 }))
        .expect_err("the paragraph is not a document");
    assert!(error.to_string().contains("not a document"), "the refusal says why: {error}");
}
