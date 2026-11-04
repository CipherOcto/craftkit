//! The **names** of a format.
//!
//! The tree's ribbon names its formats; here a format may be given by
//! its code or by its name, and the names this tree does not write are
//! refused with the reason.

use craftkit_sdk::serde_json::json;
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

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the text is read");
    outcome.data["text"].as_str().expect("the text is a string").to_string()
}

/// A number in a cell.
fn set(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// A name is a way of saying a **code**, and the code is kept.
#[test]
fn a_name_says_the_code_it_names() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 1234.5);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "currency" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$1,234.50", "the cell shows the currency");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["format"], json!("\"$\"#,##0.00"), "and the code the name said is what is kept");
}

/// The name of a **number** says two decimals.
#[test]
fn the_number_name_writes_two_decimals() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 1.2345);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "number" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.23", "the number has two decimals");
}

/// The name of a **percentage** multiplies by a hundred.
#[test]
fn the_percentage_name_multiplies_by_a_hundred() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 0.25);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "percentage" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "25.00%", "the number is a percentage");
}

/// The name of a **fraction** writes a fraction.
#[test]
fn the_fraction_name_writes_a_fraction() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 0.5);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "fraction" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1/2", "the number is a fraction");
}

/// The name of a **scientific** format writes a mantissa and an
/// exponent.
#[test]
fn the_scientific_name_writes_a_mantissa_and_an_exponent() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 12345.0);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "scientific" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.23E+04", "the number is scientific");
}

/// The **scientific** code is a format of this tree, and the exponent
/// moves one place at a time.
#[test]
fn the_scientific_code_is_a_format_of_this_tree() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 0.5);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.0E+00" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.0E-01", "the exponent is negative and padded");
}

/// The name of a **short date** says a date shape.
#[test]
fn the_short_date_name_says_a_date_shape() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.date",
            &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26", "format": "short date" }),
        )
        .expect("the date is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "10/26/2026", "the date is written the short way");
}

/// The name of a **time** says a clock shape.
#[test]
fn the_time_name_says_a_clock_shape() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.time",
            &json!({ "sheet": sheet, "cell": "A1", "time": "13:45:30", "format": "time" }),
        )
        .expect("the time is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1:45:30 PM", "the time is written on a clock of twelve");
}

/// A name may be given to a **style**, and the cell that names the
/// style is written with it.
#[test]
fn a_name_may_be_given_to_a_style() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": 1, "name": "money", "format": "currency" }))
        .expect("the style is added");
    set(&mut session, sheet, "A1", 5.0);
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "money" }))
        .expect("the style is named");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$5.00", "the cell is written with the style");
}

/// A name may be given to a **conditional format**.
#[test]
fn a_name_may_be_given_to_a_conditional_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 0.25);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "0", "format": "percentage" }),
        )
        .expect("the conditional format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "25.00%", "the matching cell is written with it");
}

/// A name this tree knows but does not write is **refused with the
/// reason**.
#[test]
fn an_accounting_name_is_refused_with_the_reason() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "accounting" }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("accounting"), "the refusal names the format: {message}");
    assert!(message.contains("fill"), "and says why: {message}");
}

/// The name of a **text** format is refused: this tree does not keep
/// the text a number was written with.
#[test]
fn a_text_name_is_refused_with_the_reason() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "text" }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("does not keep the text"), "the refusal says why: {message}");
}

/// A name this tree does not know is **refused**, and the refusal says
/// what a format is.
#[test]
fn a_name_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "purple" }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("is not a format this tree has"), "the refusal says why: {message}");
    assert!(message.contains("scientific"), "and it names the formats there are: {message}");
}

/// A **code** is still a code: the names do not take its place.
#[test]
fn a_code_is_still_a_code() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 1.2345);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.000" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.235", "the code wrote three decimals");
}
