//! The **line spacing** of a paragraph.
//!
//! The tree's spacing is one of three things: a multiple of single
//! spacing, a minimum number of points, or an exact number of points.

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

/// A **paragraph** in a document, and its id.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// The **line spacing** of a paragraph.
fn spacing_of(session: &Session, paragraph: u64) -> Option<(String, f64)> {
    let node = session.document().node(paragraph).expect("the paragraph is there");
    let spacing = node.data.get("line_spacing")?;
    let kind = spacing["kind"].as_str()?.to_string();
    let value = spacing["value"].as_f64()?;
    Some((kind, value))
}

/// The lines of a paragraph may be spaced by a **multiple** of single
/// spacing.
#[test]
fn the_lines_may_be_spaced_by_a_multiple() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "multiple": 1.5 }))
        .expect("the spacing is set");
    assert_eq!(spacing_of(&session, paragraph), Some(("multiple".to_string(), 1.5)), "the paragraph says so");
}

/// The lines of a paragraph may be spaced by a **minimum** number of
/// points.
#[test]
fn the_lines_may_be_spaced_by_a_minimum() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "at_least": 15 }))
        .expect("the spacing is set");
    assert_eq!(
        spacing_of(&session, paragraph),
        Some(("at_least".to_string(), 15.0)),
        "the paragraph says so"
    );
}

/// And by an **exact** number of points.
#[test]
fn the_lines_may_be_spaced_by_an_exact_number() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "exactly": 14 }))
        .expect("the spacing is set");
    assert_eq!(spacing_of(&session, paragraph), Some(("exactly".to_string(), 14.0)), "the paragraph says so");
}

/// A spacing that is **nothing** is refused: a line spacing is one of
/// the three.
#[test]
fn a_spacing_of_nothing_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph }))
        .expect_err("the spacing is refused");
    assert!(
        error.to_string().contains("a multiple, a minimum or an exact number of points"),
        "the error says so: {error}"
    );
}

/// A spacing that is **more than one** of the three is refused.
#[test]
fn a_spacing_of_more_than_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute(
            "prose.block.paragraph.linespacing",
            &json!({ "block": paragraph, "multiple": 1.5, "exactly": 14 }),
        )
        .expect_err("the spacing is refused");
    assert!(
        error.to_string().contains("one of the three and not more than one"),
        "the error says so: {error}"
    );
}

/// A multiple outside the tree's own range is **refused**: the tree
/// clamps it and says nothing.
#[test]
fn a_multiple_out_of_range_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "multiple": 200 }))
        .expect_err("the spacing is refused");
    assert!(
        error.to_string().contains("a multiple of single spacing is between"),
        "the error says so: {error}"
    );
    assert_eq!(spacing_of(&session, paragraph), None, "and nothing was changed");
}

/// A minimum outside the tree's own range is refused, and so is an
/// exact number below the tree's own smallest.
#[test]
fn a_minimum_and_an_exact_out_of_range_are_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let error = session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "at_least": 5000 }))
        .expect_err("the spacing is refused");
    assert!(error.to_string().contains("a minimum of points is between"), "the error says so: {error}");
    let error = session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "exactly": 0.2 }))
        .expect_err("the spacing is refused");
    assert!(error.to_string().contains("an exact number of points is between"), "the error says so: {error}");
}

/// A **table** is not a paragraph, and it may not be given a spacing.
#[test]
fn a_table_is_not_a_paragraph() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [[1, 2]] }))
        .expect("the table is added");
    let table = outcome.data["id"].as_u64().expect("the table has an id");
    let error = session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": table, "multiple": 1.5 }))
        .expect_err("the spacing is refused");
    assert!(error.to_string().contains("not a paragraph"), "the error says so: {error}");
}

/// The spacing is **saved**: it is in the document after it is done.
#[test]
fn the_spacing_is_saved() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.paragraph.linespacing", &json!({ "block": paragraph, "multiple": 2 }))
        .expect("the spacing is set");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(paragraph).expect("the paragraph is in the document");
    assert_eq!(node.data["line_spacing"]["kind"].as_str(), Some("multiple"), "the kind is there");
    assert_eq!(node.data["line_spacing"]["value"].as_f64(), Some(2.0), "and the value");
}
