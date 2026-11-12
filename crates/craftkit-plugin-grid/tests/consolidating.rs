//! The **consolidation** of several ranges of cells into one.
//!
//! The rows and the columns of the result are the rows and the columns
//! of the sources, and the values of it are the values of them added
//! up. The rules are the tree's own.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **workbook** with a sheet in it, and the id of the sheet.
fn a_sheet(session: &mut Session, name: &str) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": name }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// Put a **value** in a cell.
fn set(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **value** of a cell.
fn get(session: &mut Session, sheet: u64, cell: &str) -> Json {
    session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read").data
        ["value"]
        .clone()
}

/// Whether a value is a **number** near the one that is expected.
fn near(value: &Json, expected: f64) -> bool {
    value.as_f64().is_some_and(|number| (number - expected).abs() < 1e-9)
}

/// The two sources of the tree's own test: the regions of the east and
/// of the west, and the two quarters.
fn the_sources(session: &mut Session) -> (u64, u64, u64) {
    let (_, east) = a_sheet(session, "East");
    let (_, west) = a_sheet(session, "West");
    let (_, result) = a_sheet(session, "Result");
    for (sheet, north, south) in [(east, 10, 30), (west, 5, 7)] {
        set(session, sheet, "A1", json!("Region"));
        set(session, sheet, "B1", json!("Q1"));
        set(session, sheet, "C1", json!("Q2"));
        set(session, sheet, "A2", json!("North"));
        set(session, sheet, "B2", json!(north));
        set(session, sheet, "C2", json!(north + 10));
        set(session, sheet, "A3", json!("South"));
        set(session, sheet, "B3", json!(south));
        set(session, sheet, "C3", json!(south + 10));
    }
    (east, west, result)
}

/// The sources are added up into **one table**: the labels of the rows
/// and of the columns, and the values of them.
#[test]
fn the_sources_are_added_up_into_one_table() {
    let mut session = session();
    let (east, west, result) = the_sources(&mut session);
    let outcome = session
        .execute(
            "grid.data.consolidate",
            &json!({
                "sources": [
                    { "sheet": east, "range": "A1:C3" },
                    { "sheet": west, "range": "A1:C3" },
                ],
                "sheet": result,
                "destination": "A1",
                "top_row": true,
                "left_column": true,
            }),
        )
        .expect("the sources are consolidated");
    assert_eq!(outcome.data["rows"], json!(2), "there are two rows: {outcome:?}");
    assert_eq!(outcome.data["columns"], json!(2), "and two columns: {outcome:?}");
    // The **labels** of the columns are in the first row, and the
    // labels of the rows are in the first column.
    assert_eq!(get(&mut session, result, "B1"), json!("Q1"), "the first column is named: ");
    assert_eq!(get(&mut session, result, "C1"), json!("Q2"), "and the second one too: ");
    assert_eq!(get(&mut session, result, "A2"), json!("North"), "the first row is named: ");
    assert_eq!(get(&mut session, result, "A3"), json!("South"), "and the second one too: ");
    // The **values** are the values of both sources, added up.
    assert!(near(&get(&mut session, result, "B2"), 15.0), "the north of the first quarter is fifteen");
    assert!(near(&get(&mut session, result, "C2"), 35.0), "and of the second one it is thirty-five");
    assert!(near(&get(&mut session, result, "B3"), 37.0), "the south of the first quarter is thirty-seven");
    assert!(near(&get(&mut session, result, "C3"), 57.0), "and of the second one it is fifty-seven");
}

/// The **way** the values are added up in is the tree's own, and a way
/// that is not one is refused by name.
#[test]
fn the_way_the_values_are_added_up_in_is_the_trees_own() {
    let mut session = session();
    let (east, west, result) = the_sources(&mut session);
    let sources = json!([
        { "sheet": east, "range": "A1:C3" },
        { "sheet": west, "range": "A1:C3" },
    ]);
    for (way, expected) in [("sum", 15.0), ("average", 7.5), ("max", 10.0), ("min", 5.0), ("count", 2.0)] {
        session
            .execute(
                "grid.data.consolidate",
                &json!({
                    "sources": sources,
                    "sheet": result,
                    "destination": "A1",
                    "top_row": true,
                    "left_column": true,
                    "function": way,
                }),
            )
            .expect("the sources are consolidated");
        assert!(near(&get(&mut session, result, "B2"), expected), "the {way} of the north is {expected}");
    }
    let error = session
        .execute(
            "grid.data.consolidate",
            &json!({
                "sources": sources,
                "sheet": result,
                "destination": "A1",
                "function": "median",
            }),
        )
        .expect_err("the way is refused");
    let message = error.to_string();
    assert!(message.contains("median"), "the way is named: {message}");
}

/// A source with **no** sources is refused, and so is a source with no
/// sheet or no range.
#[test]
fn a_consolidation_with_no_source_is_refused() {
    let mut session = session();
    let (_, _, result) = the_sources(&mut session);
    let error = session
        .execute("grid.data.consolidate", &json!({ "sources": [], "sheet": result, "destination": "A1" }))
        .expect_err("the sources are refused");
    let message = error.to_string();
    assert!(message.contains("at least one source"), "the sources are named: {message}");
    let error = session
        .execute(
            "grid.data.consolidate",
            &json!({ "sources": [{ "range": "A1:C3" }], "sheet": result, "destination": "A1" }),
        )
        .expect_err("the source is refused");
    assert!(error.to_string().contains("sheet"), "the sheet of it is named: {error}");
}

/// The **labels** of the rows and of the columns are matched without
/// regard to their case: a source whose header is written in small
/// letters joins the one whose header is written in capitals.
#[test]
fn the_labels_are_matched_without_regard_to_their_case() {
    let mut session = session();
    let (east, west, result) = the_sources(&mut session);
    set(&mut session, west, "B1", json!("q1"));
    set(&mut session, west, "C1", json!("q2"));
    session
        .execute(
            "grid.data.consolidate",
            &json!({
                "sources": [
                    { "sheet": east, "range": "A1:C3" },
                    { "sheet": west, "range": "A1:C3" },
                ],
                "sheet": result,
                "destination": "A1",
                "top_row": true,
                "left_column": true,
            }),
        )
        .expect("the sources are consolidated");
    let outcome = session
        .execute("grid.data.consolidate", &json!({ "sources": [], "sheet": result, "destination": "A1" }))
        .expect_err("the sources are refused");
    assert!(outcome.to_string().contains("at least one source"), "the sources are named: {outcome}");
    assert!(near(&get(&mut session, result, "B2"), 15.0), "the two quarters are one: ");
    assert_eq!(get(&mut session, result, "B1"), json!("Q1"), "and the label of it is the first one: ");
}
