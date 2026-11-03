//! The **rotation** of a page.
//!
//! A page may be turned by a multiple of ninety degrees, and the
//! number is taken back into the circle.

use craftkit_sdk::serde_json::json;
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

/// A **page**, and its id.
fn a_page(session: &mut Session) -> u64 {
    let outcome =
        session.execute("pdf.page.add", &json!({ "object": 4, "entries": [] })).expect("the page is added");
    outcome.data["id"].as_u64().expect("the page has an id")
}

/// The **rotation** of a page, as the document holds it.
fn rotation_of(session: &mut Session, page: u64) -> Option<i64> {
    let outcome =
        session.execute("pdf.page.rotation", &json!({ "page": page })).expect("the rotation is read");
    outcome.data["rotate"].as_i64()
}

/// A page that is **not turned** is at zero.
#[test]
fn a_page_that_is_not_turned_is_at_zero() {
    let mut session = session();
    let page = a_page(&mut session);
    assert_eq!(rotation_of(&mut session, page), Some(0), "the page is not turned");
}

/// A page may be **turned** clockwise.
#[test]
fn a_page_may_be_turned() {
    let mut session = session();
    let page = a_page(&mut session);
    session.execute("pdf.page.rotate", &json!({ "page": page, "degrees": 90 })).expect("the page is turned");
    assert_eq!(rotation_of(&mut session, page), Some(90), "the page is turned a quarter");
}

/// A page turned **again** is turned further.
#[test]
fn a_page_turned_again_is_turned_further() {
    let mut session = session();
    let page = a_page(&mut session);
    session.execute("pdf.page.rotate", &json!({ "page": page, "degrees": 90 })).expect("the page is turned");
    session
        .execute("pdf.page.rotate", &json!({ "page": page, "degrees": 90 }))
        .expect("the page is turned again");
    assert_eq!(rotation_of(&mut session, page), Some(180), "the page is turned halfway");
}

/// The number is **taken back into the circle**: a page turned four
/// times is a page not turned at all.
#[test]
fn the_number_is_taken_back_into_the_circle() {
    let mut session = session();
    let page = a_page(&mut session);
    for _ in 0..4 {
        session
            .execute("pdf.page.rotate", &json!({ "page": page, "degrees": 90 }))
            .expect("the page is turned");
    }
    assert_eq!(rotation_of(&mut session, page), Some(0), "the page is not turned");
}

/// A negative amount turns the page the **other way**.
#[test]
fn a_negative_amount_turns_the_page_the_other_way() {
    let mut session = session();
    let page = a_page(&mut session);
    session.execute("pdf.page.rotate", &json!({ "page": page, "degrees": -90 })).expect("the page is turned");
    assert_eq!(rotation_of(&mut session, page), Some(270), "the page is turned back a quarter");
}

/// An amount that is **not a multiple of ninety** is refused: the
/// tree's command truncates it, and a page turned by forty-five
/// degrees is not a page that is turned.
#[test]
fn an_amount_that_is_not_a_multiple_of_ninety_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let error = session
        .execute("pdf.page.rotate", &json!({ "page": page, "degrees": 45 }))
        .expect_err("a page is turned by a multiple of ninety");
    assert!(error.to_string().contains("multiple of ninety"), "the error says so: {error}");
    assert_eq!(rotation_of(&mut session, page), Some(0), "and the page is not turned");
}

/// The rotation is **saved** with the page.
#[test]
fn the_rotation_is_saved_with_the_page() {
    let mut session = session();
    let page = a_page(&mut session);
    session.execute("pdf.page.rotate", &json!({ "page": page, "degrees": 180 })).expect("the page is turned");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(page).expect("the page is there");
    let entries = node.data["entries"].as_array().expect("the entries are there");
    let rotate =
        entries.iter().find(|entry| entry[0].as_str() == Some("Rotate")).expect("the rotation is there");
    assert_eq!(rotate[1].as_i64(), Some(180), "the page is still turned");
}
