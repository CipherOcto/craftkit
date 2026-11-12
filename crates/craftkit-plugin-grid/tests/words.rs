//! The **words** of a number and the **letters** of a text.
//!
//! A formula may ask about a number in Roman numerals, a number in
//! another base, and the characters of a text.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell and read what it computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    value_of(session, sheet, cell)
}

/// A number written in **Roman numerals**, and read back.
#[test]
fn the_roman_numerals_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ROMAN(499)"), json!("CDXCIX"), "the classic form");
    assert_eq!(computed(&mut session, sheet, "A2", "=ROMAN(499,1)"), json!("LDVLIV"), "and the first");
    assert_eq!(computed(&mut session, sheet, "A3", "=ROMAN(499,2)"), json!("XDIX"), "and the second");
    assert_eq!(computed(&mut session, sheet, "A4", "=ROMAN(499,3)"), json!("VDIV"), "and the third");
    assert_eq!(computed(&mut session, sheet, "A5", "=ROMAN(499,4)"), json!("ID"), "and the fourth");
    assert_eq!(computed(&mut session, sheet, "A6", "=ROMAN(1999)"), json!("MCMXCIX"), "and a bigger number");
    assert_eq!(computed(&mut session, sheet, "A7", "=ROMAN(0)"), json!(""), "and nothing");
    assert_eq!(computed(&mut session, sheet, "A8", "=ARABIC(\"LVII\")"), json!(57.0), "and read back");
    assert_eq!(computed(&mut session, sheet, "A9", "=ARABIC(\"mcmxii\")"), json!(1912.0), "in any case");
    assert_eq!(computed(&mut session, sheet, "A10", "=ARABIC(\"-XIV\")"), json!(-14.0), "and negative");
}

/// A number in another **base**.
#[test]
fn the_number_in_another_base_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=BASE(7,2)"), json!("111"), "in two");
    assert_eq!(computed(&mut session, sheet, "A2", "=BASE(100,16)"), json!("64"), "in sixteen");
    assert_eq!(
        computed(&mut session, sheet, "A3", "=BASE(15,2,10)"),
        json!("0000001111"),
        "and padded with zeros"
    );
    assert_eq!(computed(&mut session, sheet, "A4", "=DECIMAL(\"FF\",16)"), json!(255.0), "read back");
    assert_eq!(computed(&mut session, sheet, "A5", "=DECIMAL(\"111\",2)"), json!(7.0), "from two");
    assert_eq!(
        computed(&mut session, sheet, "A6", "=DECIMAL(\"zap\",36)"),
        json!(45745.0),
        "from thirty-six"
    );
}

/// A text **repeated**, and whether two texts are the **same**.
#[test]
fn the_repeated_text_and_the_same_texts_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=REPT(\"ab\",3)"), json!("ababab"), "the text");
    assert_eq!(computed(&mut session, sheet, "A2", "=REPT(\"ab\",0)"), json!(""), "and none of it");
    assert_eq!(computed(&mut session, sheet, "A3", "=EXACT(\"ab\",\"ab\")"), json!(true), "the same");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=EXACT(\"ab\",\"AB\")"),
        json!(false),
        "and not the same"
    );
}

/// The **characters** of a text and their **codes**.
#[test]
fn the_characters_and_the_codes_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=CHAR(65)"), json!("A"), "the character");
    assert_eq!(computed(&mut session, sheet, "A2", "=CODE(\"A\")"), json!(65.0), "and its code");
    assert_eq!(computed(&mut session, sheet, "A3", "=UNICHAR(9731)"), json!("☃"), "and in Unicode");
    assert_eq!(computed(&mut session, sheet, "A4", "=UNICODE(\"☃\")"), json!(9731.0), "and back");
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ENCODEURL(\"a b&c/é\")"),
        json!("a%20b%26c%2F%C3%A9"),
        "and a text for a URL"
    );
}

/// The **byte** versions of the text functions are the same functions.
#[test]
fn the_byte_versions_are_the_same_functions() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=LEFTB(\"abc\",2)"), json!("ab"), "the left");
    assert_eq!(computed(&mut session, sheet, "A2", "=RIGHTB(\"abc\",2)"), json!("bc"), "the right");
    assert_eq!(computed(&mut session, sheet, "A3", "=MIDB(\"abc\",2,2)"), json!("bc"), "the middle");
    assert_eq!(computed(&mut session, sheet, "A4", "=LENB(\"abc\")"), json!(3.0), "the length");
    assert_eq!(computed(&mut session, sheet, "A5", "=FINDB(\"b\",\"abc\")"), json!(2.0), "the find");
    assert_eq!(computed(&mut session, sheet, "A6", "=SEARCHB(\"B\",\"abc\")"), json!(2.0), "and the search");
    assert_eq!(
        computed(&mut session, sheet, "A7", "=REPLACEB(\"abc\",1,1,\"x\")"),
        json!("xbc"),
        "and the replace"
    );
}

/// The tree's **errors**: a base that is not one, a Roman number
/// beyond the tree's bound and a letter that is not a letter.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=BASE(-7,2)"),
        json!({ "error": "#NUM!" }),
        "a negative number in a base is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=DECIMAL(\"12\",2)"),
        json!({ "error": "#NUM!" }),
        "and a digit that is not in the base is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROMAN(4000)"),
        json!({ "error": "#VALUE!" }),
        "and a Roman number beyond the bound is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ARABIC(\"ABC\")"),
        json!({ "error": "#VALUE!" }),
        "and letters that are not numerals are an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=REPT(\"ab\",-1)"),
        json!({ "error": "#VALUE!" }),
        "and a text repeated a negative number of times is an error"
    );
}
