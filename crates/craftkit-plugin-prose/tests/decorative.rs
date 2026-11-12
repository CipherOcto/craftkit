//! The **decorative** picture.

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

/// Whether a picture is **decorative**, as the evaluator says it.
fn decorative_of(session: &Session, media: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == media)
        .and_then(|item| item.value.get("decorative").cloned())
        .unwrap_or(Json::Null)
}

/// The **rules** the checker reports, as their names.
fn rules_of(session: &mut Session, document: u64) -> Vec<String> {
    let outcome =
        session.execute("prose.accessibility", &json!({ "document": document })).expect("the checker runs");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .filter_map(|issue| issue["rule"].as_str().map(|rule| rule.to_string()))
        .collect()
}

/// A picture that says nothing is **not** decorative.
#[test]
fn a_picture_that_says_nothing_is_not_decorative() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    assert_eq!(decorative_of(&session, picture), json!(false), "the picture is not decorative");
}

/// A picture may be **said** to be decorative.
#[test]
fn a_picture_may_be_said_to_be_decorative() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.decorative", &json!({ "media": picture, "decorative": true }))
        .expect("the picture is decorative");
    assert_eq!(decorative_of(&session, picture), json!(true), "the picture is decorative");
}

/// A decorative picture is **not asked** for words.
#[test]
fn a_decorative_picture_is_not_asked_for_words() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    assert!(rules_of(&mut session, document).contains(&"altText".to_string()), "the checker asks for words");
    session
        .execute("prose.media.decorative", &json!({ "media": picture, "decorative": true }))
        .expect("the picture is decorative");
    assert!(!rules_of(&mut session, document).contains(&"altText".to_string()), "the checker does not ask");
}

/// A picture that is **not decorative** is asked for words again.
#[test]
fn a_picture_that_is_not_decorative_is_asked_for_words_again() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.decorative", &json!({ "media": picture, "decorative": true }))
        .expect("the picture is decorative");
    session
        .execute("prose.media.decorative", &json!({ "media": picture, "decorative": false }))
        .expect("the picture is not decorative");
    assert_eq!(decorative_of(&session, picture), json!(false), "the picture is not decorative");
    assert!(rules_of(&mut session, document).contains(&"altText".to_string()), "the checker asks for words");
}

/// A decorative picture that **says what it is** is still decorative.
#[test]
fn a_decorative_picture_that_says_what_it_is_is_still_decorative() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.alt", &json!({ "media": picture, "alt": "a rule" }))
        .expect("the alt text is said");
    session
        .execute("prose.media.decorative", &json!({ "media": picture, "decorative": true }))
        .expect("the picture is decorative");
    assert_eq!(decorative_of(&session, picture), json!(true), "the picture is decorative");
}

/// A picture that is **not said** to be decorative keeps the fact it
/// has.
#[test]
fn a_picture_that_is_not_said_to_be_decorative_keeps_the_fact_it_has() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.decorative", &json!({ "media": picture, "decorative": true }))
        .expect("the picture is decorative");
    session.execute("prose.media.decorative", &json!({ "media": picture })).expect("nothing is said");
    assert_eq!(decorative_of(&session, picture), json!(true), "the picture keeps the fact");
}

/// A node that is **not a picture** is refused.
#[test]
fn a_node_that_is_not_a_picture_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let refused = session
        .execute("prose.media.decorative", &json!({ "media": document, "decorative": true }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("not a picture"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session
        .execute("prose.media.decorative", &json!({ "media": 999, "decorative": true }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}
