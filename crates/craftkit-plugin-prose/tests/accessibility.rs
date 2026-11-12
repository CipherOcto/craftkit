//! The **accessibility** of a document.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **picture** in a document, and its id.
fn a_picture(session: &mut Session, document: u64) -> u64 {
    let outcome = session
        .execute(
            "prose.media.add",
            &json!({ "document": document, "name": "the picture", "path": "pictures/one.png" }),
        )
        .expect("the picture is added");
    outcome.data["id"].as_u64().expect("the picture has an id")
}

/// A **heading**: a paragraph on a style with a level.
fn a_heading(session: &mut Session, document: u64, text: &str, level: u64) -> u64 {
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": format!("heading{level}") }))
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    session
        .execute("prose.style.level", &json!({ "style": style, "level": level }))
        .expect("the level is set");
    let paragraph = a_paragraph(session, document, text);
    session
        .execute("prose.block.style", &json!({ "block": paragraph, "style": style }))
        .expect("the paragraph is on the style");
    paragraph
}

/// The **issues** of a document, as the checker holds them.
fn issues_of(session: &mut Session, document: u64) -> Vec<Json> {
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    outcome.data["issues"].as_array().expect("the issues are a list").clone()
}

/// The **rules** the document breaks.
fn rules_of(issues: &[Json]) -> Vec<String> {
    issues.iter().map(|issue| issue["rule"].as_str().unwrap_or_default().to_string()).collect()
}

/// A picture that says nothing about itself is an **error**.
#[test]
fn a_picture_that_says_nothing_about_itself_is_an_error() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    let issues = issues_of(&mut session, document);
    assert!(rules_of(&issues).contains(&"altText".to_string()), "the rule is broken: {issues:?}");
    assert_eq!(issues[0]["kind"], json!("error"), "and it is an error");
    assert_eq!(issues[0]["where"], json!(picture), "and it is about the picture");
}

/// A picture that says what it is is **not** an error.
#[test]
fn a_picture_that_says_what_it_is_is_not_an_error() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "a red door" }))
        .expect("the alt text is put");
    let issues = issues_of(&mut session, document);
    assert!(!rules_of(&issues).contains(&"altText".to_string()), "the picture says what it is: {issues:?}");
}

/// A heading that is **too deep** is a warning.
#[test]
fn a_heading_that_is_too_deep_is_a_warning() {
    let mut session = session();
    let document = a_document(&mut session);
    a_heading(&mut session, document, "the first heading", 1);
    a_heading(&mut session, document, "the third heading", 3);
    let issues = issues_of(&mut session, document);
    assert!(rules_of(&issues).contains(&"headingOrder".to_string()), "the rule is broken: {issues:?}");
    let issue =
        issues.iter().find(|issue| issue["rule"] == json!("headingOrder")).expect("the rule is broken");
    assert_eq!(issue["kind"], json!("warning"), "it is a warning");
}

/// A heading that is **one level deeper** is not a warning.
#[test]
fn a_heading_that_is_one_level_deeper_is_not_a_warning() {
    let mut session = session();
    let document = a_document(&mut session);
    a_heading(&mut session, document, "the first heading", 1);
    a_heading(&mut session, document, "the second heading", 2);
    let issues = issues_of(&mut session, document);
    assert!(!rules_of(&issues).contains(&"headingOrder".to_string()), "the outline has no gap: {issues:?}");
}

/// **Empty paragraphs** in a row are a tip.
#[test]
fn empty_paragraphs_in_a_row_are_a_tip() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "");
    a_paragraph(&mut session, document, "");
    a_paragraph(&mut session, document, "");
    let issues = issues_of(&mut session, document);
    assert!(rules_of(&issues).contains(&"blankParagraphs".to_string()), "the rule is broken: {issues:?}");
    let issue =
        issues.iter().find(|issue| issue["rule"] == json!("blankParagraphs")).expect("the rule is broken");
    assert_eq!(issue["kind"], json!("tip"), "it is a tip");
}

/// **Two** empty paragraphs in a row are not a tip.
#[test]
fn two_empty_paragraphs_in_a_row_are_not_a_tip() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "");
    a_paragraph(&mut session, document, "");
    let issues = issues_of(&mut session, document);
    assert!(!rules_of(&issues).contains(&"blankParagraphs".to_string()), "two is not a row: {issues:?}");
}

/// The words of a link that say **nothing** are a tip.
#[test]
fn the_words_of_a_link_that_say_nothing_are_a_tip() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "click here");
    session
        .execute(
            "prose.block.runs",
            &json!({ "block": paragraph, "runs": [{ "len": 10, "link": "https://example.com" }] }),
        )
        .expect("the runs are set");
    let issues = issues_of(&mut session, document);
    assert!(rules_of(&issues).contains(&"linkText".to_string()), "the rule is broken: {issues:?}");
}

/// The words of a link that say **something** are not a tip.
#[test]
fn the_words_of_a_link_that_say_something_are_not_a_tip() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the red door");
    session
        .execute(
            "prose.block.runs",
            &json!({ "block": paragraph, "runs": [{ "len": 11, "link": "https://example.com" }] }),
        )
        .expect("the runs are set");
    let issues = issues_of(&mut session, document);
    assert!(!rules_of(&issues).contains(&"linkText".to_string()), "the words say something: {issues:?}");
}

/// The **errors** come before the warnings and the tips.
#[test]
fn the_errors_come_before_the_warnings_and_the_tips() {
    let mut session = session();
    let document = a_document(&mut session);
    a_picture(&mut session, document);
    a_paragraph(&mut session, document, "");
    a_paragraph(&mut session, document, "");
    a_paragraph(&mut session, document, "");
    let issues = issues_of(&mut session, document);
    let kinds: Vec<String> =
        issues.iter().map(|issue| issue["kind"].as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(kinds.first().map(|kind| kind.as_str()), Some("error"), "the error comes first: {issues:?}");
}

/// A node that is **not a document** is refused.
#[test]
fn a_node_that_is_not_a_document_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    let refused = session
        .execute("prose.accessibility", &json!({ "document": paragraph }))
        .expect_err("the check is refused");
    assert!(refused.to_string().contains("not a document"), "the refusal says why: {refused}");
}
