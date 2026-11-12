//! **Splitting** a clip.
//!
//! A clip is divided at a place: the piece before it keeps the id, the
//! piece after it is a new clip, and the media moves with the cut.

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

/// A **bin**, an **item** in it, and a **track** in the item.
fn a_track(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 1000 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": 30,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The three numbers of a clip.
fn clock_of(session: &Session, clip: u64) -> (u64, u64, u64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    (start, duration, source_in)
}

/// A clip is **split** at a place: the piece before it is the same
/// clip, shorter, and the piece after it is a **new clip**.
#[test]
fn a_clip_is_split() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 10, 20);
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": clip, "at": 20 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("there is a new clip");
    assert_eq!(clock_of(&session, clip), (10, 10, 30), "the first piece is the same clip, shorter");
    assert_eq!(clock_of(&session, new), (20, 10, 40), "and the second piece is a new clip");
}

/// The new clip **plays the same item**, and it is **on the same
/// track**.
#[test]
fn the_new_clip_plays_the_same_item() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": clip, "at": 5 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("there is a new clip");
    let relation = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == new)
        .expect("the new clip plays an item");
    assert_eq!(relation.to, item, "and it is the same one");
    let owner = session
        .document()
        .walk()
        .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == new))
        .map(|node| node.id)
        .expect("the new clip is on a track");
    assert_eq!(owner, track, "and it is the same track");
}

/// A place that is **not inside** the clip is refused: a cut at the
/// edge would cut nothing, and a place outside the clip is not in it.
#[test]
fn a_place_that_is_not_inside_the_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 10, 20);
    for at in [10, 30, 40] {
        let error = session
            .execute("film.clip.split", &json!({ "clip": clip, "at": at }))
            .expect_err("the split is refused");
        assert!(error.to_string().contains("is not inside the clip"), "the error says why: {error}");
    }
    assert_eq!(clock_of(&session, clip), (10, 20, 30), "and the clip is as it was");
}

/// A place that is **not a whole number** is refused.
#[test]
fn a_place_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 10, 20);
    let error = session
        .execute("film.clip.split", &json!({ "clip": clip, "at": 15.5 }))
        .expect_err("the split is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error = session
        .execute("film.clip.split", &json!({ "clip": track, "at": 5 }))
        .expect_err("the split is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("film.clip.split", &json!({ "clip": 1_000_000, "at": 5 }))
        .expect_err("the split is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
