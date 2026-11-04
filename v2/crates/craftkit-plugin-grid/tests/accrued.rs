//! The **accrued interest** of a bond.
//!
//! A formula may ask what interest a bond has earned between two
//! dates.

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

/// The interest of a bond over a **year fraction**.
#[test]
fn the_interest_over_a_year_fraction_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(ACCRINTM(DATE(2008,4,1),DATE(2008,6,15),0.1,1000,3),8)"),
        json!(20.54794521),
        "the interest"
    );
}

/// The interest of a bond over the **coupon periods** it has been
/// through.
#[test]
fn the_interest_over_the_coupon_periods_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A1",
            "=ROUND(ACCRINT(DATE(2008,3,1),DATE(2008,8,31),DATE(2008,5,1),0.1,1000,2,0),8)"
        ),
        json!(16.66666667),
        "the interest"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ROUND(ACCRINT(DATE(2008,3,5),DATE(2008,8,31),DATE(2008,5,1),0.1,1000,2,0,FALSE),8)"
        ),
        json!(15.55555556),
        "and the interest of the other method"
    );
}

/// The tree's **errors**: a settlement before the issue and a rate of
/// nothing.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ACCRINTM(DATE(2008,6,15),DATE(2008,4,1),0.1)"),
        json!({ "error": "#NUM!" }),
        "a settlement before the issue is an error"
    );
    assert_eq!(
        computed(
            &mut session,
            sheet,
            "A2",
            "=ACCRINT(DATE(2008,3,1),DATE(2008,8,31),DATE(2008,5,1),0,1000,2,0)"
        ),
        json!({ "error": "#NUM!" }),
        "and a rate of nothing is an error"
    );
}
