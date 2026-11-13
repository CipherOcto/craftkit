//! The **finding of a value**, the tree's way: `XLOOKUP` and `XMATCH`.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// A sheet of **keys down** and **values beside** them.
fn keys(session: &mut Session, sheet: u64) {
    value(session, sheet, "A1", json!("apple"));
    value(session, sheet, "A2", json!("banana"));
    value(session, sheet, "A3", json!("cherry"));
    value(session, sheet, "B1", json!(10));
    value(session, sheet, "B2", json!(20));
    value(session, sheet, "B3", json!(30));
}

/// The value of the return array **at the place** of the needle.
#[test]
fn the_value_at_the_place_of_the_needle() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"banana\",A1:A3,B1:B3)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(20.0), "the banana is the twenty");
    formula(&mut session, sheet, "D2", "=XLOOKUP(\"BANANA\",A1:A3,B1:B3)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(20.0), "and the case is not the case");
}

/// The needle that is **not there** is the value that is given, and
/// `#N/A` when there is none.
#[test]
fn the_needle_that_is_not_there() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"pear\",A1:A3,B1:B3,\"nothing\")");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!("nothing"), "the value that is given");
    formula(&mut session, sheet, "D2", "=XLOOKUP(\"pear\",A1:A3,B1:B3)");
    let value = value_of(&mut session, sheet, "D2");
    assert_eq!(value["error"].as_str(), Some("#N/A"), "it says why: {value}");
}

/// The **wildcards** are a match mode of their own, and the exact
/// search does not use them.
#[test]
fn the_wildcards_are_a_match_mode_of_their_own() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"b*\",A1:A3,B1:B3,0,2)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(20.0), "the banana is the twenty");
    formula(&mut session, sheet, "D2", "=XLOOKUP(\"b*\",A1:A3,B1:B3)");
    let value = value_of(&mut session, sheet, "D2");
    assert_eq!(value["error"].as_str(), Some("#N/A"), "the exact search does not use them: {value}");
}

/// The **next smaller** and the **next larger** — and the exact match
/// wins over both.
#[test]
fn the_next_smaller_and_the_next_larger() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(10));
    value(&mut session, sheet, "A2", json!(20));
    value(&mut session, sheet, "A3", json!(30));
    value(&mut session, sheet, "B1", json!("ten"));
    value(&mut session, sheet, "B2", json!("twenty"));
    value(&mut session, sheet, "B3", json!("thirty"));
    formula(&mut session, sheet, "D1", "=XLOOKUP(25,A1:A3,B1:B3,0,-1)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!("twenty"), "the largest below twenty-five");
    formula(&mut session, sheet, "D2", "=XLOOKUP(25,A1:A3,B1:B3,0,1)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!("thirty"), "the smallest above twenty-five");
    formula(&mut session, sheet, "D3", "=XLOOKUP(20,A1:A3,B1:B3,0,-1)");
    assert_eq!(value_of(&mut session, sheet, "D3"), json!("twenty"), "the exact match wins");
}

/// The search may go from the **last** to the first.
#[test]
fn the_search_may_go_from_the_last_to_the_first() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("apple"));
    value(&mut session, sheet, "A2", json!("apple"));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "B2", json!(2));
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"apple\",A1:A2,B1:B2,0,0,-1)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(2.0), "the last apple is the two");
    formula(&mut session, sheet, "D2", "=XLOOKUP(\"apple\",A1:A2,B1:B2)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(1.0), "and the first is the one");
}

/// The **binary search** is not here.
#[test]
fn the_binary_search_is_not_here() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"banana\",A1:A3,B1:B3,0,0,2)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"].as_str(), Some("not supported yet: a binary search"), "it says why: {value}");
}

/// The lookup array is a **row or a column**, and the return array is
/// of the same shape.
#[test]
fn the_lookup_array_is_a_row_or_a_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"apple\",A1:B3,B1:B3)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
    formula(&mut session, sheet, "D2", "=XLOOKUP(\"apple\",A1:A3,A1:A2)");
    let value = value_of(&mut session, sheet, "D2");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
}

/// The tree's `XLOOKUP` may return a **whole row** of the return
/// array, and this tree has no arrays.
#[test]
fn the_return_of_a_whole_row_is_not_here() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XLOOKUP(\"banana\",A1:A3,B1:C3)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"].as_str(), Some("not supported yet: an array"), "it says why: {value}");
}

/// The **place** of a needle, counted from one.
#[test]
fn the_place_of_a_needle() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    keys(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=XMATCH(\"cherry\",A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(3.0), "the cherry is the third");
    formula(&mut session, sheet, "D2", "=XMATCH(\"pear\",A1:A3)");
    let value = value_of(&mut session, sheet, "D2");
    assert_eq!(value["error"].as_str(), Some("#N/A"), "it says why: {value}");
}

/// The modes of `XMATCH` are the tree's, and a mode that is not one
/// of them is an error.
#[test]
fn the_modes_of_xmatch() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(10));
    value(&mut session, sheet, "A2", json!(20));
    value(&mut session, sheet, "A3", json!(30));
    formula(&mut session, sheet, "D1", "=XMATCH(25,A1:A3,-1)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(2.0), "the largest below twenty-five");
    formula(&mut session, sheet, "D2", "=XMATCH(25,A1:A3,1)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(3.0), "the smallest above twenty-five");
    formula(&mut session, sheet, "D3", "=XMATCH(20,A1:A3,7)");
    let value = value_of(&mut session, sheet, "D3");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
    formula(&mut session, sheet, "D4", "=XMATCH(20,A1:A3,0,2)");
    let value = value_of(&mut session, sheet, "D4");
    assert_eq!(value["error"].as_str(), Some("not supported yet: a binary search"), "it says why: {value}");
}
