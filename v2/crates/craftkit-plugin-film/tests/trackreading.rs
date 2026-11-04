//! **Reading** a track.

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

/// A **bin** with an **item** in it, and their ids.
fn an_item(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (bin, item)
}

/// A **track** in the item, and its id.
fn a_track(session: &mut Session, item: u64) -> u64 {
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// A **clip** on the track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": start, "duration": duration }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The **answer** of a read.
fn read(session: &mut Session, track: u64) -> Json {
    let outcome = session.execute("film.track.get", &json!({ "track": track })).expect("the track is read");
    outcome.data.clone()
}

/// A track may be **read whole**.
#[test]
fn a_track_may_be_read_whole() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let track = a_track(&mut session, item);
    let answer = read(&mut session, track);
    assert_eq!(answer["id"], json!(track), "the track is named by its id");
    assert_eq!(answer["name"], Json::String("the track".to_string()), "and by its name");
    assert_eq!(answer["kind"], Json::String("video".to_string()), "and by its kind");
    assert_eq!(answer["locked"], json!(false), "and it is not locked");
    assert_eq!(answer["clips"], json!([]), "and it holds no clip");
    assert_eq!(answer["length"], json!(0), "and it is not long");
    assert_eq!(answer["sequence"], json!(item), "and it is in a sequence");
}

/// The **clips** of a track are in the order they **start**, and the
/// **length** of the track is the end of its last clip.
#[test]
fn the_clips_of_a_track_are_in_the_order_they_start() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let track = a_track(&mut session, item);
    let late = a_clip(&mut session, track, item, 120, 50);
    let early = a_clip(&mut session, track, item, 0, 30);
    let answer = read(&mut session, track);
    assert_eq!(answer["clips"], json!([early, late]), "the clips are in the order they start");
    assert_eq!(answer["length"], json!(170), "and the track is as long as its last clip");
}

/// A **locked** track says so.
#[test]
fn a_locked_track_says_so() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let track = a_track(&mut session, item);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let answer = read(&mut session, track);
    assert_eq!(answer["locked"], json!(true), "the track is locked");
}

/// A track that is **not there** is refused, and so is a node that is
/// not a track.
#[test]
fn a_track_that_is_not_there_is_refused() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let error =
        session.execute("film.track.get", &json!({ "track": 999 })).expect_err("the track is refused");
    assert!(error.to_string().contains("there is no track"), "the refusal says why: {error}");
    let error =
        session.execute("film.track.get", &json!({ "track": item })).expect_err("the item is not a track");
    assert!(error.to_string().contains("not a track"), "the refusal says why: {error}");
}
