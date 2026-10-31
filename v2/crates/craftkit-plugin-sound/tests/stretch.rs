//! The **stretch**: a clip played at another speed.
//!
//! §143 found that a clip's speed was not a thing the document could
//! say. This is the arc that gives it one — a **varispeed**, which is
//! not a time stretch: the pitch follows the speed, and the clip keeps
//! its place and its length.

use craftkit_sdk::document::RecordingCanvas;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
///
/// A test binary that named nothing of the battery would drop it, and
/// the battery would be missing in a way that looks like a bug in the
/// test.
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-stretch-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A sound of that many frames, one cycle of a wave: up, down, up,
/// down.
///
/// A long sound is a ramp, because a clip that is drawn has to be long
/// enough to be drawn: the bar of a clip has a **minimum width**, and a
/// clip shorter than that is drawn at the minimum.
fn a_wave(path: &str, frames: usize) {
    let samples: Vec<f32> = if frames <= 4 {
        vec![0.0, 1.0, 0.0, -1.0]
    } else {
        (0..frames).map(|index| (index as f32) / (frames as f32)).collect()
    };
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track, a source and a clip of that many frames.
fn a_clip(session: &mut Session, source_path: &str, length: i64) -> u64 {
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": length, "source": source }))
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

/// A clip carries its **stretch**, and one is no stretch.
#[test]
fn a_clip_carries_its_stretch() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let payload = &session.document().node(clip).expect("the clip is there").data;
    assert_eq!(
        payload.get("stretch").and_then(|stretch| stretch.as_f64()),
        Some(1.0),
        "a clip is not stretched"
    );
}

/// A stretch is **set** on a clip.
#[test]
fn a_stretch_is_set_on_the_clip() {
    let mut session = session();
    let directory = directory("set");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    let payload = &session.document().node(clip).expect("the clip is there").data;
    assert_eq!(
        payload.get("stretch").and_then(|stretch| stretch.as_f64()),
        Some(2.0),
        "the clip is stretched"
    );
}

/// A clip may be added **already stretched**.
#[test]
fn a_clip_may_be_added_already_stretched() {
    let mut session = session();
    let directory = directory("added");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
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
            &json!({ "track": track, "start": 0, "length": 4, "source": source, "stretch": 0.5 }),
        )
        .expect("the clip is added");
    let payload = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.data.clone())
        .expect("the clip is there");
    assert_eq!(
        payload.get("stretch").and_then(|stretch| stretch.as_f64()),
        Some(0.5),
        "the clip is stretched"
    );
}

/// A stretch that is **not a number** is refused.
///
/// The number came from outside, and a stretch that makes no sense is
/// not a reason to lose the clip.
#[test]
fn a_stretch_that_is_not_a_number_is_refused() {
    let mut session = session();
    let directory = directory("not-a-number");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let refused = session.execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": "half" }));
    assert!(refused.is_err(), "a stretch is a number");
}

/// A stretch of **zero** is refused: a clip that plays nothing is not a
/// clip.
#[test]
fn a_stretch_of_zero_is_refused() {
    let mut session = session();
    let directory = directory("zero");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let refused = session.execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 0.0 }));
    assert!(refused.is_err(), "a clip plays something");
}

/// A stretch that is **too large** is refused: a clip stretched a
/// thousandfold would hold a thousandth of a sample for a thousand
/// times as long.
#[test]
fn a_stretch_that_is_too_large_is_refused() {
    let mut session = session();
    let directory = directory("too-large");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let refused = session.execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 1000.0 }));
    assert!(refused.is_err(), "a stretch is bounded");
}

/// A stretched clip is **longer and lower**.
///
/// The clip keeps its length — the stretch says how the source is
/// played, and not how long the clip is — and the source is read more
/// slowly, so the pitch falls with it.
#[test]
fn a_stretched_clip_is_longer_and_lower() {
    let directory = directory("lower");
    let source_path = format!("{directory}/wave.wav");
    let output_path = format!("{directory}/out.wav");
    a_wave(&source_path, 4);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 4, "the clip is as long as it says: {}", sound.frames);
    // The source is read at half the rate: the second frame is the
    // line between the source's first and second.
    assert!((sound.samples[1] - 0.5).abs() < 1e-4, "the sound is lower: {}", sound.samples[1]);
}

/// A stretch of **one** is no stretch at all.
#[test]
fn a_stretch_of_one_is_no_stretch() {
    let directory = directory("one");
    let source_path = format!("{directory}/wave.wav");
    let output_path = format!("{directory}/out.wav");
    a_wave(&source_path, 4);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 1.0 }))
        .expect("the clip is stretched");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 4, "the clip is as long as it says");
    assert!((sound.samples[1] - 1.0).abs() < 1e-4, "the sound is the source's: {}", sound.samples[1]);
}

/// A trimmed stretched clip moves its content by the delta **divided by
/// the stretch**.
///
/// The source of a stretched clip is read more slowly, so the same
/// number of samples on the timeline is a smaller number of samples
/// in the source — and the two are the same fact.
#[test]
fn a_trimmed_stretched_clip_moves_its_content_by_the_delta_divided_by_the_stretch() {
    let mut session = session();
    let directory = directory("trimmed");
    let source_path = format!("{directory}/wave.wav");
    a_wave(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session
        .execute("sound.clip.trim", &json!({ "clip": clip, "from": 2, "to": 4 }))
        .expect("the clip is trimmed");
    let payload = &session.document().node(clip).expect("the clip is there").data;
    assert_eq!(
        payload.get("offset").and_then(|offset| offset.as_i64()),
        Some(1),
        "the content moved by the delta over the stretch"
    );
}

/// The **picture** and the **sound** agree.
///
/// A stretched clip is drawn as long as it is heard: the stretch says
/// how the source is played, and not how long the clip is.
#[test]
fn the_picture_and_the_sound_agree() {
    let directory = directory("agree");
    let source_path = format!("{directory}/wave.wav");
    let output_path = format!("{directory}/out.wav");
    // A clip of a tenth of a second: long enough for the bar to be
    // drawn at its own width and not at the minimum.
    a_wave(&source_path, 4_800);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 4_800);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    let bar = canvas.drawn.iter().find(|drawn| drawn.starts_with("rect(")).expect("the clip is a bar");
    // The bar is drawn in seconds, and the sound is in samples.
    let width: f64 = bar
        .trim_start_matches("rect(")
        .split(',')
        .nth(2)
        .and_then(|width| width.trim().parse().ok())
        .expect("the bar has a width");
    assert_eq!(sound.frames, 4_800, "the clip is as long as it says");
    assert!(
        (width - sound.frames as f64 / 48_000.0).abs() < 1e-9,
        "the bar is {width} and the sound is {} frames",
        sound.frames
    );
}
