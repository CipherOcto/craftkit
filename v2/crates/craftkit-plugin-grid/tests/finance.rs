//! The **value** of money over time.
//!
//! A formula may ask what a sum of money is worth now and later.

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

/// The **payment** of a loan, and what it is worth now and later.
#[test]
fn the_payment_of_a_loan_and_its_worth_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(PMT(0.05,10,1000),2)"),
        json!(-129.5),
        "the payment"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(FV(0.05,10,-100),2)"),
        json!(1257.79),
        "and what it is worth later"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(PV(0.05,10,-100),2)"),
        json!(772.17),
        "and what it is worth now"
    );
}

/// How many payments a loan takes, and the **rate** that makes a sum
/// work out.
#[test]
fn the_number_of_payments_and_the_rate_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(NPER(0.05,-100,1000),2)"),
        json!(14.21),
        "the payments"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(RATE(10,-129.5046,1000),3)"),
        json!(0.05),
        "and the rate"
    );
}

/// The **interest** and the **principal** of one payment.
#[test]
fn the_interest_and_the_principal_of_a_payment_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(IPMT(0.05,1,10,1000),2)"),
        json!(-50.0),
        "the interest"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(PPMT(0.05,1,10,1000),2)"),
        json!(-79.5),
        "and the principal"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(ISPMT(0.05,1,10,1000),2)"),
        json!(-45.0),
        "and the interest of a loan with even principal"
    );
}

/// A rate that is **compounded** and the rate it is made of.
#[test]
fn a_compounded_rate_and_the_rate_it_is_made_of_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(EFFECT(0.05,12),4)"),
        json!(0.0512),
        "the effective rate"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(NOMINAL(0.0511619,12),4)"),
        json!(0.05),
        "and the nominal one"
    );
}

/// How long a sum takes to grow, and the rate it grows at.
#[test]
fn how_long_a_sum_takes_to_grow_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(PDURATION(0.05,1000,2000),2)"),
        json!(14.21),
        "the periods"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(RRI(10,1000,2000),4)"),
        json!(0.0718),
        "and the rate"
    );
}

/// The tree's **errors**: a loan of no periods, a payment of nothing
/// and a rate of nothing.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=PMT(0.05,0,1000)"),
        json!({ "error": "#NUM!" }),
        "a loan of no periods is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ISPMT(0.05,1,0,1000)"),
        json!({ "error": "#DIV/0!" }),
        "and a loan of no periods of interest is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=NPER(0.05,0,1000)"),
        json!({ "error": "#NUM!" }),
        "and a payment of nothing is an error"
    );
}
