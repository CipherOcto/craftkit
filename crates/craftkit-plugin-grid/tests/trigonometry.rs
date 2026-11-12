//! The **trigonometry** of a formula.
//!
//! A formula may ask about an angle, and the rules are the tree's own.

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

/// The **sine** and the **cosine** of an angle.
#[test]
fn the_sine_and_the_cosine_of_an_angle_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=SIN(0)"), json!(0.0), "the sine of nothing");
    assert_eq!(computed(&mut session, sheet, "A2", "=COS(0)"), json!(1.0), "and the cosine");
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(SIN(PI()/2),3)"),
        json!(1.0),
        "and a quarter turn"
    );
}

/// The **tangent** and its **inverse**, and the **angle of a point**.
#[test]
fn the_tangent_and_its_inverse_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=TAN(0)"), json!(0.0), "the tangent of nothing");
    assert_eq!(computed(&mut session, sheet, "A2", "=ROUND(ASIN(1),2)"), json!(1.57), "the arc sine of one");
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(ATAN2(1,1),2)"),
        json!(0.79),
        "and the angle of a point"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ATAN2(0,0)"),
        json!({ "error": "#DIV/0!" }),
        "and the point at the origin is no point"
    );
}

/// The **hyperbolic** functions and their **inverses**.
#[test]
fn the_hyperbolic_functions_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=SINH(0)"), json!(0.0), "the hyperbolic sine of nothing");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ACOSH(1)"),
        json!(0.0),
        "and the arc hyperbolic cosine of one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(ATANH(0.5),3)"),
        json!(0.549),
        "and the arc hyperbolic tangent"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ATANH(1)"),
        json!({ "error": "#NUM!" }),
        "and a number of one has no arc hyperbolic tangent"
    );
}

/// The **reciprocals** of the three, and a reciprocal of nothing is
/// the tree's error.
#[test]
fn the_reciprocals_of_the_three_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=SEC(0)"), json!(1.0), "the secant of nothing");
    assert_eq!(computed(&mut session, sheet, "A2", "=ROUND(COT(1),3)"), json!(0.642), "the cotangent of one");
    assert_eq!(computed(&mut session, sheet, "A3", "=ROUND(CSC(PI()/2),3)"), json!(1.0), "and the cosecant");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=COT(0)"),
        json!({ "error": "#DIV/0!" }),
        "and the cotangent of nothing is an error"
    );
}

/// An angle beyond the tree's **bound** is an error.
#[test]
fn an_angle_beyond_the_bound_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=SIN(134217728)"),
        json!({ "error": "#NUM!" }),
        "the angle is beyond the bound"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ASIN(2)"),
        json!({ "error": "#NUM!" }),
        "and the arc sine of a number beyond one is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ACOSH(0.5)"),
        json!({ "error": "#NUM!" }),
        "and the arc hyperbolic cosine of a number below one"
    );
}

/// An angle may be turned from **degrees** into **radians** and back.
#[test]
fn an_angle_may_be_turned_from_degrees_into_radians() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ROUND(DEGREES(PI()),0)"), json!(180.0), "the degrees");
    assert_eq!(computed(&mut session, sheet, "A2", "=ROUND(RADIANS(180),0)"), json!(3.0), "and the radians");
}
