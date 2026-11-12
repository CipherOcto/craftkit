//! The **numbering format** of a number.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// The **form** of a number, as the command gives it.
fn form_of(session: &mut Session, value: f64, format: &str) -> String {
    let outcome = session
        .execute("prose.number.format", &json!({ "value": value, "format": format }))
        .expect("the form is read");
    outcome.data["text"].as_str().expect("the form is a text").to_string()
}

/// A number is written in **roman** numerals.
#[test]
fn a_number_is_written_in_roman_numerals() {
    let mut session = session();
    assert_eq!(form_of(&mut session, 4.0, "roman"), "iv", "four is four");
    assert_eq!(form_of(&mut session, 9.0, "roman"), "ix", "nine is nine");
    assert_eq!(form_of(&mut session, 1994.0, "roman"), "mcmxciv", "and so is nineteen ninety four");
}

/// A number is written in **letters**.
#[test]
fn a_number_is_written_in_letters() {
    let mut session = session();
    assert_eq!(form_of(&mut session, 1.0, "letters"), "a", "one is a");
    assert_eq!(form_of(&mut session, 27.0, "letters"), "aa", "and twenty seven is aa");
}

/// A number is written in **words**.
#[test]
fn a_number_is_written_in_words() {
    let mut session = session();
    assert_eq!(form_of(&mut session, 42.0, "cardinal"), "forty-two", "forty two");
    assert_eq!(form_of(&mut session, 42.0, "ordinal"), "forty-second", "and its place");
}

/// A number that is **not whole** is refused.
#[test]
fn a_number_that_is_not_whole_is_refused() {
    let mut session = session();
    let outcome = session.execute("prose.number.format", &json!({ "value": 1.5, "format": "roman" }));
    assert!(outcome.is_err(), "one and a half is not a whole number");
}

/// A **negative** number has no form here.
#[test]
fn a_negative_number_is_refused() {
    let mut session = session();
    let outcome = session.execute("prose.number.format", &json!({ "value": -1.0, "format": "roman" }));
    assert!(outcome.is_err(), "there is no negative roman numeral");
}

/// A number that is **too large** for the form is refused.
#[test]
fn a_number_that_is_too_large_is_refused() {
    let mut session = session();
    let outcome = session.execute("prose.number.format", &json!({ "value": 4000.0, "format": "roman" }));
    assert!(outcome.is_err(), "four thousand has no roman form here");
}

/// A **name** this tree does not know is refused.
#[test]
fn a_name_that_is_not_known_is_refused() {
    let mut session = session();
    let outcome = session.execute("prose.number.format", &json!({ "value": 1.0, "format": "mayan" }));
    assert!(outcome.is_err(), "there is no such form");
}
