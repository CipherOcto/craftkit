//! The **special** cells of a sheet: the cells that answer a question
//! asked of the sheet.
//!
//! The tree's own `edit.goToSpecial`, and the tree's own kinds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Outcome, Session};

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

/// A **value** in a cell.
fn a_value(session: &mut Session, sheet: u64, cell: &str, value: craftkit_sdk::serde_json::Value) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    a_value(session, sheet, cell, json!(value));
}

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, value: &str) {
    a_value(session, sheet, cell, json!(value));
}

/// A **formula** in a cell.
fn a_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **addresses** the answer lists.
fn addresses(outcome: &Outcome) -> Vec<String> {
    outcome.data["hits"]
        .as_array()
        .expect("the answer lists the hits")
        .iter()
        .map(|hit| hit["cell"].as_str().expect("a hit has an address").to_string())
        .collect()
}

/// The **special** cells of a sheet.
fn special(session: &mut Session, sheet: u64, kind: &str, range: Option<&str>) -> Outcome {
    let mut params = json!({ "sheet": sheet, "kind": kind });
    if let Some(range) = range {
        params["range"] = json!(range);
    }
    session.execute("grid.find.special", &params).expect("the question is answered")
}

/// The **blank** cells of a range.
#[test]
fn the_blank_cells_of_a_range_are_found() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "C1", 2.0);
    let outcome = special(&mut session, sheet, "blanks", Some("A1:C1"));
    assert_eq!(addresses(&outcome), vec!["B1".to_string()], "the empty cell between them is the blank");
    assert_eq!(outcome.data["count"], json!(1), "and it is the only one");
}

/// The **constants** and the **formulas** are told apart.
#[test]
fn the_constants_and_the_formulas_are_told_apart() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "B1", "=1+1");
    let outcome = special(&mut session, sheet, "constants", Some("A1:B1"));
    assert_eq!(addresses(&outcome), vec!["A1".to_string()], "the number is a constant");
    let outcome = special(&mut session, sheet, "formulas", Some("A1:B1"));
    assert_eq!(addresses(&outcome), vec!["B1".to_string()], "and the formula is a formula");
}

/// A **comment** and a **note** are two things, and each is found.
#[test]
fn the_comments_and_the_notes_are_found() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "hello" }))
        .expect("the comment is written");
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A2", "text": "hello" }))
        .expect("the note is written");
    let outcome = special(&mut session, sheet, "comments", Some("A1:A2"));
    assert_eq!(addresses(&outcome), vec!["A1".to_string()], "the comment is on A1");
    let outcome = special(&mut session, sheet, "notes", Some("A1:A2"));
    assert_eq!(addresses(&outcome), vec!["A2".to_string()], "and the note is on A2");
}

/// The **numbers**, the **text** and the **errors** are told apart.
#[test]
fn the_numbers_the_text_and_the_errors_are_told_apart() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_text(&mut session, sheet, "A2", "hello");
    a_formula(&mut session, sheet, "A3", "=1/0");
    let outcome = special(&mut session, sheet, "numbers", Some("A1:A3"));
    assert_eq!(addresses(&outcome), vec!["A1".to_string()], "the number is a number");
    let outcome = special(&mut session, sheet, "text", Some("A1:A3"));
    assert_eq!(addresses(&outcome), vec!["A2".to_string()], "the text is text");
    let outcome = special(&mut session, sheet, "errors", Some("A1:A3"));
    assert_eq!(addresses(&outcome), vec!["A3".to_string()], "and the error is an error");
}

/// A **hidden** row is not visible.
#[test]
fn the_hidden_cells_are_not_visible() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 1 })).expect("the row is hidden");
    let outcome = special(&mut session, sheet, "visible", Some("A1:A2"));
    assert_eq!(addresses(&outcome), vec!["A2".to_string()], "the hidden row is not shown");
}

/// The **conditional formats** and the **validations** are about cells.
#[test]
fn the_conditional_formats_and_the_validations_are_found() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "C1", 3.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B1", "op": ">", "value": "0", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "C1:C1",
                "kind": "whole",
                "operator": "between",
                "formula1": "1",
                "formula2": "9",
            }),
        )
        .expect("the validation is set");
    let outcome = special(&mut session, sheet, "conditionalFormats", Some("A1:C1"));
    assert_eq!(addresses(&outcome), vec!["A1".to_string(), "B1".to_string()], "the rule is about A1 and B1");
    let outcome = special(&mut session, sheet, "dataValidation", Some("A1:C1"));
    assert_eq!(addresses(&outcome), vec!["C1".to_string()], "and the validation is about C1");
}

/// The **last cell** is the far corner of the used range.
#[test]
fn the_last_cell_is_the_far_corner() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "C3", 2.0);
    let outcome = special(&mut session, sheet, "lastCell", None);
    assert_eq!(addresses(&outcome), vec!["C3".to_string()], "the far corner is C3");
    assert_eq!(outcome.data["area"], json!("C3"), "and the answer says where it is");
}

/// The **last cell** of a sheet that holds nothing is `A1`.
#[test]
fn the_last_cell_of_an_empty_sheet_is_a1() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = special(&mut session, sheet, "lastCell", None);
    assert_eq!(addresses(&outcome), vec!["A1".to_string()], "the corner of nothing is A1");
}

/// The **current region** is the block of data around a cell.
#[test]
fn the_current_region_is_the_block_around_a_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "E5", 4.0);
    let outcome = special(&mut session, sheet, "currentRegion", None);
    // The cell is `A1` when it is not given, and the region around it
    // is the block of the three cells.
    assert_eq!(outcome.data["area"], json!("A1:B2"), "the region is the block");
    assert_eq!(outcome.data["count"], json!(4), "and it holds four cells");
}

/// The **current region** starts at the cell that is named.
#[test]
fn the_current_region_starts_at_the_cell_that_is_named() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    let outcome = session
        .execute("grid.find.special", &json!({ "sheet": sheet, "kind": "currentRegion", "cell": "A1" }))
        .expect("the region is found");
    assert_eq!(outcome.data["area"], json!("A1:B2"), "the region is the block around A1");
}

/// A kind that is not one of them is **refused by name**.
#[test]
fn an_unknown_kind_is_refused_by_name() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.find.special", &json!({ "sheet": sheet, "kind": "nonsense" }))
        .expect_err("the kind is refused");
    assert!(error.to_string().contains("is not a kind"), "the refusal says so: {error}");
    assert!(error.to_string().contains("conditionalFormats"), "and it says the kinds: {error}");
}

/// The tree's own spelling and the words are **one kind**.
#[test]
fn the_tree_spelling_and_the_words_are_one_kind() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let tree = special(&mut session, sheet, "lastCell", None);
    let words = special(&mut session, sheet, "last cell", None);
    assert_eq!(addresses(&tree), addresses(&words), "the two spellings are one kind");
    assert_eq!(tree.data["kind"], json!("lastCell"), "and the answer names the tree's kind");
}
