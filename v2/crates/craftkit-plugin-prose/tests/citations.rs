//! The **citation** of a source, in the style of its document.
//!
//! The citation is worked out from the source and the style every
//! time, so it cannot be out of date.

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

/// A **document** that cites by a style, with a source in it.
fn a_document(session: &mut Session, style: &str) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    session
        .execute(
            "prose.document.source.set",
            &json!({
                "document": document,
                "tag": "Riv2020",
                "author": "Rivera, Alex",
                "title": "The Book",
                "year": "2020",
            }),
        )
        .expect("the source is said");
    session
        .execute("prose.document.citestyle.set", &json!({ "document": document, "style": style }))
        .expect("the style is said");
    document
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

/// The **citation** of a source.
fn citation_of(session: &mut Session, document: u64, tag: &str, pages: Option<&str>) -> String {
    let mut params = json!({ "document": document, "tag": tag });
    if let Some(pages) = pages {
        params["pages"] = json!(pages);
    }
    let outcome = session.execute("prose.document.citation", &params).expect("the citation is read");
    outcome.data["citation"].as_str().unwrap_or_default().to_string()
}

/// A citation is written the way the style writes it: the author and
/// the year, and the pages after them.
#[test]
fn a_citation_is_written_in_the_style_of_the_document() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    assert_eq!(citation_of(&mut session, document, "Riv2020", None), "(Rivera, 2020)", "the citation");
    assert_eq!(
        citation_of(&mut session, document, "Riv2020", Some("5")),
        "(Rivera, 2020, p. 5)",
        "and the pages after them"
    );
}

/// A style that **numbers** its citations says the number of the
/// source, and the number is where the source stands in the order the
/// sources are cited.
#[test]
fn a_numbered_style_cites_by_the_number() {
    let mut session = session();
    let document = a_document(&mut session, "IEEE");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Che2021", "author": "Chen, Bo", "title": "The Paper", "year": "2021" }),
        )
        .expect("the source is said");
    a_citation(&mut session, document, "Riv2020");
    a_citation(&mut session, document, "Che2021");
    assert_eq!(citation_of(&mut session, document, "Riv2020", None), "[1]", "the first source is one");
    assert_eq!(citation_of(&mut session, document, "Che2021", None), "[2]", "and the second is two");
}

/// A style that cites the **author and the pages** says the pages
/// alone.
#[test]
fn an_author_page_style_cites_the_author_and_the_pages() {
    let mut session = session();
    let document = a_document(&mut session, "MLA");
    assert_eq!(citation_of(&mut session, document, "Riv2020", None), "(Rivera)", "the citation");
    assert_eq!(citation_of(&mut session, document, "Riv2020", Some("5")), "(Rivera 5)", "and the pages");
}

/// The authors are **named**, or said as **et al.**, and a source with
/// no author is cited by its title.
#[test]
fn the_authors_are_named_or_said_as_et_al() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Two2020", "author": "Rivera, Alex; Chen, Bo", "title": "Two", "year": "2020" }),
        )
        .expect("the source is said");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Three2020", "author": "Rivera, Alex; Chen, Bo; Wu, Li", "title": "Three", "year": "2020" }),
        )
        .expect("the source is said");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "NoOne2020", "author": "", "title": "The Anonymous Book", "year": "2020" }),
        )
        .expect("the source is said");
    assert_eq!(
        citation_of(&mut session, document, "Two2020", None),
        "(Rivera & Chen, 2020)",
        "two authors are joined by the word of the style"
    );
    assert_eq!(
        citation_of(&mut session, document, "Three2020", None),
        "(Rivera et al., 2020)",
        "and from three the style says et al."
    );
    assert_eq!(
        citation_of(&mut session, document, "NoOne2020", None),
        "(The Anonymous Book, 2020)",
        "and a source with no author is cited by its title"
    );
}

/// A tag that names **no source** is refused by name.
#[test]
fn a_tag_that_names_no_source_is_refused() {
    let mut session = session();
    let document = a_document(&mut session, "APA");
    let error = session
        .execute("prose.document.citation", &json!({ "document": document, "tag": "Nobody2020" }))
        .expect_err("the tag is refused");
    assert!(error.to_string().contains("Nobody2020"), "the refusal names the tag: {error}");
}
