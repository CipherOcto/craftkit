//! The **character** of a style, and the character of a block.

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

/// The **character** of a style.
fn character_of(session: &mut Session, style: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.style.character.read", &json!({ "style": style }))
        .expect("the character is read");
    outcome.data["character"].clone()
}

/// **The character of a style is set.**
#[test]
fn the_character_of_a_style_is_set() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    session
        .execute(
            "prose.style.character",
            &json!({ "style": style, "font": "Serif", "size": 18.0, "bold": true }),
        )
        .expect("the character is set");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Serif"));
    assert_eq!(character["size"], json!(18.0));
    assert_eq!(character["bold"], json!(true));
    assert!(character["colour"].is_null(), "and the colour is not said");
}

/// **A style that is based on another has the character of its base
/// under it.**
#[test]
fn a_style_has_the_character_of_its_base_under_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let base = a_style(&mut session, document, "the body", None);
    session
        .execute("prose.style.character", &json!({ "style": base, "font": "Serif", "size": 12.0 }))
        .expect("the character is set");
    let style = a_style(&mut session, document, "the heading", Some(base));
    session
        .execute("prose.style.character", &json!({ "style": style, "size": 18.0 }))
        .expect("the character is set");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Serif"), "the font is the base's");
    assert_eq!(character["size"], json!(18.0), "and the size is its own");
}

/// **The character that is not said stays as it was.**
#[test]
fn the_character_that_is_not_said_stays() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    session
        .execute("prose.style.character", &json!({ "style": style, "font": "Serif" }))
        .expect("the character is set");
    session
        .execute("prose.style.character", &json!({ "style": style, "size": 18.0 }))
        .expect("the character is set");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Serif"));
    assert_eq!(character["size"], json!(18.0));
}

/// **A colour that is not a colour is refused.**
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    let failure = session
        .execute("prose.style.character", &json!({ "style": style, "colour": "red" }))
        .expect_err("the colour is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a colour"), "the refusal says why: {message}");
}

/// **A size that is not a size is refused.**
#[test]
fn a_size_that_is_not_a_size_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    let failure = session
        .execute("prose.style.character", &json!({ "style": style, "size": 0.0 }))
        .expect_err("the size is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a size"), "the refusal says why: {message}");
}

/// **A font that is nothing is refused.**
#[test]
fn a_font_that_is_nothing_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    let failure = session
        .execute("prose.style.character", &json!({ "style": style, "font": "  " }))
        .expect_err("the font is refused");
    let message = failure.to_string();
    assert!(message.contains("a font is not nothing"), "the refusal says why: {message}");
}

/// **A command that changes nothing is refused.**
#[test]
fn a_command_that_changes_nothing_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    let failure = session
        .execute("prose.style.character", &json!({ "style": style }))
        .expect_err("the command is refused");
    let message = failure.to_string();
    assert!(message.contains("says nothing to change"), "the refusal says why: {message}");
}

/// **A node that is not a style is refused.**
#[test]
fn a_node_that_is_not_a_style_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let failure = session
        .execute("prose.style.character", &json!({ "style": document, "font": "Serif" }))
        .expect_err("the style is refused");
    let message = failure.to_string();
    assert!(message.contains("and not a style"), "the refusal says why: {message}");
}

/// **The character of a block is the character of its style.**
#[test]
fn the_character_of_a_block_is_the_character_of_its_style() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the heading", None);
    session
        .execute("prose.style.character", &json!({ "style": style, "font": "Serif", "size": 18.0 }))
        .expect("the character is set");
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.style", &json!({ "block": paragraph, "style": style }))
        .expect("the style is put on the block");
    let outcome = session
        .execute("prose.block.character", &json!({ "block": paragraph }))
        .expect("the character is read");
    assert_eq!(outcome.data["character"]["font"], json!("Serif"));
    assert_eq!(outcome.data["character"]["size"], json!(18.0));
}

/// **A block on no style has no character.**
#[test]
fn a_block_on_no_style_has_no_character() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let outcome = session
        .execute("prose.block.character", &json!({ "block": paragraph }))
        .expect("the character is read");
    assert!(outcome.data["style"].is_null());
    assert_eq!(outcome.data["character"], json!({}));
}
