//! The **header row** of a table.

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

/// A **table** with the given rows, and its id.
fn a_table(session: &mut Session, document: u64, rows: usize) -> u64 {
    let grid: Vec<Json> = (0..rows).map(|_| json!(["a", "b"])).collect();
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": grid }))
        .expect("the table is added");
    outcome.data["id"].as_u64().expect("the table has an id")
}

/// The **issues** of a document, as the checker holds them.
fn issues_of(session: &mut Session, document: u64) -> Vec<Json> {
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    outcome.data["issues"].as_array().expect("the issues are a list").clone()
}

/// The **rules** the document breaks.
fn rules_of(issues: &[Json]) -> Vec<String> {
    issues.iter().map(|issue| issue["rule"].as_str().unwrap_or_default().to_string()).collect()
}

/// The **header** of a table, as the evaluator says it.
fn header_of(session: &Session, table: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == table)
        .and_then(|item| item.value.get("header").cloned())
        .unwrap_or(Json::Null)
}

/// A table of **two rows** has no header row, and that is a warning.
#[test]
fn a_table_of_two_rows_has_no_header_row_and_that_is_a_warning() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document, 2);
    let issues = issues_of(&mut session, document);
    assert!(rules_of(&issues).contains(&"tableHeader".to_string()), "the rule is broken: {issues:?}");
    let issue =
        issues.iter().find(|issue| issue["rule"] == json!("tableHeader")).expect("the rule is broken");
    assert_eq!(issue["kind"], json!("warning"), "it is a warning");
}

/// A table with **one row** is not a table with rows to name.
#[test]
fn a_table_with_one_row_is_not_a_table_with_rows_to_name() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document, 1);
    let issues = issues_of(&mut session, document);
    assert!(!rules_of(&issues).contains(&"tableHeader".to_string()), "one row is not a table: {issues:?}");
}

/// The first row of a table **is** a header row.
#[test]
fn the_first_row_of_a_table_is_a_header_row() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document, 2);
    session
        .execute("prose.table.header", &json!({ "table": table, "header": true }))
        .expect("the header is set");
    assert_eq!(header_of(&session, table), json!(true), "the table says so");
}

/// A table with a header row breaks **nothing**.
#[test]
fn a_table_with_a_header_row_breaks_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document, 2);
    session
        .execute("prose.table.header", &json!({ "table": table, "header": true }))
        .expect("the header is set");
    let issues = issues_of(&mut session, document);
    assert!(
        !rules_of(&issues).contains(&"tableHeader".to_string()),
        "the table says what its columns are: {issues:?}"
    );
}

/// The header row is **taken away**, and the warning comes back.
#[test]
fn the_header_row_is_taken_away_and_the_warning_comes_back() {
    let mut session = session();
    let document = a_document(&mut session);
    let table = a_table(&mut session, document, 2);
    session
        .execute("prose.table.header", &json!({ "table": table, "header": true }))
        .expect("the header is set");
    session
        .execute("prose.table.header", &json!({ "table": table, "header": false }))
        .expect("the header is taken away");
    assert_eq!(header_of(&session, table), json!(false), "the table says so");
    let issues = issues_of(&mut session, document);
    assert!(rules_of(&issues).contains(&"tableHeader".to_string()), "the rule is broken: {issues:?}");
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
        .execute("prose.table.header", &json!({ "table": paragraph, "header": true }))
        .expect_err("the header is refused");
    assert!(refused.to_string().contains("not a table"), "the refusal says why: {refused}");
}
