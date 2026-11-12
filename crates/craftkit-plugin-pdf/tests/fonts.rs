//! The **fonts** of a PDF file.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
}

/// The **preflight** of the document.
fn preflight(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.preflight", &json!({})).expect("the preflight is done");
    outcome.data.clone()
}

/// The **clauses** the document breaks.
fn clauses(preflight: &Json) -> Vec<String> {
    preflight["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .map(|issue| issue["clause"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The **fonts** the file says it has.
fn fonts(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("pdf.fonts", &json!({})).expect("the fonts are read");
    outcome.data["fonts"].as_array().expect("the fonts are a list").clone()
}

/// A font of the **Type 1**, with a descriptor that has no program.
fn a_font_without_a_program(session: &mut Session) {
    add_object(session, 3, json!([["Type", "FontDescriptor"], ["FontName", "Helvetica"]]));
    add_object(
        session,
        2,
        json!([["Type", "Font"], ["Subtype", "Type1"], ["BaseFont", "Helvetica"], ["FontDescriptor", { "ref": [3, 0] }]]),
    );
}

/// A font of the **Type 1**, with its program in the file.
fn a_font_with_a_program(session: &mut Session) {
    add_object(
        session,
        3,
        json!([["Type", "FontDescriptor"], ["FontName", "Helvetica"], ["FontFile", { "ref": [4, 0] }]]),
    );
    add_object(
        session,
        2,
        json!([["Type", "Font"], ["Subtype", "Type1"], ["BaseFont", "Helvetica"], ["FontDescriptor", { "ref": [3, 0] }]]),
    );
}

/// A font that is **not embedded** breaks the rule.
#[test]
fn a_font_that_is_not_embedded_breaks_the_rule() {
    let mut session = session();
    a_font_without_a_program(&mut session);
    let preflight = preflight(&mut session);
    assert!(clauses(&preflight).contains(&"6.2.11.4".to_string()), "the rule is broken: {preflight}");
}

/// The rule says **which font** it is.
#[test]
fn the_rule_says_which_font_it_is() {
    let mut session = session();
    a_font_without_a_program(&mut session);
    let preflight = preflight(&mut session);
    let issues = preflight["issues"].as_array().expect("the issues are a list");
    let issue =
        issues.iter().find(|issue| issue["clause"] == json!("6.2.11.4")).expect("the font rule is broken");
    assert!(
        issue["message"].as_str().unwrap_or_default().contains("Helvetica"),
        "the font is named: {issue}"
    );
}

/// A font that **is embedded** breaks nothing.
#[test]
fn a_font_that_is_embedded_breaks_nothing() {
    let mut session = session();
    a_font_with_a_program(&mut session);
    let preflight = preflight(&mut session);
    assert!(!clauses(&preflight).contains(&"6.2.11.4".to_string()), "the font is embedded: {preflight}");
}

/// A composite font is embedded when the font it descends from is.
#[test]
fn a_composite_font_is_embedded_when_the_font_it_descends_from_is() {
    let mut session = session();
    add_object(
        &mut session,
        4,
        json!([["Type", "FontDescriptor"], ["FontName", "Helvetica"], ["FontFile2", { "ref": [5, 0] }]]),
    );
    add_object(
        &mut session,
        3,
        json!([["Type", "Font"], ["Subtype", "CIDFontType2"], ["BaseFont", "Helvetica"], ["FontDescriptor", { "ref": [4, 0] }]]),
    );
    add_object(
        &mut session,
        2,
        json!([["Type", "Font"], ["Subtype", "Type0"], ["BaseFont", "Helvetica"], ["DescendantFonts", [{ "ref": [3, 0] }]]]),
    );
    let preflight = preflight(&mut session);
    assert!(!clauses(&preflight).contains(&"6.2.11.4".to_string()), "the font is embedded: {preflight}");
}

/// A composite font without a program breaks the rule.
#[test]
fn a_composite_font_without_a_program_breaks_the_rule() {
    let mut session = session();
    add_object(
        &mut session,
        3,
        json!([["Type", "Font"], ["Subtype", "CIDFontType2"], ["BaseFont", "Helvetica"]]),
    );
    add_object(
        &mut session,
        2,
        json!([["Type", "Font"], ["Subtype", "Type0"], ["BaseFont", "Helvetica"], ["DescendantFonts", [{ "ref": [3, 0] }]]]),
    );
    let preflight = preflight(&mut session);
    assert!(clauses(&preflight).contains(&"6.2.11.4".to_string()), "the rule is broken: {preflight}");
}

/// A **Type 3** font is its own program, and it is embedded.
#[test]
fn a_type_3_font_is_its_own_program_and_it_is_embedded() {
    let mut session = session();
    add_object(&mut session, 2, json!([["Type", "Font"], ["Subtype", "Type3"], ["BaseFont", "MyGlyphs"]]));
    let preflight = preflight(&mut session);
    assert!(!clauses(&preflight).contains(&"6.2.11.4".to_string()), "the font is embedded: {preflight}");
    let fonts = fonts(&mut session);
    assert_eq!(fonts[0]["embedded"], json!(true), "and it says so");
}

/// The **fonts** are read.
#[test]
fn the_fonts_are_read() {
    let mut session = session();
    a_font_with_a_program(&mut session);
    let fonts = fonts(&mut session);
    assert_eq!(fonts.len(), 1, "there is one font");
    assert_eq!(fonts[0]["base"], json!("Helvetica"), "the base font is named");
    assert_eq!(fonts[0]["subtype"], json!("Type1"), "and the subtype");
    assert_eq!(fonts[0]["embedded"], json!(true), "and whether it is embedded");
}

/// The **preflight** says what fonts the file has.
#[test]
fn the_preflight_says_what_fonts_the_file_has() {
    let mut session = session();
    a_font_without_a_program(&mut session);
    let preflight = preflight(&mut session);
    let fonts = preflight["fonts"].as_array().expect("the fonts are a list");
    assert_eq!(fonts.len(), 1, "the font is there");
    assert_eq!(fonts[0]["base"], json!("Helvetica"), "and it is named");
}
