//! The **statistics** of a document.

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

/// A **paragraph** in a document.
fn a_paragraph(session: &mut Session, document: u64, text: &str) {
    session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
}

/// The **statistics** of a document, as the command reads them.
fn statistics_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.document.stats", &json!({ "document": document }))
        .expect("the statistics are read");
    outcome.data.clone()
}

/// The **words** of a document are counted.
#[test]
fn the_words_of_a_document_are_counted() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "the quick brown fox");
    a_paragraph(&mut session, document, "jumps over");
    let statistics = statistics_of(&mut session, document);
    assert_eq!(statistics["words"], json!(6), "there are six words");
    assert_eq!(statistics["paragraphs"], json!(2), "and two paragraphs");
}

/// A mark that is not a letter or a digit is **not a word** of its
/// own.
#[test]
fn a_mark_is_not_a_word_of_its_own() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "one -- two");
    let statistics = statistics_of(&mut session, document);
    assert_eq!(statistics["words"], json!(2), "the dashes are not words");
}

/// The **characters** are counted with the spaces and without them.
#[test]
fn the_characters_are_counted_with_and_without_the_spaces() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "one two");
    let statistics = statistics_of(&mut session, document);
    assert_eq!(statistics["characters"], json!(7), "there are seven characters");
    assert_eq!(statistics["characters_without_spaces"], json!(6), "and six without the space");
}

/// The **footnotes** are another story and are not counted here.
#[test]
fn the_footnotes_are_not_counted() {
    let mut session = session();
    let document = a_document(&mut session);
    a_paragraph(&mut session, document, "the body");
    let outcome = session
        .execute("prose.footnote.add", &json!({ "document": document, "text": "a note of its own" }))
        .expect("the footnote is added");
    let footnote = outcome.data["id"].as_u64().expect("the footnote has an id");
    assert!(footnote > 0, "the footnote is there");
    let statistics = statistics_of(&mut session, document);
    assert_eq!(statistics["words"], json!(2), "only the body is counted");
}

/// A document that is not there is refused.
#[test]
fn a_document_that_is_not_there_is_refused() {
    let mut session = session();
    let outcome = session.execute("prose.document.stats", &json!({ "document": 9999 }));
    assert!(outcome.is_err(), "there is no such document");
}
