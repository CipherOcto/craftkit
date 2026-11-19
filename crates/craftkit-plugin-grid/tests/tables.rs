//! The **table** of a sheet: a named range whose first row names the
//! fields.

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
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A sheet whose first row names **three fields**.
fn a_sheet_with_a_header(session: &mut Session) -> u64 {
    let (_, sheet) = a_workbook(session);
    a_text(session, sheet, "A1", "who");
    a_text(session, sheet, "B1", "amount");
    a_text(session, sheet, "C1", "when");
    a_text(session, sheet, "A2", "ann");
    a_text(session, sheet, "B2", "3");
    sheet
}

/// **A table is made and read back.**
#[test]
fn a_table_is_made_and_read_back() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    let outcome = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    assert_eq!(outcome.data["name"], "spendings");
    assert_eq!(outcome.data["range"], "A1:C2");

    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0]["name"], "spendings");
    assert_eq!(tables[0]["range"], "A1:C2");
}

/// **The fields of a table are the text of its header.**
#[test]
fn the_fields_are_the_text_of_the_header() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the fields are read");
    let fields = outcome.data["fields"].as_array().expect("the fields are a list");
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0]["name"], "who");
    assert_eq!(fields[0]["column"], "A");
    assert_eq!(fields[1]["name"], "amount");
    assert_eq!(fields[2]["cell"], "C1");
    assert_eq!(outcome.data["rows"], 1);
}

/// **A header cell that holds a number does not name a field.**
#[test]
fn a_header_that_holds_a_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 7 }))
        .expect("the cell is set");
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(message.contains("B1"), "the refusal says which cell: {message}");
    assert!(message.contains("a field is named by text"), "the refusal says why: {message}");
}

/// **A table of one cell is a cell.**
#[test]
fn a_table_of_one_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "one", "range": "A1" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(message.contains("a table of one cell is a cell"), "the refusal says why: {message}");
}

/// **Two tables may not share a cell.**
#[test]
fn two_tables_that_overlap_are_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "one", "range": "A1:C2" }))
        .expect("the table is made");
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "two", "range": "B1:C3" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(
        message.contains("a cell that is in two tables is in none of them"),
        "the refusal says why: {message}"
    );
}

/// **A name that looks like a cell is not a name.**
#[test]
fn a_name_that_looks_like_a_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "A1", "range": "A1:C2" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(message.contains("looks like a cell"), "the refusal says why: {message}");
}

/// **A table is renamed.**
#[test]
fn a_table_is_renamed() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute("grid.table.rename", &json!({ "sheet": sheet, "name": "spendings", "to": "outgoings" }))
        .expect("the table is renamed");
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert_eq!(tables[0]["name"], "outgoings");
}

/// **A table is resized.**
#[test]
fn a_table_is_resized() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute("grid.table.resize", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is resized");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the fields are read");
    assert_eq!(outcome.data["range"], "A1:C2");
}

/// **A table is taken away, and the cells stay.**
#[test]
fn a_table_is_taken_away() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute("grid.table.remove", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the table is taken away");
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert!(tables.is_empty());
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A2" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], "ann");
}

/// **The fields are derived: a header that is edited names another
/// field.**
#[test]
fn the_fields_are_derived_from_the_header() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    a_text(&mut session, sheet, "B1", "sum");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the fields are read");
    let fields = outcome.data["fields"].as_array().expect("the fields are a list");
    assert_eq!(fields[1]["name"], "sum");
}

/// A formula may **name a column of a table**: the column's cells are
/// the ones the formula reads.
#[test]
fn a_formula_may_name_a_column_of_a_table() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    a_text(&mut session, sheet, "A3", "bob");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 4 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B3" }))
        .expect("the table is made");
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=SUM(Orders[amount])" }))
        .expect("the formula is worked out");
    assert_eq!(outcome.data["value"].as_f64(), Some(7.0), "the column is summed");
}

/// The column of a table is read by its **name**, and without case.
#[test]
fn the_column_of_a_table_is_read_by_its_name() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B2" }))
        .expect("the table is made");
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=SUM(Orders[AMOUNT])" }))
        .expect("the formula is worked out");
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "the column is summed");
}

