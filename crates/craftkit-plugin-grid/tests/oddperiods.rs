//! The **odd coupon periods** of a bond.
//!
//! A formula may ask about a bond whose first or last coupon period
//! is not a whole one.

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

/// The **price** of a bond with an odd **first** period.
#[test]
fn the_price_of_an_odd_first_period_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(ODDFPRICE(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0785,0.0625,100,2,1),2)"
        ),
        json!(113.6),
        "the price"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(ODDFYIELD(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0575,84.5,100,2,0),4)"
        ),
        json!(0.0772),
        "and the yield"
    );
}

/// The **price** of a bond with an odd **last** period.
#[test]
fn the_price_of_an_odd_last_period_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(ODDLPRICE(DATE(2008,2,7),DATE(2008,6,15),DATE(2007,10,15),0.0375,0.0405,100,2,0),2)"
        ),
        json!(99.88),
        "the price"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(ODDLYIELD(DATE(2008,4,20),DATE(2008,6,15),DATE(2007,12,24),0.0375,99.875,100,2,0),5)"
        ),
        json!(0.04519),
        "and the yield"
    );
}

/// The price and the yield **agree**: a price from a yield gives the
/// yield back.
#[test]
fn the_price_and_the_yield_agree() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let price = computed(
        &mut session,
        sheet,
        "A1",
        "=ODDFPRICE(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.06,0.05,100,2,0)",
    );
    let Json::Number(price) = price else {
        panic!("the price is a number");
    };
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            &format!(
                "=ROUND(ODDFYIELD(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.06,{},100,2,0),6)",
                price
            )
        ),
        json!(0.05),
        "the yield comes back"
    );
}

/// The tree's **errors**: a frequency that is not one, two or four, a
/// price that is not positive and dates that are in the wrong order.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ODDFPRICE(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0785,0.0625,100,3,1)"
        ),
        json!({ "error": "#NUM!" }),
        "a frequency that is not one, two or four is an error"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ODDFYIELD(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0575,0,100,2,0)"
        ),
        json!({ "error": "#NUM!" }),
        "and a price of nothing is an error"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A3",
            "=ODDFPRICE(DATE(2009,4,1),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.07,0.06,100,2,0)"
        ),
        json!({ "error": "#NUM!" }),
        "and a settlement before the issue is an error"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A4",
            "=ODDLPRICE(DATE(2007,10,1),DATE(2008,6,15),DATE(2007,10,15),0.0375,0.0405,100,2,0)"
        ),
        json!({ "error": "#NUM!" }),
        "and a settlement before the last interest is an error"
    );
}
