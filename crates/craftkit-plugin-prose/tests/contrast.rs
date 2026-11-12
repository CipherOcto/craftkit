//! The **contrast** of the text.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Colour, Ecosystem, Session};

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

/// The **rules** the document breaks.
fn rules_of(session: &mut Session, document: u64) -> Vec<String> {
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .map(|issue| issue["rule"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The **kinds** of the issues of a document.
fn kinds_of(session: &mut Session, document: u64) -> Vec<String> {
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .map(|issue| issue["kind"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// Black on white is the ratio of **twenty-one**.
#[test]
fn black_on_white_is_the_ratio_of_twenty_one() {
    let ratio = craftkit_plugin_prose::contrast::contrast_ratio(&Colour::BLACK, &Colour::WHITE);
    assert!((ratio - 21.0).abs() < 0.01, "the ratio is {ratio}");
}

/// Grey on white is the ratio the tree measures.
#[test]
fn grey_on_white_is_the_ratio_the_tree_measures() {
    let grey = Colour::from_hex("#777777").expect("the colour is one");
    let ratio = craftkit_plugin_prose::contrast::contrast_ratio(&grey, &Colour::WHITE);
    assert!((ratio - 4.48).abs() < 0.01, "the ratio is {ratio}");
}

/// Text that is **hard to read** is a warning.
#[test]
fn text_that_is_hard_to_read_is_a_warning() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute(
            "prose.run.format",
            &json!({ "block": paragraph, "start": 0.0, "length": 9.0, "colour": "#bbbbbb" }),
        )
        .expect("the run is coloured");
    let rules = rules_of(&mut session, document);
    assert!(rules.contains(&"contrast".to_string()), "the rule is broken: {rules:?}");
    let kinds = kinds_of(&mut session, document);
    assert!(kinds.contains(&"warning".to_string()), "and it is a warning: {kinds:?}");
}

/// Text that is **a little** hard to read is a tip.
#[test]
fn text_that_is_a_little_hard_to_read_is_a_tip() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute(
            "prose.run.format",
            &json!({ "block": paragraph, "start": 0.0, "length": 9.0, "colour": "#777777" }),
        )
        .expect("the run is coloured");
    let rules = rules_of(&mut session, document);
    assert!(rules.contains(&"contrast".to_string()), "the rule is broken: {rules:?}");
    let kinds = kinds_of(&mut session, document);
    assert!(kinds.contains(&"tip".to_string()), "and it is a tip: {kinds:?}");
}

/// **Black** text breaks nothing.
#[test]
fn black_text_breaks_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute(
            "prose.run.format",
            &json!({ "block": paragraph, "start": 0.0, "length": 9.0, "colour": "#000000" }),
        )
        .expect("the run is coloured");
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"contrast".to_string()), "the text is readable: {rules:?}");
}

/// **Large** text needs only three.
#[test]
fn large_text_needs_only_three() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute(
            "prose.run.format",
            &json!({
                "block": paragraph,
                "start": 0.0,
                "length": 9.0,
                "colour": "#777777",
                "size": 18.0
            }),
        )
        .expect("the run is coloured");
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"contrast".to_string()), "the text is large enough: {rules:?}");
}

/// **Bold** text of fourteen points needs only three.
#[test]
fn bold_text_of_fourteen_points_needs_only_three() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute(
            "prose.run.format",
            &json!({
                "block": paragraph,
                "start": 0.0,
                "length": 9.0,
                "colour": "#777777",
                "size": 14.0,
                "bold": true
            }),
        )
        .expect("the run is coloured");
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"contrast".to_string()), "the text is large enough: {rules:?}");
}

/// Text with **no colour** is not measured.
#[test]
fn text_with_no_colour_is_not_measured() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "the words");
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"contrast".to_string()), "there is no colour to measure: {rules:?}");
}

/// The **issue** says what the ratio is.
#[test]
fn the_issue_says_what_the_ratio_is() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the words");
    session
        .execute(
            "prose.run.format",
            &json!({ "block": paragraph, "start": 0.0, "length": 9.0, "colour": "#bbbbbb" }),
        )
        .expect("the run is coloured");
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    let issues = outcome.data["issues"].as_array().expect("the issues are a list");
    let issue = issues.iter().find(|issue| issue["rule"] == json!("contrast")).expect("the rule is broken");
    let message = issue["issue"].as_str().unwrap_or_default();
    assert!(message.contains(":1"), "the ratio is said: {message}");
}
