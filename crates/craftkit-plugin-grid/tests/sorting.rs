//! The **sorting** of a range.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **values** of a range, as the answer holds them.
fn values_of(session: &mut Session, sheet: u64, range: &str) -> Vec<Json> {
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": range }))
        .expect("the range is read");
    outcome.data["values"].as_array().expect("the values are a list").clone()
}

/// The rows are put in **order**.
#[test]
fn the_rows_are_put_in_order() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "A2", json!(1));
    value(&mut session, sheet, "A3", json!(2));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], json!(1.0), "the one is first");
    assert_eq!(values[2][0], json!(3.0), "and the three is last");
}

/// The order may be **descending**.
#[test]
fn the_order_may_be_descending() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "A2", json!(1));
    value(&mut session, sheet, "A3", json!(2));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A", "descending": true }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], json!(3.0), "the three is first");
    assert_eq!(values[2][0], json!(1.0), "and the one is last");
}

/// The **empty** cells are last, whatever the order is.
#[test]
fn the_empty_cells_are_last_whatever_the_order_is() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "A2", json!(1));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[1][0], json!(3.0), "the three is under the one");
    assert_eq!(values[2][0], Json::Null, "and the empty cell is last");
}

/// The text is compared **without case**.
#[test]
fn the_text_is_compared_without_case() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("b"));
    value(&mut session, sheet, "A2", json!("A"));
    value(&mut session, sheet, "A3", json!("c"));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], Json::String("A".to_string()), "the A is first");
    assert_eq!(values[2][0], Json::String("c".to_string()), "and the c is last");
}

/// A **header** is guessed and stays where it is.
#[test]
fn a_header_is_guessed_and_stays_where_it_is() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("name"));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(1));
    let outcome = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    assert_eq!(outcome.data["header"], Json::Bool(true), "the first row is a header");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], Json::String("name".to_string()), "the header stays");
    assert_eq!(values[1][0], json!(1.0), "and the rows are in order");
}

/// A **formula** moves with its row, and its relative references move
/// with it.
#[test]
fn a_formula_moves_with_its_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(5));
    value(&mut session, sheet, "A2", json!(1));
    formula(&mut session, sheet, "B1", "=A1*2");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:B2", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B2");
    assert_eq!(values[1][0], json!(5.0), "the five moved down");
    assert_eq!(values[1][1], json!(10.0), "and the formula still reads the row it is on");
}

/// An **anchored** reference does not move.
#[test]
fn an_anchored_reference_does_not_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(5));
    value(&mut session, sheet, "A2", json!(1));
    formula(&mut session, sheet, "B1", "=$A$1*2");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:B2", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B2");
    assert_eq!(values[1][1], json!(2.0), "the formula still reads the first cell");
}

/// The **note** and the **link** travel with the row.
#[test]
fn the_note_and_the_link_travel_with_the_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "A2", json!(1));
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "about the two" }))
        .expect("the note is written");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "somewhere" }))
        .expect("the link is written");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "A" }))
        .expect("the range is sorted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], Json::String("about the two".to_string()), "the note moved");
    let outcome = session
        .execute("grid.cell.link.get", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the link is read");
    assert_eq!(outcome.data["link"]["target"], Json::String("somewhere".to_string()), "and the link");
}

/// A column that is **not in the range** is refused.
#[test]
fn a_column_that_is_not_in_the_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    let error = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "C" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("not in the range"), "it says why: {error}");
}

/// A range with a **merge** in it is refused.
#[test]
fn a_range_with_a_merge_in_it_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A2:B2" }))
        .expect("the merge is set");
    let error = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:B2", "column": "A" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("merge"), "it says why: {error}");
}

