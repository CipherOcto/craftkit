//! The **bases** and the **bits**.
//!
//! A formula may ask about a number written in another base, or about
//! the bits of a number.

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

/// From the **binary** to the others.
#[test]
fn the_binary_is_read_in_the_other_bases() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=BIN2DEC(1100100)"), json!(100.0), "to ten");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=BIN2DEC(1111111111)"),
        json!(-1.0),
        "and a full number with the top bit set is negative"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=BIN2HEX(11111011,4)"), json!("00FB"), "to sixteen");
    assert_eq!(computed(&mut session, sheet, "A4", "=BIN2HEX(1110)"), json!("E"), "and without places");
    assert_eq!(computed(&mut session, sheet, "A5", "=BIN2OCT(1001,3)"), json!("011"), "to eight");
    assert_eq!(computed(&mut session, sheet, "A6", "=BIN2OCT(1111111111)"), json!("7777777777"), "and full");
}

/// From the **decimal** to the others.
#[test]
fn the_decimal_is_written_in_the_other_bases() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=DEC2BIN(9,4)"), json!("1001"), "to two");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=DEC2BIN(-100)"),
        json!("1110011100"),
        "and a negative number is written in twos complement"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=DEC2HEX(100,4)"), json!("0064"), "to sixteen");
    assert_eq!(computed(&mut session, sheet, "A4", "=DEC2HEX(-54)"), json!("FFFFFFFFCA"), "and negative");
    assert_eq!(computed(&mut session, sheet, "A5", "=DEC2OCT(58,3)"), json!("072"), "to eight");
    assert_eq!(computed(&mut session, sheet, "A6", "=DEC2OCT(-100)"), json!("7777777634"), "and negative");
    assert_eq!(
        computed(&mut session, sheet, "A7", "=DEC2BIN(9.7)"),
        json!("1001"),
        "and the fraction is cut"
    );
}

/// From the **hexadecimal** and the **octal**.
#[test]
fn the_hexadecimal_and_the_octal_are_read() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=HEX2BIN(\"F\",8)"), json!("00001111"), "to two");
    assert_eq!(computed(&mut session, sheet, "A2", "=HEX2DEC(\"A5\")"), json!(165.0), "to ten");
    assert_eq!(
        computed(&mut session, sheet, "A3", "=HEX2DEC(\"FFFFFFFF5B\")"),
        json!(-165.0),
        "and a full number with the top bit set is negative"
    );
    assert_eq!(computed(&mut session, sheet, "A4", "=HEX2OCT(\"F\",3)"), json!("017"), "to eight");
    assert_eq!(computed(&mut session, sheet, "A5", "=OCT2DEC(54)"), json!(44.0), "and from eight");
    assert_eq!(computed(&mut session, sheet, "A6", "=OCT2HEX(100,4)"), json!("0040"), "to sixteen");
    assert_eq!(
        computed(&mut session, sheet, "A7", "=OCT2DEC(7777777533)"),
        json!(-165.0),
        "and a full number with the top bit set is negative"
    );
}

/// The **bits** of a number.
#[test]
fn the_bits_of_a_number_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=BITAND(13,25)"), json!(9.0), "the and");
    assert_eq!(computed(&mut session, sheet, "A2", "=BITOR(23,10)"), json!(31.0), "the or");
    assert_eq!(computed(&mut session, sheet, "A3", "=BITXOR(5,3)"), json!(6.0), "the exclusive or");
    assert_eq!(computed(&mut session, sheet, "A4", "=BITLSHIFT(4,2)"), json!(16.0), "a shift to the left");
    assert_eq!(computed(&mut session, sheet, "A5", "=BITRSHIFT(13,2)"), json!(3.0), "and to the right");
    assert_eq!(
        computed(&mut session, sheet, "A6", "=BITRSHIFT(13,-2)"),
        json!(52.0),
        "and a shift to the right by a negative number is a shift to the left"
    );
}

/// Whether two numbers are the **same**, and whether one has reached a
/// **step**.
#[test]
fn the_delta_and_the_step_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=DELTA(5,5)"), json!(1.0), "the same");
    assert_eq!(computed(&mut session, sheet, "A2", "=DELTA(5,4)"), json!(0.0), "and not");
    assert_eq!(computed(&mut session, sheet, "A3", "=GESTEP(5,4)"), json!(1.0), "the step reached");
    assert_eq!(computed(&mut session, sheet, "A4", "=GESTEP(-4)"), json!(0.0), "and not");
}

/// The tree's **errors**: a number that is not written in the base, a
/// number too long, and a bit that is not a bit.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=HEX2DEC(\"GG\")"),
        json!({ "error": "#NUM!" }),
        "a number that is not written in the base is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=BIN2DEC(\"11111111111\")"),
        json!({ "error": "#NUM!" }),
        "and a number too long for the base is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=DEC2BIN(512)"),
        json!({ "error": "#NUM!" }),
        "and a number too big for the base is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=BITAND(-1,5)"),
        json!({ "error": "#NUM!" }),
        "and a bit that is not a bit is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=BITLSHIFT(1,54)"),
        json!({ "error": "#NUM!" }),
        "and a shift beyond the width of a number is an error"
    );
}
