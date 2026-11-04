//! The **measure** of a thing.
//!
//! A formula may ask what a measurement is in another unit.

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

/// A **mass** and a **length**.
#[test]
fn a_mass_and_a_length_are_converted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=CONVERT(1,\"lbm\",\"kg\")"),
        json!(0.45359237),
        "the pound"
    );
    assert_eq!(computed(&mut session, sheet, "A2", "=CONVERT(1,\"in\",\"cm\")"), json!(2.54), "the inch");
    assert_eq!(computed(&mut session, sheet, "A3", "=CONVERT(1,\"mi\",\"km\")"), json!(1.609344), "the mile");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(CONVERT(6,\"ft\",\"yd\"),10)"),
        json!(2.0),
        "and a few feet"
    );
}

/// A **temperature**, which is turned through the **kelvin**.
#[test]
fn a_temperature_is_converted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=CONVERT(68,\"F\",\"C\")"),
        json!(20.0),
        "the Fahrenheit"
    );
    assert_eq!(computed(&mut session, sheet, "A2", "=CONVERT(100,\"C\",\"F\")"), json!(212.0), "the Celsius");
    assert_eq!(
        computed(&mut session, sheet, "A3", "=CONVERT(0,\"C\",\"K\")"),
        json!(273.15),
        "and the kelvin"
    );
}

/// A **volume**, an **area** and a **speed**.
#[test]
fn a_volume_an_area_and_a_speed_are_converted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=CONVERT(1,\"gal\",\"l\")"),
        json!(3.785411784),
        "the gallon"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(CONVERT(1,\"m2\",\"ft2\"),6)"),
        json!(10.76391),
        "the square metre"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(CONVERT(100,\"km/h\",\"m/s\"),6)"),
        json!(27.777778),
        "and the speed"
    );
}

/// The **prefixes**: the SI ones and the **binary** ones.
#[test]
fn the_prefixes_are_used() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=CONVERT(1,\"kibyte\",\"byte\")"),
        json!(1024.0),
        "the binary prefix"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=CONVERT(1,\"byte\",\"bit\")"),
        json!(8.0),
        "and no prefix"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(CONVERT(1,\"km\",\"m\"),6)"),
        json!(1000.0),
        "and the SI one"
    );
}

/// The tree's **errors**: a unit that is not one, and a unit of one
/// kind turned into a unit of another kind.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=CONVERT(1,\"xyz\",\"m\")"),
        json!({ "error": "#N/A" }),
        "a unit that is not one is missing"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=CONVERT(1,\"kg\",\"m\")"),
        json!({ "error": "#N/A" }),
        "and a mass is not a length"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=CONVERT(1,\"LBM\",\"kg\")"),
        json!({ "error": "#N/A" }),
        "and the case of a unit matters"
    );
}
