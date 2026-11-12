//! The **date** of a cell.
//!
//! A date in a spreadsheet is a number with a date format on it, and
//! the tree's calendar has a day in it that never happened.

use craftkit_sdk::serde_json::{Value, json};
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
        .expect("the cell has a text");
    outcome.data["text"].as_str().unwrap_or_default().to_string()
}

/// The **value** of a cell, as the payload holds it.
fn value_of(session: &Session, sheet: u64, cell: &str) -> Option<Value> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].clone())
}

/// Put a **date** in a cell, and give back the serial the tree gave it.
fn set_date(session: &mut Session, sheet: u64, cell: &str, date: &str) -> f64 {
    let outcome = session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": cell, "date": date }))
        .expect("the date is set");
    outcome.data["serial"].as_f64().expect("the date has a serial")
}

/// A date is a **number** with a date format on it.
#[test]
fn a_date_is_a_number_with_a_format_on_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let serial = set_date(&mut session, sheet, "A1", "2026-10-26");
    assert_eq!(serial, 46321.0, "the serial is the tree's");
    assert_eq!(value_of(&session, sheet, "A1"), Some(json!(46321.0)), "the cell holds the number");
    assert_eq!(text_of(&mut session, sheet, "A1"), "2026-10-26", "and the text is the date");
}

/// The serial 1 is the 1st of January 1900.
#[test]
fn the_first_of_january_1900_is_the_serial_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(set_date(&mut session, sheet, "A1", "1900-01-01"), 1.0, "the serial is one");
}

/// The tree's calendar has a **day that never happened**: the serial 60
/// is the 29th of February 1900.
#[test]
fn the_tree_has_a_day_that_never_happened() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(set_date(&mut session, sheet, "A1", "1900-02-29"), 60.0, "the serial is sixty");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1900-02-29", "and the text says the day");
    assert_eq!(set_date(&mut session, sheet, "A2", "1900-03-01"), 61.0, "the next day is the next serial");
    assert_eq!(text_of(&mut session, sheet, "A2"), "1900-03-01", "and it is the first of March");
}

/// A date that did not happen is refused.
#[test]
fn a_date_that_did_not_happen_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-02-30" }))
        .expect_err("the date is refused");
    assert!(error.to_string().contains("is not a date"), "the refusal says why: {error}");
}

/// A date that is not **written** as a date is refused: this tree reads
/// one shape of date and not another.
#[test]
fn a_date_that_is_not_written_as_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "26/10/2026" }))
        .expect_err("the date is refused");
    assert!(error.to_string().contains("is not a date"), "the refusal says why: {error}");
}

/// A **format** that is not a date format is refused, and the refusal
/// names the ones there are.
#[test]
fn a_format_that_is_not_a_date_format_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.cell.date",
            &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26", "format": "0.00" }),
        )
        .expect_err("the format is refused");
    assert!(error.to_string().contains("is not a date format"), "the refusal says why: {error}");
}

/// A date may be written in any of the tree's shapes.
#[test]
fn a_date_may_be_written_in_any_of_its_shapes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let shapes = [
        ("yyyy-mm-dd", "2026-10-26"),
        ("dd/mm/yyyy", "26/10/2026"),
        ("m/d/yyyy", "10/26/2026"),
        ("d-mmm-yyyy", "26-Oct-2026"),
        ("mmm", "Oct"),
        ("mmmm", "October"),
        ("dddd", "Monday"),
        ("ddd", "Mon"),
        ("yyyy", "2026"),
    ];
    for (index, (format, expected)) in shapes.iter().enumerate() {
        let cell = format!("A{}", index + 1);
        session
            .execute(
                "grid.cell.date",
                &json!({ "sheet": sheet, "cell": cell, "date": "2026-10-26", "format": format }),
            )
            .expect("the date is set");
        assert_eq!(text_of(&mut session, sheet, &cell), *expected, "the shape {format} writes the date");
    }
}

/// A date may be written the **tree's** way, and the format it is
/// written with is the shape it was written in.
#[test]
fn a_date_may_be_written_the_trees_way() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "10/26/2026" }))
        .expect("the date is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "10/26/2026", "the shape is the format");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A2", "date": "4-Mar-2026" }))
        .expect("the date is set");
    assert_eq!(text_of(&mut session, sheet, "A2"), "4-Mar-2026", "and so is the month by name");
}

/// The tree reads the **month first**, and the day before the month is
/// not a date to it.
#[test]
fn the_tree_reads_the_month_first() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "26/10/2026" }))
        .expect_err("the date is refused");
    assert!(error.to_string().contains("is not a date"), "the refusal says why: {error}");
}

/// A year of **two digits** is the tree's: below thirty it is this
/// century, and above thirty it is the last.
#[test]
fn a_year_of_two_digits_is_the_trees() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "10/26/26" }))
        .expect("the date is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "10/26/2026", "the year is this century");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A2", "date": "10/26/98" }))
        .expect("the date is set");
    assert_eq!(text_of(&mut session, sheet, "A2"), "10/26/1998", "and the other one is the last");
}

/// A **number** may be given a date format, and then it is a date.
#[test]
fn a_number_with_a_date_format_is_a_date() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 46321 })).expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "yyyy-mm-dd" }))
        .expect("the cell is formatted");
    assert_eq!(text_of(&mut session, sheet, "A1"), "2026-10-26", "the number is a date now");
}

/// A **format** that this tree does not have is refused, and the
/// refusal names the ones there are — the numbers and the dates.
#[test]
fn a_format_this_tree_does_not_have_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "dd/mm/yy" }))
        .expect_err("the format is refused");
    let message = error.to_string();
    assert!(message.contains("is not a format this tree has"), "the refusal says why: {message}");
    assert!(message.contains("yyyy-mm-dd"), "and it names the dates: {message}");
}

/// A number that is not a date is written as a **number**, even when
/// the format is a date.
#[test]
fn a_number_that_is_not_a_date_is_a_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": -5 })).expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "yyyy-mm-dd" }))
        .expect("the cell is formatted");
    assert_eq!(text_of(&mut session, sheet, "A1"), "-5", "the number is a number");
}
