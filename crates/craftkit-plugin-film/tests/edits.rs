//! The **edit points** of a sequence, and the **razor**.

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

/// A **sequence** with a track and two clips in it.
fn a_sequence(session: &mut Session) -> (u64, u64, u64) {
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
        .execute("film.clip.add", &json!({ "track": track, "item": sequence, "start": 0, "duration": 20 }))
        .expect("the first clip is added");
    session
        .execute("film.clip.add", &json!({ "track": track, "item": sequence, "start": 30, "duration": 20 }))
        .expect("the second clip is added");
    (bin, sequence, track)
}

/// The **frame** the playhead is at.
fn frame(session: &mut Session, sequence: u64) -> u64 {
    let outcome =
        session.execute("film.playhead.get", &json!({ "sequence": sequence })).expect("the playhead is read");
    outcome.data["frame"].as_u64().expect("the playhead has a frame")
}

/// The playhead walks the **edit points**.
#[test]
fn the_playhead_walks_the_edit_points() {
    let mut session = session();
    let (_bin, sequence, _track) = a_sequence(&mut session);
    assert_eq!(frame(&mut session, sequence), 0, "the playhead begins at the start");
    session
        .execute("film.playhead.nextedit", &json!({ "sequence": sequence }))
        .expect("the playhead goes to the next edit point");
    assert_eq!(frame(&mut session, sequence), 20, "and the end of the first clip is one");
    session
        .execute("film.playhead.nextedit", &json!({ "sequence": sequence }))
        .expect("the playhead goes on");
    assert_eq!(frame(&mut session, sequence), 30, "and the start of the second clip is the next");
    session
        .execute("film.playhead.prevedit", &json!({ "sequence": sequence }))
        .expect("the playhead goes back");
    assert_eq!(frame(&mut session, sequence), 20, "and the edit point before it");
    session
        .execute("film.playhead.nextedit", &json!({ "sequence": sequence }))
        .expect("the playhead goes on");
    session
        .execute("film.playhead.nextedit", &json!({ "sequence": sequence }))
        .expect("the playhead goes on again");
    assert_eq!(frame(&mut session, sequence), 50, "and the end of the second clip is the last");
    session
        .execute("film.playhead.nextedit", &json!({ "sequence": sequence }))
        .expect("the playhead goes on");
    assert_eq!(frame(&mut session, sequence), 50, "and with no edit point it stays");
}

/// The **razor** cuts the clip that is under the playhead.
#[test]
fn the_razor_cuts_the_clip_under_the_playhead() {
    let mut session = session();
    let (_bin, sequence, track) = a_sequence(&mut session);
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the playhead is set");
    let outcome = session
        .execute("film.timeline.razor", &json!({ "sequence": sequence, "track": track }))
        .expect("the razor cuts");
    assert_eq!(outcome.data["at"], json!(10), "the cut is at the playhead");
    let clips = session.execute("film.track.get", &json!({ "track": track })).expect("the track is read");
    let clips = clips.data["clips"].as_array().expect("the track has clips");
    assert_eq!(clips.len(), 3, "and the clip is cut in two");
}

/// The razor refuses a place where there is no clip.
#[test]
fn the_razor_refuses_a_place_with_no_clip() {
    let mut session = session();
    let (_bin, sequence, track) = a_sequence(&mut session);
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 25 }))
        .expect("the playhead is set");
    let error = session
        .execute("film.timeline.razor", &json!({ "sequence": sequence, "track": track }))
        .expect_err("there is no clip at the playhead");
    assert!(error.to_string().contains("25"), "and the error says where");
}
