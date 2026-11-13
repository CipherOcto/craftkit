//! The **Bessel** functions, and the **euro**.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **number** a formula computes.
fn number_of(session: &mut Session, sheet: u64, cell: &str) -> f64 {
    let value = value_of(session, sheet, cell);
    value.as_f64().unwrap_or_else(|| panic!("{cell} is a number: {value}"))
}

/// The four Bessel functions are the tree's.
#[test]
fn the_bessel_functions_are_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=BESSELJ(1,0)");
    assert!((number_of(&mut session, sheet, "A1") - 0.765_197_686_557_966_6).abs() < 1e-12, "J_0(1)");
    formula(&mut session, sheet, "A2", "=BESSELJ(1,1)");
    assert!((number_of(&mut session, sheet, "A2") - 0.440_050_585_744_933_5).abs() < 1e-12, "J_1(1)");
    formula(&mut session, sheet, "A3", "=BESSELY(2.5,1)");
    assert!((number_of(&mut session, sheet, "A3") - 0.145_918_138).abs() < 1e-6, "Y_1(2.5)");
    formula(&mut session, sheet, "A4", "=BESSELI(1.5,1)");
    assert!((number_of(&mut session, sheet, "A4") - 0.981_666_428).abs() < 1e-6, "I_1(1.5)");
    formula(&mut session, sheet, "A5", "=BESSELK(1.5,1)");
    assert!((number_of(&mut session, sheet, "A5") - 0.277_387_804).abs() < 1e-6, "K_1(1.5)");
}

/// An order that is **not there** is an error, and a second kind at a
/// number that is not positive is an error.
#[test]
fn an_order_that_is_not_there_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=BESSELJ(1,-1)");
    assert_eq!(value_of(&mut session, sheet, "A1")["error"], Json::String("#NUM!".to_string()), "no order");
    formula(&mut session, sheet, "A2", "=BESSELJ(1,100001)");
    assert_eq!(value_of(&mut session, sheet, "A2")["error"], Json::String("#NUM!".to_string()), "too large");
    formula(&mut session, sheet, "A3", "=BESSELY(0,1)");
    assert_eq!(value_of(&mut session, sheet, "A3")["error"], Json::String("#NUM!".to_string()), "no value");
    formula(&mut session, sheet, "A4", "=BESSELK(-1,1)");
    assert_eq!(value_of(&mut session, sheet, "A4")["error"], Json::String("#NUM!".to_string()), "no value");
}

/// The euro is converted with the tree's own rates.
#[test]
fn the_euro_is_converted_with_the_trees_own_rates() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=EUROCONVERT(1.2,\"DEM\",\"EUR\")");
    assert!((number_of(&mut session, sheet, "A1") - 0.61).abs() < 1e-9, "a mark is euros");
    formula(&mut session, sheet, "A2", "=EUROCONVERT(10,\"eur\",\"ITL\")");
    assert!((number_of(&mut session, sheet, "A2") - 19_363.0).abs() < 1e-9, "euros are liras");
    formula(&mut session, sheet, "A3", "=EUROCONVERT(1.23456,\"DEM\",\"DEM\")");
    assert!((number_of(&mut session, sheet, "A3") - 1.23456).abs() < 1e-9, "the same currency is itself");
}

/// A currency that is **not known** is an error, and a triangulation
/// below three digits is an error.
#[test]
fn a_currency_that_is_not_known_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=EUROCONVERT(1,\"USD\",\"EUR\")");
    assert_eq!(
        value_of(&mut session, sheet, "A1")["error"],
        Json::String("#VALUE!".to_string()),
        "no currency"
    );
    formula(&mut session, sheet, "A2", "=EUROCONVERT(1,\"FRF\",\"DEM\",TRUE,2)");
    assert_eq!(
        value_of(&mut session, sheet, "A2")["error"],
        Json::String("#VALUE!".to_string()),
        "too few digits"
    );
}
