//! The **information** of a value.
//!
//! A formula may ask what a value is.

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

/// What a value **is**.
#[test]
fn what_a_value_is_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ISNUMBER(1)"), json!(true), "a number");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ISNUMBER(\"1\")"),
        json!(false),
        "and a text that says one"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=ISTEXT(\"a\")"), json!(true), "text");
    assert_eq!(computed(&mut session, sheet, "A4", "=ISNONTEXT(\"a\")"), json!(false), "and not text");
    assert_eq!(computed(&mut session, sheet, "A5", "=ISLOGICAL(TRUE)"), json!(true), "a boolean");
    assert_eq!(computed(&mut session, sheet, "A6", "=ISLOGICAL(0)"), json!(false), "and not a number");
    assert_eq!(computed(&mut session, sheet, "A7", "=ISBLANK(B9)"), json!(true), "nothing");
    assert_eq!(computed(&mut session, sheet, "A8", "=ISBLANK(\"\")"), json!(false), "and not an empty text");
}

/// What an **error** is.
#[test]
fn what_an_error_is_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ISERROR(1/0)"), json!(true), "an error");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ISERR(1/0)"),
        json!(true),
        "and an error that is not missing"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=ISNA(NA())"), json!(true), "and missing");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ISERR(NA())"),
        json!(false),
        "which is not an error of the other kind"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ISERROR(1)"),
        json!(false),
        "and a number is not an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=ERROR.TYPE(1/0)"),
        json!(2.0),
        "and the number that names it"
    );
    assert_eq!(computed(&mut session, sheet, "A7", "=ERROR.TYPE(NA())"), json!(7.0), "and of another");
}

/// The **kind** of a value.
#[test]
fn the_kind_of_a_value_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=TYPE(1)"), json!(1.0), "a number");
    assert_eq!(computed(&mut session, sheet, "A2", "=TYPE(\"a\")"), json!(2.0), "text");
    assert_eq!(computed(&mut session, sheet, "A3", "=TYPE(TRUE)"), json!(4.0), "a boolean");
    assert_eq!(computed(&mut session, sheet, "A4", "=TYPE(1/0)"), json!(16.0), "and an error");
    assert_eq!(computed(&mut session, sheet, "A5", "=TYPE(B9)"), json!(1.0), "and nothing");
}

/// Whether a number is **even** or **odd**.
#[test]
fn whether_a_number_is_even_or_odd_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ISEVEN(-2.5)"), json!(true), "even");
    assert_eq!(computed(&mut session, sheet, "A2", "=ISODD(3.9)"), json!(true), "odd");
    assert_eq!(computed(&mut session, sheet, "A3", "=ISODD(-3)"), json!(true), "and negative");
    assert_eq!(computed(&mut session, sheet, "A4", "=ISEVEN(B9)"), json!(true), "and nothing is even");
}

/// A value as a **number**.
#[test]
fn a_value_as_a_number_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=N(7)"), json!(7.0), "a number stays");
    assert_eq!(computed(&mut session, sheet, "A2", "=N(TRUE)"), json!(1.0), "and a boolean becomes one");
    assert_eq!(computed(&mut session, sheet, "A3", "=N(\"7\")"), json!(0.0), "and a text becomes nothing");
    assert_eq!(computed(&mut session, sheet, "A4", "=N(B9)"), json!(0.0), "and so does nothing");
}

/// The tree's **errors**: a value that is not a number cannot be even
/// or odd, and `ERROR.TYPE` of a value that is not an error is
/// `#N/A`.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ISEVEN(\"x\")"),
        json!({ "error": "#VALUE!" }),
        "a text is not even or odd"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ISEVEN(TRUE)"),
        json!({ "error": "#VALUE!" }),
        "and a boolean is not"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ERROR.TYPE(1)"),
        json!({ "error": "#N/A" }),
        "and the kind of a number that is not an error is missing"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=NA()"),
        json!({ "error": "#N/A" }),
        "and the error itself is missing"
    );
}
