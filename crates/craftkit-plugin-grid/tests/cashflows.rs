//! The **cash flows** of a plan.
//!
//! A formula may ask about a series of payments.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell and read what it computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    value_of(session, sheet, cell)
}

/// Put **numbers** in a column of cells.
fn column(session: &mut Session, sheet: u64, letter: &str, numbers: &[f64]) {
    for (index, number) in numbers.iter().enumerate() {
        let cell = format!("{letter}{}", index + 1);
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": number }))
            .expect("the cell is set");
    }
}

/// The **rate** at which a series of payments is worth nothing.
#[test]
fn the_internal_rate_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[-70000.0, 12000.0, 15000.0, 18000.0, 21000.0]);
    assert_eq!(computed(&mut session, sheet, "D1", "=ROUND(IRR(A1:A5),8)"), json!(-0.02124485), "the rate");
    column(&mut session, sheet, "B", &[-70000.0, 12000.0, 15000.0, 18000.0, 21000.0, 26000.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(IRR(B1:B6),8)"),
        json!(0.08663095),
        "and of a longer series"
    );
    column(&mut session, sheet, "C", &[-70000.0, 12000.0, 15000.0]);
    assert_eq!(
        computed(&mut session, sheet, "D3", "=ROUND(IRR(C1:C3,-0.1),8)"),
        json!(-0.44350694),
        "and with a guess of its own"
    );
}

/// The same rate with a **date** on every payment.
#[test]
fn the_dated_rate_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[-10000.0, 2750.0, 4250.0, 3250.0, 2750.0]);
    column(&mut session, sheet, "B", &[39448.0, 39508.0, 39751.0, 39859.0, 39904.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(XNPV(0.09,A1:A5,B1:B5),6)"),
        json!(2086.647602),
        "what the series is worth today"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(XIRR(A1:A5,B1:B5),8)"),
        json!(0.37336253),
        "and the rate of it"
    );
}

/// The rate with the payments **put out and taken in** at two rates.
#[test]
fn the_modified_rate_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[-120000.0, 39000.0, 30000.0, 21000.0, 37000.0, 46000.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(MIRR(A1:A6,0.1,0.12),7)"),
        json!(0.1260941),
        "the rate"
    );
    column(&mut session, sheet, "B", &[-120000.0, 39000.0, 30000.0, 21000.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(MIRR(B1:B4,0.1,0.12),8)"),
        json!(-0.04804466),
        "and of a shorter series"
    );
}

/// What a sum grows to, and the **interest** and the **principal** of a
/// run of payments.
#[test]
fn the_schedule_and_the_run_of_payments_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[0.09, 0.11, 0.1]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(FVSCHEDULE(1,A1:A3),5)"),
        json!(1.33089),
        "what a sum grows to"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(CUMIPMT(0.09/12,360,125000,13,24,0),4)"),
        json!(-11135.2321),
        "the interest of the run"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=ROUND(CUMPRINC(0.09/12,360,125000,13,24,0),4)"),
        json!(-934.1071),
        "and its principal"
    );
    assert_eq!(
        computed(&mut session, sheet, "D4", "=CUMIPMT(0.09/12,360,125000,1,1,0)"),
        json!(-937.5),
        "and of one payment"
    );
}

/// A price written as a **fraction** of a dollar, and back.
#[test]
fn the_fraction_of_a_dollar_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=DOLLARDE(1.02,16)"), json!(1.125), "the price");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(DOLLARDE(1.1,32),10)"),
        json!(1.3125),
        "and another"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=ROUND(DOLLARFR(1.125,16),10)"), json!(1.02), "and back");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(DOLLARFR(1.125,32),10)"),
        json!(1.04),
        "and back again"
    );
}

/// The tree's **errors**: a series with one way only, a run of
/// payments with a type that is not one, and a fraction of nothing.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[1.0, 2.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=IRR(A1:A2)"),
        json!({ "error": "#NUM!" }),
        "a series with one way only has no rate"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=CUMIPMT(0.01,360,125000,1,1,2)"),
        json!({ "error": "#NUM!" }),
        "and a type that is not one is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=DOLLARDE(1.02,0)"),
        json!({ "error": "#DIV/0!" }),
        "and a fraction of nothing is a division by nothing"
    );
    assert_eq!(
        computed(&mut session, sheet, "D4", "=XNPV(0.09,A1:A2,A1:A3)"),
        json!({ "error": "#NUM!" }),
        "and two series of different lengths have no worth"
    );
}

/// The **worth today** of a series of payments.
#[test]
fn the_worth_today_of_a_series_of_payments() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // The tree's own example: a rate of a tenth and four payments.
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(NPV(0.1,-10000,3000,4200,6800),6)"),
        json!(1188.443412),
        "the worth today"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=NPV(-1,1)"),
        json!({ "error": "#DIV/0!" }),
        "and a rate of less than none is a division by nothing"
    );
}
