//! The **sources** of a document, and the **citations** of them.
//!
//! A source is a thing a document points at; a citation is a run that
//! names one; the bibliography is worked out from them.

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
fn a_paragraph(session: &mut Session, text: &str) -> (u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    (document, block)
}

/// The **sources** of a document, as the command reads them.
fn sources_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.document.source.read", &json!({ "document": document }))
        .expect("the sources are read");
    outcome.data["sources"].clone()
}

/// The **bibliography** of a document, as the command reads it.
fn bibliography_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.document.bibliography", &json!({ "document": document }))
        .expect("the bibliography is read");
    outcome.data["bibliography"].clone()
}

/// A source is said, and it is **given a tag** when it is not given
/// one: the tree's own rule — three letters of the author and two
/// digits of the year.
#[test]
fn a_source_is_given_a_tag() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    let outcome = session
        .execute(
            "prose.document.source.set",
            &json!({
                "document": document,
                "kind": "book",
                "author": "Ada Lovelace",
                "title": "Notes on the Engine",
                "year": "1843",
            }),
        )
        .expect("the source is said");
    assert_eq!(outcome.data["source"]["tag"], json!("Ada43"), "the tag is made: {outcome:?}");
    assert_eq!(outcome.data["source"]["kind"], json!("book"), "the kind is the one that was given");
    let sources = sources_of(&mut session, document);
    assert_eq!(sources.as_array().map(|sources| sources.len()), Some(1), "the document has one source");
}

/// A source that is **named by its tag** is changed, and the parts
/// that are not named keep what they were.
#[test]
fn a_source_is_changed_by_its_tag() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "title": "Notes on the Engine" }),
        )
        .expect("the source is said");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "year": "1844" }),
        )
        .expect("the source is changed");
    let sources = sources_of(&mut session, document);
    assert_eq!(sources.as_array().map(|sources| sources.len()), Some(1), "there is still one source");
    assert_eq!(sources[0]["title"], json!("Notes on the Engine"), "the title is kept");
    assert_eq!(sources[0]["year"], json!("1844"), "and the year is the new one");
}

/// A source that is **not one** is refused: a kind that is not the
/// tree's, and a source with no title.
#[test]
fn a_source_that_is_not_one_is_refused() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    let error = session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "kind": "scroll" }),
        )
        .expect_err("the kind is refused");
    assert!(error.to_string().contains("no kind"), "the refusal says why: {error}");
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "tag": "Ada43" }))
        .expect_err("the source with no title is refused");
    assert!(error.to_string().contains("has a title"), "the refusal says why: {error}");
    let sources = sources_of(&mut session, document);
    assert_eq!(sources.as_array().map(|sources| sources.len()), Some(0), "and no source was kept");
}

/// A run may carry a **citation**, and a tag that names no source is
/// refused.
#[test]
fn a_run_may_carry_a_citation() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "as she wrote");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "title": "Notes on the Engine" }),
        )
        .expect("the source is said");
    session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 11, "citation": "Ada43" }] }))
        .expect("the run is set");
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": [{ "len": 11, "citation": "Bea44" }] }))
        .expect_err("the tag that names no source is refused");
    assert!(error.to_string().contains("no source"), "the refusal says why: {error}");
}

/// The **bibliography** is derived: it is the sources that are cited,
/// in the order they are first cited.
#[test]
fn the_bibliography_is_derived() {
    let mut session = session();
    let (document, block) = a_paragraph(&mut session, "as she wrote");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Bea44", "title": "On the Analytical Engine" }),
        )
        .expect("the source is said");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "title": "Notes on the Engine" }),
        )
        .expect("the source is said");
    session
        .execute(
            "prose.block.runs",
            &json!({
                "block": block,
                "runs": [
                    { "len": 2, "citation": "Ada43" },
                    { "len": 3 },
                    { "len": 6, "citation": "Bea44" },
                ],
            }),
        )
        .expect("the runs are set");
    let bibliography = bibliography_of(&mut session, document);
    let entries = bibliography.as_array().expect("the bibliography is a list");
    assert_eq!(entries.len(), 2, "the two sources are cited: {bibliography}");
    assert_eq!(entries[0]["tag"], json!("Ada43"), "and they are in the order they are cited");
    assert_eq!(entries[1]["tag"], json!("Bea44"), "and the second is the second");
    assert_eq!(entries[0]["title"], json!("Notes on the Engine"), "and the source is whole");
}

/// A source is **taken away**, and a source that is not there is
/// refused.
#[test]
fn a_source_is_taken_away() {
    let mut session = session();
    let (document, _block) = a_paragraph(&mut session, "hello");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Ada43", "title": "Notes on the Engine" }),
        )
        .expect("the source is said");
    session
        .execute("prose.document.source.remove", &json!({ "document": document, "tag": "Ada43" }))
        .expect("the source is taken away");
    let sources = sources_of(&mut session, document);
    assert_eq!(sources.as_array().map(|sources| sources.len()), Some(0), "the source is gone");
    let error = session
        .execute("prose.document.source.remove", &json!({ "document": document, "tag": "Ada43" }))
        .expect_err("the source that is not there is refused");
    assert!(error.to_string().contains("no source"), "the refusal says why: {error}");
}
