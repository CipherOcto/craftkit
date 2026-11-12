//! The **outline** of a document: its headings, in the order they
//! stand.

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

/// A **style** in a document.
fn a_style(session: &mut Session, document: u64, name: &str, based_on: Option<u64>) -> u64 {
    let mut params = json!({ "document": document, "name": name });
    if let Some(base) = based_on {
        params["based_on"] = json!(base);
    }
    let outcome = session.execute("prose.style.add", &params).expect("the style is added");
    outcome.data["id"].as_u64().expect("the style has an id")
}

/// A **paragraph** in a document.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// A block **on** a style.
fn on_style(session: &mut Session, block: u64, style: u64) {
    session
        .execute("prose.block.style", &json!({ "block": block, "style": style }))
        .expect("the style is put on the block");
}

/// The **level** of a style.
fn level_of(session: &mut Session, style: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.style.level.read", &json!({ "style": style })).expect("the level is read");
    outcome.data["level"].clone()
}

/// The **outline** of a document.
fn outline_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("prose.document.outline", &json!({ "document": document }))
        .expect("the outline is read");
    outcome.data["outline"].as_array().expect("the outline is a list").clone()
}

/// **The level of a style may be set, and it is read back.**
#[test]
fn the_level_of_a_style_may_be_set() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "Heading 1", None);
    session.execute("prose.style.level", &json!({ "style": style, "level": 1 })).expect("the level is set");
    assert_eq!(level_of(&mut session, style), json!(1));
}

/// **A style with no level is not a heading.**
#[test]
fn a_style_with_no_level_is_not_a_heading() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "Body", None);
    assert_eq!(level_of(&mut session, style), json!(null));
}

/// **A style has the level of its base under it.**
#[test]
fn a_style_has_the_level_of_its_base_under_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let base = a_style(&mut session, document, "Heading 1", None);
    let style = a_style(&mut session, document, "Subheading", Some(base));
    session
        .execute("prose.style.level", &json!({ "style": base, "level": 1 }))
        .expect("the level of the base is set");
    assert_eq!(level_of(&mut session, style), json!(1), "the style based on a heading is a heading");
    session.execute("prose.style.level", &json!({ "style": style, "level": 2 })).expect("the level is set");
    assert_eq!(level_of(&mut session, style), json!(2), "and the nearer style wins");
}

/// **A level that is not a level is refused.**
#[test]
fn a_level_that_is_not_a_level_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "Heading 1", None);
    for level in [0.0, 10.0, 1.5] {
        let outcome = session.execute("prose.style.level", &json!({ "style": style, "level": level }));
        assert!(outcome.is_err(), "the level {level} is refused");
    }
}

/// **The outline is the headings of the document, in order.**
#[test]
fn the_outline_is_the_headings_in_order() {
    let mut session = session();
    let document = a_document(&mut session);
    let heading = a_style(&mut session, document, "Heading 1", None);
    let body = a_style(&mut session, document, "Body", None);
    session.execute("prose.style.level", &json!({ "style": heading, "level": 1 })).expect("the level is set");
    let first = a_paragraph(&mut session, document, "the first heading");
    let text = a_paragraph(&mut session, document, "just text");
    let second = a_paragraph(&mut session, document, "the second heading");
    on_style(&mut session, first, heading);
    on_style(&mut session, text, body);
    on_style(&mut session, second, heading);
    let outline = outline_of(&mut session, document);
    assert_eq!(outline.len(), 2, "the text is not a heading");
    assert_eq!(outline[0]["text"], json!("the first heading"));
    assert_eq!(outline[1]["text"], json!("the second heading"));
    assert_eq!(outline[0]["level"], json!(1));
}

/// **A block on no style is not in the outline.**
#[test]
fn a_block_on_no_style_is_not_in_the_outline() {
    let mut session = session();
    let document = a_document(&mut session);
    let heading = a_style(&mut session, document, "Heading 1", None);
    session.execute("prose.style.level", &json!({ "style": heading, "level": 1 })).expect("the level is set");
    let block = a_paragraph(&mut session, document, "a heading that is on no style");
    on_style(&mut session, block, heading);
    let plain = a_paragraph(&mut session, document, "just text");
    let _ = plain;
    assert_eq!(outline_of(&mut session, document).len(), 1);
}

/// **The level may be taken away.**
#[test]
fn the_level_may_be_taken_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "Heading 1", None);
    session.execute("prose.style.level", &json!({ "style": style, "level": 1 })).expect("the level is set");
    session.execute("prose.style.level", &json!({ "style": style })).expect("the level is taken away");
    assert_eq!(level_of(&mut session, style), json!(null));
}

/// **A node that is not a style is refused.**
#[test]
fn a_node_that_is_not_a_style_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let block = a_paragraph(&mut session, document, "the text");
    let outcome = session.execute("prose.style.level", &json!({ "style": block, "level": 1 }));
    assert!(outcome.is_err(), "a paragraph is not a style");
}