/// A table that is **not there**, and a column that is not there, are
/// refused — and the refusal says why.
#[test]
fn a_table_that_is_not_there_is_refused() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B2" }))
        .expect("the table is made");
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=SUM(Ledger[amount])" }))
        .expect("the formula is worked out");
    assert_eq!(outcome.data["type"].as_str(), Some("error"), "the table is not there");
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=SUM(Orders[cost])" }))
        .expect("the formula is worked out");
    assert_eq!(outcome.data["type"].as_str(), Some("error"), "the column is not there");
}

/// A formula in a **cell of the table** may name the column of its own
/// row: `[@amount]`.
#[test]
fn a_formula_in_a_cell_of_a_table_may_name_its_own_column() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    a_text(&mut session, sheet, "A3", "bob");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 4 }))
        .expect("the cell is set");
    a_text(&mut session, sheet, "C1", "twice");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:C3" }))
        .expect("the table is made");
    // A formula in a **column of the table**: it names the column of
    // the row it is in.
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "C2", "formula": "=[@amount]*2" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C2" })).expect("the cell is read");
    assert_eq!(outcome.data["value"].as_f64(), Some(6.0), "the row's own amount is doubled");
    // A **copy** of the formula: the structured reference does not move
    // with the cell, and the row it names is the row the cell is in.
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "C2" }))
        .expect("the cell is copied");
    session.execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C3" })).expect("the cell is pasted");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C3" })).expect("the cell is read");
    assert_eq!(outcome.data["value"].as_f64(), Some(8.0), "the next row's own amount is doubled");
    assert_eq!(outcome.data["formula"].as_str(), Some("=[@amount] * 2"), "and the formula is as it was");
}

/// A **rename follows** the formulas that name it.
#[test]
fn a_rename_follows_the_formulas_that_name_it() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B2" }))
        .expect("the table is made");
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "C1", "formula": "=SUM(Orders[amount])" }),
        )
        .expect("the formula is set");
    let outcome = session
        .execute("grid.table.rename", &json!({ "sheet": sheet, "name": "Orders", "to": "Ledger" }))
        .expect("the table is renamed");
    assert_eq!(outcome.data["followed"].as_u64(), Some(1), "the formula follows the rename");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C1" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=SUM(Ledger[amount])"),
        "and it names the table by its new name"
    );
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "and it still reads the column");
}

/// A **column** of a table is renamed by its **header**, and the rename
/// follows the formulas that name the column.
#[test]
fn a_column_renamed_follows_the_formulas_that_name_it() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "C1", "twice");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    a_text(&mut session, sheet, "A3", "bob");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 4 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:C3" }))
        .expect("the table is made");
    // The **column of the row the cell is in**.
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "C2", "formula": "=[@amount]*2" }))
        .expect("the formula is set");
    // The **column of the table**, named from outside it.
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "E1", "formula": "=SUM(Orders[amount])" }),
        )
        .expect("the formula is set");
    // The **header** of the column is edited: the column is renamed.
    let outcome = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": "sum" }))
        .expect("the header is set");
    assert_eq!(outcome.data["followed"].as_u64(), Some(2), "both formulas follow the rename");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C2" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=[@sum] * 2"),
        "and the row of the cell names the column by its new name"
    );
    assert_eq!(outcome.data["value"].as_f64(), Some(6.0), "and it still reads the column");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "E1" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=SUM(Orders[sum])"),
        "and the table names the column by its new name"
    );
    assert_eq!(outcome.data["value"].as_f64(), Some(7.0), "and it still reads the column");
}

