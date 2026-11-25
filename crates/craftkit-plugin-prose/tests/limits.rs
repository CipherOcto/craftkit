//! The **limits of the prose battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// A document with a **paragraph** in it, and the block's id.
fn a_paragraph(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a paragraph" }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// A field **code** over `MAX_CODE` long is refused.
///
/// The code of a field is an instruction, and an instruction of any
/// length at all would be a hole in the document.
#[test]
fn a_field_code_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let code = "c".repeat(4097);
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 0, "end": 1, "code": code }))
        .expect_err("that code is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **space** over `MAX_PARAGRAPH_SPACE` is refused.
///
/// The room above and below a paragraph is a number from zero to the
/// limit, and a number outside it is not a space.
#[test]
fn a_space_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "space_before": 10_001.0 }))
        .expect_err("that space is too big");
    assert!(error.to_string().contains("a number from zero to"), "{error}");
}

/// A **line spacing** over `MAX_LINE_SPACING` is refused.
#[test]
fn a_line_spacing_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "line_spacing": 101.0 }))
        .expect_err("that spacing is too big");
    assert!(error.to_string().contains("from zero to"), "{error}");
}

/// A bookmark **name** over `MAX_BOOKMARK_NAME` is refused.
#[test]
fn a_bookmark_name_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let name = "b".repeat(41);
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": block, "name": name }))
        .expect_err("that name is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **style** in a document, and its id.
fn a_style(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the style" }))
        .expect("the style is added");
    outcome.data["id"].as_u64().expect("the style has an id")
}
