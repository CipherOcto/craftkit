//! The **citation styles** of a document.
//!
//! A document names the style it cites by, and the styles are the
//! tree's own.

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

/// A **document** of its own.
fn a_document(session: &mut Session, name: &str) -> u64 {
    let outcome =
        session.execute("prose.document.add", &json!({ "name": name })).expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// The **styles** of the tree, as they are listed.
fn styles_of(session: &mut Session) -> Vec<String> {
    let outcome = session.execute("prose.citation.styles", &json!({})).expect("the styles are read");
    outcome.data["styles"]
        .as_array()
        .map(|styles| {
            styles
                .iter()
                .filter_map(|style| {
                    style.get("name").and_then(|name| name.as_str()).map(|name| name.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **styles** of the tree are the tree's own twelve.
#[test]
fn the_styles_of_the_tree_are_listed() {
    let mut session = session();
    let styles = styles_of(&mut session);
    assert_eq!(styles.len(), 12, "the tree has twelve styles: {styles:?}");
    assert_eq!(styles.first().map(|name| name.as_str()), Some("APA"), "and APA is the first");
    assert!(styles.iter().any(|name| name == "MLA"), "MLA is one of them: {styles:?}");
    assert!(styles.iter().any(|name| name == "GB/T 7714"), "and GB/T 7714 is one of them: {styles:?}");
}

/// A document that names **no** style cites by **APA**: the tree's own
/// default.
#[test]
fn a_document_that_names_no_style_cites_by_apa() {
    let mut session = session();
    let document = a_document(&mut session, "the document");
    let outcome = session
        .execute("prose.document.citestyle.read", &json!({ "document": document }))
        .expect("the style is read");
    assert_eq!(outcome.data["style"]["name"], json!("APA"), "the default is APA: {outcome:?}");
}

/// The style of a document is **said**, and it is said by the name of
/// the style.
#[test]
fn the_style_of_a_document_is_said() {
    let mut session = session();
    let document = a_document(&mut session, "the document");
    session
        .execute("prose.document.citestyle.set", &json!({ "document": document, "style": "MLA" }))
        .expect("the style is said");
    let outcome = session
        .execute("prose.document.citestyle.read", &json!({ "document": document }))
        .expect("the style is read");
    assert_eq!(outcome.data["style"]["name"], json!("MLA"), "the document cites by MLA: {outcome:?}");
    assert_eq!(outcome.data["style"]["bib_title"], json!("Works Cited"), "and MLA's list is a works cited");
}

/// A style that is **not one of the tree's** is refused by name, and
/// the refusal names the styles there are.
#[test]
fn a_style_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let document = a_document(&mut session, "the document");
    let error = session
        .execute("prose.document.citestyle.set", &json!({ "document": document, "style": "Vancouver" }))
        .expect_err("the style is refused");
    let message = error.to_string();
    assert!(message.contains("Vancouver"), "the refusal names the style: {message}");
    assert!(message.contains("APA"), "and it names the styles there are: {message}");
}

/// A name that **begins** with an alias names the style: the tree's own
/// rule, and the name of a style sheet file names it too.
#[test]
fn an_alias_names_a_style() {
    let mut session = session();
    let document = a_document(&mut session, "the document");
    session
        .execute("prose.document.citestyle.set", &json!({ "document": document, "style": "\\GostTitle.XSL" }))
        .expect("the style is said");
    let outcome = session
        .execute("prose.document.citestyle.read", &json!({ "document": document }))
        .expect("the style is read");
    assert_eq!(
        outcome.data["style"]["name"],
        json!("GOST (title sort)"),
        "the name of the style sheet names the style: {outcome:?}"
    );
}
