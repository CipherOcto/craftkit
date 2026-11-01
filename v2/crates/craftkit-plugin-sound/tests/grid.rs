//! The **grid**: the lines a clip is put on.
//!
//! The tree's sound editor has a grid — a division of the bar — and a
//! command that puts a clip on it. Here the grid is a setting of the
//! session and the lines are counted from the bar line.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// A track with two clips on it, and the ids of the clips.
fn two_clips(session: &mut Session) -> (u64, Vec<u64>) {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let mut clips = Vec::new();
    for start in [0, 1000] {
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": start, "length": 500 }))
            .expect("the clip is added");
        let id = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.clip")
            .last()
            .map(|node| node.id)
            .expect("the clip is there");
        clips.push(id);
    }
    (track, clips)
}

/// Where a clip starts.
fn start_of(session: &Session, clip: u64) -> i64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0)
}

/// The grid is **set**, and it is read back.
#[test]
fn the_grid_is_set_and_read() {
    let mut session = session();
    session.execute("sound.grid.set", &json!({ "value": "1/8" })).expect("the grid is set");
    let outcome = session.execute("sound.grid.read", &json!({})).expect("the grid is read");
    assert_eq!(outcome.data.get("value").and_then(|value| value.as_str()), Some("1/8"));
    assert_eq!(outcome.data.get("label").and_then(|label| label.as_str()), Some("1/8"));
}

/// A grid that is not a note value is **refused**: a grid of `1/5` is
/// not a grid, and a grid that is quietly a quarter note is a grid the
/// caller did not ask for.
#[test]
fn a_grid_that_is_not_a_note_value_is_refused() {
    let mut session = session();
    let refused = session
        .execute("sound.grid.set", &json!({ "value": "1/5" }))
        .expect_err("a fifth is not a note value");
    assert!(refused.to_string().contains("note value"), "the error says why: {refused}");
    let outcome = session.execute("sound.grid.read", &json!({})).expect("the grid is read");
    assert_eq!(
        outcome.data.get("value").and_then(|value| value.as_str()),
        Some("1/16"),
        "the grid is the one that was there"
    );
}

/// The grid is **dotted** or a **triplet**, and the flags say which.
#[test]
fn a_grid_may_be_dotted_or_a_triplet() {
    let mut session = session();
    session.execute("sound.grid.set", &json!({ "value": "1/8", "dotted": true })).expect("the grid is set");
    let outcome = session.execute("sound.grid.read", &json!({})).expect("the grid is read");
    assert_eq!(outcome.data.get("dotted").and_then(|dotted| dotted.as_bool()), Some(true));
    assert_eq!(outcome.data.get("label").and_then(|label| label.as_str()), Some("1/8 dotted"));
    session.execute("sound.grid.set", &json!({ "value": "1/8", "triplet": true })).expect("the grid is set");
    let outcome = session.execute("sound.grid.read", &json!({})).expect("the grid is read");
    assert_eq!(outcome.data.get("triplet").and_then(|triplet| triplet.as_bool()), Some(true));
    assert_eq!(outcome.data.get("label").and_then(|label| label.as_str()), Some("1/8 triplet"));
}

/// The lines are counted **from the bar line**: a bar of three beats
/// has no even number of eighths in it, and lines counted from the
/// start of the song would drift.
#[test]
fn the_lines_are_counted_from_the_bar_line() {
    let mut session = session();
    // Three beats to the bar.
    session
        .execute("sound.meter.set", &json!({ "numerator": 3, "denominator": 4 }))
        .expect("the meter is set");
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    let outcome = session.execute("sound.grid.read", &json!({})).expect("the grid is read");
    let lines = outcome.data.get("lines").and_then(|lines| lines.as_array()).expect("the lines are there");
    let lines: Vec<i64> = lines.iter().filter_map(|line| line.as_i64()).collect();
    // At 120 beats a minute a quarter note is half a second, and the
    // sample rate is 48 kHz: the lines are every 24000 samples.
    let step = 24_000;
    assert_eq!(lines[0], 0, "the first line is the start");
    for (index, line) in lines.iter().enumerate() {
        assert_eq!(*line, step * index as i64, "the line {index} is {step} samples on");
    }
    // And the bar is three lines long: the fourth line is the bar line
    // again.
    assert_eq!(lines[3], 3 * step, "the fourth line is the next bar line");
}

