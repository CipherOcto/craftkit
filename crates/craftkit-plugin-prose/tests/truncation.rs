//! The **prose battery says what it cut**.
//!
//! A reader that stops at a bound and says nothing leaves a caller with
//! a number that is wrong: the count of the issues that fitted, told as
//! the count of the issues there were. The checker of a document is such
//! a reader, and it now says what it cut — how many issues there were,
//! how many are listed, and whether any were left out.

use craftkit_plugin_prose::{MAX_ACCESSIBILITY_COLUMNS, MAX_ACCESSIBILITY_ISSUES};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// A **document**, with a **title**, and its id.
///
/// The title matters: a document without one has an issue of its own,
/// and the count of the issues would not be the count of the pictures.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "the document" }))
        .expect("the document has a title");
    document
}

/// A **picture** that says nothing about itself, and the document it is in.
fn a_picture(session: &mut Session, document: u64, index: usize) {
    session
        .execute(
            "prose.media.add",
            &json!({ "document": document, "name": format!("the picture {index}"), "path": "pictures/one.png" }),
        )
        .expect("the picture is added");
}

/// The **answer** of the checker, as the caller gets it.
fn check(session: &mut Session, document: u64) -> Json {
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    outcome.data
}

/// The **issues** of the answer, as a list.
fn issues_of(answer: &Json) -> Vec<Json> {
    answer["issues"].as_array().expect("the issues are a list").clone()
}

/// The **count** of the answer: how many issues there were.
fn count_of(answer: &Json) -> u64 {
    answer["count"].as_u64().expect("the answer has a count")
}

/// The **number listed** of the answer: how many issues are in the list.
fn listed_of(answer: &Json) -> u64 {
    answer["listed"].as_u64().expect("the answer says how many are listed")
}

/// The issues are **cut** at `MAX_ACCESSIBILITY_ISSUES`, and the answer
/// says so.
///
/// A document with more issues than the bound has is a document the
/// report cannot hold, and the answer says that it did not hold all of
/// them.
#[test]
fn the_issues_are_cut_at_the_bound_and_the_answer_says_so() {
    let mut session = session();
    let document = a_document(&mut session);
    for index in 0..=MAX_ACCESSIBILITY_ISSUES {
        a_picture(&mut session, document, index);
    }
    let answer = check(&mut session, document);
    let count = count_of(&answer);
    let listed = listed_of(&answer);
    assert!(count > MAX_ACCESSIBILITY_ISSUES as u64, "there are more issues than the bound: {answer:?}");
    assert_eq!(listed, MAX_ACCESSIBILITY_ISSUES as u64, "the issues are cut at the bound: {answer:?}");
    assert_eq!(answer["truncated"], json!(true), "the answer says it cut: {answer:?}");
    assert_eq!(issues_of(&answer).len(), MAX_ACCESSIBILITY_ISSUES, "the list holds the bound: {answer:?}");
}

/// The issues are **not cut** at the bound.
///
/// A document with exactly the bound's worth of issues is a document the
/// report holds whole, and saying that it was cut would be a lie.
#[test]
fn the_issues_are_not_cut_at_the_bound() {
    let mut session = session();
    let document = a_document(&mut session);
    for index in 0..MAX_ACCESSIBILITY_ISSUES {
        a_picture(&mut session, document, index);
    }
    let answer = check(&mut session, document);
    assert_eq!(count_of(&answer), MAX_ACCESSIBILITY_ISSUES as u64, "the count is the count: {answer:?}");
    assert_eq!(listed_of(&answer), MAX_ACCESSIBILITY_ISSUES as u64, "every issue is listed: {answer:?}");
    assert_eq!(answer["truncated"], json!(false), "nothing was cut: {answer:?}");
}

/// The **count** of the answer is the count of the issues there were.
///
/// The count used to be the count of the issues that fitted, so a
/// document with more issues than the bound was told it had exactly the
/// bound: the number and the list said the same thing, and the number
/// was wrong.
#[test]
fn the_count_is_the_count_of_the_issues_there_were() {
    let mut session = session();
    let document = a_document(&mut session);
    for index in 0..=MAX_ACCESSIBILITY_ISSUES {
        a_picture(&mut session, document, index);
    }
    let answer = check(&mut session, document);
    let count = count_of(&answer);
    assert_eq!(count, MAX_ACCESSIBILITY_ISSUES as u64 + 1, "the count is every issue there was: {answer:?}");
    assert!(count > listed_of(&answer), "and it is more than the list holds: {answer:?}");
}

/// A table with **more columns** than `MAX_ACCESSIBILITY_COLUMNS` has
/// columns that are not looked at, and the answer says how many.
#[test]
fn the_columns_past_the_bound_are_not_looked_at_and_are_counted() {
    let mut session = session();
    let document = a_document(&mut session);
    let width = MAX_ACCESSIBILITY_COLUMNS + 2;
    let mut row = Vec::new();
    for _ in 0..width {
        row.push(Json::Null);
    }
    let mut filled = row.clone();
    filled[0] = json!("something");
    session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [filled, row] }))
        .expect("the table is added");
    let answer = check(&mut session, document);
    assert_eq!(
        answer["columns_cut"],
        json!(2),
        "the answer says how many columns were not looked at: {answer:?}"
    );
    let issues = issues_of(&answer);
    let columns: Vec<String> = issues
        .iter()
        .filter(|issue| issue["rule"] == json!("blankColumn"))
        .map(|issue| issue["issue"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        columns.len(),
        MAX_ACCESSIBILITY_COLUMNS - 1,
        "the columns that were looked at are the ones reported: {columns:?}"
    );
    assert!(
        !columns.iter().any(|column| column.contains("Column 64")),
        "a column past the bound is not reported: {columns:?}"
    );
    assert!(
        !columns.iter().any(|column| column.contains("Column 65")),
        "a column past the bound is not reported: {columns:?}"
    );
}

/// A table with **as many columns** as the bound allows is looked at
/// whole, and nothing is cut.
#[test]
fn the_columns_at_the_bound_are_looked_at_whole() {
    let mut session = session();
    let document = a_document(&mut session);
    let width = MAX_ACCESSIBILITY_COLUMNS;
    let mut row = Vec::new();
    for _ in 0..width {
        row.push(Json::Null);
    }
    let mut filled = row.clone();
    filled[0] = json!("something");
    session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [filled, row] }))
        .expect("the table is added");
    let answer = check(&mut session, document);
    assert_eq!(answer["columns_cut"], json!(0), "nothing was cut: {answer:?}");
    let issues = issues_of(&answer);
    let columns: Vec<String> = issues
        .iter()
        .filter(|issue| issue["rule"] == json!("blankColumn"))
        .map(|issue| issue["issue"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        columns.len(),
        MAX_ACCESSIBILITY_COLUMNS - 1,
        "every column that was looked at is reported: {columns:?}"
    );
}
