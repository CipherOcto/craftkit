//! The **printing** of a sheet: the paper, the margins and the pages.

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

/// **The default setup is the tree's: Letter, portrait, and the normal
/// margins.**
#[test]
fn the_default_setup_is_the_trees() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["paper"], "Letter");
    assert_eq!(setup["orientation"], "portrait");
    assert_eq!(setup["margins"]["left"], json!(0.7 * 72.0));
    assert_eq!(setup["margins"]["top"], json!(0.75 * 72.0));
    assert_eq!(setup["page"]["width"], json!(612.0));
    assert_eq!(setup["page"]["height"], json!(792.0));
}

/// **The paper may be set, and it is one of the tree's papers.**
#[test]
fn the_paper_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.print.paper", &json!({ "sheet": sheet, "paper": "A4" })).expect("the paper is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["paper"], "A4");
    assert_eq!(setup["page"]["width"], json!(595.28));
    assert_eq!(setup["page"]["height"], json!(841.89));
}

/// **A paper that is not one of the tree's papers is refused.**
#[test]
fn a_paper_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.paper", &json!({ "sheet": sheet, "paper": "A0" }))
        .expect_err("the paper is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a paper this tree has"), "the refusal says why: {message}");
}

/// **The orientation turns the page.**
#[test]
fn the_orientation_turns_the_page() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.print.orientation", &json!({ "sheet": sheet, "orientation": "landscape" }))
        .expect("the orientation is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["orientation"], "landscape");
    assert_eq!(setup["page"]["width"], json!(792.0));
    assert_eq!(setup["page"]["height"], json!(612.0));
}

/// **An orientation that is not one is refused.**
#[test]
fn an_orientation_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.orientation", &json!({ "sheet": sheet, "orientation": "sideways" }))
        .expect_err("the orientation is refused");
    let message = failure.to_string();
    assert!(message.contains("is `portrait` or `landscape`"), "the refusal says why: {message}");
}

/// **The margins may be set by a preset and by inches.**
#[test]
fn the_margins_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.print.margins", &json!({ "sheet": sheet, "preset": "wide" }))
        .expect("the margins are set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["margins"]["left"], json!(72.0));
    session
        .execute("grid.print.margins", &json!({ "sheet": sheet, "left": 0.5 }))
        .expect("the left margin is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["margins"]["left"], json!(36.0));
    assert_eq!(setup["margins"]["right"], json!(72.0), "the other margins are as they were");
}

/// **A margin that is not a margin is refused.**
#[test]
fn a_margin_that_is_not_a_margin_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.margins", &json!({ "sheet": sheet, "left": 30.0 }))
        .expect_err("the margin is refused");
    let message = failure.to_string();
    assert!(message.contains("a margin is a number between zero"), "the refusal says why: {message}");
}

/// **The pages are derived: a sheet that is wider than the paper is
/// two pages.**
#[test]
fn the_pages_are_derived() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        a_number(&mut session, sheet, &format!("{column}1"), 1.0);
    }
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the sheet is two pages wide: {pages:?}");
    assert_eq!(pages[0]["columns"], json!(["A", "G"]));
    assert_eq!(pages[1]["columns"], json!(["H", "H"]));
    assert_eq!(pages[0]["rows"], json!([1, 1]));
}

/// **A tall sheet is two pages down.**
#[test]
fn a_tall_sheet_is_two_pages_down() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=50 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the sheet is two pages down: {pages:?}");
    assert_eq!(pages[0]["rows"], json!([1, 45]));
    assert_eq!(pages[1]["rows"], json!([46, 50]));
}

/// **A sheet with nothing in it has no pages.**
#[test]
fn a_sheet_with_nothing_in_it_has_no_pages() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let pages = pages_of(&mut session, sheet);
    assert!(pages.is_empty());
}

/// **The size of a row and of a column is in the pages.**
#[test]
fn the_size_of_a_row_and_a_column_is_in_the_pages() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 1.0);
    // A column of a hundred characters is wider than the paper.
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "A", "width": 100.0 }))
        .expect("the column is wide");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the wide column and the tall row are two pages: {pages:?}");
}
