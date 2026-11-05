//! The **edit decision list** of a sequence.

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

/// A **bin**, an **item** in it, a **track** in the item.
fn a_timeline(session: &mut Session, name: &str) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": name, "bin": bin, "item": 1, "duration": 600 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64, source_in: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": source_in,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The **list** of a sequence.
fn edl_of(session: &mut Session, sequence: u64) -> String {
    let outcome =
        session.execute("film.export.edl", &json!({ "sequence": sequence })).expect("the list is written");
    outcome.data["edl"].as_str().expect("the list is a text").to_string()
}

/// **The list says what was cut, where it came from and where it
/// sits.**
#[test]
fn the_list_says_what_was_cut() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let list = edl_of(&mut session, sequence);
    let lines: Vec<&str> = list.lines().collect();
    assert_eq!(lines[0], "TITLE: the media");
    assert_eq!(lines[1], "FCM: NON-DROP FRAME");
    assert_eq!(lines[2], "");
    let event = lines[3];
    assert!(event.starts_with("001  "), "the event is numbered: {event}");
    assert!(event.contains(" V "), "the channel is the video one: {event}");
    assert!(event.contains(" C "), "the edit is a cut: {event}");
    assert!(
        event.contains("00:00:00:00 00:00:05:00 00:00:00:00 00:00:05:00"),
        "the four timecodes are the source and the record: {event}"
    );
}

/// **The reel is the name of the item, in eight characters.**
#[test]
fn the_reel_is_the_name_of_the_item() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "a long name for a reel");
    a_clip(&mut session, track, sequence, 0, 25, 0);
    let list = edl_of(&mut session, sequence);
    let event = list.lines().nth(3).expect("there is an event");
    assert!(event.contains("A_LONG_N"), "the reel is the name in eight characters: {event}");
}

/// **The title may be given.**
#[test]
fn the_title_may_be_given() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 25, 0);
    let outcome = session
        .execute("film.export.edl", &json!({ "sequence": sequence, "title": "THE CUT" }))
        .expect("the list is written");
    let list = outcome.data["edl"].as_str().expect("the list is a text");
    assert!(list.starts_with("TITLE: THE CUT\n"), "the title is the one that was given: {list}");
}

/// **The timecodes are counted at the rate of the sequence.**
#[test]
fn the_timecodes_are_counted_at_the_rate() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "30" }))
        .expect("the rate is set");
    a_clip(&mut session, track, sequence, 90, 30, 0);
    let list = edl_of(&mut session, sequence);
    let event = list.lines().nth(3).expect("there is an event");
    assert!(event.contains("00:00:00:00 00:00:01:00"), "the source is a second: {event}");
    assert!(event.contains("00:00:03:00 00:00:04:00"), "the record is a second: {event}");
}

/// **Two events are numbered one after the other.**
#[test]
fn two_events_are_numbered_one_after_the_other() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 25, 0);
    a_clip(&mut session, track, sequence, 25, 25, 25);
    let outcome =
        session.execute("film.export.edl", &json!({ "sequence": sequence })).expect("the list is written");
    assert_eq!(outcome.data["events"], 2);
    let list = outcome.data["edl"].as_str().expect("the list is a text");
    assert!(list.contains("001  "), "the first event is numbered one: {list}");
    assert!(list.contains("002  "), "the second event is numbered two: {list}");
}

/// **An audio track is on an audio channel.**
#[test]
fn an_audio_track_is_on_an_audio_channel() {
    let mut session = session();
    let (sequence, _) = a_timeline(&mut session, "the media");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": sequence, "kind": "audio" }))
        .expect("the track is added");
    let sound = outcome.data["id"].as_u64().expect("the track has an id");
    a_clip(&mut session, sound, sequence, 0, 25, 0);
    let list = edl_of(&mut session, sequence);
    let event = list.lines().nth(3).expect("there is an event");
    assert!(event.contains(" A "), "the channel is the audio one: {event}");
}

/// **A transition is a dissolve, and this list does not write
/// dissolves.**
#[test]
fn a_transition_is_refused() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let first = a_clip(&mut session, track, sequence, 0, 25, 0);
    let second = a_clip(&mut session, track, sequence, 25, 25, 25);
    session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 5 }),
        )
        .expect("the transition is added");
    let failure = session
        .execute("film.export.edl", &json!({ "sequence": sequence }))
        .expect_err("the list is refused");
    let message = failure.to_string();
    assert!(message.contains("does not write dissolves"), "the refusal says why: {message}");
}

/// **A node that is not a sequence is refused.**
#[test]
fn a_node_that_is_not_a_sequence_is_refused() {
    let mut session = session();
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let failure =
        session.execute("film.export.edl", &json!({ "sequence": bin })).expect_err("the list is refused");
    let message = failure.to_string();
    assert!(message.contains("a sequence is an item"), "the refusal says why: {message}");
}
