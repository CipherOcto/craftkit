//! The **bibliography** of a document, written in its style.
//!
//! The entries are worked out from the sources, the citations and the
//! style every time, so they cannot be out of date.

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

/// A **document** that cites by a style.
fn a_document(session: &mut Session, style: &str) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    session
        .execute("prose.document.citestyle.set", &json!({ "document": document, "style": style }))
        .expect("the style is said");
    document
}

/// A **source** of a document.
fn a_source(session: &mut Session, document: u64, tag: &str, author: &str, title: &str, year: &str) {
    session
        .execute(
            "prose.document.source.set",
            &json!({
                "document": document,
                "tag": tag,
                "author": author,
                "title": title,
                "year": year,
            }),
        )
        .expect("the source is said");
}

/// A **paragraph** that cites a source.
fn a_citation(session: &mut Session, document: u64, tag: &str) {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a word" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 5, "citation": tag }] }))
        .expect("the run is set");
}

/// The **entries** of the bibliography of a document.
fn entries_of(session: &mut Session, document: u64) -> Vec<String> {
    let outcome = session
        .execute("prose.document.bibliography", &json!({ "document": document }))
        .expect("the bibliography is read");
    outcome.data["entries"]
        .as_array()
        .map(|entries| {
            entries.iter().filter_map(|entry| entry.as_str().map(|entry| entry.to_string())).collect()
        })
        .unwrap_or_default()
}

/// The bibliography is written in the **style** of the document, and the
/// style's title is the title of the list.
#[test]
fn the_bibliography_is_written_in_the_style_of_the_document() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    a_source(&mut session, document, "Riv2020", "Rivera, Alex", "The Book", "2020");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "publisher": "The Press" }),
        )
        .expect("the publisher is said");
    a_citation(&mut session, document, "Riv2020");
    let outcome = session
        .execute("prose.document.bibliography", &json!({ "document": document }))
        .expect("the bibliography is read");
    assert_eq!(outcome.data["style"], json!("APA"), "the style is said: {outcome:?}");
    assert_eq!(outcome.data["title"], json!("References"), "and the title of the list");
    let entries = entries_of(&mut session, document);
    assert_eq!(
        entries,
        vec!["Rivera, A. (2020). The Book. The Press.".to_string()],
        "the entry: {entries:?}"
    );
}

/// A source with a **journal** is written as an article, and the parts
/// that are empty are left out.
#[test]
fn a_source_with_a_journal_is_written_as_an_article() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    a_source(&mut session, document, "Riv2020", "Rivera, Alex", "The Article", "2020");
    session
        .execute(
            "prose.document.source.set",
            &json!({
                "document": document,
                "tag": "Riv2020",
                "journal": "The Journal",
                "volume": "3",
                "pages": "10-20",
            }),
        )
        .expect("the journal is said");
    a_citation(&mut session, document, "Riv2020");
    let entries = entries_of(&mut session, document);
    assert_eq!(
        entries,
        vec!["Rivera, A. (2020). The Article. The Journal, 3, 10-20.".to_string()],
        "the entry: {entries:?}"
    );
}

/// The **order** of the bibliography is the order the style asks for:
/// by the author, by the title, or in the order the sources are cited.
#[test]
fn the_order_of_the_bibliography_is_the_order_the_style_asks() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    a_source(&mut session, document, "Wu2020", "Wu, Li", "The Later Book", "2020");
    a_source(&mut session, document, "Riv2020", "Rivera, Alex", "The Book", "2020");
    a_citation(&mut session, document, "Wu2020");
    a_citation(&mut session, document, "Riv2020");
    let entries = entries_of(&mut session, document);
    assert_eq!(entries.len(), 2, "both sources are there: {entries:?}");
    assert!(entries[0].starts_with("Rivera"), "APA sorts by the surname of the author: {entries:?}");
}

/// A style that **numbers** its entries numbers them, and a source
/// with **no year** is written `n.d.`
#[test]
fn a_numbered_style_numbers_the_entries() {
    let mut session = session();
    let document = a_document(&mut session, "GB/T 7714");
    a_source(&mut session, document, "Riv2020", "Rivera, Alex", "The Book", "2020");
    session
        .execute(
            "prose.document.source.set",
            &json!({
                "document": document,
                "tag": "Riv2020",
                "city": "Oxford",
                "publisher": "The Press",
            }),
        )
        .expect("the place is said");
    a_citation(&mut session, document, "Riv2020");
    let entries = entries_of(&mut session, document);
    assert_eq!(
        entries,
        vec!["[1] RIVERA A. The Book[M]. Oxford: The Press, 2020.".to_string()],
        "the entry: {entries:?}"
    );
    // A source with **no year** is written `n.d.`
    let document = a_document(&mut session, "APA");
    a_source(&mut session, document, "Riv2020", "Rivera, Alex", "The Book", "");
    a_citation(&mut session, document, "Riv2020");
    let entries = entries_of(&mut session, document);
    assert_eq!(entries, vec!["Rivera, A. (n.d.). The Book.".to_string()], "the entry: {entries:?}");
}

/// The bibliography of a document with **nothing cited** is empty, and
/// a source that is not cited is left out.
#[test]
fn a_source_that_is_not_cited_is_left_out() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    a_source(&mut session, document, "Riv2020", "Rivera, Alex", "The Book", "2020");
    a_source(&mut session, document, "Wu2020", "Wu, Li", "The Other Book", "2020");
    a_citation(&mut session, document, "Riv2020");
    let entries = entries_of(&mut session, document);
    assert_eq!(entries.len(), 1, "only the cited source is there: {entries:?}");
    assert!(entries[0].contains("The Book"), "and it is the one that is cited: {entries:?}");
    let outcome = session
        .execute("prose.document.bibliography", &json!({ "document": document }))
        .expect("the bibliography is read");
    let sources: &Json = &outcome.data["bibliography"];
    assert_eq!(
        sources.as_array().map(|sources| sources.len()),
        Some(1),
        "the sources are there too: {outcome:?}"
    );
}
