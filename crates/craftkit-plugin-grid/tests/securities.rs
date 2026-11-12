//! The **price** of a discount security.
//!
//! A formula may ask about a security that is sold at a discount and
//! redeemed at its value.

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

/// The **discount** of a security that is sold below its value.
#[test]
fn the_discount_of_a_security_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(DISC(DATE(2007,1,25),DATE(2007,6,15),97.975,100,1),9)"),
        json!(0.052420213),
        "the discount"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(YIELDDISC(DATE(2008,2,16),DATE(2008,3,1),99.795,100,2),6)"
        ),
        json!(0.052823),
        "and the yield of one"
    );
}

/// The **price** of a security, from its discount.
#[test]
fn the_price_of_a_security_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(PRICEDISC(DATE(2008,2,16),DATE(2008,3,1),0.0525,100,2),8)"
        ),
        json!(99.79583333),
        "the price"
    );
}

/// The **rate of interest** of a security and the amount **received**
/// at the end.
#[test]
fn the_interest_of_a_security_and_what_is_received_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(INTRATE(DATE(2008,2,15),DATE(2008,5,15),1000000,1014420,2),5)"
        ),
        json!(0.05768),
        "the rate of interest"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(RECEIVED(DATE(2008,2,15),DATE(2008,5,15),1000000,0.0575,2),3)"
        ),
        json!(1014584.654),
        "and what is received"
    );
}

/// The **price** of a bill and its **yield**.
#[test]
fn the_price_and_the_yield_of_a_bill_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(TBILLPRICE(DATE(2008,3,31),DATE(2008,6,1),0.09),10)"),
        json!(98.45),
        "the price"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(TBILLYIELD(DATE(2008,3,31),DATE(2008,6,1),98.45),6)"),
        json!(0.091417),
        "and the yield"
    );
}

/// The **equivalent yield** of a bill.
#[test]
fn the_equivalent_yield_of_a_bill_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(TBILLEQ(DATE(2008,3,31),DATE(2008,6,1),0.0914),6)"),
        json!(0.094151),
        "the equivalent yield"
    );
}

/// The tree's **errors**: a settlement that is not before the
/// maturity, a bill of more than a year and a basis that is not one.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=DISC(DATE(2008,2,15),DATE(2008,1,15),97,100)"),
        json!({ "error": "#NUM!" }),
        "a settlement after the maturity is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=TBILLPRICE(DATE(2008,3,31),DATE(2009,6,1),0.09)"),
        json!({ "error": "#NUM!" }),
        "and a bill of more than a year is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=DISC(DATE(2008,2,15),DATE(2008,5,15),97,100,7)"),
        json!({ "error": "#NUM!" }),
        "and a basis that is not one is an error"
    );
}
