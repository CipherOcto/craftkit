//! The **sparkline** of a cell: a little picture of a range.

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

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// The **sparkline** of a cell, as it is read.
fn sparkline_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.sparkline.read", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the sparkline is read");
    outcome.data.clone()
}

/// **A sparkline may be set, and it is read back.**
#[test]
fn a_sparkline_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A10" }))
        .expect("the sparkline is set");
    let read = sparkline_of(&mut session, sheet, "C1");
    assert_eq!(read["sparkline"]["range"], json!("A1:A10"));
    assert_eq!(read["sparkline"]["kind"], json!("line"));
}

/// **The values are derived: they follow the sheet.**
#[test]
fn the_values_follow_the_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    session
        .execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A2" }))
        .expect("the sparkline is set");
    assert_eq!(sparkline_of(&mut session, sheet, "C1")["data"]["values"], json!([1.0, 2.0]));
    a_number(&mut session, sheet, "A2", 5.0);
    assert_eq!(sparkline_of(&mut session, sheet, "C1")["data"]["values"], json!([1.0, 5.0]));
}

/// **A cell that holds no number shows nothing, and not a zero.**
#[test]
fn a_cell_that_holds_no_number_shows_nothing() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_text(&mut session, sheet, "A2", "many");
    session
        .execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A2" }))
        .expect("the sparkline is set");
    assert_eq!(sparkline_of(&mut session, sheet, "C1")["data"]["values"], json!([1.0, null]));
}

/// **A win and a loss are about the sign of a number.**
#[test]
fn a_win_and_a_loss_are_about_the_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    a_number(&mut session, sheet, "A2", -3.0);
    a_number(&mut session, sheet, "A3", 0.0);
    session
        .execute(
            "grid.sparkline.set",
            &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A3", "kind": "win_loss" }),
        )
        .expect("the sparkline is set");
    let read = sparkline_of(&mut session, sheet, "C1");
    assert_eq!(read["data"]["signs"], json!(["win", "loss", null]), "a zero is nothing");
}

/// **A kind that is not a kind is refused.**
#[test]
fn a_kind_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute(
        "grid.sparkline.set",
        &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A10", "kind": "spiral" }),
    );
    assert!(outcome.is_err(), "`spiral` is not a kind of sparkline");
}

/// **A range of one cell is refused.**
#[test]
fn a_range_of_one_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": "C1", "range": "A1" }));
    assert!(outcome.is_err(), "a sparkline reads a range");
}

/// **A colour that is not a colour is refused.**
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute(
        "grid.sparkline.set",
        &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A10", "colour": "red" }),
    );
    assert!(outcome.is_err(), "`red` is not a colour");
}

/// **A cell may hold a value and a sparkline: they are two facts.**
#[test]
fn a_cell_may_hold_a_value_and_a_sparkline() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "C1", 42.0);
    session
        .execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A10" }))
        .expect("the sparkline is set");
    let read = sparkline_of(&mut session, sheet, "C1");
    assert_eq!(read["sparkline"]["range"], json!("A1:A10"));
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C1" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], json!(42.0), "the value is still there");
}

/// **A sparkline may be taken away.**
#[test]
fn a_sparkline_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": "C1", "range": "A1:A10" }))
        .expect("the sparkline is set");
    session
        .execute("grid.sparkline.remove", &json!({ "sheet": sheet, "cell": "C1" }))
        .expect("the sparkline is taken away");
    let outcome = session.execute("grid.sparkline.read", &json!({ "sheet": sheet, "cell": "C1" }));
    assert!(outcome.is_err(), "the cell holds no sparkline");
}

/// **Every sparkline of the sheet may be read at once.**
#[test]
fn every_sparkline_of_the_sheet_may_be_read() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for cell in ["C1", "D1"] {
        session
            .execute("grid.sparkline.set", &json!({ "sheet": sheet, "cell": cell, "range": "A1:A10" }))
            .expect("the sparkline is set");
    }
    let outcome =
        session.execute("grid.sparkline.read", &json!({ "sheet": sheet })).expect("the sparklines are read");
    let sparklines = outcome.data["sparklines"].as_array().expect("the sparklines are a list");
    assert_eq!(sparklines.len(), 2);
}
