//! **Reading** a clip.

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

/// A **clip** on a track in the item, and the ids of the track and the
/// sequence.
fn a_clip(session: &mut Session, item: u64) -> (u64, u64, u64) {
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": 120, "duration": 50, "source_in": 10 }),
        )
        .expect("the clip is added");
    let clip = outcome.data["id"].as_u64().expect("the clip has an id");
    (track, item, clip)
}

/// The **answer** of a read.
fn read(session: &mut Session, clip: u64) -> Json {
    let outcome = session.execute("film.clip.get", &json!({ "clip": clip })).expect("the clip is read");
    outcome.data.clone()
}

/// A clip may be **read whole**.
#[test]
fn a_clip_may_be_read_whole() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let (_track, _sequence, clip) = a_clip(&mut session, item);
    let answer = read(&mut session, clip);
    assert_eq!(answer["id"], json!(clip), "the clip is named by its id");
    assert_eq!(answer["name"], Json::String("clip at 120".to_string()), "and by the place it is at");
    assert_eq!(answer["start"], json!(120), "and it starts where it starts");
    assert_eq!(answer["duration"], json!(50), "and it is as long as it is");
    assert_eq!(answer["end"], json!(170), "and it ends where it ends");
    assert_eq!(answer["source_in"], json!(10), "and it begins in the media where it begins");
    assert_eq!(answer["speed"], json!(1.0), "and it plays at the speed it plays at");
    assert_eq!(answer["media_duration"], json!(50), "and it plays that much of the media");
    assert_eq!(answer["enabled"], json!(true), "and it plays");
    assert_eq!(answer["label"], Json::Null, "and it carries no label");
}

/// The **track**, the **sequence** and the **item** of a clip are
/// derived: they are its place in the tree and the edges that name it.
#[test]
fn the_track_the_sequence_and_the_item_are_derived() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let (track, sequence, clip) = a_clip(&mut session, item);
    let answer = read(&mut session, clip);
    assert_eq!(answer["track"], json!(track), "the track is said");
    assert_eq!(answer["sequence"], json!(sequence), "and the sequence");
    assert_eq!(answer["item"], json!(item), "and the item the clip plays");
}

/// A **label**, a **link** and a **frame hold** are said by the answer.
#[test]
fn a_label_a_link_and_a_frame_hold_are_said() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let (_track, _sequence, clip) = a_clip(&mut session, item);
    session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "Violet" }))
        .expect("the clip is labelled");
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 20 })).expect("the clip holds a frame");
    let answer = read(&mut session, clip);
    assert_eq!(answer["label"], Json::String("Violet".to_string()), "the label is said");
    assert_eq!(answer["frame_hold"], json!(20), "and the frame it holds");
    assert_eq!(answer["link"], Json::Null, "and it is linked to nothing");
}

/// A clip that does not **play** says so.
#[test]
fn a_clip_that_does_not_play_says_so() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let (_track, _sequence, clip) = a_clip(&mut session, item);
    session.execute("film.clip.enable", &json!({ "clip": clip, "on": false })).expect("the clip is disabled");
    let answer = read(&mut session, clip);
    assert_eq!(answer["enabled"], json!(false), "the clip does not play");
}

/// A clip that is **not there** is refused, and so is a node that is
/// not a clip.
#[test]
fn a_clip_that_is_not_there_is_refused() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let error = session.execute("film.clip.get", &json!({ "clip": 999 })).expect_err("the clip is refused");
    assert!(error.to_string().contains("there is no clip"), "the refusal says why: {error}");
    let error =
        session.execute("film.clip.get", &json!({ "clip": item })).expect_err("the item is not a clip");
    assert!(error.to_string().contains("not a clip"), "the refusal says why: {error}");
}
