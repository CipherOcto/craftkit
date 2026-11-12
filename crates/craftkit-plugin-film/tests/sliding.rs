//! **Sliding** a clip between its neighbours.
//!
//! A slide moves a clip and the clips around it give way, so that the
//! span they share stays without a gap.

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
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64, source_in: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": source_in,
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

/// A slide moves the clip and the clips around it give way: the one
/// before it grows and the one after it moves and shrinks.
#[test]
fn a_slide_moves_the_clip_and_its_neighbours() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    let after = a_clip(&mut session, track, item, 40, 20, 40);
    session.execute("film.clip.slide", &json!({ "clip": middle, "delta": 5 })).expect("the clip is slid");
    assert_eq!(clock_of(&session, before), (0, 25, 0), "the clip before it gave way");
    assert_eq!(clock_of(&session, middle), (25, 20, 20), "the clip slid and kept its media");
    assert_eq!(clock_of(&session, after), (45, 15, 45), "the clip after it moved and shrank");
}

/// The span stays **without a gap**: the clip before the slid one ends
/// where the slid one begins, and the slid one ends where the clip
/// after it begins.
#[test]
fn the_span_stays_without_a_gap() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    let after = a_clip(&mut session, track, item, 40, 20, 40);
    session.execute("film.clip.slide", &json!({ "clip": middle, "delta": -7 })).expect("the clip is slid");
    let (before_start, before_duration, _) = clock_of(&session, before);
    let (middle_start, middle_duration, _) = clock_of(&session, middle);
    let (after_start, _after_duration, _) = clock_of(&session, after);
    assert_eq!(before_start + before_duration, middle_start, "the first gap is closed");
    assert_eq!(middle_start + middle_duration, after_start, "and so is the second");
}

/// A slide the other way: the clip before it shrinks and the clip after
/// it grows and takes the media before it.
#[test]
fn a_slide_may_go_backwards() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    let after = a_clip(&mut session, track, item, 40, 20, 40);
    session.execute("film.clip.slide", &json!({ "clip": middle, "delta": -5 })).expect("the clip is slid");
    assert_eq!(clock_of(&session, before), (0, 15, 0), "the clip before it shrank");
    assert_eq!(clock_of(&session, middle), (15, 20, 20), "the clip slid back");
    assert_eq!(clock_of(&session, after), (35, 25, 35), "and the clip after it grew");
}

/// A slide that would leave a neighbour **shorter than a frame** is
/// refused.
#[test]
fn a_slide_past_the_end_of_a_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": -20 }))
        .expect_err("the slide is refused");
    assert_eq!(clock_of(&session, before), (0, 20, 0), "and the clips are unchanged");
    assert_eq!(clock_of(&session, middle), (20, 20, 20), "and the clips are unchanged");
}

/// A slide that would run the clip before it **past the end of the
/// media** is refused, and the refusal says how long the media is.
#[test]
fn a_slide_past_the_end_of_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": 90 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("the media is 100 frames long"), "the error says so: {error}");
    assert_eq!(clock_of(&session, before), (0, 20, 0), "and the clips are unchanged");
}

/// A slide that would take the clip after it **before the media
/// begins** is refused.
#[test]
fn a_slide_before_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let middle = a_clip(&mut session, track, item, 30, 20, 20);
    let after = a_clip(&mut session, track, item, 50, 20, 20);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": -25 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("before the media begins"), "the error says so: {error}");
    assert_eq!(clock_of(&session, after), (50, 20, 20), "and the clips are unchanged");
}

/// A slide that would start the clip **before the timeline** is
/// refused.
#[test]
fn a_slide_before_the_timeline_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let middle = a_clip(&mut session, track, item, 5, 20, 20);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": -10 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("the timeline begins at frame 0"), "the error says so: {error}");
}

/// A neighbour that is **not next to** the slid clip is a bound: the
/// clip may not run into it.
#[test]
fn a_gap_is_a_bound() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 30, 20, 20);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": -15 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("the clip before it ends at frame 20"), "the error says so: {error}");
    assert_eq!(clock_of(&session, before), (0, 20, 0), "and the clips are unchanged");
}

/// A clip that is **locked** on its track may not be slid.
#[test]
fn a_slide_on_a_locked_track_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let _before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": 5 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("is locked"), "the error says so: {error}");
    assert_eq!(clock_of(&session, middle), (20, 20, 20), "and the clips are unchanged");
}

/// The **link** is honoured: the linked clips slide together, so that
/// the picture and the sound stay in step.
#[test]
fn a_linked_clip_slides_too() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let sound_track = outcome.data["id"].as_u64().expect("the track has an id");
    let picture = a_clip(&mut session, track, item, 20, 20, 20);
    let sound = a_clip(&mut session, sound_track, item, 20, 20, 20);
    session
        .execute("film.clip.link", &json!({ "clip": picture, "with": sound }))
        .expect("the clips are linked");
    session.execute("film.clip.slide", &json!({ "clip": picture, "delta": 5 })).expect("the clip is slid");
    assert_eq!(clock_of(&session, sound), (25, 20, 20), "the linked clip slid too");
}

/// A linked clip that **cannot** take the slide refuses the whole
/// slide: the tree scales the group down and says nothing.
#[test]
fn a_linked_clip_that_cannot_slide_refuses_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let sound_track = outcome.data["id"].as_u64().expect("the track has an id");
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let picture = a_clip(&mut session, track, item, 20, 20, 20);
    let sound = a_clip(&mut session, sound_track, item, 20, 20, 20);
    session
        .execute("film.clip.link", &json!({ "clip": picture, "with": sound }))
        .expect("the clips are linked");
    // The clip before the picture would run past the end of its media,
    // so the picture cannot be slid so far.
    let error = session
        .execute("film.clip.slide", &json!({ "clip": picture, "delta": 500 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("the media"), "the error says so: {error}");
    assert_eq!(clock_of(&session, picture), (20, 20, 20), "and the clips are unchanged");
    assert_eq!(clock_of(&session, before), (0, 20, 0), "and the clips are unchanged");
}

/// A slide of **nothing** is refused.
#[test]
fn a_slide_of_nothing_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": 0 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("zero frames"), "the error says so: {error}");
}

/// A slide that is not a whole number of frames is refused.
#[test]
fn a_slide_of_half_a_frame_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": middle, "delta": 1.5 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("whole number"), "the error says so: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let error = session
        .execute("film.clip.slide", &json!({ "clip": track, "delta": 5 }))
        .expect_err("the slide is refused");
    assert!(error.to_string().contains("not a clip"), "the error says so: {error}");
    assert_eq!(item, item, "the item is where it was");
}

/// The slide is **saved**: it is in the document after it is done.
#[test]
fn the_slide_is_saved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 0);
    let middle = a_clip(&mut session, track, item, 20, 20, 20);
    session.execute("film.clip.slide", &json!({ "clip": middle, "delta": 5 })).expect("the clip is slid");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(middle).expect("the clip is in the document");
    assert_eq!(node.data["start"].as_u64(), Some(25), "and it is where it was slid");
    let node = saved.node(before).expect("the neighbour is in the document");
    assert_eq!(node.data["duration"].as_u64(), Some(25), "and so is its neighbour");
}
