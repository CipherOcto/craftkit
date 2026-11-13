//! The **host's** names: the functions the tree's program carries and
//! its registry does not.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **number** a formula computes.
fn number_of(session: &mut Session, sheet: u64, cell: &str) -> f64 {
    let value = value_of(session, sheet, cell);
    value.as_f64().unwrap_or_else(|| panic!("{cell} is a number: {value}"))
}

/// The **error** a formula computes.
fn error_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let value = value_of(session, sheet, cell);
    value["error"].as_str().unwrap_or_default().to_string()
}

/// The **place** a formula is in, and the row and the column of it.
#[test]
fn a_formula_knows_where_it_is() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "C3", "=ROW()");
    assert_eq!(number_of(&mut session, sheet, "C3"), 3.0, "the row of the cell");
    formula(&mut session, sheet, "D3", "=COLUMN()");
    assert_eq!(number_of(&mut session, sheet, "D3"), 4.0, "the column of the cell");
}

/// A **reference** names a place too.
#[test]
fn a_reference_names_a_place() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "D1", "=ROW(B7)");
    assert_eq!(number_of(&mut session, sheet, "D1"), 7.0, "the row of the reference");
    formula(&mut session, sheet, "D2", "=COLUMN(B7)");
    assert_eq!(number_of(&mut session, sheet, "D2"), 2.0, "the column of the reference");
}

/// A reference that covers more than one row is a **table**.
#[test]
fn a_reference_that_covers_more_than_one_row_is_a_table() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "D1", "=ROW(A1:A5)");
    assert_eq!(error_of(&mut session, sheet, "D1"), "not supported yet: an array", "a table of rows");
    formula(&mut session, sheet, "D2", "=COLUMN(A1:C1)");
    assert_eq!(error_of(&mut session, sheet, "D2"), "not supported yet: an array", "a table of columns");
}

/// The **areas** of a reference, and whether a thing is a reference.
#[test]
fn the_areas_of_a_reference_and_whether_a_thing_is_one() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "D1", "=AREAS(A1:B2)");
    assert_eq!(number_of(&mut session, sheet, "D1"), 1.0, "one area");
    formula(&mut session, sheet, "D2", "=ISREF(A1:B2)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(true), "a reference is a reference");
    formula(&mut session, sheet, "D3", "=ISREF(1+1)");
    assert_eq!(value_of(&mut session, sheet, "D3"), json!(false), "a number is not a reference");
    formula(&mut session, sheet, "D4", "=AREAS(1+1)");
    assert_eq!(error_of(&mut session, sheet, "D4"), "#VALUE!", "a number has no areas");
}

/// The **formula** of a cell, and whether there is one.
#[test]
fn the_formula_of_a_cell_and_whether_there_is_one() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=1+1");
    value(&mut session, sheet, "B1", json!(7));
    formula(&mut session, sheet, "D1", "=ISFORMULA(A1)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(true), "a cell with a formula");
    formula(&mut session, sheet, "D2", "=ISFORMULA(B1)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(false), "a cell with a value");
    formula(&mut session, sheet, "D3", "=FORMULATEXT(A1)");
    assert_eq!(value_of(&mut session, sheet, "D3"), json!("=1+1"), "the formula of a cell");
    formula(&mut session, sheet, "D4", "=FORMULATEXT(B1)");
    assert_eq!(error_of(&mut session, sheet, "D4"), "#N/A", "a cell with no formula");
    formula(&mut session, sheet, "D5", "=ISFORMULA(1+1)");
    assert_eq!(error_of(&mut session, sheet, "D5"), "#N/A", "the tree's rule: a reference is asked for");
}

/// The **sheets** of a book, and the number of one of them.
#[test]
fn the_sheets_of_a_book_and_the_number_of_one_of_them() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the second sheet is added");
    let second = outcome.data["id"].as_u64().expect("the sheet has an id");
    formula(&mut session, sheet, "D1", "=SHEETS()");
    assert_eq!(number_of(&mut session, sheet, "D1"), 2.0, "two sheets in the book");
    formula(&mut session, sheet, "D2", "=SHEET()");
    assert_eq!(number_of(&mut session, sheet, "D2"), 1.0, "the first sheet");
    formula(&mut session, sheet, "D3", "=SHEET(\"Sheet2\")");
    assert_eq!(number_of(&mut session, sheet, "D3"), 2.0, "the second sheet by name");
    formula(&mut session, sheet, "D4", "=SHEET(\"Sheet3\")");
    assert_eq!(error_of(&mut session, sheet, "D4"), "#N/A", "a sheet that is not there");
    formula(&mut session, second, "D1", "=SHEET()");
    assert_eq!(number_of(&mut session, second, "D1"), 2.0, "the second sheet itself");
}

/// The host's names that give back a **table** are refused as one.
#[test]
fn the_trees_tables_are_refused_as_tables() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, name) in
        ["BYROW", "BYCOL", "MAKEARRAY", "MAP", "SCAN", "GROUPBY", "PIVOTBY"].iter().enumerate()
    {
        let cell = format!("D{}", index + 1);
        formula(&mut session, sheet, &cell, &format!("={name}(A1:B2)"));
        assert_eq!(
            error_of(&mut session, sheet, &cell),
            "not supported yet: an array",
            "{name} gives back a table"
        );
    }
}

