//! The **scale** of a print: the scale a sheet prints at, and the fit
//! that works a scale out.

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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A column of **numbers**, one in each row from one.
fn a_column(session: &mut Session, sheet: u64, column: &str, rows: u32) {
    for row in 1..=rows {
        a_number(session, sheet, &format!("{column}{row}"), f64::from(row));
    }
}

/// The **letter** of a column, counted from zero.
fn letter(index: u32) -> String {
    let mut name = String::new();
    let mut index = index + 1;
    while index > 0 {
        let rest = index - 1;
        name.insert(0, (b'A' + (rest % 26) as u8) as char);
        index = rest / 26;
    }
    name
}

/// The **setup** of a sheet.
fn setup_of(session: &mut Session, sheet: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the setup is read");
    outcome.data.clone()
}

/// The **pages** of a sheet.
fn pages_of(session: &mut Session, sheet: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.print.pages", &json!({ "sheet": sheet })).expect("the pages are read");
    outcome.data["pages"].as_array().expect("the pages are a list").clone()
}

/// **The scale may be set, and it is read back.**
#[test]
fn the_scale_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": 50 })).expect("the scale is set");
    assert_eq!(setup_of(&mut session, sheet)["scale"], json!(50.0));
}

/// **A scale outside the tree's bounds is refused.**
#[test]
fn a_scale_outside_the_bounds_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for percent in [5, 0, 401, -50] {
        let outcome = session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": percent }));
        assert!(outcome.is_err(), "a scale of {percent} is refused");
    }
    assert_eq!(setup_of(&mut session, sheet)["scale"], json!(100.0));
}

/// **A scale that is not a whole number is refused.**
#[test]
fn a_scale_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": 12.5 }));
    assert!(outcome.is_err(), "a scale of twelve and a half percent is refused");
}

/// **A smaller scale puts more on a page.**
#[test]
fn a_smaller_scale_puts_more_on_a_page() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_column(&mut session, sheet, "A", 60);
    assert_eq!(pages_of(&mut session, sheet).len(), 2, "sixty rows are two pages at full size");
    session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": 50 })).expect("the scale is set");
    assert_eq!(pages_of(&mut session, sheet).len(), 1, "at half size the rows are on one page");
}

/// **The pages say the scale they are at.**
#[test]
fn the_pages_say_the_scale() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_column(&mut session, sheet, "A", 10);
    session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": 25 })).expect("the scale is set");
    let outcome =
        session.execute("grid.print.pages", &json!({ "sheet": sheet })).expect("the pages are read");
    assert_eq!(outcome.data["scale"], json!(25.0));
}

/// **The fit may be set, and the scale that does it is worked out.**
#[test]
fn the_fit_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in 0..20 {
        a_number(&mut session, sheet, &format!("{}1", letter(column)), 1.0);
    }
    session
        .execute("grid.print.fit", &json!({ "sheet": sheet, "wide": 1, "tall": 1 }))
        .expect("the fit is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["fit"], json!({ "wide": 1, "tall": 1 }));
    let scale = setup["fit_scale"].as_f64().expect("the fit has a scale");
    assert!(scale < 100.0, "twenty columns do not fit on one page at full size: {scale}");
    assert_eq!(pages_of(&mut session, sheet).len(), 1, "the fitted sheet is on one page");
}

/// **A fit of no pages at all is refused.**
#[test]
fn a_fit_of_no_pages_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.print.fit", &json!({ "sheet": sheet, "wide": 0, "tall": 0 }));
    assert!(outcome.is_err(), "a fit of no pages is not a fit");
}

/// **A fit that cannot be met is refused.**
#[test]
fn a_fit_that_cannot_be_met_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in 0..100 {
        a_number(&mut session, sheet, &format!("{}1", letter(column)), 1.0);
    }
    let outcome = session.execute("grid.print.fit", &json!({ "sheet": sheet, "wide": 1, "tall": 1 }));
    assert!(outcome.is_err(), "a hundred columns do not fit on one page even at ten percent");
    assert_eq!(setup_of(&mut session, sheet)["fit"], json!(null));
}

/// **A fit and a scale are two ways of saying one thing.**
#[test]
fn a_fit_and_a_scale_are_one_thing() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": 75 })).expect("the scale is set");
    session
        .execute("grid.print.fit", &json!({ "sheet": sheet, "wide": 1, "tall": 0 }))
        .expect("the fit is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["fit"], json!({ "wide": 1, "tall": 0 }));
    assert_eq!(setup["scale"], json!(100.0), "a fit is the scale, and the scale is the fit");
    session.execute("grid.print.scale", &json!({ "sheet": sheet, "percent": 75 })).expect("the scale is set");
    assert_eq!(setup_of(&mut session, sheet)["fit"], json!(null), "a scale takes the fit away");
}

/// **A fitted sheet is broken by the fit and not by hand.**
#[test]
fn a_fitted_sheet_is_broken_by_the_fit() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_column(&mut session, sheet, "A", 60);
    session.execute("grid.print.break", &json!({ "sheet": sheet, "row": 30 })).expect("the break is set");
    assert_eq!(pages_of(&mut session, sheet).len(), 2, "the break is a place where a page begins");
    session
        .execute("grid.print.fit", &json!({ "sheet": sheet, "wide": 1, "tall": 1 }))
        .expect("the fit is set");
    assert_eq!(pages_of(&mut session, sheet).len(), 1, "the fit is the way the pages are chosen");
}
