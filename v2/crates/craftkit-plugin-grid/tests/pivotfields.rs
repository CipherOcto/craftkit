//! The **fields** of a pivot: the columns of the range it reads, and
//! the places of the report each of them sits in.

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

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A sheet of three **spendings**: ann three in 2024, bob five in 2024
/// and ann two in 2025.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    a_text(session, sheet, "A1", "who");
    a_text(session, sheet, "B1", "amount");
    a_text(session, sheet, "C1", "when");
    a_text(session, sheet, "A2", "ann");
    a_number(session, sheet, "B2", 3.0);
    a_number(session, sheet, "C2", 45292.0);
    a_text(session, sheet, "A3", "bob");
    a_number(session, sheet, "B3", 5.0);
    a_number(session, sheet, "C3", 45292.0);
    a_text(session, sheet, "A4", "ann");
    a_number(session, sheet, "B4", 2.0);
    a_number(session, sheet, "C4", 45658.0);
    sheet
}

/// The **fields** of a pivot.
fn fields(session: &mut Session, sheet: u64, name: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("grid.pivot.fields", &json!({ "sheet": sheet, "name": name }))
        .expect("the fields are read");
    outcome.data["fields"].as_array().expect("the fields are a list").clone()
}

/// The **field** of a pivot that is the column of a letter.
fn field(session: &mut Session, sheet: u64, name: &str, column: &str) -> craftkit_sdk::serde_json::Value {
    let fields = fields(session, sheet, name);
    fields
        .into_iter()
        .find(|field| field["column"] == json!(column))
        .unwrap_or_else(|| panic!("there is a field for the column {column}"))
}

/// **The fields of a pivot are the columns of the range it reads.**
#[test]
fn the_fields_of_a_pivot_are_the_columns_of_its_range() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let fields = fields(&mut session, sheet, "by who");
    assert_eq!(fields.len(), 3, "the three columns of the range: {fields:?}");
    let names: Vec<&str> = fields.iter().map(|field| field["name"].as_str().unwrap_or_default()).collect();
    assert_eq!(names, vec!["who", "amount", "when"], "named by their headers");
    let columns: Vec<&str> =
        fields.iter().map(|field| field["column"].as_str().unwrap_or_default()).collect();
    assert_eq!(columns, vec!["A", "B", "C"], "and each says which column it is");
}

/// **A field of numbers is numeric, and a field of text is not.**
#[test]
fn a_field_of_numbers_is_numeric_and_a_field_of_text_is_not() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let amount = field(&mut session, sheet, "by who", "B");
    assert_eq!(amount["numeric"], json!(true), "every amount is a number");
    assert_eq!(amount["date"], json!(false), "and none of them is a date");
    let who = field(&mut session, sheet, "by who", "A");
    assert_eq!(who["numeric"], json!(false), "a name is not a number");
}

/// **A field of dates is a date field.**
#[test]
fn a_field_of_dates_is_a_date_field() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "C2", "format": "yyyy-mm-dd" }))
        .expect("the cell is formatted");
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let when = field(&mut session, sheet, "by who", "C");
    assert_eq!(when["date"], json!(true), "the numbers are dates");
    assert_eq!(when["numeric"], json!(true), "and they are numbers too");
    let samples = when["samples"].as_array().expect("the samples are a list");
    assert_eq!(samples[0], json!("2024-01-01"), "and they are written as dates: {samples:?}");
}

/// **The samples of a field are without repeats and in the tree's own
/// order.**
#[test]
fn the_samples_of_a_field_are_without_repeats() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let who = field(&mut session, sheet, "by who", "A");
    let samples = who["samples"].as_array().expect("the samples are a list");
    assert_eq!(samples, &vec![json!("ann"), json!("bob")], "ann is said once: {samples:?}");
    let amount = field(&mut session, sheet, "by who", "B");
    let samples = amount["samples"].as_array().expect("the samples are a list");
    assert_eq!(samples, &vec![json!("2"), json!("3"), json!("5")], "the numbers in order: {samples:?}");
}

/// **The samples of a field are five at the most.**
#[test]
fn the_samples_of_a_field_are_five_at_the_most() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 2..=9 {
        a_number(&mut session, sheet, &format!("B{row}"), row as f64);
        a_text(&mut session, sheet, &format!("A{row}"), &format!("who{row}"));
    }
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C9", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let who = field(&mut session, sheet, "by who", "A");
    let samples = who["samples"].as_array().expect("the samples are a list");
    assert_eq!(samples.len(), 5, "five of them: {samples:?}");
    assert_eq!(samples[0], json!("who2"), "the first of them");
}

/// **The areas of a field are the places of the report it sits in.**
#[test]
fn the_areas_of_a_field_are_the_places_it_sits_in() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "by who",
                "range": "A1:C4",
                "row": "A",
                "column": "C",
                "value": "B",
            }),
        )
        .expect("the pivot is made");
    let who = field(&mut session, sheet, "by who", "A");
    assert_eq!(who["areas"], json!(["rows"]), "the rows are gathered by it");
    let amount = field(&mut session, sheet, "by who", "B");
    assert_eq!(amount["areas"], json!(["values"]), "the numbers are put together from it");
    let when = field(&mut session, sheet, "by who", "C");
    assert_eq!(when["areas"], json!(["columns"]), "and the groups are spread over it");
}

/// **The records of a pivot are the rows of the range that hold
/// something.**
#[test]
fn the_records_of_a_pivot_are_the_rows_that_hold_something() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    // An empty row in the middle of the range.
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "A5" }))
        .expect("the cell is cleared");
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C5", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let outcome = session
        .execute("grid.pivot.fields", &json!({ "sheet": sheet, "name": "by who" }))
        .expect("the fields are read");
    assert_eq!(outcome.data["records"], json!(3), "the three rows that hold something");
}

/// **A header that holds nothing does not name a field, and the fields
/// are refused.**
#[test]
fn a_header_that_holds_nothing_does_not_name_a_field() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "B1" }))
        .expect("the header is cleared");
    let refusal = session
        .execute("grid.pivot.fields", &json!({ "sheet": sheet, "name": "by who" }))
        .expect_err("the fields are refused");
    let message = refusal.to_string();
    assert!(message.contains("B1"), "the refusal says which cell: {message}");
}
