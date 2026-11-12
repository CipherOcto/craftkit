//! **Reading** a project.

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
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 7, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (bin, item)
}

/// The **project**, as the answer holds it.
fn project(session: &mut Session) -> Json {
    let outcome = session.execute("film.project", &json!({})).expect("the project is read");
    outcome.data.clone()
}

/// The bins and the items are **read**.
#[test]
fn the_bins_and_the_items_are_read() {
    let mut session = session();
    let (bin, item) = an_item(&mut session);
    let data = project(&mut session);
    let bins = data["bins"].as_array().expect("the bins are a list");
    assert_eq!(bins.len(), 1, "one bin");
    assert_eq!(bins[0]["id"], json!(bin), "and it is the bin");
    assert_eq!(bins[0]["items"][0]["id"], json!(item), "and the item is in it");
    let items = data["items"].as_array().expect("the items are a list");
    assert_eq!(items.len(), 1, "one item");
    assert_eq!(items[0]["name"], Json::String("the media".to_string()), "the name is said");
    assert_eq!(items[0]["item"], json!(7), "the editor's own id is said");
    assert_eq!(items[0]["duration"], json!(100), "and the duration");
    assert_eq!(items[0]["bin"], json!(bin), "and the bin it is in");
}

/// The clips that play an item are **derived** and are said.
#[test]
fn the_clips_that_play_an_item_are_derived_and_are_said() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
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
    let data = project(&mut session);
    let items = data["items"].as_array().expect("the items are a list");
    assert_eq!(items[0]["clips"][0], json!(clip), "the clip is said");
    assert_eq!(items[0]["tracks"][0], json!(track), "and the track the item holds");
}

/// An item that **no clip plays** has no clips.
#[test]
fn an_item_that_no_clip_plays_has_no_clips() {
    let mut session = session();
    let (_bin, _item) = an_item(&mut session);
    let data = project(&mut session);
    let items = data["items"].as_array().expect("the items are a list");
    assert!(items[0]["clips"].as_array().expect("the clips are a list").is_empty(), "there are none");
}

/// A project with **nothing** in it is empty, and not an error.
#[test]
fn a_project_with_nothing_in_it_is_empty() {
    let mut session = session();
    let data = project(&mut session);
    assert!(data["bins"].as_array().expect("the bins are a list").is_empty(), "there are no bins");
    assert!(data["items"].as_array().expect("the items are a list").is_empty(), "and no items");
}
