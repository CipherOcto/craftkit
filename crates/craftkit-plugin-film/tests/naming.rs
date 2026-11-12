//! The **name** of a clip.

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

/// A **bin**, an **item** in it, a **track** in the item.
fn a_timeline(session: &mut Session, media: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": media }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": start, "duration": 10, "source_in": 0 }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The name of a clip, as the document shows it.
fn name_of(session: &Session, clip: u64) -> String {
    session.document().node(clip).expect("the clip is there").name.clone()
}

/// A clip that has **not** been named is shown by the place it is at.
#[test]
fn a_clip_that_has_not_been_named_is_shown_by_its_place() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 120);
    assert_eq!(name_of(&session, clip), "clip at 120", "the clip is shown by its place");
}

/// A clip may be **named**.
#[test]
fn a_clip_may_be_named() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 120);
    let outcome = session
        .execute("film.clip.rename", &json!({ "clip": clip, "name": "the shot" }))
        .expect("the clip is named");
    assert_eq!(outcome.data["name"].as_str(), Some("the shot"), "the name is said");
    assert_eq!(name_of(&session, clip), "the shot", "and the clip carries it");
}

/// The name a clip has been given **survives a move**.
#[test]
fn the_name_survives_a_move() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 120);
    session
        .execute("film.clip.rename", &json!({ "clip": clip, "name": "the shot" }))
        .expect("the clip is named");
    session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 200 }))
        .expect("the clip is moved");
    assert_eq!(name_of(&session, clip), "the shot", "the name is still the clip's own");
}

/// A clip that has **not** been named follows the place it is at.
#[test]
fn an_unnamed_clip_follows_the_place_it_is_at() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 120);
    session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 200 }))
        .expect("the clip is moved");
    assert_eq!(name_of(&session, clip), "clip at 200", "the name follows the clip");
}

/// A name that is **blank** is refused.
#[test]
fn a_blank_name_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 120);
    let error = session
        .execute("film.clip.rename", &json!({ "clip": clip, "name": "   " }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("a clip has a name"), "it says why: {message}");
    assert_eq!(name_of(&session, clip), "clip at 120", "and nothing was written");
}

/// The **split** of a named clip: the piece that keeps the id keeps the
/// name, and the new piece is a new clip.
#[test]
fn the_split_of_a_named_clip() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0);
    session
        .execute("film.clip.rename", &json!({ "clip": clip, "name": "the shot" }))
        .expect("the clip is named");
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": clip, "at": 5 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("the new clip is there");
    assert_eq!(name_of(&session, clip), "the shot", "the piece that kept the id keeps the name");
    assert_eq!(name_of(&session, new), "clip at 5", "and the new piece is a new clip");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.clip.rename", &json!({ "clip": item, "name": "the shot" }))
        .expect_err("the rename is refused");
    let message = error.to_string();
    assert!(message.contains("not a clip"), "it says why: {message}");
    let error = session
        .execute("film.clip.rename", &json!({ "clip": 9999, "name": "the shot" }))
        .expect_err("the rename is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}

/// The name is **saved** with the document.
#[test]
fn the_name_is_saved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 120);
    session
        .execute("film.clip.rename", &json!({ "clip": clip, "name": "the shot" }))
        .expect("the clip is named");
    let text = session.document().to_json().to_string();
    assert!(text.contains("the shot"), "the name is in the document");
}
