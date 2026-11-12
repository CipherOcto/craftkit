//! The **borders** of a page.
//!
//! The style, the width and the colour of the border of each side of
//! the page: the tree's page borders.

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

/// A border on **every side** of the page, and the tree's own: single,
/// a point wide, and no colour of its own.
#[test]
fn a_border_is_put_on_every_side() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box" }))
        .expect("the borders are set");
    assert_eq!(outcome.data["borders"]["top"]["style"], json!("single"), "the border is single");
    assert_eq!(outcome.data["borders"]["top"]["width"], json!(1.0), "and it is a point wide");
    assert!(outcome.data["borders"]["left"].get("colour").is_none(), "and it has no colour of its own");
    let page = page_of(&mut session, document);
    assert_eq!(page["borders"]["bottom"]["style"], json!("single"), "and the page is read back with it");
}

/// A border is set **part by part**: the style, the width and the
/// colour.
#[test]
fn a_border_is_set_part_by_part() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.page.borders",
            &json!({
                "document": document,
                "kind": "box",
                "style": "double",
                "width": 2.5,
                "colour": "#112233",
            }),
        )
        .expect("the borders are set");
    let page = page_of(&mut session, document);
    assert_eq!(page["borders"]["top"]["style"], json!("double"), "the border is double");
    assert_eq!(page["borders"]["top"]["width"], json!(2.5), "and it is two and a half points wide");
    assert_eq!(page["borders"]["top"]["colour"], json!("#112233"), "and it is the colour that was given");
}

/// A **side** may be named, and the sides that are not named keep what
/// they have.
#[test]
fn a_side_may_be_named() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.page.borders",
            &json!({ "document": document, "kind": "box", "style": "dotted", "colour": "#445566" }),
        )
        .expect("the borders are set");
    session
        .execute(
            "prose.page.borders",
            &json!({ "document": document, "sides": { "top": { "style": "wave" } } }),
        )
        .expect("the side is set");
    let page = page_of(&mut session, document);
    assert_eq!(page["borders"]["top"]["style"], json!("wave"), "the side that was named has the new style");
    assert_eq!(page["borders"]["top"]["colour"], json!("#445566"), "and it keeps the colour it had");
    assert_eq!(page["borders"]["left"]["style"], json!("dotted"), "and the other sides keep what they had");
}

/// A side that is given **nothing** takes no border at all.
#[test]
fn a_side_that_is_given_nothing_takes_no_border() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box" }))
        .expect("the borders are set");
    session
        .execute("prose.page.borders", &json!({ "document": document, "sides": { "left": null } }))
        .expect("the side is set");
    let page = page_of(&mut session, document);
    assert!(page["borders"]["left"].is_null(), "the side has no border: {page}");
    assert_eq!(page["borders"]["top"]["style"], json!("single"), "and the other sides have theirs");
}

/// The kind **`none`** takes every border away, and the page then has
/// no borders at all.
#[test]
fn the_kind_none_takes_every_border_away() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box" }))
        .expect("the borders are set");
    session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "none" }))
        .expect("the borders are taken away");
    let page = page_of(&mut session, document);
    assert!(page["borders"].is_null(), "the page has no borders: {page}");
}

/// A **style** that is not one of the tree's, a **width** that is
/// outside the range, a **colour** that is not a colour and a **side**
/// that is not a side are all refused.
#[test]
fn a_border_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box", "style": "wavy" }))
        .expect_err("the style is refused");
    assert!(error.to_string().contains("no style"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box", "width": 12.0 }))
        .expect_err("the width is refused");
    assert!(error.to_string().contains("border is from"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.borders", &json!({ "document": document, "kind": "box", "colour": "blue" }))
        .expect_err("the colour is refused");
    assert!(error.to_string().contains('#'), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.page.borders",
            &json!({ "document": document, "sides": { "middle": { "style": "single" } } }),
        )
        .expect_err("the side is refused");
    assert!(error.to_string().contains("no side"), "the refusal says why: {error}");
    let page = page_of(&mut session, document);
    assert!(page["borders"].is_null(), "and the page has no borders: {page}");
}

/// A border that is **not named** is refused: a command that says
/// nothing changes nothing.
#[test]
fn a_border_that_is_not_named_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.borders", &json!({ "document": document }))
        .expect_err("the borders are refused");
    assert!(error.to_string().contains("named"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.page.borders",
            &json!({ "document": document, "kind": "box", "sides": { "top": null } }),
        )
        .expect_err("the borders are refused");
    assert!(error.to_string().contains("not both given"), "the refusal says why: {error}");
}
