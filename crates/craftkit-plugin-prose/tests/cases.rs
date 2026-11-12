//! The **case** of the text of a paragraph.
//!
//! The text is changed in the way the tree changes it, and the runs
//! move with the text.

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

/// A **document** with one **paragraph** that says the given words.
fn a_paragraph(session: &mut Session, text: &str) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// The **text** of a paragraph, as it is kept.
fn text_of(session: &Session, block: u64) -> String {
    session
        .document()
        .node(block)
        .and_then(|node| node.data.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or_default()
        .to_string()
}

/// The **runs** of a paragraph, as they are kept.
fn runs_of(session: &Session, block: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    session
        .document()
        .node(block)
        .and_then(|node| node.data.get("runs"))
        .and_then(|runs| runs.as_array())
        .cloned()
        .unwrap_or_default()
}

/// The **upper** and the **lower** case: every character is changed,
/// and the Greek capital sigma takes its final form at the end of a
/// word.
#[test]
fn the_upper_and_the_lower_case() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello world");
    session
        .execute("prose.block.case", &json!({ "block": block, "mode": "upper" }))
        .expect("the case is changed");
    assert_eq!(text_of(&session, block), "HELLO WORLD", "the text is uppercased");
    session
        .execute("prose.block.case", &json!({ "block": block, "mode": "lower" }))
        .expect("the case is changed");
    assert_eq!(text_of(&session, block), "hello world", "and lowercased again");
    let block = a_paragraph(&mut session, "ΣΟΦΟΣ ΟΔΟΣ");
    session
        .execute("prose.block.case", &json!({ "block": block, "mode": "lower" }))
        .expect("the case is changed");
    assert_eq!(
        text_of(&session, block),
        "σοφος οδος",
        "and the sigma is the final form: the tree's own rule"
    );
}

/// The **sentence** case: the first letter of the paragraph and the
/// first letter after a full stop — and a full stop between two
/// digits is a decimal point.
#[test]
fn the_sentence_case() {
    let mut session = session();
    for (text, expected) in [
        ("hello world. again", "Hello world. Again"),
        ("one 3.14 apples and two 2.5 pears.", "One 3.14 apples and two 2.5 pears."),
        ("pi is 3.14. next sentence.", "Pi is 3.14. Next sentence."),
    ] {
        let block = a_paragraph(&mut session, text);
        session
            .execute("prose.block.case", &json!({ "block": block, "mode": "sentence" }))
            .expect("the case is changed");
        assert_eq!(text_of(&session, block), expected, "the sentence case of {text:?}");
    }
}

/// The **title** case capitalises the first letter of every word, and
/// the **toggle** case flips each character.
#[test]
fn the_title_and_the_toggle_case() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello WORLD");
    session
        .execute("prose.block.case", &json!({ "block": block, "mode": "title" }))
        .expect("the case is changed");
    assert_eq!(text_of(&session, block), "Hello World", "every word is capitalised");
    let block = a_paragraph(&mut session, "Hello");
    session
        .execute("prose.block.case", &json!({ "block": block, "mode": "toggle" }))
        .expect("the case is changed");
    assert_eq!(text_of(&session, block), "hELLO", "and every character is flipped");
}

/// The text may **change length**, and the **runs** move with it.
#[test]
fn the_runs_move_with_the_text() {
    let mut session = session();
    let block = a_paragraph(&mut session, "straße");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 6, "bold": true }] }))
        .expect("the runs are set");
    session
        .execute("prose.block.case", &json!({ "block": block, "mode": "upper" }))
        .expect("the case is changed");
    assert_eq!(text_of(&session, block), "STRASSE", "the text is uppercased: the tree's own rule");
    let runs = runs_of(&session, block);
    assert_eq!(runs.len(), 1, "the run is still there: {runs:?}");
    assert_eq!(runs[0]["len"], json!(7), "and it covers the text that is now there: {runs:?}");
    assert_eq!(runs[0]["bold"], json!(true), "and it is still bold: {runs:?}");
}

/// A mode that is **not one of the tree's** is refused.
#[test]
fn a_mode_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, "hello");
    let error = session
        .execute("prose.block.case", &json!({ "block": block, "mode": "sideways" }))
        .expect_err("there is no such mode");
    assert!(error.to_string().contains("sideways"), "the refusal says why: {error}");
}