/// A **file** spells the row of the cell `[[#This Row],[Column]]`, and
/// the tree reads that too.
#[test]
fn a_file_spells_the_row_of_the_cell_and_the_tree_reads_it() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    a_text(&mut session, sheet, "A3", "bob");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 4 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B3" }))
        .expect("the table is made");
    // The **column of the row the cell is in**, as a file spells it.
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "D2", "formula": "=Orders[[#This Row],[amount]]" }),
        )
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "D2" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=Orders[[#This Row],[amount]]"),
        "the formula is as it was written"
    );
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "and it reads the row of the cell");
    // The **whole row** the cell is in.
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "D3", "formula": "=SUM(Orders[[#This Row]])" }),
        )
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "D3" })).expect("the cell is read");
    assert_eq!(outcome.data["value"].as_f64(), Some(4.0), "and it reads the row of the cell");
    // The **whole table**, as a file spells it.
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "E1", "formula": "=SUM(Orders[])" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "E1" })).expect("the cell is read");
    assert_eq!(outcome.data["value"].as_f64(), Some(7.0), "and it reads the whole table");
}

/// The **file's spelling** of the row of the cell survives a **rename**.
#[test]
fn the_files_spelling_of_the_row_of_the_cell_survives_a_rename() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "C1", "twice");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "C2", "formula": "=Orders[[#This Row],[amount]]" }),
        )
        .expect("the formula is set");
    session
        .execute("grid.table.rename", &json!({ "sheet": sheet, "name": "Orders", "to": "Ledger" }))
        .expect("the table is renamed");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C2" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=Ledger[[#This Row],[amount]]"),
        "and the row of the cell is still the row of the cell"
    );
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "and it still reads the column");
}

/// A name that **two columns share** names neither of them: the tree
/// does not guess which one is meant.
#[test]
fn a_name_that_two_columns_share_names_neither_of_them() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "amount");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A3", "value": 3 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B3" }))
        .expect("the table is made");
    // The **fields** say the name that two of them share.
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "Orders" }))
        .expect("the fields are read");
    let repeated = outcome.data["repeated"].as_array().expect("the repeated names are a list");
    assert_eq!(repeated.len(), 1, "one name is shared");
    assert_eq!(repeated[0]["name"].as_str(), Some("amount"), "and it is the name of the two columns");
    assert_eq!(repeated[0]["count"].as_u64(), Some(2), "and two columns share it");
    // A **formula** that names it is refused.
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "D1", "formula": "=SUM(Orders[amount])" }),
        )
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "D1" })).expect("the cell is read");
    assert_eq!(
        outcome.data["value"]["error"].as_str(),
        Some(
            "the table `Orders` has 2 columns called `amount`, and a name that is two columns does not say which one"
        ),
        "and the name names neither of them"
    );
}

/// A **paste** that writes over the header of a table renames the
/// column, and the rename follows the formulas that name it.
#[test]
fn a_paste_that_changes_a_header_follows_the_formulas() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "amount");
    a_text(&mut session, sheet, "B1", "who");
    a_text(&mut session, sheet, "A2", "ann");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A3", "value": 3 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B3" }))
        .expect("the table is made");
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "D1", "formula": "=SUM(Orders[amount])" }),
        )
        .expect("the formula is set");
    a_text(&mut session, sheet, "F1", "sum");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "F1" }))
        .expect("the cell is copied");
    let outcome = session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A1", "overwrite": true }))
        .expect("the cell is pasted");
    assert_eq!(outcome.data["followed"].as_u64(), Some(1), "the formula follows the paste");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "D1" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=SUM(Orders[sum])"),
        "and it names the column by its new name"
    );
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "and it still reads the column");
}

/// A **fill** that writes over the header of a table renames the
/// column, and the rename follows the formulas that name it.
#[test]
fn a_fill_that_changes_a_header_follows_the_formulas() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "sum");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    a_text(&mut session, sheet, "A3", "bob");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 4 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders", "range": "A1:B3" }))
        .expect("the table is made");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "D1", "formula": "=Orders[who]" }))
        .expect("the formula is set");
    // The **source** of the fill is the cell under the header, and it
    // holds the new name of the column.
    let outcome = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A2", "to": "A1:A2", "mode": "copy" }))
        .expect("the cell is filled");
    assert_eq!(outcome.data["followed"].as_u64(), Some(1), "the formula follows the fill");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "D1" })).expect("the cell is read");
    assert_eq!(
        outcome.data["formula"].as_str(),
        Some("=Orders[sum]"),
        "and it names the column by its new name"
    );
    assert_eq!(outcome.data["value"].as_str(), Some("2 values"), "and it still reads the column");
}
