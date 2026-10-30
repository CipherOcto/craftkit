//! The numeric battery: the **typed value** arrives, as a plugin and a
//! library.
//!
//! What is proved here: the core has no unit, the typed value is not a
//! command parameter, the other units are derived, and the arithmetic is
//! the library's and not the core's.

use craftkit_sdk::Ecosystem;
#[cfg(feature = "numeric")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "numeric")]
#[test]
fn the_numeric_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("numeric.length.add").is_some(), "the battery is in");
    assert!(ecosystem.kinds.get("numeric.length").is_some());
    assert!(ecosystem.evaluators.get("numeric.measures").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "numeric"))]
#[test]
fn the_numeric_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("numeric.length.add").is_none(), "the battery is out");
}

/// A length is stored in **millimetres**, whatever unit it was given in.
#[cfg(feature = "numeric")]
#[test]
fn a_length_is_stored_in_millimetres() {
    let mut session = ecosystem().session();
    session
        .execute("numeric.length.add", &json!({ "value": 2.5, "unit": "cm" }))
        .expect("the length is added");
    let node = session.document().node(1).expect("the length is there");
    assert_eq!(node.kind, "numeric.length");
    assert_eq!(node.data["millimetres"], json!(25.0));
}

/// The unit is the **plugin's** business, and the core never sees it:
/// the payload is a number and nothing else.
#[cfg(feature = "numeric")]
#[test]
fn the_core_only_sees_a_number() {
    let mut session = ecosystem().session();
    session.execute("numeric.length.add", &json!({ "value": 10.0 })).expect("the length is added");
    let node = session.document().node(1).expect("the length is there");
    assert_eq!(node.data["millimetres"], json!(10.0));
    // There is no unit anywhere in the payload: the unit is what the
    // plugin says the number means.
    assert!(node.data.get("unit").is_none(), "the payload carries no unit");
}

/// The **other units are derived**: the millimetres are stored once and
/// the rest is worked out every time.
#[cfg(feature = "numeric")]
#[test]
fn the_other_units_are_derived() {
    let mut session = ecosystem().session();
    session.execute("numeric.length.add", &json!({ "value": 25.4 })).expect("the length is added");
    let plan = session.evaluate();
    let measures: Vec<_> = plan.items.iter().filter(|item| item.kind == "numeric.measures").collect();
    assert_eq!(measures[0].value["millimetres"], json!(25.4));
    assert_eq!(measures[0].value["inches"], json!(1.0));
    assert_eq!(measures[0].value["points"], json!(72.0));
}

/// An angle is stored in **radians**, and its degrees are derived.
#[cfg(feature = "numeric")]
#[test]
fn an_angle_is_stored_in_radians() {
    let mut session = ecosystem().session();
    session
        .execute("numeric.angle.add", &json!({ "value": 180.0, "unit": "deg" }))
        .expect("the angle is added");
    let node = session.document().node(1).expect("the angle is there");
    assert!((node.data["radians"].as_f64().expect("a number") - std::f64::consts::PI).abs() < 1e-9);
    let plan = session.evaluate();
    let measures: Vec<_> = plan.items.iter().filter(|item| item.kind == "numeric.measures").collect();
    assert!((measures[0].value["degrees"].as_f64().expect("a number") - 180.0).abs() < 1e-9);
}

/// A **box** is two lengths, and the area is the library's arithmetic.
#[cfg(feature = "numeric")]
#[test]
fn the_area_of_a_box_is_derived() {
    let mut session = ecosystem().session();
    session
        .execute("numeric.box.add", &json!({ "width": 210.0, "height": 297.0 }))
        .expect("the box is added");
    let plan = session.evaluate();
    let measures: Vec<_> = plan.items.iter().filter(|item| item.kind == "numeric.measures").collect();
    assert_eq!(measures[0].value["area_mm2"], json!(62370.0));
}

/// **Scaling** a length is the library's arithmetic, written back: the
/// core cannot tell a length from a factor.
#[cfg(feature = "numeric")]
#[test]
fn scaling_a_length_is_the_librarys_arithmetic() {
    let mut session = ecosystem().session();
    session.execute("numeric.length.add", &json!({ "value": 10.0 })).expect("the length is added");
    session
        .execute("numeric.length.scale", &json!({ "length": 1, "factor": 2.5 }))
        .expect("the length is scaled");
    let node = session.document().node(1).expect("the length is there");
    assert_eq!(node.data["millimetres"], json!(25.0));
}

/// The **total** of some lengths is the library's arithmetic too.
#[cfg(feature = "numeric")]
#[test]
fn the_total_of_lengths_is_derived() {
    let mut session = ecosystem().session();
    session
        .execute("numeric.length.total", &json!({ "lengths": [1.0, 2.0, 39.5] }))
        .expect("the total is added");
    let node = session.document().node(1).expect("the total is there");
    assert_eq!(node.data["millimetres"], json!(42.5));
}

/// A unit that is not a unit is refused.
#[cfg(feature = "numeric")]
#[test]
fn a_unit_that_is_not_a_unit_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("numeric.length.add", &json!({ "value": 1.0, "unit": "furlongs" }))
        .expect_err("that is not a unit");
    assert!(error.to_string().contains("not a unit"), "{error}");
}

/// A number that is not a number is refused: the input is hostile.
#[cfg(feature = "numeric")]
#[test]
fn a_value_that_is_not_a_number_is_refused() {
    let mut session = ecosystem().session();
    let error =
        session.execute("numeric.length.add", &json!({ "value": "ten" })).expect_err("that is not a number");
    assert!(error.to_string().contains("number"), "{error}");
}

/// A length that is too long is refused: the library holds a range.
#[cfg(feature = "numeric")]
#[test]
fn a_length_that_is_too_long_is_refused() {
    let mut session = ecosystem().session();
    let error =
        session.execute("numeric.length.add", &json!({ "value": 1e12 })).expect_err("that is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// The lengths of a total are a list, and a list it must be.
#[cfg(feature = "numeric")]
#[test]
fn lengths_that_are_not_a_list_are_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("numeric.length.total", &json!({ "lengths": "ten" }))
        .expect_err("that is not a list");
    assert!(error.to_string().contains("a list"), "{error}");
}

/// A hundred thousand lengths is refused: the count is capped.
#[cfg(feature = "numeric")]
#[test]
fn a_hundred_thousand_lengths_are_refused() {
    let mut session = ecosystem().session();
    let lengths: Vec<f64> = (0..100_001).map(|_| 1.0).collect();
    let error = session
        .execute("numeric.length.total", &json!({ "lengths": lengths }))
        .expect_err("that is too many");
    assert!(error.to_string().contains("hundred thousand"), "{error}");
}

/// The **library** is a library: the application names the **plugin**
/// and not the library, so the linker does not wire the library in.
///
/// The library contributes nothing to the registry — it has no command,
/// no kind and no panel — and the application does not name it, which
/// is what keeps it a library rather than a plugin.
#[test]
fn the_application_names_the_plugin_and_not_the_library() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).expect("the manifest is there");
    assert!(text.contains("craftkit-plugin-numeric"), "the plugin is named");
    for library in ["craftkit-numeric", "craftkit-vector-geom", "craftkit-sound-time"] {
        assert!(!text.contains(library), "the application does not name the library {library}");
    }
}
