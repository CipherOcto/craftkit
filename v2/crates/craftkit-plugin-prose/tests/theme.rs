//! The **theme** of a document.
//!
//! The fonts of the headings and of the body, and the accent colour
//! the headings are drawn in — under every style that says nothing of
//! its own.

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

/// A **document** with a **style** of the given name.
fn a_document_with_a_style(session: &mut Session, name: &str) -> (u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": name }))
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    (document, style)
}

/// The **character** of a style, as the command reads it.
fn character_of(session: &mut Session, style: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.style.character.read", &json!({ "style": style }))
        .expect("the character is read");
    outcome.data["character"].clone()
}

/// The theme is said, and the part that is **not named** keeps what
/// it was.
#[test]
fn the_theme_is_said() {
    let mut session = session();
    let (document, _style) = a_document_with_a_style(&mut session, "Heading 1");
    let outcome = session
        .execute(
            "prose.document.theme",
            &json!({ "document": document, "major": "Georgia", "minor": "Garamond" }),
        )
        .expect("the theme is said");
    assert_eq!(outcome.data["theme"]["major"], json!("Georgia"), "the major font is said");
    assert_eq!(outcome.data["theme"]["minor"], json!("Garamond"), "and the minor font");
    session
        .execute("prose.document.theme", &json!({ "document": document, "accent": "#112233" }))
        .expect("the theme is said");
    let outcome =
        session.execute("prose.document.theme", &json!({ "document": document })).expect("the theme is read");
    assert_eq!(outcome.data["theme"]["major"], json!("Georgia"), "the major font is kept");
    assert_eq!(outcome.data["theme"]["minor"], json!("Garamond"), "and so is the minor font");
    assert_eq!(outcome.data["theme"]["accent"], json!("#112233"), "and the accent is said");
}

/// The theme is **under** the styles: a style that says nothing of
/// its own takes the font of the theme.
#[test]
fn a_style_takes_the_font_of_the_theme() {
    let mut session = session();
    let (document, style) = a_document_with_a_style(&mut session, "Body");
    session
        .execute(
            "prose.document.theme",
            &json!({ "document": document, "major": "Georgia", "minor": "Garamond" }),
        )
        .expect("the theme is said");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Garamond"), "the body takes the minor font: {character}");
}

/// A **heading** takes the major font, and the accent colour when it
/// says no colour of its own.
#[test]
fn a_heading_takes_the_major_font_and_the_accent() {
    let mut session = session();
    let (document, style) = a_document_with_a_style(&mut session, "Heading 1");
    session
        .execute(
            "prose.document.theme",
            &json!({
                "document": document,
                "major": "Georgia",
                "minor": "Garamond",
                "accent": "#112233",
            }),
        )
        .expect("the theme is said");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Georgia"), "the heading takes the major font: {character}");
    assert_eq!(character["colour"], json!("#112233"), "and the accent colour");
}

/// A style that says a **font of its own** keeps it, and so does a
/// heading that says a **colour of its own**.
#[test]
fn a_style_that_says_its_own_keeps_it() {
    let mut session = session();
    let (document, style) = a_document_with_a_style(&mut session, "Heading 1");
    session
        .execute(
            "prose.document.theme",
            &json!({
                "document": document,
                "major": "Georgia",
                "minor": "Garamond",
                "accent": "#112233",
            }),
        )
        .expect("the theme is said");
    session
        .execute(
            "prose.style.character",
            &json!({ "style": style, "font": "Iowan Old Style", "colour": "#445566" }),
        )
        .expect("the character is set");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Iowan Old Style"), "the font of the style is kept");
    assert_eq!(character["colour"], json!("#445566"), "and so is its colour");
}

/// A theme that is **changed** is felt by every style that says
/// nothing of its own.
#[test]
fn a_theme_that_is_changed_is_felt() {
    let mut session = session();
    let (document, style) = a_document_with_a_style(&mut session, "Body");
    session
        .execute("prose.document.theme", &json!({ "document": document, "minor": "Garamond" }))
        .expect("the theme is said");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Garamond"), "the body takes the font: {character}");
    session
        .execute("prose.document.theme", &json!({ "document": document, "minor": "Palatino" }))
        .expect("the theme is said");
    let character = character_of(&mut session, style);
    assert_eq!(character["font"], json!("Palatino"), "and the new one: {character}");
}

/// A font that is **empty** is refused, an **empty** colour takes the
/// accent away, and a node that is **not a document** is refused.
#[test]
fn a_theme_that_is_not_one_is_refused() {
    let mut session = session();
    let (document, _style) = a_document_with_a_style(&mut session, "Body");
    let error = session
        .execute("prose.document.theme", &json!({ "document": document, "major": "  " }))
        .expect_err("the empty font is refused");
    assert!(error.to_string().contains("not nothing"), "the refusal says why: {error}");
    session
        .execute("prose.document.theme", &json!({ "document": document, "accent": "#112233" }))
        .expect("the theme is said");
    session
        .execute("prose.document.theme", &json!({ "document": document, "accent": "" }))
        .expect("the theme is said");
    let outcome =
        session.execute("prose.document.theme", &json!({ "document": document })).expect("the theme is read");
    assert!(outcome.data["theme"]["accent"].is_null(), "and the accent is taken away: {outcome:?}");
    let error = session
        .execute("prose.document.theme", &json!({ "document": _style }))
        .expect_err("the style is not a document");
    assert!(error.to_string().contains("not a document"), "the refusal says why: {error}");
}