/// A clip is put **on the grid**, and it is its **beat** that lands on
/// the line.
#[test]
fn a_clip_is_put_on_the_grid_by_its_beat() {
    let mut session = session();
    let (track, _) = two_clips(&mut session);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 24000, "length": 24000 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .last()
        .map(|node| node.id)
        .expect("the clip is there");
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    // The clip's beat is 4800 samples inside it, and the clip starts a
    // whole quarter note late, so the beat is late too.
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 28800 })).expect("the beat is set");
    session.execute("sound.clip.quantize", &json!({ "clips": [clip] })).expect("the clip is quantised");
    // The beat lands on the nearest line — the line at 24000, which is
    // nearer than the one at 48000 — and the clip moves so that it is
    // there: the clip starts 4800 samples before its own beat.
    assert_eq!(start_of(&session, clip), 19200, "the clip moved so that its beat is on the line");
}

/// A clip with no beat is put on the grid by its **head**.
#[test]
fn a_clip_with_no_beat_is_put_on_the_grid_by_its_head() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    // The second clip starts 1000 samples in, which is nearer the line
    // at 0 than the line at 24000.
    session.execute("sound.clip.quantize", &json!({ "clips": [clips[1]] })).expect("the clip is quantised");
    assert_eq!(start_of(&session, clips[1]), 0, "the head is on the line");
}

/// A clip that is **already** on the grid does not change, and a
/// command that changes nothing leaves no undo step behind.
#[test]
fn a_clip_on_the_grid_does_not_change() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    session.execute("sound.clip.quantize", &json!({ "clips": [clips[0]] })).expect("the clip is quantised");
    let outcome = session
        .execute("sound.clip.quantize", &json!({ "clips": [clips[0]] }))
        .expect("the clip is quantised again");
    assert!(!outcome.changed, "nothing changed");
}

/// A clip that would be put **before the beginning** is refused: the
/// tree clamps it at zero, and a clip whose beat is not on a line is
/// not on the grid.
#[test]
fn a_clip_that_would_start_before_the_beginning_is_refused() {
    let mut session = session();
    let (track, _) = two_clips(&mut session);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 24000 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .last()
        .map(|node| node.id)
        .expect("the clip is there");
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    // The beat is 4800 samples inside the clip, and the nearest line is
    // the one at zero: putting the beat there would put the clip's head
    // before the beginning.
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 4800 })).expect("the beat is set");
    let refused = session
        .execute("sound.clip.quantize", &json!({ "clips": [clip] }))
        .expect_err("the clip cannot start before the beginning");
    assert!(refused.to_string().contains("beginning"), "the error says why: {refused}");
    assert_eq!(start_of(&session, clip), 0, "the clip is where it was");
}

/// A clip whose **place** is locked does not move, and the command
/// refuses it.
#[test]
fn a_locked_clip_is_not_put_on_the_grid() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    session
        .execute("sound.clip.lock.time", &json!({ "clip": clips[1], "locked": true }))
        .expect("the clip is locked");
    let refused = session
        .execute("sound.clip.quantize", &json!({ "clips": [clips[1]] }))
        .expect_err("a locked clip does not move");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    assert_eq!(start_of(&session, clips[1]), 1000, "the clip is where it was");
}

/// Only a **clip** is put on the grid.
#[test]
fn only_a_clip_is_put_on_the_grid() {
    let mut session = session();
    let (track, clips) = two_clips(&mut session);
    let refused = session
        .execute("sound.clip.quantize", &json!({ "clips": [clips[0], track] }))
        .expect_err("a track is not a clip");
    assert!(refused.to_string().contains("not a clip"), "the error says why: {refused}");
}
