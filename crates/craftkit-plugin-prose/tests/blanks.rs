//! The **empty row** and the **empty column** of a table.

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
fn a_table(session: &mut Session, document: u64, rows: Vec<Vec<Json>>) -> u64 {
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": rows }))
        .expect("the table is added");
    outcome.data["id"].as_u64().expect("the table has an id")
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

/// An **empty row** looks like spacing.
#[test]
fn an_empty_row_looks_like_spacing() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(
        &mut session,
        document,
        vec![vec![json!("a"), json!("b")], vec![json!(""), json!("")], vec![json!("c"), json!("d")]],
    );
    let rules = rules_of(&mut session, document);
    assert!(rules.contains(&"blankRow".to_string()), "the rule is broken: {rules:?}");
    let kinds = kinds_of(&mut session, document);
    assert!(kinds.contains(&"warning".to_string()), "and it is a warning: {kinds:?}");
}

/// The issue says **which** row it is.
#[test]
fn the_issue_says_which_row_it_is() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(
        &mut session,
        document,
        vec![vec![json!("a"), json!("b")], vec![json!(""), json!("")], vec![json!("c"), json!("d")]],
    );
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    let issues = outcome.data["issues"].as_array().expect("the issues are a list");
    let issue = issues.iter().find(|issue| issue["rule"] == json!("blankRow")).expect("the rule is broken");
    let message = issue["issue"].as_str().unwrap_or_default();
    assert!(message.contains("Row 2"), "the row is said: {message}");
}

/// An **empty column** looks like spacing.
#[test]
fn an_empty_column_looks_like_spacing() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(
        &mut session,
        document,
        vec![vec![json!("a"), json!("")], vec![json!("b"), json!("")], vec![json!("c"), json!("")]],
    );
    let rules = rules_of(&mut session, document);
    assert!(rules.contains(&"blankColumn".to_string()), "the rule is broken: {rules:?}");
}

/// A table with **nothing** in it is being filled in.
#[test]
fn a_table_with_nothing_in_it_is_being_filled_in() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document, vec![vec![json!(""), json!("")], vec![json!(""), json!("")]]);
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"blankRow".to_string()), "the table is being filled in: {rules:?}");
    assert!(!rules.contains(&"blankColumn".to_string()), "the table is being filled in: {rules:?}");
}

/// A table with **no empty row** breaks nothing.
#[test]
fn a_table_with_no_empty_row_breaks_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document, vec![vec![json!("a"), json!("b")], vec![json!("c"), json!("d")]]);
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"blankRow".to_string()), "every row has something in it: {rules:?}");
    assert!(!rules.contains(&"blankColumn".to_string()), "and so has every column: {rules:?}");
}

/// A table of **one row** has no empty row to be spacing.
#[test]
fn a_table_of_one_row_has_no_empty_row_to_be_spacing() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document, vec![vec![json!(""), json!("")]]);
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"blankRow".to_string()), "one row is not a table of rows: {rules:?}");
}

/// A **cell** that is a number is not empty.
#[test]
fn a_cell_that_is_a_number_is_not_empty() {
    let mut session = session();
    let document = a_document(&mut session);
    a_table(&mut session, document, vec![vec![json!(1), json!(2)], vec![json!(3), json!(4)]]);
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"blankRow".to_string()), "the rows have numbers in them: {rules:?}");
    assert!(!rules.contains(&"blankColumn".to_string()), "and so have the columns: {rules:?}");
}
