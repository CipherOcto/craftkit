//! The **dissolve** of a transition, as the edit decision list writes
//! it.

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

/// The **events** of a list: the lines that are not the title and not
/// the frame count.
fn events_of(list: &str) -> Vec<&str> {
    list.lines().filter(|line| line.starts_with("0") || line.starts_with("1")).collect()
}

/// A **dissolve** is written: the cut of the outgoing clip and the
/// incoming clip with the length of the dissolve.
#[test]
fn a_dissolve_is_written() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let first = a_clip(&mut session, track, sequence, 0, 25, 0);
    let second = a_clip(&mut session, track, sequence, 25, 25, 25);
    session
        .execute(
            "film.transition.add",
            &json!({
                "track": track,
                "from": first,
                "to": second,
                "duration": 5,
                "align": "start",
            }),
        )
        .expect("the transition is added");
    let list = edl_of(&mut session, sequence);
    let events = events_of(&list);
    assert_eq!(events.len(), 3, "the outgoing clip, the cut and the dissolve: {list}");
    // The **outgoing clip** is a cut of its own.
    assert!(events[0].contains(" C "), "the first event is a cut: {events:?}");
    assert!(
        events[0].contains("00:00:00:00 00:00:01:00 00:00:00:00 00:00:01:00"),
        "it is the whole clip: {events:?}"
    );
    // The **cut** is a line of no length at the point the dissolve
    // begins.
    assert!(events[1].contains(" C "), "the second event is a cut: {events:?}");
    assert!(
        events[1].contains("00:00:01:00 00:00:01:00 00:00:01:00 00:00:01:00"),
        "the cut is at the cut: {events:?}"
    );
    // The **dissolve** is the incoming clip and the length of it.
    assert!(events[2].contains(" D "), "the third event is a dissolve: {events:?}");
    assert!(events[2].contains("D    005"), "the dissolve is five frames long: {events:?}");
    assert!(
        events[2].contains("00:00:01:00 00:00:02:00 00:00:01:00 00:00:02:00"),
        "the incoming clip is in it: {events:?}"
    );
}

/// **The outgoing clip's own event ends at the cut.**
#[test]
fn the_outgoing_clip_ends_at_the_cut() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let first = a_clip(&mut session, track, sequence, 0, 50, 0);
    let second = a_clip(&mut session, track, sequence, 50, 25, 50);
    session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 5, "align": "start" }),
        )
        .expect("the transition is added");
    let list = edl_of(&mut session, sequence);
    let events = events_of(&list);
    assert!(
        events[0].contains("00:00:00:00 00:00:02:00 00:00:00:00 00:00:02:00"),
        "the outgoing clip ends at the cut: {events:?}"
    );
}

/// **A dissolve that is centred on the cut is refused.**
#[test]
fn a_centred_dissolve_is_refused() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let first = a_clip(&mut session, track, sequence, 0, 25, 0);
    let second = a_clip(&mut session, track, sequence, 25, 25, 25);
    session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 5, "align": "center" }),
        )
        .expect("the transition is added");
    let failure = session
        .execute("film.export.edl", &json!({ "sequence": sequence }))
        .expect_err("the list is refused");
    let message = failure.to_string();
    assert!(message.contains("source media that the clip does not have"), "the refusal says why: {message}");
}

/// **A dissolve that ends at the cut is refused too.**
#[test]
fn a_dissolve_that_ends_at_the_cut_is_refused() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let first = a_clip(&mut session, track, sequence, 0, 25, 0);
    let second = a_clip(&mut session, track, sequence, 25, 25, 25);
    session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 5, "align": "end" }),
        )
        .expect("the transition is added");
    let failure = session
        .execute("film.export.edl", &json!({ "sequence": sequence }))
        .expect_err("the list is refused");
    let message = failure.to_string();
    assert!(message.contains("and not at the cut"), "the refusal says why: {message}");
}
