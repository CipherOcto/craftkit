//! The **list filter**: a filter that keeps the rows whose cell shows
//! one of a list of values.
//!
//! The tree's rules: the rows whose cell shows one of the values are
//! kept, the comparison is of the text the cell shows and it is
//! without case, and an empty cell is kept when blanks are asked for.

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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **rows** a sheet's filter hides.
fn filtered_rows(session: &mut Session, sheet: u64) -> Vec<u64> {
    let outcome = session.execute("grid.filter.rows", &json!({ "sheet": sheet })).expect("the rows are read");
    outcome.data["rows"]
        .as_array()
        .cloned()
        .expect("the rows are a list")
        .iter()
        .filter_map(|row| row.as_u64())
        .collect()
}

/// The rows of a sheet, with a value in the column `A`.
fn a_sheet_with(session: &mut Session, values: &[&str]) -> u64 {
    let sheet = a_sheet(session);
    for (at, value) in values.iter().enumerate() {
        let cell = format!("A{}", at + 1);
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
            .expect("the cell is set");
    }
    sheet
}

/// The rows whose cell shows one of the values are **kept**.
#[test]
fn the_rows_that_show_one_of_the_values_are_kept() {
    let mut session = session();
    let sheet = a_sheet_with(&mut session, &["East", "West", "North"]);
    session
        .execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": ["East", "West"] }))
        .expect("the sheet is filtered");
    assert_eq!(filtered_rows(&mut session, sheet), vec![3], "the third row is hidden");
}

/// The comparison is **without case**.
#[test]
fn the_comparison_is_without_case() {
    let mut session = session();
    let sheet = a_sheet_with(&mut session, &["East", "West"]);
    session
        .execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": ["east"] }))
        .expect("the sheet is filtered");
    assert_eq!(filtered_rows(&mut session, sheet), vec![2], "the second row is hidden");
}

/// A cell that shows a **number** is compared as its text.
#[test]
fn a_cell_that_shows_a_number_is_compared_as_its_text() {
    let mut session = session();
    let sheet = a_sheet_with(&mut session, &["10", "20"]);
    session
        .execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": ["10"] }))
        .expect("the sheet is filtered");
    assert_eq!(filtered_rows(&mut session, sheet), vec![2], "the second row is hidden");
}

/// An empty cell is kept when **blanks** are asked for.
#[test]
fn an_empty_cell_is_kept_when_blanks_are_asked_for() {
    let mut session = session();
    let sheet = a_sheet_with(&mut session, &["East", ""]);
    session
        .execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": ["East"] }))
        .expect("the sheet is filtered");
    assert_eq!(filtered_rows(&mut session, sheet), vec![2], "the empty row is hidden");
    session
        .execute(
            "grid.filter.values",
            &json!({ "sheet": sheet, "column": "A", "values": ["East"], "blanks": true }),
        )
        .expect("the sheet is filtered");
    assert!(filtered_rows(&mut session, sheet).is_empty(), "the empty row is kept");
}

/// A list that is **not a list** is refused.
#[test]
fn a_list_that_is_not_a_list_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": "East" }));
    assert!(outcome.is_err(), "the values are not a list: {outcome:?}");
}

/// A value that is **not text** is refused.
#[test]
fn a_value_that_is_not_text_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": ["East", 7] }));
    assert!(outcome.is_err(), "the value is not text: {outcome:?}");
}

/// A list that holds **too many** values is refused.
#[test]
fn a_list_that_holds_too_many_values_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let values: Vec<String> = (0..1025).map(|at| format!("value {at}")).collect();
    let outcome =
        session.execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": values }));
    assert!(outcome.is_err(), "the list is too long: {outcome:?}");
}

/// The filter is taken away when it is **cleared**.
#[test]
fn the_filter_is_taken_away_when_it_is_cleared() {
    let mut session = session();
    let sheet = a_sheet_with(&mut session, &["East", "West"]);
    session
        .execute("grid.filter.values", &json!({ "sheet": sheet, "column": "A", "values": ["East"] }))
        .expect("the sheet is filtered");
    assert_eq!(filtered_rows(&mut session, sheet), vec![2], "the second row is hidden");
    session.execute("grid.filter.clear", &json!({ "sheet": sheet })).expect("the filter is cleared");
    assert!(filtered_rows(&mut session, sheet).is_empty(), "no row is hidden");
}
