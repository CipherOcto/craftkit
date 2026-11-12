//! The **arithmetic** of a formula.
//!
//! A formula may do arithmetic on a number, and the rules are the
//! tree's own.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The value of a formula, as it is computed.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    formula(session, sheet, cell, text);
    value_of(session, sheet, cell)
}

/// The rounding is done on the number written with **fifteen digits**,
/// so that a half is a half and not a hair less.
#[test]
fn a_half_rounds_away_from_zero() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ROUND(2.675,2)"), json!(2.68), "two and a bit");
    assert_eq!(computed(&mut session, sheet, "A2", "=ROUND(2.345,2)"), json!(2.35), "and another");
    assert_eq!(computed(&mut session, sheet, "A3", "=ROUND(-2.5,0)"), json!(-3.0), "and away from zero");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(1234.5678,-2)"),
        json!(1200.0),
        "and to the hundreds"
    );
}

/// `ROUNDUP` rounds away from zero and `ROUNDDOWN` toward zero, and
/// `TRUNC` is the same as `ROUNDDOWN`.
#[test]
fn rounding_up_and_down_keep_their_way() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ROUNDUP(12.3456,3)"), json!(12.346), "up");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUNDUP(-12.3456,3)"),
        json!(-12.346),
        "and away from zero"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=ROUNDDOWN(12.3456,3)"), json!(12.345), "down");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUNDDOWN(-12.3456,3)"),
        json!(-12.345),
        "and toward zero"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=TRUNC(12.3456,3)"),
        json!(12.345),
        "and the cut is the same"
    );
    assert_eq!(computed(&mut session, sheet, "A6", "=INT(-3.5)"), json!(-4.0), "and the whole is the floor");
}

/// A number may be rounded to a **multiple** of another.
#[test]
fn a_number_may_be_rounded_to_a_multiple() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=MROUND(10,3)"), json!(9.0), "the nearest multiple");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=CEILING(4.42,0.05)"),
        json!(4.45),
        "the next multiple up"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=FLOOR(4.42,0.05)"), json!(4.4), "and down");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=MROUND(10,-3)"),
        json!({ "error": "#NUM!" }),
        "and a multiple of the other sign is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=FLOOR(5,0)"),
        json!({ "error": "#DIV/0!" }),
        "and a multiple of nothing is an error"
    );
}

/// The rest of a division keeps the **sign of the divisor**.
#[test]
fn the_rest_of_a_division_keeps_the_sign_of_the_divisor() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=MOD(-3,2)"), json!(1.0), "the rest is one");
    assert_eq!(computed(&mut session, sheet, "A2", "=MOD(3,2)"), json!(1.0), "and one the other way");
    assert_eq!(computed(&mut session, sheet, "A3", "=QUOTIENT(7,2)"), json!(3.0), "and the whole is three");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=MOD(3,0)"),
        json!({ "error": "#DIV/0!" }),
        "and the rest of a division by nothing is an error"
    );
}

/// A number may be **rooted**, **raised** and **signed**.
#[test]
fn a_number_may_be_rooted_raised_and_signed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=SQRT(9)"), json!(3.0), "the root of nine");
    assert_eq!(computed(&mut session, sheet, "A2", "=POWER(2,10)"), json!(1024.0), "the power");
    assert_eq!(computed(&mut session, sheet, "A3", "=ABS(-3)"), json!(3.0), "the magnitude");
    assert_eq!(computed(&mut session, sheet, "A4", "=SIGN(-3)"), json!(-1.0), "the sign");
    assert_eq!(
        computed(&mut session, sheet, "A5", "=SQRT(-1)"),
        json!({ "error": "#NUM!" }),
        "and the root of a negative number is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=POWER(0,-1)"),
        json!({ "error": "#DIV/0!" }),
        "and zero to a negative power is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A7", "=POWER(-2,0.5)"),
        json!({ "error": "#NUM!" }),
        "and a negative number to a half power is an error"
    );
}

/// The **logarithms** of a number.
#[test]
fn the_logarithms_of_a_number_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=LN(1)"), json!(0.0), "the natural logarithm of one");
    assert_eq!(computed(&mut session, sheet, "A2", "=LOG10(100)"), json!(2.0), "the common logarithm");
    assert_eq!(computed(&mut session, sheet, "A3", "=LOG(100)"), json!(2.0), "and the logarithm of ten");
    assert_eq!(computed(&mut session, sheet, "A4", "=LOG(8,2)"), json!(3.0), "and to another base");
    assert_eq!(
        computed(&mut session, sheet, "A5", "=LN(0)"),
        json!({ "error": "#NUM!" }),
        "and the logarithm of nothing is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=LOG(8,1)"),
        json!({ "error": "#DIV/0!" }),
        "and a base of one is an error"
    );
}

/// A set of numbers may be **multiplied** or **squared** and added up.
#[test]
fn a_set_of_numbers_may_be_multiplied_or_squared() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=PRODUCT(2,3,4)"), json!(24.0), "the product");
    assert_eq!(computed(&mut session, sheet, "A2", "=SUMSQ(3,4)"), json!(25.0), "and the sum of the squares");
}

/// The **greatest common divisor** and the **least common multiple**
/// are the whole numbers of the numbers.
#[test]
fn the_divisor_and_the_multiple_are_whole_numbers() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=GCD(12,18)"), json!(6.0), "the divisor");
    assert_eq!(computed(&mut session, sheet, "A2", "=LCM(4,6)"), json!(12.0), "and the multiple");
    assert_eq!(
        computed(&mut session, sheet, "A3", "=LCM(0,5)"),
        json!(0.0),
        "and nothing has nothing in common"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=GCD(-4,6)"),
        json!({ "error": "#NUM!" }),
        "and a negative number is not a whole number"
    );
}

/// The constant **pi** is the tree's.
#[test]
fn the_constant_pi_is_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=INT(PI())"),
        json!(3.0),
        "the constant is between three and four"
    );
    assert_eq!(computed(&mut session, sheet, "A2", "=IF(PI()>3,1,0)"), json!(1.0), "and above three");
}
