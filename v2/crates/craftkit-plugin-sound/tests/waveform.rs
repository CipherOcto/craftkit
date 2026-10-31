//! The **waveform**: the shape of the sound a clip plays.
//!
//! A clip was a bar and nothing else: a rectangle of ink that said
//! where a sound was and not what it was. This is the arc that draws
//! the sound — the **peaks** of the source, measured once, and the
//! shape they make on the bar.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-waveform-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A sound of that many frames, at that level.
fn a_sound(path: &str, frames: usize, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &vec![level; frames]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A source of that sound, measured or not.
fn a_source(session: &mut Session, path: &str) -> u64 {
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there")
}

/// The peaks of a source, as the payload holds them.
fn peaks(session: &Session, source: u64) -> Vec<f64> {
    session
        .document()
        .node(source)
        .and_then(|node| node.data.get("peaks"))
        .and_then(|peaks| peaks.as_array())
        .map(|peaks| peaks.iter().filter_map(|peak| peak.as_f64()).collect())
        .expect("the source has peaks")
}

/// The **peaks** are measured from the file: the shape of the sound in
/// a small table.
#[test]
fn the_peaks_are_measured_from_the_file() {
    let directory = directory("peaks");
    let path = format!("{directory}/tone.wav");
    a_sound(&path, 48_000, 0.5);
    let mut session = session();
    let source = a_source(&mut session, &path);
    assert!(peaks(&session, source).is_empty(), "nobody has measured the sound yet");
    session.execute("sound.source.peaks", &json!({ "source": source })).expect("the peaks are measured");
    let peaks = peaks(&session, source);
    assert_eq!(peaks.len(), 512, "two hundred and fifty-six buckets, a least and a most each");
    assert!((peaks[0] - 0.5).abs() < 1e-6, "the sound is half as loud as it can be");
    assert!((peaks[1] - 0.5).abs() < 1e-6, "and it is no louder than that");
    assert!((peaks[510] - 0.5).abs() < 1e-6, "the last bucket is the same sound");
}

/// A sound that is **shorter** than the table is silence for the rest
/// of it: silence is a shape too.
#[test]
fn a_short_sound_is_silence_for_the_rest_of_the_table() {
    let directory = directory("short");
    let path = format!("{directory}/short.wav");
    a_sound(&path, 100, 0.25);
    let mut session = session();
    let source = a_source(&mut session, &path);
    session.execute("sound.source.peaks", &json!({ "source": source })).expect("the peaks are measured");
    let peaks = peaks(&session, source);
    assert_eq!(peaks.len(), 512);
    assert!((peaks[0] - 0.25).abs() < 1e-6, "the sound is there");
    assert_eq!(peaks[510], 0.0, "and the rest of the table is silence");
}

/// A clip is drawn with the **shape of its sound**: the bar grows a
/// waveform when the source has been measured.
#[test]
fn a_clip_is_drawn_with_the_shape_of_its_sound() {
    let directory = directory("drawn");
    let path = format!("{directory}/tone.wav");
    a_sound(&path, 48_000, 0.5);
    let mut session = session();
    let source = a_source(&mut session, &path);
    session.execute("sound.track.add", &json!({ "name": "track" })).expect("the track is added");
    let track = session.document().node_by_name("track").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    // Before the measurement the clip is a bar and nothing else.
    let mut before = RecordingCanvas::default();
    session.paint(&mut before);
    let paths = before.drawn.iter().filter(|drawn| drawn.starts_with("path(")).count();
    session.execute("sound.source.peaks", &json!({ "source": source })).expect("the peaks are measured");
    let mut after = RecordingCanvas::default();
    session.paint(&mut after);
    let drawn = after.drawn.iter().filter(|drawn| drawn.starts_with("path(")).count();
    assert_eq!(drawn, paths + 1, "the waveform is one more shape on the bar: {:?}", after.drawn);
}

/// A source whose file is **not there** cannot be measured.
#[test]
fn a_source_that_cannot_be_read_is_refused() {
    let directory = directory("not-there");
    let path = format!("{directory}/nothing.wav");
    let mut session = session();
    let source = a_source(&mut session, &path);
    let error = session
        .execute("sound.source.peaks", &json!({ "source": source }))
        .expect_err("the file is not there");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
}

/// A node that is **not a source** is refused.
#[test]
fn a_node_that_is_not_a_source_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "track" })).expect("the track is added");
    let track = session.document().node_by_name("track").expect("the track is there").id;
    let error = session
        .execute("sound.source.peaks", &json!({ "source": track }))
        .expect_err("a track is not a source");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
}