/// A sort may have **more than one key**: the first is the one that
/// decides, and the next is looked at only when the first cannot.
#[test]
fn a_sort_may_have_more_than_one_key() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // Three rows: two of them share a name, and the second key tells
    // them apart.
    value(&mut session, sheet, "A1", json!("ann"));
    value(&mut session, sheet, "B1", json!(2));
    value(&mut session, sheet, "A2", json!("bob"));
    value(&mut session, sheet, "B2", json!(1));
    value(&mut session, sheet, "A3", json!("ann"));
    value(&mut session, sheet, "B3", json!(1));
    session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:B3",
                "keys": [{ "column": "A" }, { "column": "B" }],
            }),
        )
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B3");
    assert_eq!(values[0][0], json!("ann"), "the ann is first");
    assert_eq!(values[0][1], json!(1.0), "and the one of it");
    assert_eq!(values[1][0], json!("ann"), "then the other ann");
    assert_eq!(values[1][1], json!(2.0), "and the two of it");
    assert_eq!(values[2][0], json!("bob"), "and the bob is last");
}

/// A key may be **descending**, and the rest of the keys decide when it
/// cannot.
#[test]
fn a_key_may_be_descending() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "A2", json!("ann"));
    value(&mut session, sheet, "B2", json!(2));
    session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:B2",
                "keys": [{ "column": "A" }, { "column": "B", "descending": true }],
            }),
        )
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B2");
    assert_eq!(values[0][1], json!(2.0), "the two of the ann is first");
    assert_eq!(values[1][1], json!(1.0), "and the one of it is last");
}

/// The **keys** and the **column** may not be given together: a sort is
/// sorted by one of them.
#[test]
fn the_keys_and_the_column_are_not_given_together() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "A2", json!(1));
    let failure = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "column": "A",
                "keys": [{ "column": "A" }],
            }),
        )
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("and not by both"), "the refusal says why: {message}");
}

/// A key may order the words by one of the workbook's **lists**: the
/// words of the list in the order of the list, and the words that are
/// not in it after them.
#[test]
fn a_key_may_order_the_words_by_a_list() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let workbook = session
        .document()
        .walk()
        .find(|node| node.kind == "grid.workbook")
        .expect("the workbook is there")
        .id;
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": workbook, "words": "Mon\nTue\nWed" }))
        .expect("the list is taught");
    value(&mut session, sheet, "A1", json!("Wed"));
    value(&mut session, sheet, "A2", json!("Mon"));
    value(&mut session, sheet, "A3", json!("Tue"));
    value(&mut session, sheet, "A4", json!("Sat"));
    session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:A4",
                "keys": [{ "column": "A", "list": 1 }],
            }),
        )
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A4");
    assert_eq!(values[0][0], json!("Mon"), "the Monday is first");
    assert_eq!(values[1][0], json!("Tue"), "then the Tuesday");
    assert_eq!(values[2][0], json!("Wed"), "then the Wednesday");
    assert_eq!(values[3][0], json!("Sat"), "and a word that is not in the list is last");
}

/// A key that names a list that is **not there** is refused.
#[test]
fn a_list_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("b"));
    value(&mut session, sheet, "A2", json!("a"));
    let failure = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "keys": [{ "column": "A", "list": 3 }],
            }),
        )
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("there is no list"), "the refusal says why: {message}");
}

/// The values of **different kinds** are ordered by their kind: the
/// numbers, then the text, then the booleans, then the errors, and the
/// empty cells last.
#[test]
fn the_values_of_different_kinds_are_ordered_by_their_kind() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("banana"));
    value(&mut session, sheet, "A2", json!(true));
    value(&mut session, sheet, "A3", json!(2));
    value(&mut session, sheet, "A4", json!("apple"));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A4", "column": "A", "header": false }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A4");
    assert_eq!(values[0][0], json!(2.0), "the number is first");
    assert_eq!(values[1][0], json!("apple"), "then the text");
    assert_eq!(values[2][0], json!("banana"), "then the other text");
    assert_eq!(values[3][0], json!(true), "and the boolean is last");
}

// The **orientation** of a sort: the rows of the range, or its columns.

