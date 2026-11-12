//! The **header** and the **footer** of a print.

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

/// The **header** of a page, as it is printed.
fn header_of(page: &craftkit_sdk::serde_json::Value) -> craftkit_sdk::serde_json::Value {
    page["header"].clone()
}

/// **The header may be set, and it is read back.**
#[test]
fn the_header_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "the report" }))
        .expect("the header is set");
    assert_eq!(setup_of(&mut session, sheet)["header"]["odd"], json!("the report"));
}

/// **The header stands in the three sections.**
#[test]
fn the_header_stands_in_sections() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "&Lleft&Cmiddle&Rright" }))
        .expect("the header is set");
    let pages = pages_of(&mut session, sheet);
    let header = header_of(&pages[0]);
    assert_eq!(header["left"]["text"], json!("left"));
    assert_eq!(header["centre"]["text"], json!("middle"));
    assert_eq!(header["right"]["text"], json!("right"));
}

/// **The codes of a header are worked out for the page they are on.**
#[test]
fn the_codes_are_worked_out() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=60 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session
        .execute("grid.print.footer", &json!({ "sheet": sheet, "text": "&P of &N - &A" }))
        .expect("the footer is set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "sixty rows are two pages");
    assert_eq!(pages[0]["footer"]["centre"]["text"], json!("1 of 2 - Sheet1"));
    assert_eq!(pages[1]["footer"]["centre"]["text"], json!("2 of 2 - Sheet1"));
}

/// **A section may be bold and a section may be italic.**
#[test]
fn a_section_may_be_bold() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "&Bthe report" }))
        .expect("the header is set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(header_of(&pages[0])["centre"]["bold"], json!(true));
    assert_eq!(header_of(&pages[0])["centre"]["italic"], json!(false));
}

/// **The first page may have a header of its own.**
#[test]
fn the_first_page_may_have_its_own_header() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=60 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "the rest" }))
        .expect("the header is set");
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "page": "first", "text": "the first" }))
        .expect("the first header is set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(header_of(&pages[0])["centre"]["text"], json!("the first"));
    assert_eq!(header_of(&pages[1])["centre"]["text"], json!("the rest"));
}

/// **An even page may have a header of its own.**
#[test]
fn an_even_page_may_have_its_own_header() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=60 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "the odd" }))
        .expect("the header is set");
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "page": "even", "text": "the even" }))
        .expect("the even header is set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(header_of(&pages[0])["centre"]["text"], json!("the odd"));
    assert_eq!(header_of(&pages[1])["centre"]["text"], json!("the even"));
}

/// **A header may be taken away.**
#[test]
fn a_header_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "the report" }))
        .expect("the header is set");
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "" }))
        .expect("the header is taken away");
    assert_eq!(setup_of(&mut session, sheet)["header"]["odd"], json!(null));
}

/// **A code that is about the machine is refused.**
#[test]
fn a_code_about_the_machine_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for text in ["&D", "&T", "&F", "&Z", "&X"] {
        let outcome = session.execute("grid.print.header", &json!({ "sheet": sheet, "text": text }));
        assert!(outcome.is_err(), "the code `{text}` is refused");
    }
    assert_eq!(setup_of(&mut session, sheet)["header"]["odd"], json!(null));
}

/// **A kind of page that is not a kind of page is refused.**
#[test]
fn a_kind_of_page_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.print.header", &json!({ "sheet": sheet, "page": "middle", "text": "x" }));
    assert!(outcome.is_err(), "`middle` is not a kind of page");
}

/// **A header that is too long is refused.**
#[test]
fn a_header_that_is_too_long_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let text = "x".repeat(10_001);
    let outcome = session.execute("grid.print.header", &json!({ "sheet": sheet, "text": text }));
    assert!(outcome.is_err(), "a header of ten thousand and one characters is refused");
}
