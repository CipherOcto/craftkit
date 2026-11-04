//! The **enable** of a clip.

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
fn a_clip(session: &mut Session, track: u64, item: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": 0, "duration": 10, "source_in": 0 }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// Whether a clip **plays**, as the document holds it.
fn enabled_of(session: &Session, clip: u64) -> bool {
    session
        .document()
        .node(clip)
        .expect("the clip is there")
        .data
        .get("enabled")
        .and_then(|enabled| enabled.as_bool())
        .unwrap_or(true)
}

/// How many clips of a track **play**, as the timeline says it.
fn playing_of(session: &mut Session, track: u64) -> u64 {
    let outcome = session.execute("film.timeline", &json!({})).expect("the timeline is read");
    let tracks = outcome.data["tracks"].as_array().expect("the tracks are a list");
    let entry =
        tracks.iter().find(|entry| entry["track"].as_u64() == Some(track)).expect("the track is there");
    entry["playing"].as_u64().expect("the track says how many play")
}

/// How many clips a track **has**, as the timeline says it.
fn clips_of(session: &mut Session, track: u64) -> usize {
    let outcome = session.execute("film.timeline", &json!({})).expect("the timeline is read");
    let tracks = outcome.data["tracks"].as_array().expect("the tracks are a list");
    let entry =
        tracks.iter().find(|entry| entry["track"].as_u64() == Some(track)).expect("the track is there");
    entry["clips"].as_array().expect("the clips are a list").len()
}

/// A clip **plays** when it is made.
#[test]
fn a_clip_plays_when_it_is_made() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    assert!(enabled_of(&session, clip), "the clip plays");
    assert_eq!(playing_of(&mut session, track), 1, "and the track says so");
}

/// A clip may be **disabled**.
#[test]
fn a_clip_may_be_disabled() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    let outcome = session
        .execute("film.clip.enable", &json!({ "clip": clip, "on": false }))
        .expect("the clip is disabled");
    assert_eq!(outcome.data["enabled"].as_bool(), Some(false), "the answer says it");
    assert!(!enabled_of(&session, clip), "and the clip does not play");
    assert_eq!(playing_of(&mut session, track), 0, "and the track says so");
}

/// A clip that does not play is **still on the track**.
#[test]
fn a_clip_that_does_not_play_is_still_on_the_track() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    session.execute("film.clip.enable", &json!({ "clip": clip, "on": false })).expect("the clip is disabled");
    assert_eq!(clips_of(&mut session, track), 1, "the clip is still there");
    assert_eq!(playing_of(&mut session, track), 0, "and it does not play");
}

/// The flag is **toggled** when the command does not say which way.
#[test]
fn the_flag_is_toggled_when_the_command_does_not_say() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    let outcome = session.execute("film.clip.enable", &json!({ "clip": clip })).expect("the flag is toggled");
    assert_eq!(outcome.data["enabled"].as_bool(), Some(false), "the answer says which way it went");
    let outcome = session.execute("film.clip.enable", &json!({ "clip": clip })).expect("the flag is toggled");
    assert_eq!(outcome.data["enabled"].as_bool(), Some(true), "and again");
}

/// The command **sets** the flag when it says which way.
#[test]
fn the_command_sets_the_flag_when_it_says_which_way() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    session.execute("film.clip.enable", &json!({ "clip": clip, "on": false })).expect("the clip is disabled");
    session
        .execute("film.clip.enable", &json!({ "clip": clip, "on": false }))
        .expect("the clip is disabled again");
    assert!(!enabled_of(&session, clip), "the flag is set and not toggled");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error =
        session.execute("film.clip.enable", &json!({ "clip": item })).expect_err("the command is refused");
    let message = error.to_string();
    assert!(message.contains("not a clip"), "it says why: {message}");
    let error =
        session.execute("film.clip.enable", &json!({ "clip": 9999 })).expect_err("the command is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}

/// The flag is **saved** with the document.
#[test]
fn the_flag_is_saved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    session.execute("film.clip.enable", &json!({ "clip": clip, "on": false })).expect("the clip is disabled");
    let text = session.document().to_json().to_string();
    assert!(text.contains("enabled"), "the flag is in the document");
}
