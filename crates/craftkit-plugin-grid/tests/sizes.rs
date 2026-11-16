//! The **sizes** of a sheet.
//!
//! A row has a height in points and a column has a width in characters.

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
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **height** of a row, if the sheet says one.
fn height_of(session: &Session, sheet: u64, row: &str) -> Option<f64> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["rows"][row].as_f64()
}

/// The **width** of a column, if the sheet says one.
fn width_of(session: &Session, sheet: u64, column: &str) -> Option<f64> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["columns"][column].as_f64()
}

/// A row may be given a **height**, in points.
#[test]
fn a_row_may_be_given_a_height() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 3, "height": 30 }))
        .expect("the height is set");
    assert_eq!(height_of(&session, sheet, "3"), Some(30.0), "the sheet says so");
}

/// A column may be given a **width**, in characters.
#[test]
fn a_column_may_be_given_a_width() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "B", "width": 12.5 }))
        .expect("the width is set");
    assert_eq!(width_of(&session, sheet, "B"), Some(12.5), "the sheet says so");
}

/// A row may be given a height **again**, and the last one is the one.
#[test]
fn a_row_may_be_given_a_height_again() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 3, "height": 30 }))
        .expect("the height is set");
    session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 3, "height": 20 }))
        .expect("the height is set again");
    assert_eq!(height_of(&session, sheet, "3"), Some(20.0), "the sheet says so");
}

/// A row that is **not there** is refused: a row is counted from one.
#[test]
fn a_row_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 0, "height": 30 }))
        .expect_err("the height is refused");
    assert!(error.to_string().contains("counted from one"), "the error says so: {error}");
}

/// A height of **nothing** is refused: it would be a hidden row, and the
/// hiding of a row is not here.
#[test]
fn a_height_of_nothing_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 3, "height": 0 }))
        .expect_err("the height is refused");
    assert!(error.to_string().contains("hidden row"), "the error says so: {error}");
}

/// A height that is **not there** is refused.
#[test]
fn a_height_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 3, "height": -5 }))
        .expect_err("the height is refused");
    assert!(error.to_string().contains("is a number that is there"), "the error says so: {error}");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "3", "width": 10 }))
        .expect_err("the width is refused");
    assert!(error.to_string().contains("is not a column"), "the error says so: {error}");
}

/// A column may be named in **any case**: `b` and `B` are the same
/// column.
#[test]
fn a_column_may_be_named_in_any_case() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "b", "width": 9 }))
        .expect("the width is set");
    assert_eq!(width_of(&session, sheet, "B"), Some(9.0), "the sheet says so");
}

/// A width of **nothing** is refused, for the same reason a height of
/// nothing is.
#[test]
fn a_width_of_nothing_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "B", "width": 0 }))
        .expect_err("the width is refused");
    assert!(error.to_string().contains("hidden"), "the error says so: {error}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.height", &json!({ "sheet": workbook, "row": 3, "height": 30 }))
        .expect_err("the height is refused");
    assert!(error.to_string().contains("is not a sheet"), "the error says so: {error}");
}

/// The size is **saved**: it is in the document after it is done.
#[test]
fn the_size_is_saved() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.row.height", &json!({ "sheet": sheet, "row": 3, "height": 30 }))
        .expect("the height is set");
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "B", "width": 12.5 }))
        .expect("the width is set");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    assert_eq!(node.data["rows"]["3"].as_f64(), Some(30.0), "the height is there");
    assert_eq!(node.data["columns"]["B"].as_f64(), Some(12.5), "and the width is there");
}

/// The **default width** of a sheet may be given, in characters: the
/// width of a column that has no width of its own.
#[test]
fn a_sheet_may_be_given_a_default_width() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.sheet.default.width", &json!({ "sheet": sheet, "width": 20 }))
        .expect("the default width is set");
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["default_width"].as_f64(), Some(20.0), "the sheet says so");
}

/// The **default height** of a sheet may be given, in points: the
/// height of a row that has no height of its own.
#[test]
fn a_sheet_may_be_given_a_default_height() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.sheet.default.height", &json!({ "sheet": sheet, "height": 24 }))
        .expect("the default height is set");
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["default_height"].as_f64(), Some(24.0), "the sheet says so");
}

/// A sheet that has **no** default width has none: the tree's own
/// width is a fact of the tree's renderer and not of the sheet.
#[test]
fn a_sheet_with_no_default_width_has_none() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data.get("default_width"), None, "the sheet says nothing");
}

/// A default width of **nothing** is refused, for the same reason a
/// width of nothing is.
#[test]
fn a_default_width_of_nothing_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.sheet.default.width", &json!({ "sheet": sheet, "width": 0 }))
        .expect_err("the default width is refused");
    assert!(error.to_string().contains("hidden"), "the error says so: {error}");
}

/// A default height of **nothing** is refused, for the same reason a
/// height of nothing is.
#[test]
fn a_default_height_of_nothing_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.sheet.default.height", &json!({ "sheet": sheet, "height": 0 }))
        .expect_err("the default height is refused");
    assert!(error.to_string().contains("hidden"), "the error says so: {error}");
}

/// A node that is **not a sheet** is refused, for the default width as
/// for the rest.
#[test]
fn a_node_that_is_not_a_sheet_is_refused_for_the_default_width() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.sheet.default.width", &json!({ "sheet": workbook, "width": 20 }))
        .expect_err("the default width is refused");
    assert!(error.to_string().contains("is not a sheet"), "the error says so: {error}");
}

/// A column that has **no width of its own** has the **sheet's**
/// default width: the text of a cell is written in the width of the
/// column it is in.
#[test]
fn a_column_has_the_default_width_of_its_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1234.5 }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "comma" }))
        .expect("the format is set");
    // A sheet with no default width: the text is as long as it is.
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["text"].as_str(), Some("1,234.50"), "the text is as long as it is");
    // A sheet with a default width: the text is as wide as the column.
    session
        .execute("grid.sheet.default.width", &json!({ "sheet": sheet, "width": 20 }))
        .expect("the default width is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    let text = outcome.data["text"].as_str().expect("the cell has text");
    assert_eq!(text.chars().count(), 20, "the text is as wide as the column");
}

/// A column that has a **width of its own** keeps it: the default
/// width of the sheet is for the columns that have no width.
#[test]
fn a_column_with_a_width_of_its_own_keeps_it() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1234.5 }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "comma" }))
        .expect("the format is set");
    session
        .execute("grid.sheet.default.width", &json!({ "sheet": sheet, "width": 20 }))
        .expect("the default width is set");
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "A", "width": 12 }))
        .expect("the width is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    let text = outcome.data["text"].as_str().expect("the cell has text");
    assert_eq!(text.chars().count(), 12, "the text is as wide as its own column");
}

/// The **defaults** of a sheet are said when the workbook is read.
#[test]
fn the_defaults_of_a_sheet_are_said() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.sheet.default.width", &json!({ "sheet": sheet, "width": 20 }))
        .expect("the default width is set");
    session
        .execute("grid.sheet.default.height", &json!({ "sheet": sheet, "height": 24 }))
        .expect("the default height is set");
    let outcome =
        session.execute("grid.workbook.get", &json!({ "workbook": workbook })).expect("the workbook is read");
    let sheets = outcome.data["sheets"].as_array().expect("the workbook has sheets");
    assert_eq!(sheets[0]["default_width"].as_f64(), Some(20.0), "the width is said");
    assert_eq!(sheets[0]["default_height"].as_f64(), Some(24.0), "and the height is said");
}
