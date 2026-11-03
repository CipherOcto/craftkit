//! The **fraction** of a cell.
//!
//! A number may be written as a fraction: the **nearest** fraction with
//! a denominator no bigger than the limit, and the limit is what the
//! code's placeholders say — nine for one digit of denominator, and
//! ninety-nine for two.

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

/// A cell with a **value** and a **format**.
fn a_cell(session: &mut Session, sheet: u64, cell: &str, value: f64, format: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": format }))
        .expect("the format is set");
}

/// A number is written as the **nearest** fraction.
#[test]
fn a_number_is_the_nearest_fraction() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.5, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1/2", "the half is a half");
}

/// The **whole** part is written beside the fraction.
#[test]
fn the_whole_part_is_beside_the_fraction() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1.5, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1 1/2", "the whole is in front of the fraction");
}

/// A **whole** number is written as a whole number.
#[test]
fn a_whole_number_is_a_whole_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 2.0, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "2", "and there is no fraction after it");
}

/// **Zero** is a whole number too.
#[test]
fn zero_is_a_whole_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.0, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "0", "and it is written");
}

/// Without a **whole part** the numerator carries the whole value.
#[test]
fn without_a_whole_part_the_numerator_carries_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1.5, "?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "3/2", "the numerator is the whole number");
}

/// The limit is what the **placeholders** say: one digit of
/// denominator is nine, and two are ninety-nine.
#[test]
fn the_limit_is_what_the_placeholders_say() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.3, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "2/7", "the best fraction with a denominator of nine");
    a_cell(&mut session, sheet, "A2", 0.3, "# ??/??");
    assert_eq!(text_of(&mut session, sheet, "A2"), "3/10", "and with ninety-nine it is better");
}

/// The **nearest** fraction is not always the obvious one.
#[test]
fn the_nearest_fraction_is_the_best_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.99, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1", "and it is carried into the whole part");
}

/// A code may name the **denominator**: the fraction is written with
/// it, and it is not reduced.
#[test]
fn a_named_denominator_is_used_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.5, "?/8");
    assert_eq!(text_of(&mut session, sheet, "A1"), "4/8", "the eighths are not reduced");
}

/// A named denominator with a **whole part**.
#[test]
fn a_named_denominator_may_have_a_whole_part() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 2.125, "# ?/8");
    assert_eq!(text_of(&mut session, sheet, "A1"), "2 1/8", "the whole is in front of the eighths");
}

/// A **negative** fraction keeps its sign, in front of the whole.
#[test]
fn a_negative_fraction_keeps_its_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -1.5, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "-1 1/2", "the sign is in front");
}

/// A negative number that **rounds to zero** has no sign: the sign of
/// a number that is not there is not written.
#[test]
fn a_negative_that_rounds_to_zero_has_no_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -0.001, "# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "0", "there is no sign");
}

/// A code that is **not** a fraction is refused.
#[test]
fn a_code_that_is_not_a_fraction_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect("the value is set");
    let refused = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "# ?/? and more" }));
    assert!(refused.is_err(), "that is not a fraction");
}

/// A fraction in a **style** is a fraction of every cell that names
/// the style.
#[test]
fn a_fraction_in_a_style_is_a_fraction() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": 1, "name": "halves", "format": "# ?/?" }))
        .expect("the style is added");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0.5 }))
        .expect("the value is set");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "halves" }))
        .expect("the style is applied");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1/2", "the cell shows the fraction");
}

/// The **TEXT** function writes a fraction too.
#[test]
fn the_text_function_writes_a_fraction() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "A1", "formula": "=TEXT(0.25, \"# ?/?\")" }),
        )
        .expect("the formula is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1/4", "the quarter is a quarter");
}
