//! The **rate** of a sequence.
//!
//! A frame is a number, and the rate of a sequence says how many frames
//! there are in a second.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **sequence**: an item with a track and a clip in it.
fn a_sequence(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let sequence = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "V1", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    session
        .execute("film.clip.add", &json!({ "track": track, "item": sequence, "start": 0, "duration": 50 }))
        .expect("the clip is added");
    (bin, sequence)
}

/// The **answer** of a read.
fn read(session: &mut Session, sequence: u64) -> Json {
    let outcome = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    outcome.data.clone()
}

/// A rate may be **set** and **read**.
#[test]
fn a_rate_may_be_set_and_read() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    let answer = read(&mut session, sequence);
    assert_eq!(answer["rate"]["num"], json!(25), "the rate is twenty-five");
    assert_eq!(answer["rate"]["den"], json!(1), "over one");
}

/// The tree's **rational** rates may be written as the fraction behind
/// them, and under the name the tree shows.
#[test]
fn the_tree_rates_may_be_written_two_ways() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "30000/1001" }))
        .expect("the rate is set");
    let answer = read(&mut session, sequence);
    assert_eq!(answer["rate"]["num"], json!(30000), "the fraction is the rate");
    assert_eq!(answer["rate"]["den"], json!(1001), "and its denominator");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "29.97" }))
        .expect("the rate is set again");
    let answer = read(&mut session, sequence);
    assert_eq!(answer["rate"]["num"], json!(30000), "the name says the same rate");
    assert_eq!(answer["rate"]["den"], json!(1001), "and the same denominator");
}

/// The length of a sequence in **seconds** is derived: the frames and
/// the rate.
#[test]
fn the_length_in_seconds_is_derived() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    let answer = read(&mut session, sequence);
    assert_eq!(answer["frames"], json!(50), "the sequence is fifty frames long");
    assert_eq!(answer["seconds"], json!(2.0), "and two seconds long");
}

/// A sequence with **no rate** has no length in seconds, and that is
/// said as nothing.
#[test]
fn a_sequence_with_no_rate_has_no_length_in_seconds() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    let answer = read(&mut session, sequence);
    assert_eq!(answer["rate"], Json::Null, "there is no rate");
    assert_eq!(answer["frames"], json!(50), "but the frames are said");
    assert_eq!(answer["seconds"], Json::Null, "and there is no length in seconds");
}

/// A rate that is **not a rate** is refused.
#[test]
fn a_rate_that_is_not_a_rate_is_refused() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    let error = session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "quickly" }))
        .expect_err("the rate is refused");
    assert!(error.to_string().contains("is not a rate"), "the refusal says why: {error}");
}

/// A rate that is **not positive** is refused.
#[test]
fn a_rate_that_is_not_positive_is_refused() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    let error = session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "0" }))
        .expect_err("the rate is refused");
    assert!(error.to_string().contains("positive"), "the refusal says why: {error}");
}

/// A sequence that is **not there** is refused, and so is a node that
/// is not an item.
#[test]
fn a_sequence_that_is_not_there_is_refused() {
    let mut session = session();
    let (_bin, _sequence) = a_sequence(&mut session);
    let error = session
        .execute("film.sequence.rate.get", &json!({ "sequence": 999 }))
        .expect_err("the sequence is refused");
    assert!(error.to_string().contains("there is no sequence"), "the refusal says why: {error}");
    let outcome =
        session.execute("film.bin.add", &json!({ "name": "another bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let error = session
        .execute("film.sequence.rate.get", &json!({ "sequence": bin }))
        .expect_err("the bin is not a sequence");
    assert!(error.to_string().contains("a sequence is an item"), "the refusal says why: {error}");
}
