//! The **coupons** of a bond, and the **price** of one.
//!
//! A formula may ask about a bond — a security that pays its interest
//! on a fixed day of the year and is redeemed at its value.

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

/// The days **around** a coupon period.
#[test]
fn the_days_around_a_coupon_period_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=COUPDAYBS(DATE(2011,1,25),DATE(2011,11,15),2,1)"),
        json!(71.0),
        "the days from the beginning of the period"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),2,1)"),
        json!(181.0),
        "the days of the period"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=COUPDAYSNC(DATE(2011,1,25),DATE(2011,11,15),2,1)"),
        json!(110.0),
        "and the days to the next coupon"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),2,0)"),
        json!(180.0),
        "and the days of the period on the thirty-over-three-hundred-and-sixty"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),4,3)"),
        json!(91.25),
        "and on the three-hundred-and-sixty-five"
    );
}

/// How many coupons a bond pays, and **when**.
#[test]
fn the_coupons_of_a_bond_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),2,1)"),
        json!(2.0),
        "the number of coupons"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=COUPNUM(DATE(2007,1,25),DATE(2008,11,15),2,1)"),
        json!(4.0),
        "and of a longer one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=COUPPCD(DATE(2011,3,15),DATE(2011,11,30),2)"),
        json!(40512.0),
        "and the date of the previous coupon"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=COUPNCD(DATE(2011,3,15),DATE(2011,11,30),2)"),
        json!(40694.0),
        "and of the next one"
    );
}

/// The **price** of a bond.
#[test]
fn the_price_of_a_bond_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(PRICE(DATE(2008,2,15),DATE(2017,11,15),0.0575,0.065,100,2,0),5)"
        ),
        json!(94.63436),
        "the price"
    );
}

/// The **yield** of a bond, and how long the money is tied up.
#[test]
fn the_yield_and_the_duration_of_a_bond_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(YIELD(DATE(2008,2,15),DATE(2016,11,15),0.0575,95.04287,100,2,0),6)"
        ),
        json!(0.065),
        "the yield"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A3",
            "=ROUND(MDURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,0.09,2,1),5)"
        ),
        json!(5.73567),
        "and the modified duration"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(DURATION(DATE(2018,7,1),DATE(2048,1,1),0.08,0.09,2,1),7)"
        ),
        json!(10.9191453),
        "the duration"
    );
}

/// A bond that is **paid out**.
#[test]
fn a_bond_that_is_paid_out_is_priced() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(PRICEMAT(DATE(2008,2,15),DATE(2008,4,13),DATE(2007,11,11),0.061,0.061,0),8)"
        ),
        json!(99.98449888),
        "the price"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(YIELDMAT(DATE(2008,3,15),DATE(2008,11,3),DATE(2007,11,8),0.0625,100.0123,0),6)"
        ),
        json!(0.060954),
        "and the yield"
    );
}

/// The tree's **errors**: a settlement that is not before the
/// maturity, a frequency that is not one, two or four, and a basis
/// that is not one.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=COUPNUM(DATE(2012,1,25),DATE(2011,11,15),2)"),
        json!({ "error": "#NUM!" }),
        "a maturity before the settlement is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),3)"),
        json!({ "error": "#NUM!" }),
        "and a frequency that is not one, two or four is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),2,5)"),
        json!({ "error": "#NUM!" }),
        "and a basis that is not one is an error"
    );
}
