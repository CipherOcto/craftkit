//! The **generated** items: the clips that play something the project
//! makes and not a file it reads.
//!
//! The tree's generators: the black video, the transparent video, the
//! bars and the tone, the counting leader and the colour matte.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **bin** to put the items in.
fn a_bin(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    outcome.data["id"].as_u64().expect("the bin has an id")
}

/// The **generator** the tree's own way: five seconds at the tree's
/// own rate.
fn a_generator(session: &mut Session, bin: u64, kind: &str) -> Result<u64, String> {
    let outcome =
        session.execute("film.item.generator", &json!({ "name": kind, "bin": bin, "item": 7, "kind": kind }));
    match outcome {
        Ok(outcome) => Ok(outcome.data["id"].as_u64().expect("the item has an id")),
        Err(error) => Err(error.to_string()),
    }
}

/// The **item** the generator made.
fn the_item(session: &mut Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("film.item.get", &json!({ "item": id })).expect("the item is read");
    outcome.data.clone()
}

/// A generator is **five seconds** long, at the tree's own rate.
#[test]
fn a_generator_is_five_seconds_long() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let id = a_generator(&mut session, bin, "Black Video").expect("the generator is added");
    let item = the_item(&mut session, id);
    // Five seconds at 23.976 frames a second.
    assert_eq!(item["duration"].as_u64(), Some(120), "the generator is five seconds long");
    assert_eq!(item["generator"]["kind"].as_str(), Some("Black Video"), "and it says what it is");
}

/// The **bars and tone** are ten seconds: the tree's own rule.
#[test]
fn the_bars_and_tone_are_ten_seconds() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let id = a_generator(&mut session, bin, "Bars and Tone").expect("the generator is added");
    let item = the_item(&mut session, id);
    assert_eq!(item["duration"].as_u64(), Some(240), "the bars and the tone are ten seconds long");
}

/// The **rate** may be named, and the seconds are at that rate.
#[test]
fn the_rate_may_be_named() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let outcome = session.execute(
        "film.item.generator",
        &json!({ "name": "black", "bin": bin, "item": 7, "kind": "Black Video", "rate": "25" }),
    );
    let id = outcome.expect("the generator is added").data["id"].as_u64().expect("the item has an id");
    let item = the_item(&mut session, id);
    assert_eq!(item["duration"].as_u64(), Some(125), "five seconds at twenty-five a second");
}

/// The **matte** takes a colour, and its own colour is the tree's dark
/// grey.
#[test]
fn the_matte_takes_a_colour() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let id = a_generator(&mut session, bin, "Color Matte").expect("the generator is added");
    let item = the_item(&mut session, id);
    assert_eq!(item["generator"]["colour"].as_str(), Some("#1a1a1a"), "the matte is the tree's dark grey");
    let outcome = session.execute(
        "film.item.generator",
        &json!({ "name": "matte", "bin": bin, "item": 8, "kind": "Color Matte", "colour": "#ff0000" }),
    );
    let id = outcome.expect("the generator is added").data["id"].as_u64().expect("the item has an id");
    let item = the_item(&mut session, id);
    assert_eq!(item["generator"]["colour"].as_str(), Some("#ff0000"), "and it takes the colour it is given");
}

/// A colour on a generator that is **not** a matte is refused.
#[test]
fn a_colour_on_a_generator_that_is_not_a_matte_is_refused() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let outcome = session.execute(
        "film.item.generator",
        &json!({ "name": "black", "bin": bin, "item": 9, "kind": "Black Video", "colour": "#ff0000" }),
    );
    let error = outcome.expect_err("the colour is refused");
    assert!(error.to_string().contains("the matte's own"), "the refusal says why: {error}");
}

/// A kind that is **not** a generator is refused, and the refusal names
/// the generators there are.
#[test]
fn a_kind_that_is_not_a_generator_is_refused() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let error = a_generator(&mut session, bin, "Purple").expect_err("the kind is refused");
    assert!(error.contains("is not a generator"), "the refusal says so: {error}");
    assert!(error.contains("Black Video"), "and names the generators there are: {error}");
}

/// The name of the matte may be written the way this tree writes it.
#[test]
fn the_name_of_the_matte_may_be_written_either_way() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let id = a_generator(&mut session, bin, "Colour Matte").expect("the generator is added");
    let item = the_item(&mut session, id);
    assert_eq!(item["generator"]["kind"].as_str(), Some("Color Matte"), "it is the tree's own name");
}
