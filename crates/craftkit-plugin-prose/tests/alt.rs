//! The **alt text** of a picture.

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

/// The **alt text** of a picture, as the evaluator says it.
fn alt_of(session: &Session, media: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == media)
        .and_then(|item| item.value.get("alt").cloned())
        .unwrap_or(Json::Null)
}

/// A picture that says nothing about itself has **no** alt text.
#[test]
fn a_picture_that_says_nothing_about_itself_has_no_alt_text() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    assert_eq!(alt_of(&session, picture), Json::Null, "the picture says nothing");
}

/// A picture **says** what it is.
#[test]
fn a_picture_says_what_it_is() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "a red door" }))
        .expect("the alt text is put");
    assert_eq!(alt_of(&session, picture), json!("a red door"), "the picture says what it is");
}

/// The alt text is **changed**, not added twice.
#[test]
fn the_alt_text_is_changed_and_not_added_twice() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "a red door" }))
        .expect("the alt text is put");
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "a blue window" }))
        .expect("the alt text is put again");
    assert_eq!(alt_of(&session, picture), json!("a blue window"), "the alt text is the new one");
}

/// An **empty** alt text takes the text away.
#[test]
fn an_empty_alt_text_takes_the_text_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "a red door" }))
        .expect("the alt text is put");
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "" }))
        .expect("the alt text is taken away");
    assert_eq!(alt_of(&session, picture), Json::Null, "the picture says nothing again");
}

/// A node that is **not a picture** may not have alt text.
#[test]
fn a_node_that_is_not_a_picture_may_not_have_alt_text() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let refused = session
        .execute("prose.media.alt", &json!({ "media": paragraph, "alt": "a red door" }))
        .expect_err("the alt text is refused");
    assert!(refused.to_string().contains("not a picture"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session
        .execute("prose.media.alt", &json!({ "media": 9999, "alt": "a red door" }))
        .expect_err("the alt text is refused");
    assert!(refused.to_string().contains("9999"), "the refusal says why: {refused}");
}
