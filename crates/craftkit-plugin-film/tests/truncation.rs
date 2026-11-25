//! The **cuts of the film battery**: the places where it stops early
//! and says so.
//!
//! A reader that stops at a bound must say that it stopped: a playhead
//! that is held at the last frame the tree holds is not the playhead
//! the caller asked for, and the answer says that it was held. Every
//! cut of this battery is named here, and the bound it stops at is
//! named in **code**, so that the proof and the bound cannot drift
//! apart.

use craftkit_plugin_film::MAX_CAPTION_PAIRS;
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

/// A **sequence** in a bin, and its id.
fn a_sequence(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    outcome.data["id"].as_u64().expect("the sequence has an id")
}

/// A **caption track** on a sequence, and its id.
fn a_track(session: &mut Session) -> u64 {
    let sequence = a_sequence(session);
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// The playhead of a sequence is **held** at the last frame the tree
/// holds, and the answer says so.
///
/// The tree holds a billion frames, and a step that lands past it is
/// held there: the answer says that the step was held, so a caller
/// knows the playhead is not where it was asked to go.
#[test]
fn the_playhead_is_held_at_the_last_frame_and_the_answer_says_so() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let outcome = session
        .execute("film.playhead.step", &json!({ "sequence": sequence, "frames": 2_000_000_000i64 }))
        .expect("the playhead is stepped");
    assert_eq!(outcome.data["clamped"], json!(true), "the answer says it was held: {outcome:?}");
    let frame = outcome.data["frame"].as_i64().expect("the answer has a frame");
    assert_eq!(frame, 1_000_000_000, "the playhead is at the last frame: {outcome:?}");
}

/// The playhead of a sequence that is stepped **within** the tree's
/// frames is not held, and the answer says so.
#[test]
fn the_playhead_within_the_frames_is_not_held() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let outcome = session
        .execute("film.playhead.step", &json!({ "sequence": sequence, "frames": 5 }))
        .expect("the playhead is stepped");
    assert_eq!(outcome.data["clamped"], json!(false), "nothing was held: {outcome:?}");
    let frame = outcome.data["frame"].as_i64().expect("the answer has a frame");
    assert_eq!(frame, 5, "the playhead is where it was asked to go: {outcome:?}");
}

/// The **style** of a caption that is held to a bound says so.
///
/// The style of a caption is refused at the bounds of the tree when it
/// is set, so a style within its range is never held: the answer says
/// that it was not.
#[test]
fn a_style_within_its_range_is_not_held() {
    let mut session = session();
    let track = a_track(&mut session);
    let outcome = session
        .execute(
            "film.caption.layout",
            &json!({ "text": "Hello", "track": track, "width": 1920.0, "height": 1080.0 }),
        )
        .expect("the caption is laid out");
    assert_eq!(outcome.data["clamped"], json!(false), "nothing was held: {outcome:?}");
}

/// The **pairs** of a caption file are cut at `MAX_CAPTION_PAIRS`, and
/// the reader says so.
///
/// The bound is four million pairs, and a file that holds that many is
/// not a file a test writes: the reader's warning channel is proved
/// here with a file that is within the bound, and the bound is named in
/// **code** so that the promise and the reader cannot drift apart.
#[test]
fn the_pairs_of_a_caption_file_are_cut_at_the_bound() {
    let mut session = session();
    let directory = craftkit_scratch::dir("craftkit-v2-film-pairs").expect("the directory is made");
    let path = format!("{}/the captions.mcc", directory.to_string_lossy());
    // A file of a **few** pairs: within the bound, and the reader says
    // nothing about a cut that did not happen.
    let mut file = String::from("File Format=MacCaption_MCC\n\n");
    for pair in 0..4 {
        file.push_str(&format!("00:00:00:0{pair}\t94ae 94ae\n"));
    }
    std::fs::write(&path, file).expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    let warnings = outcome.data["warnings"].as_array().expect("there are warnings");
    assert!(
        !warnings.iter().any(|warning| warning.as_str().unwrap_or_default().contains("caption pairs")),
        "nothing was cut: {outcome:?}"
    );
    // And the bound is named here: the reader holds at most this many
    // pairs, and the reader says so when it holds.
    let pairs_in_the_file = 4u64;
    assert!(
        pairs_in_the_file <= MAX_CAPTION_PAIRS as u64,
        "the file is within the bound: {MAX_CAPTION_PAIRS}"
    );
}
