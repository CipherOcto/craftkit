//! The **alt text** of a table.

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

/// A **table** with two rows, and its id.
fn a_table(session: &mut Session, document: u64) -> u64 {
    let grid: Vec<Json> = (0..2).map(|_| json!(["a", "b"])).collect();
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": grid }))
        .expect("the table is added");
    outcome.data["id"].as_u64().expect("the table has an id")
}

/// The **alt text** of a table, as the evaluator says it.
fn alt_of(session: &Session, table: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == table)
        .and_then(|item| item.value.get("alt").cloned())
        .unwrap_or(Json::Null)
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

/// A table that says nothing has **no** alt text.
#[test]
fn a_table_that_says_nothing_has_no_alt_text() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document);
    assert_eq!(alt_of(&session, table), Json::Null, "the table says nothing");
}

/// A table **says** what it is.
#[test]
fn a_table_says_what_it_is() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document);
    session
        .execute("prose.table.alt", &json!({ "table": table, "alt": "the years and the sales" }))
        .expect("the alt text is put");
    assert_eq!(alt_of(&session, table), json!("the years and the sales"), "the table says what it is");
}

/// An **empty** alt text takes the text away.
#[test]
fn an_empty_alt_text_takes_the_text_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document);
    session
        .execute("prose.table.alt", &json!({ "table": table, "alt": "the years" }))
        .expect("the alt text is put");
    session
        .execute("prose.table.alt", &json!({ "table": table, "alt": "" }))
        .expect("the alt text is taken away");
    assert_eq!(alt_of(&session, table), Json::Null, "the table says nothing again");
}

/// A table that says nothing about itself is a **warning**.
#[test]
fn a_table_that_says_nothing_about_itself_is_a_warning() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document);
    let rules = rules_of(&mut session, document);
    assert!(rules.contains(&"tableAltText".to_string()), "the rule is broken: {rules:?}");
    let kinds = kinds_of(&mut session, document);
    assert!(kinds.contains(&"warning".to_string()), "and it is a warning: {kinds:?}");
}

/// A table that says what it is breaks **nothing**.
#[test]
fn a_table_that_says_what_it_is_breaks_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document);
    session
        .execute("prose.table.alt", &json!({ "table": table, "alt": "the years and the sales" }))
        .expect("the alt text is put");
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"tableAltText".to_string()), "the table says what it is: {rules:?}");
}

/// A node that is **not a table** is refused.
#[test]
fn a_node_that_is_not_a_table_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let refused = session
        .execute("prose.table.alt", &json!({ "table": paragraph, "alt": "the years" }))
        .expect_err("the alt text is refused");
    assert!(refused.to_string().contains("not a table"), "the refusal says why: {refused}");
}
