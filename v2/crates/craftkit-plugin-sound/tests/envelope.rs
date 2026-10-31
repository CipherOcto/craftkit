//! The **clip's own curve**: a lane that automates a clip.
//!
//! §147 found that a clip had a gain and a track had a curve, and that
//! the family's clip carried a curve of its own. This is the arc that
//! gives it one: a lane is under the node it automates, and that node
//! may be a clip.

use craftkit_sdk::document::RecordingCanvas;
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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-envelope-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A session with a track, a source of that many full samples, and a
/// clip of that many frames at that place.
fn a_clip(session: &mut Session, source_path: &str, frames: usize, start: i64) -> u64 {
    let bytes = craftkit_wav::write(1, 48_000, &vec![1.0_f32; frames]);
    std::fs::write(source_path, bytes).expect("the sound is written");
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": start, "length": frames, "source": source }),
        )
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// A lane may automate a **clip**.
#[test]
fn a_lane_may_automate_a_clip() {
    let mut session = session();
    let directory = directory("clip");
    let source_path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &source_path, 48_000, 0);
    session
        .execute(
            "sound.lane.add",
            &json!({ "clip": clip, "param": "volume", "points": [[0, -60.0], [48_000, 0.0]] }),
        )
        .expect("the clip is automated");
    let lane = session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there");
    let owner = session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == lane.id))
        .expect("the lane is under something");
    assert_eq!(owner.id, clip, "and the lane is under the clip");
}

/// A lane automates **one** thing.
///
/// A track and a clip at the same time is two things, and no thing at
/// all is nothing.
#[test]
fn a_lane_automates_one_thing() {
    let mut session = session();
    let directory = directory("one-thing");
    let source_path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &source_path, 48_000, 0);
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    let both = session.execute("sound.lane.add", &json!({ "track": track, "clip": clip, "param": "volume" }));
    assert!(both.is_err(), "a lane automates one thing");
    let neither = session.execute("sound.lane.add", &json!({ "param": "volume" }));
    assert!(neither.is_err(), "and it automates something");
}

/// The clip's curve is **heard**.
///
/// The curve is the clip's own: its times are the clip's times, and it
/// moves the clip's sound and not the track's.
#[test]
fn the_clips_curve_is_heard() {
    let directory = directory("heard");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 48_000, 0);
    session
        .execute(
            "sound.lane.add",
            &json!({ "clip": clip, "param": "volume", "points": [[0, -60.0], [48_000, 0.0]] }),
        )
        .expect("the clip is automated");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!(sound.samples[0].abs() < 0.01, "the clip starts quiet: {}", sound.samples[0]);
    assert!(sound.samples[47_999].abs() > 0.9, "and ends loud: {}", sound.samples[47_999]);
}

/// The clip's curve is **drawn**, on the clip's bar.
///
/// The curve's times are the clip's own, so it is drawn where the clip
/// is and not where the track's timeline is.
#[test]
fn the_clips_curve_is_drawn_on_the_clips_bar() {
    let directory = directory("drawn");
    let source_path = format!("{directory}/sound.wav");
    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 48_000, 24_000);
    session
        .execute(
            "sound.lane.add",
            &json!({
                "clip": clip,
                "param": "volume",
                "points": [[0, -60.0], [24_000, -20.0], [48_000, 0.0]],
            }),
        )
        .expect("the clip is automated");
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    // The curve has three points and so two segments; the playhead is
    // a line of one.
    let curve = canvas.drawn.iter().any(|drawn| drawn.starts_with("path(2 segment(s)"));
    assert!(curve, "the curve is drawn: {:?}", canvas.drawn);
    // The lane's **place**: the clip starts half a second in, so the
    // curve is drawn from there and not from the beginning.
    let lane = session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there");
    let place = session.place_of(&lane).expect("the lane has a place");
    assert!((place.x - 0.5).abs() < 1e-9, "the curve starts with the clip: {}", place.x);
}

/// A **track's** lane is still the track's.
///
/// The times of a track's lane are the track's timeline, and the curve
/// is drawn from the beginning and not from a clip's place.
#[test]
fn a_tracks_lane_is_still_the_tracks() {
    let mut session = session();
    let directory = directory("track");
    let source_path = format!("{directory}/sound.wav");
    a_clip(&mut session, &source_path, 48_000, 24_000);
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute(
            "sound.lane.add",
            &json!({ "track": track, "param": "volume", "points": [[0, -60.0], [48_000, 0.0]] }),
        )
        .expect("the track is automated");
    let lane = session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there");
    let place = session.place_of(&lane).expect("the lane has a place");
    assert!((place.x - 0.0).abs() < 1e-9, "the curve starts at the track's beginning: {}", place.x);
}
