//! The **markers** of the timeline: the named places, and the way the
//! playhead goes to them.
//!
//! The tree's own `markers.recall`, `markers.next` and
//! `markers.previous`.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// A **marker** in the document.
fn a_marker(session: &mut Session, name: &str, start: i64) -> u64 {
    let outcome = session
        .execute("sound.marker.add", &json!({ "name": name, "start": start }))
        .expect("the marker is added");
    outcome.data["id"].as_u64().expect("the marker has an id")
}

/// Where the **playhead** is.
fn playhead_of(session: &Session) -> Option<i64> {
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.playhead")
        .and_then(|node| node.data.get("position").and_then(|position| position.as_i64()))
}

/// The **recall** puts the playhead at the marker.
#[test]
fn the_recall_puts_the_playhead_at_the_marker() {
    let mut session = session();
    let marker = a_marker(&mut session, "chorus", 4_800);
    let outcome =
        session.execute("sound.marker.recall", &json!({ "marker": marker })).expect("the marker is recalled");
    assert_eq!(outcome.data["at"], json!(4_800), "the answer says where the marker is");
    assert_eq!(playhead_of(&session), Some(4_800), "and the playhead is there");
}

/// The **recall** finds the marker by its name.
#[test]
fn the_recall_finds_the_marker_by_its_name() {
    let mut session = session();
    a_marker(&mut session, "chorus", 4_800);
    session.execute("sound.marker.recall", &json!({ "name": "chorus" })).expect("the marker is recalled");
    assert_eq!(playhead_of(&session), Some(4_800), "the playhead is at the marker");
}

/// The **recall** chooses the marker.
#[test]
fn the_recall_chooses_the_marker() {
    let mut session = session();
    let marker = a_marker(&mut session, "chorus", 4_800);
    session.execute("sound.marker.recall", &json!({ "marker": marker })).expect("the marker is recalled");
    assert_eq!(session.selected_ids(), vec![marker], "the marker is chosen");
}

/// A marker that is **not there** is refused by name.
#[test]
fn a_marker_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("sound.marker.recall", &json!({ "name": "chorus" }))
        .expect_err("the marker is refused");
    assert!(error.to_string().contains("there is no marker called"), "the refusal says so: {error}");
}

/// The **next** marker is the one that starts after the playhead.
#[test]
fn the_next_marker_is_the_one_that_starts_after_the_playhead() {
    let mut session = session();
    a_marker(&mut session, "one", 1_000);
    let second = a_marker(&mut session, "two", 2_000);
    a_marker(&mut session, "three", 3_000);
    session.execute("sound.playhead.set", &json!({ "position": 1_000 })).expect("the playhead is set");
    let outcome = session.execute("sound.marker.next", &json!({})).expect("the playhead steps");
    assert_eq!(outcome.data["at"], json!(2_000), "the playhead steps to the next marker");
    assert_eq!(playhead_of(&session), Some(2_000), "and it is there");
    assert_eq!(session.selected_ids(), vec![second], "and the marker is chosen");
}

/// The **previous** marker is the one that starts before the playhead.
#[test]
fn the_previous_marker_is_the_one_that_starts_before_the_playhead() {
    let mut session = session();
    a_marker(&mut session, "one", 1_000);
    a_marker(&mut session, "two", 2_000);
    a_marker(&mut session, "three", 3_000);
    session.execute("sound.playhead.set", &json!({ "position": 3_000 })).expect("the playhead is set");
    let outcome = session.execute("sound.marker.previous", &json!({})).expect("the playhead steps");
    assert_eq!(outcome.data["at"], json!(2_000), "the playhead steps back to the marker before it");
    assert_eq!(playhead_of(&session), Some(2_000), "and it is there");
}

/// A step that has **nowhere to go** says so and changes nothing.
#[test]
fn a_step_with_nowhere_to_go_says_so() {
    let mut session = session();
    a_marker(&mut session, "one", 1_000);
    session.execute("sound.playhead.set", &json!({ "position": 5_000 })).expect("the playhead is set");
    let outcome = session.execute("sound.marker.next", &json!({})).expect("the step is answered");
    assert_eq!(outcome.data["moved"], json!(false), "the answer says it did not move");
    assert_eq!(playhead_of(&session), Some(5_000), "and the playhead is where it was");
}
