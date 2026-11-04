//! The **links** of a block.
//!
//! A run may carry a link, and a link that the system may open is a
//! web link or a mail link and nothing else.

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

/// A **paragraph** with a **run** that carries a link.
fn a_linked_paragraph(session: &mut Session, document: u64, target: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.runs", &json!({ "block": paragraph, "runs": [{ "len": 3, "link": target }] }))
        .expect("the runs are set");
    paragraph
}

/// The **links** of a block, as the answer holds them.
fn links_of(session: &mut Session, block: u64) -> Vec<Json> {
    let outcome =
        session.execute("prose.block.links", &json!({ "block": block })).expect("the links are read");
    outcome.data["links"].as_array().expect("the links are a list").clone()
}

/// A **web link** may be opened, and the words it covers are derived.
#[test]
fn a_web_link_may_be_opened() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_linked_paragraph(&mut session, document, "https://example.com/a");
    let links = links_of(&mut session, paragraph);
    assert_eq!(links.len(), 1, "there is one link");
    assert_eq!(links[0]["target"], Json::String("https://example.com/a".to_string()), "the target is said");
    assert_eq!(links[0]["text"], Json::String("the".to_string()), "and the words it covers");
    assert_eq!(links[0]["may_be_opened"], json!(true), "and it may be opened");
}

/// A **mail link** may be opened too.
#[test]
fn a_mail_link_may_be_opened() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_linked_paragraph(&mut session, document, "mailto:someone@example.com");
    let links = links_of(&mut session, paragraph);
    assert_eq!(links[0]["may_be_opened"], json!(true), "the mail link may be opened");
}

/// A **script** is not a link that may be opened, and the refusal says
/// why.
#[test]
fn a_script_is_not_a_link_that_may_be_opened() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_linked_paragraph(&mut session, document, "javascript:alert(1)");
    let links = links_of(&mut session, paragraph);
    assert_eq!(links[0]["may_be_opened"], json!(false), "the script may not be opened");
    let reason = links[0]["reason"].as_str().expect("there is a reason").to_string();
    assert!(reason.contains("a script"), "the reason says so: {reason}");
}

/// A link to **another machine** is not a link that may be opened.
#[test]
fn a_link_to_another_machine_is_not_opened() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_linked_paragraph(&mut session, document, "//host/share");
    let links = links_of(&mut session, paragraph);
    assert_eq!(links[0]["may_be_opened"], json!(false), "the link may not be opened");
    let reason = links[0]["reason"].as_str().expect("there is a reason").to_string();
    assert!(reason.contains("another machine"), "the reason says so: {reason}");
}

/// The rule is **exact**: a leading space makes the target not a link.
#[test]
fn the_rule_is_exact() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_linked_paragraph(&mut session, document, " https://example.com");
    let links = links_of(&mut session, paragraph);
    assert_eq!(links[0]["may_be_opened"], json!(false), "the target is not a link");
}

/// A **relative** link is not a link that may be opened.
#[test]
fn a_relative_link_is_not_opened() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_linked_paragraph(&mut session, document, "other.html");
    let links = links_of(&mut session, paragraph);
    assert_eq!(links[0]["may_be_opened"], json!(false), "the link may not be opened");
    let reason = links[0]["reason"].as_str().expect("there is a reason").to_string();
    assert!(reason.contains("relative"), "the reason says so: {reason}");
}

/// A run with **no link** is not a link, and a target that is not a
/// text is refused.
#[test]
fn a_run_with_no_link_is_not_a_link() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.runs", &json!({ "block": paragraph, "runs": [{ "len": 3 }] }))
        .expect("the runs are set");
    assert!(links_of(&mut session, paragraph).is_empty(), "there is no link");
    let error = session
        .execute("prose.block.runs", &json!({ "block": paragraph, "runs": [{ "len": 3, "link": 42 }] }))
        .expect_err("the link is refused");
    assert!(error.to_string().contains("a link is a target"), "the refusal says why: {error}");
}

/// A block that is **not there** is refused, and so is a node that is
/// not a block.
#[test]
fn a_block_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error =
        session.execute("prose.block.links", &json!({ "block": 999 })).expect_err("the block is refused");
    assert!(error.to_string().contains("there is no block"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.links", &json!({ "block": document }))
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
}
