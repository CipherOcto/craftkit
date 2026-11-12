//! The **import of a list of objects**: the tree's JSON.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the grid battery.
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

/// A **workbook**.
fn a_workbook(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    outcome.data["id"].as_u64().expect("the workbook has an id")
}

/// A **list** imported into a workbook, and the sheet it made.
fn an_import(session: &mut Session, workbook: u64, data: craftkit_sdk::serde_json::Value) -> u64 {
    let outcome = session
        .execute("grid.import.json", &json!({ "workbook": workbook, "data": data }))
        .expect("the data is imported");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **value** of a cell.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Option<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    let value = outcome.data["value"].clone();
    if value.is_null() { None } else { Some(value) }
}

/// **The keys of the objects are the columns, and the first row holds
/// them.**
#[test]
fn the_keys_are_the_columns() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let sheet = an_import(
        &mut session,
        workbook,
        json!([{ "who": "ann", "amount": 3 }, { "who": "bob", "amount": 5 }]),
    );
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("amount")));
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!("who")));
    assert_eq!(value_of(&mut session, sheet, "B2"), Some(json!("ann")));
    assert_eq!(value_of(&mut session, sheet, "A2"), Some(json!(3.0)));
    assert_eq!(value_of(&mut session, sheet, "A3"), Some(json!(5.0)));
}

/// **A key that an object does not have is nothing in that row.**
#[test]
fn a_key_that_is_not_there_is_nothing() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let sheet = an_import(&mut session, workbook, json!([{ "who": "ann", "amount": 3 }, { "who": "bob" }]));
    assert_eq!(value_of(&mut session, sheet, "A3"), None, "and the cell is empty");
}

/// **Every key is a column, and a key of the second object is a column
/// too.**
#[test]
fn the_columns_are_in_the_order_the_objects_hold_them() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let sheet = an_import(
        &mut session,
        workbook,
        json!([{ "who": "ann", "amount": 3 }, { "when": "today", "who": "bob" }]),
    );
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("amount")));
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!("who")));
    assert_eq!(
        value_of(&mut session, sheet, "C1"),
        Some(json!("when")),
        "and the key of the second object is a column too"
    );
}

/// **A boolean is a boolean and a null is nothing.**
#[test]
fn a_boolean_is_a_boolean_and_a_null_is_nothing() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let sheet = an_import(&mut session, workbook, json!([{ "paid": true, "note": null }]));
    assert_eq!(value_of(&mut session, sheet, "B2"), Some(json!(true)));
    assert_eq!(value_of(&mut session, sheet, "A2"), None);
}

/// **A value that is a structure is refused.**
#[test]
fn a_value_that_is_a_structure_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute(
            "grid.import.json",
            &json!({ "workbook": workbook, "data": json!([{ "who": { "first": "ann" } }]) }),
        )
        .expect_err("the data is refused");
    let message = failure.to_string();
    assert!(message.contains("a cell holds a value and not a structure"), "the refusal says why: {message}");
}

/// **A thing in the list that is not an object is refused.**
#[test]
fn a_thing_that_is_not_an_object_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.json", &json!({ "workbook": workbook, "data": json!([1, 2]) }))
        .expect_err("the data is refused");
    let message = failure.to_string();
    assert!(message.contains("the data is a list of objects"), "the refusal says why: {message}");
}

/// **Data that is not a list is refused.**
#[test]
fn data_that_is_not_a_list_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.json", &json!({ "workbook": workbook, "data": json!({ "who": "ann" }) }))
        .expect_err("the data is refused");
    let message = failure.to_string();
    assert!(message.contains("the data is a list of objects"), "the refusal says why: {message}");
}

/// **A list that holds nothing is refused.**
#[test]
fn a_list_that_holds_nothing_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.json", &json!({ "workbook": workbook, "data": json!([]) }))
        .expect_err("the data is refused");
    let message = failure.to_string();
    assert!(message.contains("holds nothing to import"), "the refusal says why: {message}");
}
