//! The **protection** of a sheet.

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

/// A **workbook** with a sheet in it, and their ids.
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// Protect a sheet, with the permissions that are given.
fn protect(session: &mut Session, sheet: u64, permissions: Json) {
    let mut params = json!({ "sheet": sheet });
    for (name, value) in permissions.as_object().expect("the permissions are an object") {
        params[name] = value.clone();
    }
    session.execute("grid.sheet.protect", &params).expect("the sheet is protected");
}

/// The **cells** of a protected sheet are locked.
#[test]
fn the_cells_of_a_protected_sheet_are_locked() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    protect(&mut session, sheet, json!({}));
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("locked"), "it says why: {error}");
}

/// A **formula** may not be put in a locked cell either.
#[test]
fn a_formula_may_not_be_put_in_a_locked_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    protect(&mut session, sheet, json!({}));
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=1+1" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("locked"), "it says why: {error}");
}

/// The **sorting** may be allowed, and it may not.
#[test]
fn the_sorting_may_be_allowed_and_it_may_not() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2 }))
        .expect("the value is set");
    protect(&mut session, sheet, json!({}));
    let error = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "A" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("sorting is not allowed"), "it says why: {error}");
    session.execute("grid.sheet.unprotect", &json!({ "sheet": sheet })).expect("the sheet is unprotected");
    protect(&mut session, sheet, json!({ "sort": true }));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "A" }))
        .expect("the range is sorted");
}

/// The **filtering** may be allowed, and it may not.
#[test]
fn the_filtering_may_be_allowed_and_it_may_not() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    protect(&mut session, sheet, json!({}));
    let error = session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "=", "value": "red" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("filtering is not allowed"), "it says why: {error}");
    session.execute("grid.sheet.unprotect", &json!({ "sheet": sheet })).expect("the sheet is unprotected");
    protect(&mut session, sheet, json!({ "filter": true }));
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "=", "value": "red" }))
        .expect("the filter is set");
}

/// The **formatting** may be allowed, and it may not.
#[test]
fn the_formatting_may_be_allowed_and_it_may_not() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    protect(&mut session, sheet, json!({}));
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("formatting is not allowed"), "it says why: {error}");
    session.execute("grid.sheet.unprotect", &json!({ "sheet": sheet })).expect("the sheet is unprotected");
    protect(&mut session, sheet, json!({ "format": true }));
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
}

/// A sheet may be **unprotected**, and then its cells may be changed
/// again.
#[test]
fn a_sheet_may_be_unprotected_and_then_its_cells_may_be_changed_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    protect(&mut session, sheet, json!({}));
    let outcome =
        session.execute("grid.sheet.unprotect", &json!({ "sheet": sheet })).expect("it is unprotected");
    assert_eq!(outcome.data["was_there"], Json::Bool(true), "there was a protection");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error =
        session.execute("grid.sheet.protect", &json!({ "sheet": workbook })).expect_err("it is refused");
    assert!(error.to_string().contains("not a sheet"), "it says why: {error}");
    let error =
        session.execute("grid.sheet.unprotect", &json!({ "sheet": 9999 })).expect_err("it is refused");
    assert!(error.to_string().contains("there is no sheet"), "and there is none: {error}");
}
