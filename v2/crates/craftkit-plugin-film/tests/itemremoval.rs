//! **Taking away** a media item.

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

/// A **clip** on a track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": 0, "duration": 10, "source_in": 0 }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// An item that **no clip plays** is taken away.
#[test]
fn an_item_that_no_clip_plays_is_taken_away() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let outcome =
        session.execute("film.item.remove", &json!({ "item": item })).expect("the item is taken away");
    assert_eq!(outcome.data["name"].as_str(), Some("the media"), "the name is said");
    assert!(session.document().node(item).is_none(), "and the item is gone");
}

/// An item that a clip **plays** is not taken away.
#[test]
fn an_item_that_a_clip_plays_is_not_taken_away() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let track = a_track(&mut session, item);
    let clip = a_clip(&mut session, track, item);
    let error = session.execute("film.item.remove", &json!({ "item": item })).expect_err("it is refused");
    let message = error.to_string();
    assert!(message.contains("is played by"), "it says why: {message}");
    assert!(message.contains(&clip.to_string()), "and which clip: {message}");
    assert!(session.document().node(item).is_some(), "and the item is still there");
}

/// The item is taken away when the **clip** is taken away first.
#[test]
fn the_item_is_taken_away_when_the_clip_is_taken_away_first() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let track = a_track(&mut session, item);
    let clip = a_clip(&mut session, track, item);
    session.execute("film.clip.remove", &json!({ "clip": clip })).expect("the clip is taken away");
    session.execute("film.item.remove", &json!({ "item": item })).expect("the item is taken away");
    assert!(session.document().node(item).is_none(), "and the item is gone");
}

/// A node that is **not an item** is refused.
#[test]
fn a_node_that_is_not_an_item_is_refused() {
    let mut session = session();
    let (bin, _item) = an_item(&mut session);
    let error = session.execute("film.item.remove", &json!({ "item": bin })).expect_err("it is refused");
    assert!(error.to_string().contains("not an item"), "it says why: {error}");
    let error = session.execute("film.item.remove", &json!({ "item": 9999 })).expect_err("it is refused");
    assert!(error.to_string().contains("there is no node"), "and there is none: {error}");
}