/// The **columns** are put in order, by a **row** of the range.
#[test]
fn the_columns_are_put_in_order_by_a_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("b"));
    value(&mut session, sheet, "B1", json!("a"));
    value(&mut session, sheet, "C1", json!("c"));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "B2", json!(3));
    value(&mut session, sheet, "C2", json!(4));
    let outcome = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:C2",
                "orientation": "columns",
                "keys": [{ "row": 1 }],
            }),
        )
        .expect("the range is sorted");
    assert_eq!(outcome.data["orientation"], Json::String("columns".to_string()), "it says which way");
    assert_eq!(outcome.data["row"], json!(1), "and by which row");
    let values = values_of(&mut session, sheet, "A1:C2");
    assert_eq!(values[0][0], json!("a"), "the a is first");
    assert_eq!(values[0][1], json!("b"), "then the b");
    assert_eq!(values[1][0], json!(3.0), "and the column took its row with it");
    assert_eq!(values[1][1], json!(2.0), "the whole of it");
}

/// The **first column** is the **header** of a sort of the columns, and
/// it stays where it is.
#[test]
fn the_first_column_is_the_header_of_a_sort_of_the_columns() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("name"));
    value(&mut session, sheet, "B1", json!("x"));
    value(&mut session, sheet, "C1", json!("y"));
    value(&mut session, sheet, "A2", json!("ann"));
    value(&mut session, sheet, "B2", json!(2));
    value(&mut session, sheet, "C2", json!(1));
    let outcome = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:C2",
                "orientation": "columns",
                "keys": [{ "row": 2 }],
            }),
        )
        .expect("the range is sorted");
    assert_eq!(outcome.data["header"], Json::Bool(true), "the first column is a header");
    let values = values_of(&mut session, sheet, "A1:C2");
    assert_eq!(values[1][0], json!("ann"), "the header stays where it is");
    assert_eq!(values[1][1], json!(1.0), "and the columns are in order");
    assert_eq!(values[1][2], json!(2.0), "the whole of them");
}

/// A **formula** moves with its column, and its relative references move
/// with it.
#[test]
fn a_formula_moves_with_its_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "C1", json!(5));
    formula(&mut session, sheet, "B2", "=C1*2");
    session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:B2",
                "orientation": "columns",
                "keys": [{ "row": 1 }],
            }),
        )
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B2");
    assert_eq!(values[0][0], json!(1.0), "the one is first");
    assert_eq!(values[1][0], json!(4.0), "and the formula still reads the column it is on");
}

/// The **note** travels with the column.
#[test]
fn the_note_travels_with_the_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "B1", json!(1));
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "B1", "text": "about the one" }))
        .expect("the note is written");
    session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:B1",
                "orientation": "columns",
                "keys": [{ "row": 1 }],
            }),
        )
        .expect("the range is sorted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], Json::String("about the one".to_string()), "the note moved");
}

/// A key that names the **wrong kind of line** is refused: the tree does
/// not guess which way a sort was meant to run.
#[test]
fn a_key_that_names_the_wrong_kind_of_line_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    let failure = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "orientation": "columns",
                "keys": [{ "column": "A" }],
            }),
        )
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("by a row"), "the refusal says why: {message}");
    let failure = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "keys": [{ "row": 1 }],
            }),
        )
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("by a column"), "the refusal says why: {message}");
}

/// A sort of the columns may not be given a **column**: it is by a row.
#[test]
fn a_sort_of_the_columns_is_not_given_a_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    let failure = session
        .execute(
            "grid.sort",
            &json!({ "sheet": sheet, "range": "A1:A2", "orientation": "columns", "column": "A" }),
        )
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("by a row"), "the refusal says why: {message}");
}

/// An **orientation** that is neither the rows nor the columns is
/// refused.
#[test]
fn an_orientation_that_is_neither_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    let failure = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "orientation": "sideways" }))
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("the rows or of the columns"), "the refusal says why: {message}");
}

/// A row that is **not in the range** is refused.
#[test]
fn a_row_that_is_not_in_the_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    let failure = session
        .execute(
            "grid.sort",
            &json!({
                "sheet": sheet,
                "range": "A1:B2",
                "orientation": "columns",
                "keys": [{ "row": 5 }],
            }),
        )
        .expect_err("the sort is refused");
    let message = failure.to_string();
    assert!(message.contains("not in the range"), "the refusal says why: {message}");
}
