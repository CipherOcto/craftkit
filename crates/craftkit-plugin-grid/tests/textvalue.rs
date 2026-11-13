//! The **value of a text**: a text that holds a number, a date or a
//! time.

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
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **number** a text holds.
#[test]
fn the_number_of_a_text_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=VALUE(\"1,234.5\")"),
        json!(1234.5),
        "a number with a separator"
    );
    assert_eq!(computed(&mut session, sheet, "A2", "=VALUE(\"50%\")"), json!(0.5), "a percent");
    assert_eq!(computed(&mut session, sheet, "A3", "=VALUE(\"$10\")"), json!(10.0), "a currency");
    assert_eq!(computed(&mut session, sheet, "A4", "=VALUE(\"12:00\")"), json!(0.5), "a time");
    assert_eq!(computed(&mut session, sheet, "A5", "=VALUE(\"2020-01-01\")"), json!(43831.0), "a date");
    assert_eq!(
        computed(&mut session, sheet, "A6", "=VALUE(\"abc\")"),
        json!({ "error": "#VALUE!" }),
        "and a text that holds no number"
    );
    assert_eq!(
        computed(&mut session, sheet, "A7", "=VALUE(\"$(5)\")"),
        json!(-5.0),
        "and the accounting form of a negative"
    );
    assert_eq!(
        computed(&mut session, sheet, "A8", "=VALUE(\"(5)%\")"),
        json!(-0.05),
        "and the accounting form of a percent"
    );
    assert_eq!(
        computed(&mut session, sheet, "A9", "=VALUE(\"\")"),
        json!({ "error": "#VALUE!" }),
        "and an empty text is not a number"
    );
}

/// The **date** and the **time** a text holds.
#[test]
fn the_date_and_the_time_of_a_text_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=DATEVALUE(\"2020-01-01\")"), json!(43831.0), "the date");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=DATEVALUE(\"1900-02-29\")"),
        json!(60.0),
        "and the day that never happened"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=TIMEVALUE(\"12:00\")"), json!(0.5), "and the time");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=DATEVALUE(\"12:00\")"),
        json!({ "error": "#VALUE!" }),
        "and a time is not a date"
    );
}

/// A text read as a number with the **separators** of the text's own.
#[test]
fn a_text_read_with_the_separators_of_its_own() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=NUMBERVALUE(\"2.500,27\",\",\",\".\")"),
        json!(2500.27),
        "the separators of the text"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=NUMBERVALUE(\"3.5%\")"),
        json!(0.035),
        "the percent sign makes it a hundredth"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=NUMBERVALUE(\" 1 000 \")"),
        json!(1000.0),
        "the spaces are not there"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=NUMBERVALUE(\"\")"),
        json!(0.0),
        "and an empty text is zero"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=NUMBERVALUE(\"1.2.3\")"),
        json!({ "error": "#VALUE!" }),
        "and a text with two points is not a number"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=NUMBERVALUE(\"1.5\",\",\",\" \")"),
        json!({ "error": "#VALUE!" }),
        "and the point is not a separator of that text"
    );
}
