//! The **placement** of a picture.

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

/// The **placement** of a picture, as the evaluator says it.
fn placement_of(session: &Session, media: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == media)
        .and_then(|item| item.value.get("placement").cloned())
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

/// A picture that says nothing is **inline**.
#[test]
fn a_picture_that_says_nothing_is_inline() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    assert_eq!(placement_of(&session, picture), json!("inline"), "the picture is in the line");
}

/// A picture may be said to **float**.
#[test]
fn a_picture_may_be_said_to_float() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.placement", &json!({ "media": picture, "placement": "floating" }))
        .expect("the picture floats");
    assert_eq!(placement_of(&session, picture), json!("floating"), "the picture floats");
}

/// A floating picture makes the **reading order** unclear.
#[test]
fn a_floating_picture_makes_the_reading_order_unclear() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    assert!(!rules_of(&mut session, document).contains(&"floating".to_string()), "nothing is said");
    session
        .execute("prose.media.placement", &json!({ "media": picture, "placement": "floating" }))
        .expect("the picture floats");
    assert!(rules_of(&mut session, document).contains(&"floating".to_string()), "the checker says so");
}

/// A picture that is **inline** is not asked about.
#[test]
fn a_picture_that_is_inline_is_not_asked_about() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.placement", &json!({ "media": picture, "placement": "inline" }))
        .expect("the picture is inline");
    assert!(!rules_of(&mut session, document).contains(&"floating".to_string()), "the checker is quiet");
}

/// A placement that is **not a placement** is refused.
#[test]
fn a_placement_that_is_not_a_placement_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    let refused = session
        .execute("prose.media.placement", &json!({ "media": picture, "placement": "beside" }))
        .expect_err("the placement is refused");
    assert!(refused.to_string().contains("not a placement"), "the refusal says why: {refused}");
}

/// A placement that is **not said** leaves the picture as it was.
#[test]
fn a_placement_that_is_not_said_leaves_the_picture_as_it_was() {
    let mut session = session();
    let document = a_document(&mut session);
    let picture = a_picture(&mut session, document);
    session
        .execute("prose.media.placement", &json!({ "media": picture, "placement": "floating" }))
        .expect("the picture floats");
    session.execute("prose.media.placement", &json!({ "media": picture })).expect("nothing is said");
    assert_eq!(placement_of(&session, picture), json!("floating"), "the picture still floats");
}

/// A node that is **not a picture** is refused.
#[test]
fn a_node_that_is_not_a_picture_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let refused = session
        .execute("prose.media.placement", &json!({ "media": document, "placement": "floating" }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("not a picture"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session
        .execute("prose.media.placement", &json!({ "media": 999, "placement": "floating" }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}
