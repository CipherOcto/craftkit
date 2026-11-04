//! **Taking away** a bin.

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

/// A bin whose item is **played** by a clip.
fn a_played_bin(session: &mut Session) -> (u64, u64, u64) {
    let (bin, item) = an_item(session);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": 0, "duration": 10, "source_in": 0 }),
        )
        .expect("the clip is added");
    let clip = outcome.data["id"].as_u64().expect("the clip has an id");
    (bin, item, clip)
}

/// A bin is taken away, and the items of it go with it.
#[test]
fn a_bin_is_taken_away_and_the_items_of_it_go_with_it() {
    let mut session = session();
    let (bin, item) = an_item(&mut session);
    let outcome = session.execute("film.bin.remove", &json!({ "bin": bin })).expect("the bin is taken away");
    assert_eq!(outcome.data["items"][0], json!(item), "the item is said");
    assert!(session.document().node(bin).is_none(), "the bin is gone");
    assert!(session.document().node(item).is_none(), "and the item is gone with it");
}

/// A bin whose item is **played** is not taken away.
#[test]
fn a_bin_whose_item_is_played_is_not_taken_away() {
    let mut session = session();
    let (bin, item, clip) = a_played_bin(&mut session);
    let error = session.execute("film.bin.remove", &json!({ "bin": bin })).expect_err("it is refused");
    let message = error.to_string();
    assert!(message.contains("a clip that plays nothing"), "it says why: {message}");
    assert!(message.contains(&item.to_string()), "and which item: {message}");
    assert!(message.contains(&clip.to_string()), "and which clip: {message}");
    assert!(session.document().node(bin).is_some(), "and the bin is still there");
}

/// The bin is taken away when the **clip** is taken away first.
#[test]
fn the_bin_is_taken_away_when_the_clip_is_taken_away_first() {
    let mut session = session();
    let (bin, _item, clip) = a_played_bin(&mut session);
    session.execute("film.clip.remove", &json!({ "clip": clip })).expect("the clip is taken away");
    session.execute("film.bin.remove", &json!({ "bin": bin })).expect("the bin is taken away");
    assert!(session.document().node(bin).is_none(), "and the bin is gone");
}

/// A node that is **not a bin** is refused.
#[test]
fn a_node_that_is_not_a_bin_is_refused() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let error = session.execute("film.bin.remove", &json!({ "bin": item })).expect_err("it is refused");
    assert!(error.to_string().contains("not a bin"), "it says why: {error}");
    let error = session.execute("film.bin.remove", &json!({ "bin": 9999 })).expect_err("it is refused");
    assert!(error.to_string().contains("there is no node"), "and there is none: {error}");
}
