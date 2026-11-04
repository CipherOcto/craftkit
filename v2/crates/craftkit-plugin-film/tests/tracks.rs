//! The **settings** of a track.

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

/// A **sequence** with a track in it.
fn a_track(session: &mut Session) -> (u64, u64) {
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
    (sequence, track)
}

/// The **track** as the document has it.
fn the_track(session: &mut Session, track: u64) -> Json {
    let outcome = session.execute("film.track.get", &json!({ "track": track })).expect("the track is read");
    outcome.data.clone()
}

/// The **flags** of a track may be set.
#[test]
fn the_flags_of_a_track_may_be_set() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute("film.track.enable", &json!({ "track": track, "on": false }))
        .expect("the track is disabled");
    session.execute("film.track.mute", &json!({ "track": track, "on": true })).expect("the track is muted");
    session.execute("film.track.solo", &json!({ "track": track, "on": true })).expect("the track is soloed");
    let track_data = the_track(&mut session, track);
    assert_eq!(track_data["enabled"], json!(false), "the track is disabled");
    assert_eq!(track_data["muted"], json!(true), "the track is muted");
    assert_eq!(track_data["solo"], json!(true), "the track is soloed");
}

/// The **volume** and the **pan** of a track may be set.
#[test]
fn the_volume_and_the_pan_of_a_track_may_be_set() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session.execute("film.track.volume", &json!({ "track": track, "db": -6.0 })).expect("the volume is set");
    session.execute("film.track.pan", &json!({ "track": track, "pan": -25.0 })).expect("the pan is set");
    let track_data = the_track(&mut session, track);
    assert_eq!(track_data["volume_db"], json!(-6.0), "the volume is six down");
    assert_eq!(track_data["pan"], json!(-25.0), "and the pan is to the left");
}

/// A pan outside its bounds is **refused**.
#[test]
fn a_pan_outside_its_bounds_is_refused() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    let error = session
        .execute("film.track.pan", &json!({ "track": track, "pan": 150.0 }))
        .expect_err("a pan of a hundred and fifty is refused");
    assert!(error.to_string().contains("100"), "and the error says the bound");
    let track_data = the_track(&mut session, track);
    assert_eq!(track_data["pan"], json!(null), "and the pan is the one that was there");
}

/// A **label** may be put on a track and taken off.
#[test]
fn a_label_may_be_put_on_a_track_and_taken_off() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute("film.track.label", &json!({ "track": track, "label": "Violet" }))
        .expect("the label is put on");
    assert_eq!(the_track(&mut session, track)["label"], json!("Violet"), "the label is violet");
    session
        .execute("film.track.label", &json!({ "track": track, "label": "" }))
        .expect("the label is taken off");
    assert_eq!(the_track(&mut session, track)["label"], json!(null), "and there is no label");
    let error = session
        .execute("film.track.label", &json!({ "track": track, "label": "Puce" }))
        .expect_err("a label that is not on the list is refused");
    assert!(error.to_string().contains("Puce"), "and the error says which");
}

/// A track may be **renamed**.
#[test]
fn a_track_may_be_renamed() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute("film.track.rename", &json!({ "track": track, "name": "the pictures" }))
        .expect("the track is renamed");
    assert_eq!(the_track(&mut session, track)["name"], json!("the pictures"), "the name is the name");
}
