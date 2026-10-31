//! The **fade**: the curve from silence to full at the ends of a clip.
//!
//! §144 found that a clip could be placed, trimmed and stretched, and
//! that there was no way to say that it should come up slowly or go
//! down slowly. This is the arc that gives it one — a curve, a shape
//! and a bend, in the document and in the sound.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-fade-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A sound of that many frames, all of them full.
fn a_sound(path: &str, frames: usize) {
    let bytes = craftkit_wav::write(1, 48_000, &vec![1.0_f32; frames]);
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

/// A clip carries its **fades**, and they are no fades at all.
#[test]
fn a_clip_carries_its_fades() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let payload = &session.document().node(clip).expect("the clip is there").data;
    let fade = payload.get("fade_in").expect("the clip has a fade in");
    assert_eq!(fade.get("len").and_then(|len| len.as_i64()), Some(0), "and it is no fade at all");
}

/// A fade is **set** on a clip.
#[test]
fn a_fade_is_set_on_the_clip() {
    let mut session = session();
    let directory = directory("set");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.fade", &json!({ "clip": clip, "fade_in": 2, "shape": "linear" }))
        .expect("the clip is faded");
    let payload = &session.document().node(clip).expect("the clip is there").data;
    let fade = payload.get("fade_in").expect("the clip has a fade in");
    assert_eq!(fade.get("len").and_then(|len| len.as_i64()), Some(2), "the fade is two samples long");
    assert_eq!(fade.get("shape").and_then(|shape| shape.as_str()), Some("linear"), "and it is straight");
}

/// A fade **longer than the clip** is refused.
///
/// A fade that does not finish is a fade that is not there, and a fade
/// that is quietly shortened is a fade the caller did not ask for.
#[test]
fn a_fade_longer_than_the_clip_is_refused() {
    let mut session = session();
    let directory = directory("too-long");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let refused = session.execute("sound.clip.fade", &json!({ "clip": clip, "fade_in": 5 }));
    assert!(refused.is_err(), "a fade cannot be longer than its clip");
}

/// A fade with **no length** is refused: there is nothing to fade.
#[test]
fn a_fade_with_no_length_is_refused() {
    let mut session = session();
    let directory = directory("no-length");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let refused = session.execute("sound.clip.fade", &json!({ "clip": clip }));
    assert!(refused.is_err(), "a fade needs a length");
}

/// A **shape that is not a shape** is refused, and it is refused with
/// its name.
#[test]
fn a_shape_that_is_not_a_shape_is_refused() {
    let mut session = session();
    let directory = directory("no-shape");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    let refused =
        session.execute("sound.clip.fade", &json!({ "clip": clip, "fade_in": 2, "shape": "wobble" }));
    let error = refused.expect_err("the shape is refused").to_string();
    assert!(error.contains("wobble"), "and the name is in the refusal: {error}");
}

/// A faded clip **starts quiet**.
#[test]
fn a_faded_clip_starts_quiet() {
    let directory = directory("starts-quiet");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    a_sound(&source_path, 4);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.fade", &json!({ "clip": clip, "fade_in": 2, "shape": "linear" }))
        .expect("the clip is faded");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 4, "the fade does not change the length");
    assert!((sound.samples[0] - 0.0).abs() < 1e-4, "the first sample is nothing: {}", sound.samples[0]);
    assert!((sound.samples[1] - 0.5).abs() < 1e-4, "the second is halfway: {}", sound.samples[1]);
    assert!((sound.samples[2] - 1.0).abs() < 1e-4, "and the third is full");
}

/// A faded clip **ends quiet**.
#[test]
fn a_faded_clip_ends_quiet() {
    let directory = directory("ends-quiet");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    a_sound(&source_path, 4);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 4);
    session
        .execute("sound.clip.fade", &json!({ "clip": clip, "fade_out": 2, "shape": "linear" }))
        .expect("the clip is faded");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 1.0).abs() < 1e-4, "the first sample is full");
    assert!((sound.samples[3] - 0.5).abs() < 1e-4, "and the last is halfway down: {}", sound.samples[3]);
}

/// The **default shape** is the equal-power one.
///
/// The curve is quiet for longer, which is what a fade in a mix wants.
#[test]
fn the_default_shape_is_the_equal_power_one() {
    let mut session = session();
    let directory = directory("default-shape");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4);
    let clip = a_clip(&mut session, &source_path, 4);
    session.execute("sound.clip.fade", &json!({ "clip": clip, "fade_in": 2 })).expect("the clip is faded");
    let payload = &session.document().node(clip).expect("the clip is there").data;
    let fade = payload.get("fade_in").expect("the clip has a fade in");
    assert_eq!(
        fade.get("shape").and_then(|shape| shape.as_str()),
        Some("equal_power"),
        "the shape is the default one"
    );
}

/// The fade is **drawn**.
///
/// The quiet part of a clip is a notch in its bar: the paper showing
/// through the ink, and the notch is the fade's own curve.
#[test]
fn the_fade_is_drawn() {
    let mut session = session();
    let directory = directory("drawn");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4_800);
    let clip = a_clip(&mut session, &source_path, 4_800);
    session
        .execute("sound.clip.fade", &json!({ "clip": clip, "fade_in": 2_400, "fade_out": 2_400 }))
        .expect("the clip is faded");
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    // The notch is a **filled** path: the playhead is a stroked one.
    let notches =
        canvas.drawn.iter().filter(|drawn| drawn.starts_with("path(") && drawn.contains("filled")).count();
    assert_eq!(notches, 2, "the clip has two notches: {:?}", canvas.drawn);
}

/// A clip with **no fade** is drawn with no notch.
#[test]
fn a_clip_with_no_fade_is_drawn_with_no_notch() {
    let mut session = session();
    let directory = directory("no-notch");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4_800);
    a_clip(&mut session, &source_path, 4_800);
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    let notches =
        canvas.drawn.iter().filter(|drawn| drawn.starts_with("path(") && drawn.contains("filled")).count();
    assert_eq!(notches, 0, "the clip is a plain bar: {:?}", canvas.drawn);
}
