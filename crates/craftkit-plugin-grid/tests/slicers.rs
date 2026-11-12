//! The **slicer** of a table: a widget that filters a column of the
//! table by the values that are chosen.
//!
//! The tree's rules: the values of the slicer are the texts the cells
//! of the column show, a single click keeps that value alone, a multi
//! click adds a value to the ones that are kept, and a filter that
//! keeps everything filters nothing.

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

/// A sheet with a **table** of regions and amounts, and its id.
fn a_sheet_with_a_table(session: &mut Session) -> u64 {
    let sheet = a_sheet(session);
    let cells = [
        ("A1", "region"),
        ("B1", "amount"),
        ("A2", "East"),
        ("B2", "10"),
        ("A3", "West"),
        ("B3", "20"),
        ("A4", "East"),
        ("B4", "30"),
    ];
    for (cell, value) in cells {
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
            .expect("the cell is set");
    }
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "sales", "range": "A1:B4" }))
        .expect("the table is made");
    sheet
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

/// The **values** the filter of the sheet keeps.
fn filter_values(session: &Session, sheet: u64) -> Option<Vec<String>> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["filter"]["values"].as_array().cloned().map(|values| {
        values.iter().filter_map(|value| value.as_str().map(|value| value.to_string())).collect()
    })
}

/// The **items** of a slicer, as the answer holds them.
fn items_of(session: &mut Session, sheet: u64) -> Vec<(String, bool)> {
    let outcome =
        session.execute("grid.slicer.read", &json!({ "sheet": sheet })).expect("the slicer is read");
    outcome.data["items"]
        .as_array()
        .cloned()
        .expect("the items are a list")
        .iter()
        .map(|item| {
            (
                item["value"].as_str().expect("the item has a value").to_string(),
                item["selected"].as_bool().expect("the item says whether it is selected"),
            )
        })
        .collect()
}

/// A slicer is bound to a **column of a table**, and it starts with
/// every value kept.
#[test]
fn a_slicer_is_bound_to_a_column_of_a_table() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    let outcome = session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    assert_eq!(outcome.data["column"], json!("region"), "the slicer is on the column");
    assert_eq!(
        items_of(&mut session, sheet),
        vec![("East".to_string(), true), ("West".to_string(), true)],
        "the values of the column are its items, and they are all kept"
    );
}

/// A value that is kept **hides the rows** that do not show it.
#[test]
fn a_value_that_is_kept_hides_the_rows_that_do_not_show_it() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East" }))
        .expect("the value is toggled");
    // The **heading row** is hidden as well: the filter of a sheet is
    // over the sheet, and not over the range of the table.
    assert_eq!(filtered_rows(&mut session, sheet), vec![1, 3], "the row of the west is hidden");
}

/// A single toggle keeps that value **alone**.
#[test]
fn a_single_toggle_keeps_that_value_alone() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East" }))
        .expect("the value is toggled");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "West" }))
        .expect("the value is toggled");
    assert_eq!(filter_values(&session, sheet), Some(vec!["West".to_string()]), "only the west is kept");
}

/// A toggle of the **only value that is kept** takes the filter away.
#[test]
fn a_toggle_of_the_only_value_that_is_kept_takes_the_filter_away() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East" }))
        .expect("the value is toggled");
    assert_eq!(filtered_rows(&mut session, sheet), vec![1, 3], "the row of the west is hidden");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East" }))
        .expect("the value is toggled again");
    assert!(filter_values(&session, sheet).is_none(), "the filter is taken away");
    assert!(filtered_rows(&mut session, sheet).is_empty(), "no row is hidden");
}

/// A **multi** toggle adds a value and takes it away again.
#[test]
fn a_multi_toggle_adds_a_value_and_takes_it_away() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    // A multi toggle on a slicer that keeps everything takes the value
    // away: the tree's own rule.
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East", "multi": true }))
        .expect("the value is toggled");
    assert_eq!(filter_values(&session, sheet), Some(vec!["West".to_string()]), "the east is taken away");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East", "multi": true }))
        .expect("the value is toggled again");
    // Every value is kept, and a filter that keeps everything filters
    // nothing.
    assert!(filter_values(&session, sheet).is_none(), "the filter is taken away");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "West", "multi": true }))
        .expect("the value is toggled");
    assert_eq!(filter_values(&session, sheet), Some(vec!["East".to_string()]), "only the east is kept");
}

/// A value that is **not in the column** is refused.
#[test]
fn a_value_that_is_not_in_the_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    let outcome = session.execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "North" }));
    assert!(outcome.is_err(), "the north is not in the column: {outcome:?}");
}

/// A slicer that is **cleared** keeps every value.
#[test]
fn a_slicer_that_is_cleared_keeps_every_value() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East" }))
        .expect("the value is toggled");
    session.execute("grid.slicer.clear", &json!({ "sheet": sheet })).expect("the slicer is cleared");
    assert!(filter_values(&session, sheet).is_none(), "the filter is taken away");
    assert_eq!(
        items_of(&mut session, sheet),
        vec![("East".to_string(), true), ("West".to_string(), true)],
        "every value is kept"
    );
}

/// A slicer that is **removed** takes its filter away.
#[test]
fn a_slicer_that_is_removed_takes_its_filter_away() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    session
        .execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "region" }))
        .expect("the slicer is added");
    session
        .execute("grid.slicer.toggle", &json!({ "sheet": sheet, "value": "East" }))
        .expect("the value is toggled");
    session.execute("grid.slicer.remove", &json!({ "sheet": sheet })).expect("the slicer is removed");
    assert!(filter_values(&session, sheet).is_none(), "the filter is taken away");
    let outcome = session.execute("grid.slicer.read", &json!({ "sheet": sheet }));
    assert!(outcome.is_err(), "there is no slicer any more: {outcome:?}");
}

/// A **table** that is not there is refused, and so is a column.
#[test]
fn a_table_that_is_not_there_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_table(&mut session);
    let outcome = session.execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "costs" }));
    assert!(outcome.is_err(), "there is no table of that name: {outcome:?}");
    let outcome =
        session.execute("grid.slicer.add", &json!({ "sheet": sheet, "table": "sales", "column": "who" }));
    assert!(outcome.is_err(), "the table has no column of that name: {outcome:?}");
}
