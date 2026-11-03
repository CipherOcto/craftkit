//! The **label** of a clip.
//!
//! A label is a colour and not a number, and the clips that carry the
//! same one are a group.

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

/// A **clip** on a track, and the ids of the clip and its item.
fn a_clip(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "bin": bin, "name": "the item", "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute("film.clip.add", &json!({ "track": track, "item": item, "start": 0, "duration": 10 }))
        .expect("the clip is added");
    let clip = outcome.data["id"].as_u64().expect("the clip has an id");
    (clip, track)
}

/// The **label** of a clip, as the document holds it.
fn label_of(session: &Session, clip: u64) -> Option<String> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("label").and_then(|label| label.as_str()).map(|label| label.to_string())
}

/// A clip may be **labelled**.
#[test]
fn a_clip_may_be_labelled() {
    let mut session = session();
    let (clip, _) = a_clip(&mut session);
    session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "Violet" }))
        .expect("the clip is labelled");
    assert_eq!(label_of(&session, clip), Some("Violet".to_string()), "the clip is violet");
}

/// The tree's **names** are the tree's own, and the case does not
/// matter.
#[test]
fn the_trees_names_are_the_trees() {
    let mut session = session();
    let (clip, _) = a_clip(&mut session);
    session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "cerulean" }))
        .expect("the clip is labelled");
    assert_eq!(label_of(&session, clip), Some("Cerulean".to_string()), "the label is the tree's own");
}

/// A label that is **not a label** is refused: the tree's list is
/// closed.
#[test]
fn a_label_that_is_not_a_label_is_refused() {
    let mut session = session();
    let (clip, _) = a_clip(&mut session);
    let error = session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "Wobble" }))
        .expect_err("there is no such label");
    assert!(error.to_string().contains("no label"), "the error says so: {error}");
    assert_eq!(label_of(&session, clip), None, "and the clip is unchanged");
}

/// The label may be taken **away**.
#[test]
fn the_label_may_be_taken_away() {
    let mut session = session();
    let (clip, _) = a_clip(&mut session);
    session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "Rose" }))
        .expect("the clip is labelled");
    session.execute("film.clip.label.delete", &json!({ "clip": clip })).expect("the label is taken away");
    assert_eq!(label_of(&session, clip), None, "the clip is in no group");
}

/// The clips that carry the same label are a **group**, and the group
/// is derived.
#[test]
fn the_clips_with_the_same_label_are_a_group() {
    let mut session = session();
    let (clip, track) = a_clip(&mut session);
    let bin = session.document().walk().find(|node| node.kind == "film.bin").expect("the bin is there").id;
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": "the other item", "bin": bin, "item": 2, "duration": 100 }),
        )
        .expect("the other item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.clip.add", &json!({ "track": track, "item": item, "start": 20, "duration": 10 }))
        .expect("the other clip is added");
    let other = outcome.data["id"].as_u64().expect("the other clip has an id");
    session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "Forest" }))
        .expect("the clip is labelled");
    session
        .execute("film.clip.label", &json!({ "clip": other, "label": "Forest" }))
        .expect("the other clip is labelled");
    let outcome =
        session.execute("film.clip.label.group", &json!({ "clip": clip })).expect("the group is read");
    let group = outcome.data["clips"].as_array().expect("the group is there");
    assert_eq!(group.len(), 2, "both clips are in the group");
}

/// A clip with **no label** is in no group.
#[test]
fn a_clip_with_no_label_is_in_no_group() {
    let mut session = session();
    let (clip, _) = a_clip(&mut session);
    let outcome =
        session.execute("film.clip.label.group", &json!({ "clip": clip })).expect("the group is read");
    let group = outcome.data["clips"].as_array().expect("the group is there");
    assert!(group.is_empty(), "the group is empty");
}

/// A clip that is **not a clip** is refused.
#[test]
fn a_clip_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_, track) = a_clip(&mut session);
    let error = session
        .execute("film.clip.label", &json!({ "clip": track, "label": "Violet" }))
        .expect_err("a track is not a clip");
    assert!(error.to_string().contains("not a clip"), "the error says so: {error}");
}

/// The label is **saved** with the clip.
#[test]
fn the_label_is_saved_with_the_clip() {
    let mut session = session();
    let (clip, _) = a_clip(&mut session);
    session
        .execute("film.clip.label", &json!({ "clip": clip, "label": "Teal" }))
        .expect("the clip is labelled");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(clip).expect("the clip is there");
    assert_eq!(node.data["label"].as_str(), Some("Teal"), "the clip is still teal");
}