/// The host's names that need a **lambda** are refused with that
/// reason.
#[test]
fn the_trees_lambdas_are_refused_as_lambdas() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, name) in ["LAMBDA", "LET", "ISOMITTED", "REDUCE"].iter().enumerate() {
        let cell = format!("D{}", index + 1);
        formula(&mut session, sheet, &cell, &format!("={name}(A1)"));
        assert_eq!(
            error_of(&mut session, sheet, &cell),
            "not supported yet: a lambda",
            "{name} needs a lambda"
        );
    }
}

/// The host's names that give back a **reference** are refused with
/// that reason.
#[test]
fn the_trees_references_are_refused_as_references() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, name) in ["OFFSET", "INDIRECT", "TRIMRANGE"].iter().enumerate() {
        let cell = format!("D{}", index + 1);
        formula(&mut session, sheet, &cell, &format!("={name}(A1)"));
        assert_eq!(
            error_of(&mut session, sheet, &cell),
            "not supported yet: a reference that is worked out",
            "{name} gives back a place"
        );
    }
}

/// The tree's **`CELL`**: what it says about a cell.
#[test]
fn the_trees_cell_says_what_it_says_about_a_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B2", json!("a word"));
    formula(&mut session, sheet, "D1", "=CELL(\"address\", B2)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!("$B$2"), "the address of a cell");
    formula(&mut session, sheet, "D2", "=CELL(\"row\", B2)");
    assert_eq!(number_of(&mut session, sheet, "D2"), 2.0, "the row of a cell");
    formula(&mut session, sheet, "D3", "=CELL(\"col\", B2)");
    assert_eq!(number_of(&mut session, sheet, "D3"), 2.0, "the column of a cell");
    formula(&mut session, sheet, "D4", "=CELL(\"contents\", B2)");
    assert_eq!(value_of(&mut session, sheet, "D4"), json!("a word"), "the contents of a cell");
    formula(&mut session, sheet, "D5", "=CELL(\"type\", B2)");
    assert_eq!(value_of(&mut session, sheet, "D5"), json!("l"), "a cell of text");
    formula(&mut session, sheet, "D6", "=CELL(\"type\", C2)");
    assert_eq!(value_of(&mut session, sheet, "D6"), json!("b"), "an empty cell");
    formula(&mut session, sheet, "D7", "=CELL(\"sheetname\")");
    assert_eq!(value_of(&mut session, sheet, "D7"), json!("Sheet1"), "the name of the sheet");
    formula(&mut session, sheet, "D8", "=CELL(\"protect\", B2)");
    assert_eq!(number_of(&mut session, sheet, "D8"), 1.0, "a cell is locked by its style");
    formula(&mut session, sheet, "D9", "=CELL(\"nonesuch\", B2)");
    assert_eq!(error_of(&mut session, sheet, "D9"), "#VALUE!", "a question the tree does not know");
}

/// The tree's **`AGGREGATE`**: the tree's `SUBTOTAL` with nineteen
/// functions and four options.
#[test]
fn the_trees_aggregate_sums_what_it_is_given() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, number) in [1.0, 2.0, 3.0, 4.0].iter().enumerate() {
        value(&mut session, sheet, &format!("A{}", index + 1), json!(number));
    }
    formula(&mut session, sheet, "D1", "=AGGREGATE(9,0,A1:A4)");
    assert_eq!(number_of(&mut session, sheet, "D1"), 10.0, "the sum");
    formula(&mut session, sheet, "D2", "=AGGREGATE(1,0,A1:A4)");
    assert_eq!(number_of(&mut session, sheet, "D2"), 2.5, "the mean");
    formula(&mut session, sheet, "D3", "=AGGREGATE(12,0,A1:A4)");
    assert_eq!(number_of(&mut session, sheet, "D3"), 2.5, "the median");
    formula(&mut session, sheet, "D4", "=AGGREGATE(14,0,A1:A4,2)");
    assert_eq!(number_of(&mut session, sheet, "D4"), 3.0, "the second largest");
    formula(&mut session, sheet, "D5", "=AGGREGATE(15,0,A1:A4,2)");
    assert_eq!(number_of(&mut session, sheet, "D5"), 2.0, "the second smallest");
    formula(&mut session, sheet, "D6", "=AGGREGATE(16,0,A1:A4,0.5)");
    assert_eq!(number_of(&mut session, sheet, "D6"), 2.5, "the median as a percentile");
    formula(&mut session, sheet, "D7", "=AGGREGATE(9,0,A1:A4,1)");
    assert_eq!(number_of(&mut session, sheet, "D7"), 11.0, "the sum, and the extra argument is a value");
    formula(&mut session, sheet, "D8", "=AGGREGATE(20,0,A1:A4)");
    assert!(error_of(&mut session, sheet, "D8").contains("20"), "a function number that is not one");
}

/// The **errors** of an aggregate are left out when the options say
/// so, and are the answer when they do not.
#[test]
fn the_errors_of_an_aggregate_are_left_out_when_the_options_say_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1.0));
    formula(&mut session, sheet, "A2", "=1/0");
    value(&mut session, sheet, "A3", json!(3.0));
    formula(&mut session, sheet, "D1", "=AGGREGATE(9,2,A1:A3)");
    assert_eq!(number_of(&mut session, sheet, "D1"), 4.0, "the errors are left out");
    formula(&mut session, sheet, "D2", "=AGGREGATE(9,0,A1:A3)");
    assert_eq!(error_of(&mut session, sheet, "D2"), "#DIV/0!", "the error is the answer");
}
