//! The **paragraph** of a style, and the paragraph of a block.

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

/// The **paragraph** of a style.
fn paragraph_of(session: &mut Session, style: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.style.paragraph.read", &json!({ "style": style }))
        .expect("the paragraph is read");
    outcome.data["paragraph"].clone()
}

/// **The paragraph of a style is set.**
#[test]
fn the_paragraph_of_a_style_is_set() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    session
        .execute(
            "prose.style.paragraph",
            &json!({
                "style": style,
                "alignment": "centre",
                "space_before": 6.0,
                "space_after": 12.0,
                "line_spacing": 1.5,
                "indent_left": 36.0,
            }),
        )
        .expect("the paragraph is set");
    let paragraph = paragraph_of(&mut session, style);
    assert_eq!(paragraph["alignment"], json!("centre"));
    assert_eq!(paragraph["space_before"], json!(6.0));
    assert_eq!(paragraph["space_after"], json!(12.0));
    assert_eq!(paragraph["line_spacing"], json!(1.5));
    assert_eq!(paragraph["indent_left"], json!(36.0));
    assert!(paragraph["indent_right"].is_null(), "and the indent at the right is not said");
}

/// **The paragraph that is not said stays as it was.**
#[test]
fn the_paragraph_that_is_not_said_stays() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    session
        .execute("prose.style.paragraph", &json!({ "style": style, "alignment": "right" }))
        .expect("the alignment is set");
    session
        .execute("prose.style.paragraph", &json!({ "style": style, "space_after": 3.0 }))
        .expect("the space is set");
    let paragraph = paragraph_of(&mut session, style);
    assert_eq!(paragraph["alignment"], json!("right"), "the alignment is still there");
    assert_eq!(paragraph["space_after"], json!(3.0));
}

/// **A style has the paragraph of its base under it.**
#[test]
fn a_style_has_the_paragraph_of_its_base_under_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let base = a_style(&mut session, document, "the base", None);
    let style = a_style(&mut session, document, "the style", Some(base));
    session
        .execute(
            "prose.style.paragraph",
            &json!({ "style": base, "alignment": "justify", "indent_left": 20.0 }),
        )
        .expect("the paragraph of the base is set");
    session
        .execute("prose.style.paragraph", &json!({ "style": style, "indent_left": 40.0 }))
        .expect("the paragraph of the style is set");
    let paragraph = paragraph_of(&mut session, style);
    assert_eq!(paragraph["alignment"], json!("justify"), "the alignment of the base is under it");
    assert_eq!(paragraph["indent_left"], json!(40.0), "and the nearer style wins");
}

/// **The first line may hang.**
#[test]
fn the_first_line_may_hang() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    session
        .execute("prose.style.paragraph", &json!({ "style": style, "indent_first": -18.0 }))
        .expect("the paragraph is set");
    assert_eq!(paragraph_of(&mut session, style)["indent_first"], json!(-18.0));
}

/// **An alignment that is not one is refused.**
#[test]
fn an_alignment_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    let outcome = session.execute("prose.style.paragraph", &json!({ "style": style, "alignment": "middle" }));
    assert!(outcome.is_err(), "`middle` is not an alignment");
}

/// **A space that is not a space is refused.**
#[test]
fn a_space_that_is_not_a_space_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    let outcome = session.execute("prose.style.paragraph", &json!({ "style": style, "space_before": -6.0 }));
    assert!(outcome.is_err(), "a space below zero is not a space");
}

/// **A line spacing that is not one is refused.**
#[test]
fn a_line_spacing_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    let outcome = session.execute("prose.style.paragraph", &json!({ "style": style, "line_spacing": 0.0 }));
    assert!(outcome.is_err(), "a line spacing of zero is not a line spacing");
}

/// **A command that says nothing is refused.**
#[test]
fn a_command_that_says_nothing_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    let outcome = session.execute("prose.style.paragraph", &json!({ "style": style }));
    assert!(outcome.is_err(), "a command that says nothing changes nothing");
}

/// **The paragraph of a block is the paragraph of its style.**
#[test]
fn the_paragraph_of_a_block_is_the_paragraph_of_its_style() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the body", None);
    let block = a_paragraph(&mut session, document, "the text");
    session
        .execute("prose.block.style", &json!({ "block": block, "style": style }))
        .expect("the style is put on the block");
    session
        .execute("prose.style.paragraph", &json!({ "style": style, "alignment": "centre" }))
        .expect("the paragraph is set");
    let outcome =
        session.execute("prose.block.paragraph", &json!({ "block": block })).expect("the paragraph is read");
    assert_eq!(outcome.data["paragraph"]["alignment"], json!("centre"));
    assert_eq!(outcome.data["style"], json!(style));
}

/// **A block on no style has no paragraph.**
#[test]
fn a_block_on_no_style_has_no_paragraph() {
    let mut session = session();
    let document = a_document(&mut session);
    let block = a_paragraph(&mut session, document, "the text");
    let outcome =
        session.execute("prose.block.paragraph", &json!({ "block": block })).expect("the paragraph is read");
    assert!(outcome.data["style"].is_null());
    assert!(outcome.data["paragraph"].as_object().expect("an object").is_empty());
}

/// **A node that is not a style is refused.**
#[test]
fn a_node_that_is_not_a_style_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let block = a_paragraph(&mut session, document, "the text");
    let outcome = session
        .execute("prose.style.paragraph", &json!({ "style": block, "alignment": "left" }))
        .expect_err("a paragraph is not a style");
    assert!(outcome.to_string().contains("not a style"));
}
