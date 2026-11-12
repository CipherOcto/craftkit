//! The **boxes** of a page.
//!
//! A page has five boxes — media, crop, bleed, trim and art — and
//! they are a chain: each falls back to the one before it and each is
//! cut back to the media box.

use craftkit_sdk::serde_json::{Value, json};
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

/// The **boxes** of a page, read from the plan.
fn boxes_of(session: &mut Session, page: u64) -> Vec<Value> {
    let outcome = session.execute("pdf.page.boxes", &json!({ "page": page })).expect("the boxes are read");
    outcome.data["boxes"].as_array().expect("the boxes are there").clone()
}

/// The box of a page, by its place in the tree's own order.
fn box_at(session: &mut Session, page: u64, index: usize) -> Vec<f64> {
    boxes_of(session, page)[index]
        .as_array()
        .expect("the box is a rectangle")
        .iter()
        .map(|number| number.as_f64().expect("the box has numbers"))
        .collect()
}

/// A page with **no boxes** has the letter size, and every other box
/// falls back to it.
#[test]
fn a_page_with_no_boxes_has_the_letter_size() {
    let mut session = session();
    let page = a_page(&mut session);
    let boxes = boxes_of(&mut session, page);
    assert_eq!(boxes.len(), 5, "a page has five boxes");
    assert_eq!(box_at(&mut session, page, 0), vec![0.0, 0.0, 612.0, 792.0], "the media box is the letter");
    assert_eq!(box_at(&mut session, page, 1), vec![0.0, 0.0, 612.0, 792.0], "the crop box falls back to it");
    assert_eq!(box_at(&mut session, page, 4), vec![0.0, 0.0, 612.0, 792.0], "and so does the art box");
}

/// The **media box** may be set.
#[test]
fn the_media_box_may_be_set() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [0, 0, 595, 842] }))
        .expect("the media box is set");
    assert_eq!(box_at(&mut session, page, 0), vec![0.0, 0.0, 595.0, 842.0], "the media box is A4");
}

/// The other boxes fall back to the **crop box**, and the crop box
/// falls back to the **media box**.
#[test]
fn the_boxes_fall_back_to_one_another() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [0, 0, 595, 842] }))
        .expect("the media box is set");
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "CropBox", "rect": [10, 10, 500, 800] }))
        .expect("the crop box is set");
    assert_eq!(box_at(&mut session, page, 1), vec![10.0, 10.0, 500.0, 800.0], "the crop box is its own");
    assert_eq!(
        box_at(&mut session, page, 2),
        vec![10.0, 10.0, 500.0, 800.0],
        "the bleed box falls back to it"
    );
    assert_eq!(box_at(&mut session, page, 3), vec![10.0, 10.0, 500.0, 800.0], "and so does the trim box");
}

/// A box is **cut back to the media box**: the tree's rule.
#[test]
fn a_box_is_cut_back_to_the_media_box() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [0, 0, 595, 842] }))
        .expect("the media box is set");
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "CropBox", "rect": [0, 0, 900, 900] }))
        .expect("the crop box is set");
    assert_eq!(box_at(&mut session, page, 1), vec![0.0, 0.0, 595.0, 842.0], "the crop box is the paper");
}

/// A rectangle that is **not four numbers** is refused.
#[test]
fn a_rectangle_that_is_not_four_numbers_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let error = session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [0, 0, 595] }))
        .expect_err("that is not a rectangle");
    assert!(error.to_string().contains("not four numbers"), "the error says so: {error}");
}

/// A page that would be **smaller than a point** is refused: the
/// tree's rule, and a page that cannot be seen is not a page.
#[test]
fn a_page_smaller_than_a_point_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let error = session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [0, 0, 0.5, 0.5] }))
        .expect_err("that page is smaller than a point");
    assert!(error.to_string().contains("smaller than a point"), "the error says so: {error}");
}

/// A box that is **not a box** is refused.
#[test]
fn a_box_that_is_not_a_box_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let error = session
        .execute("pdf.page.box", &json!({ "page": page, "which": "WobbleBox", "rect": [0, 0, 10, 10] }))
        .expect_err("there is no such box");
    assert!(error.to_string().contains("no box"), "the error says so: {error}");
}

/// The **corners** of a rectangle are put in order: a rectangle whose
/// lower corner is above its upper corner is the same rectangle the
/// other way up.
#[test]
fn the_corners_of_a_rectangle_are_put_in_order() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [595, 842, 0, 0] }))
        .expect("the media box is set");
    assert_eq!(box_at(&mut session, page, 0), vec![0.0, 0.0, 595.0, 842.0], "the corners are in order");
}

/// The boxes are **saved** with the page.
#[test]
fn the_boxes_are_saved_with_the_page() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("pdf.page.box", &json!({ "page": page, "which": "MediaBox", "rect": [0, 0, 595, 842] }))
        .expect("the media box is set");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(page).expect("the page is there");
    let entries = node.data["entries"].as_array().expect("the entries are there");
    let media =
        entries.iter().find(|entry| entry[0].as_str() == Some("MediaBox")).expect("the media box is there");
    assert_eq!(media[1].as_array().expect("the box is a rectangle").len(), 4, "the box is saved");
}
