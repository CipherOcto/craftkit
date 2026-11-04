//! The **complex numbers**: a pair of numbers, written as text.

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
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **text** a formula computes.
fn text_of(session: &mut Session, sheet: u64, cell: &str, text: &str) -> String {
    match computed(session, sheet, cell, text) {
        Json::String(text) => text,
        other => panic!("expected a text and got {other:?}"),
    }
}

/// The two **parts** of a complex number written as text.
fn parts(text: &str) -> (f64, f64) {
    let body = text.trim_end_matches(['i', 'j']);
    let bytes = body.as_bytes();
    let mut split = None;
    for index in (1..bytes.len()).rev() {
        let character = bytes[index];
        if (character == b'+' || character == b'-') && bytes[index - 1] != b'e' && bytes[index - 1] != b'E' {
            split = Some(index);
            break;
        }
    }
    match split {
        Some(index) => {
            let real: f64 = body[..index].parse().expect("the real part");
            let imaginary: f64 = body[index..].parse().expect("the imaginary part");
            (real, imaginary)
        }
        None => {
            let real: f64 = body.parse().expect("the real part");
            (real, 0.0)
        }
    }
}

/// Two numbers are **near** each other.
fn near(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

/// The complex numbers are **written** and **read**.
#[test]
fn the_complex_numbers_are_written_and_read() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(text_of(&mut session, sheet, "A1", "=COMPLEX(3,4)"), "3+4i", "a number");
    assert_eq!(text_of(&mut session, sheet, "A2", "=COMPLEX(3,4,\"j\")"), "3+4j", "and its suffix");
    assert_eq!(text_of(&mut session, sheet, "A3", "=COMPLEX(0,1)"), "i", "and no real part");
    assert_eq!(text_of(&mut session, sheet, "A4", "=COMPLEX(0,-1)"), "-i", "and a part of minus one");
    assert_eq!(text_of(&mut session, sheet, "A5", "=COMPLEX(1.5,-2)"), "1.5-2i", "and a fraction");
    assert_eq!(text_of(&mut session, sheet, "A6", "=COMPLEX(3,0)"), "3", "and no imaginary part");
    assert_eq!(
        computed(&mut session, sheet, "A7", "=COMPLEX(1,1,\"k\")"),
        json!({ "error": "#VALUE!" }),
        "and a suffix that is no suffix is an error"
    );
    assert_eq!(computed(&mut session, sheet, "A8", "=IMABS(\"5+12i\")"), json!(13.0), "the modulus");
    assert_eq!(computed(&mut session, sheet, "A9", "=IMREAL(\"6-9i\")"), json!(6.0), "the real part");
    assert_eq!(
        computed(&mut session, sheet, "A10", "=IMAGINARY(\"3+4i\")"),
        json!(4.0),
        "the imaginary part"
    );
    assert_eq!(
        computed(&mut session, sheet, "A11", "=IMAGINARY(\"-j\")"),
        json!(-1.0),
        "and the imaginary part of a lonely i"
    );
    assert_eq!(
        computed(&mut session, sheet, "A12", "=IMREAL(7)"),
        json!(7.0),
        "and the real part of a plain number"
    );
    assert_eq!(
        computed(&mut session, sheet, "A13", "=ROUND(IMARGUMENT(\"3+4i\"),12)"),
        json!(0.927295218002),
        "the angle"
    );
    assert_eq!(
        computed(&mut session, sheet, "A14", "=IMARGUMENT(\"0\")"),
        json!({ "error": "#DIV/0!" }),
        "and nothing has no angle"
    );
    assert_eq!(
        computed(&mut session, sheet, "A15", "=IMABS(\"abc\")"),
        json!({ "error": "#NUM!" }),
        "and a text that holds no number is an error"
    );
}

/// The complex numbers are **added**, **multiplied** and **divided**.
#[test]
fn the_complex_numbers_are_added_and_divided() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(text_of(&mut session, sheet, "A1", "=IMCONJUGATE(\"3+4i\")"), "3-4i", "the conjugate");
    assert_eq!(
        text_of(&mut session, sheet, "A2", "=IMDIV(\"-238+240i\",\"10+24i\")"),
        "5+12i",
        "the quotient"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=IMDIV(\"1+i\",0)"),
        json!({ "error": "#NUM!" }),
        "and nothing divides nothing"
    );
    assert_eq!(text_of(&mut session, sheet, "A4", "=IMPRODUCT(\"3+4i\",\"5-3i\")"), "27+11i", "the product");
    assert_eq!(
        text_of(&mut session, sheet, "A5", "=IMPRODUCT(\"1+2i\",30)"),
        "30+60i",
        "and of a plain number"
    );
    assert_eq!(text_of(&mut session, sheet, "A6", "=IMSUM(\"3+4i\",\"5-3i\")"), "8+i", "the sum");
    assert_eq!(text_of(&mut session, sheet, "A7", "=IMSUB(\"13+4i\",\"5+3i\")"), "8+i", "and the difference");
    assert_eq!(
        computed(&mut session, sheet, "A8", "=IMSUM(\"1+i\",\"1+j\")"),
        json!({ "error": "#VALUE!" }),
        "and two suffixes that differ may not be put together"
    );
}

/// The **functions** of a complex number.
#[test]
fn the_functions_of_a_complex_number_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let exponential = parts(&text_of(&mut session, sheet, "A1", "=IMEXP(\"1+i\")"));
    assert!(near(exponential.0, 1.4686939399158852), "the exponential");
    assert!(near(exponential.1, 2.287355287178842), "and its imaginary part");
    let logarithm = parts(&text_of(&mut session, sheet, "A2", "=IMLN(\"3+4i\")"));
    assert!(near(logarithm.0, 1.6094379124341003), "the logarithm");
    assert!(near(logarithm.1, 0.9272952180016122), "and its imaginary part");
    let root = parts(&text_of(&mut session, sheet, "A3", "=IMSQRT(\"1+i\")"));
    assert!(near(root.0, 1.0986841134678099), "the root");
    assert!(near(root.1, 0.4550898605622273), "and its imaginary part");
    assert_eq!(text_of(&mut session, sheet, "A4", "=IMSQRT(\"-4\")"), "2i", "and the root of a negative");
    assert_eq!(text_of(&mut session, sheet, "A5", "=IMSQRT(\"-2i\")"), "1-i", "and of a lonely i");
    assert_eq!(text_of(&mut session, sheet, "A6", "=IMPOWER(\"2+3i\",3)"), "-46+9i", "and a whole power");
    let sine = parts(&text_of(&mut session, sheet, "A7", "=IMSIN(\"3+4i\")"));
    assert!(near(sine.0, 3.853738037919377), "the sine");
    assert!(near(sine.1, -27.01681325800393), "and its imaginary part");
    let tangent = parts(&text_of(&mut session, sheet, "A8", "=IMTAN(\"4+3i\")"));
    assert!(near(tangent.0, 0.00490825806749606), "the tangent");
    assert!(near(tangent.1, 1.000709536067233), "and its imaginary part");
    assert_eq!(text_of(&mut session, sheet, "A9", "=IMSECH(\"0\")"), "1", "and the secant of nothing");
    assert_eq!(
        computed(&mut session, sheet, "A10", "=IMCSCH(\"0\")"),
        json!({ "error": "#NUM!" }),
        "and the cosecant of nothing is an error"
    );
}
