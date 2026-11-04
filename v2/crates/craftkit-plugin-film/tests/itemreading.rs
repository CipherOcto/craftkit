//! **Reading** a media item.

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
        .execute(
            "film.item.add",
            &json!({ "name": "the media", "bin": bin, "item": 7, "duration": 100, "path": "/media/a.mov" }),
        )
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (bin, item)
}

/// The **answer** of a read.
fn read(session: &mut Session, item: u64) -> Json {
    let outcome = session.execute("film.item.get", &json!({ "item": item })).expect("the item is read");
    outcome.data.clone()
}

/// A media item may be **read whole**.
#[test]
fn a_media_item_may_be_read_whole() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let answer = read(&mut session, item);
    assert_eq!(answer["id"], json!(item), "the item is named by its id");
    assert_eq!(answer["name"], Json::String("the media".to_string()), "and by its name");
    assert_eq!(answer["item"], json!(7), "and by the id it has in the editor");
    assert_eq!(answer["path"], Json::String("/media/a.mov".to_string()), "and by the path of the media");
    assert_eq!(answer["duration"], json!(100), "and by its length in frames");
    assert_eq!(answer["offline"], json!(false), "and by whether it is offline");
    assert_eq!(answer["label"], Json::Null, "and it carries no label");
}

/// The **bin** of an item is derived: it is the item's place in the
/// tree.
#[test]
fn the_bin_of_an_item_is_derived() {
    let mut session = session();
    let (bin, item) = an_item(&mut session);
    let answer = read(&mut session, item);
    assert_eq!(answer["bin"], json!(bin), "the item is in the bin it was put in");
}

/// The **clips** that play an item are derived: they are the edges
/// that name it.
#[test]
fn the_clips_that_play_an_item_are_derived() {
    let mut session = session();
    let (bin, item) = an_item(&mut session);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "V1", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute("film.clip.add", &json!({ "track": track, "item": item, "start": 0, "duration": 50 }))
        .expect("the clip is added");
    let clip = outcome.data["id"].as_u64().expect("the clip has an id");
    let answer = read(&mut session, item);
    assert_eq!(answer["clips"], json!([clip]), "the clip that plays the item is named");
    assert_eq!(answer["bin"], json!(bin), "and the item is still in its bin");
}

/// A **label** on an item is said by the answer.
#[test]
fn a_label_on_an_item_is_said_by_the_answer() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    session
        .execute("film.item.label", &json!({ "item": item, "label": "Violet" }))
        .expect("the item is labelled");
    let answer = read(&mut session, item);
    assert_eq!(answer["label"], Json::String("Violet".to_string()), "the label is said");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session.execute("film.item.get", &json!({ "item": 999 })).expect_err("the item is not there");
    assert!(error.to_string().contains("there is no item"), "the refusal says why: {error}");
}

/// A node that is **not an item** is refused.
#[test]
fn a_node_that_is_not_an_item_is_refused() {
    let mut session = session();
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let error =
        session.execute("film.item.get", &json!({ "item": bin })).expect_err("the bin is not an item");
    assert!(error.to_string().contains("not an item"), "the refusal says why: {error}");
}
