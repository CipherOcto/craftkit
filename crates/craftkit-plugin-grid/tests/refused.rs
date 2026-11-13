//! The tree's functions that are **answered with a reason**: the
//! pattern and the cube.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The formula a name of the table is called with.
fn call_of(name: &str) -> String {
    match name {
        "REGEXTEST" => format!("={name}(\"abc\",\"a\")"),
        "REGEXEXTRACT" => format!("={name}(\"abc\",\"(a)\")"),
        "REGEXREPLACE" => format!("={name}(\"abc\",\"a\",\"x\")"),
        "CUBEMEMBER" => format!("={name}(\"c\",\"m\")"),
        "CUBEMEMBERPROPERTY" => format!("={name}(\"c\",\"m\",\"p\")"),
        "CUBERANKEDMEMBER" => format!("={name}(\"c\",\"s\",1)"),
        "CUBESET" => format!("={name}(\"c\",\"s\")"),
        "CUBESETCOUNT" => format!("={name}(\"s\")"),
        "CUBEVALUE" => format!("={name}(\"c\",\"m\")"),
        "CUBEKPIMEMBER" => format!("={name}(\"c\",\"k\",\"p\")"),
        other => format!("={other}(1)"),
    }
}

/// **Every** refused function is refused, and it says why.
#[test]
fn every_refused_function_is_refused_and_says_why() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let table = craftkit_plugin_grid::refused::REFUSED_FUNCTIONS;
    assert!(!table.is_empty(), "the table of the refused is not empty");
    for (index, (name, reason)) in table.iter().enumerate() {
        let cell = format!("A{}", index + 1);
        formula(&mut session, sheet, &cell, &call_of(name));
        let value = value_of(&mut session, sheet, &cell);
        assert_eq!(
            value["error"],
            Json::String(reason.to_string()),
            "{name} is refused and says why: {value}"
        );
    }
}

/// The table and the lookup **agree**.
#[test]
fn the_table_and_the_lookup_agree() {
    for (name, reason) in craftkit_plugin_grid::refused::REFUSED_FUNCTIONS {
        assert_eq!(craftkit_plugin_grid::refused::reason_for(name), Some(*reason));
    }
    assert_eq!(craftkit_plugin_grid::refused::reason_for("SUM"), None);
}

/// The **matrix** functions that give back a table are refused as an
/// array.
#[test]
fn the_matrix_functions_that_give_back_a_table_are_refused_as_an_array() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, name) in ["MMULT", "MINVERSE", "MUNIT"].iter().enumerate() {
        let cell = format!("C{}", index + 1);
        formula(&mut session, sheet, &cell, &format!("={name}(A1:A2)"));
        let value = value_of(&mut session, sheet, &cell);
        assert_eq!(
            value["error"],
            Json::String("not supported yet: an array".to_string()),
            "{name} is refused as an array: {value}"
        );
    }
}
