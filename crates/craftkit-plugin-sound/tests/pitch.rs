//! The **pitch** of a clip: the same sound and another note.
//!
//! A stretch makes the sound longer and the pitch stays; a pitch shift
//! moves the pitch and the sound stays as long as it was. The tree
//! does the shift as a **process**: it renders the material and
//! replaces the clip with the rendered file. Here the shift is a field
//! of the clip and the sound is worked out when it is played.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-pitch-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **tone** of that frequency, a second long.
fn a_tone(path: &str, frequency: f32) {
    let frames = 48_000;
    let samples: Vec<f32> = (0..frames)
        .map(|frame| {
            let t = frame as f64 / 48_000.0;
            (2.0 * std::f64::consts::PI * f64::from(frequency) * t).sin() as f32
        })
        .collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and a tone on it.
fn a_clip(session: &mut Session, source_path: &str) -> u64 {
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
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

/// The **pitch** of the sound: the number of samples between its
/// upward crossings.
fn pitch(samples: &[f32]) -> f64 {
    let mut crossings = Vec::new();
    for index in 1..samples.len() {
        if samples[index - 1] <= 0.0 && samples[index] > 0.0 {
            crossings.push(index);
        }
    }
    assert!(crossings.len() >= 10, "the sound crosses zero often: {}", crossings.len());
    let total: usize = crossings.windows(2).map(|pair| pair[1] - pair[0]).sum();
    total as f64 / (crossings.len() - 1) as f64
}

/// The pitch a clip carries.
fn pitch_of(session: &Session, clip: u64) -> Option<f64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("pitch_semitones").and_then(|pitch| pitch.as_f64())
}

/// A clip carries a **pitch**, and it is the pitch of the source.
#[test]
fn a_clip_carries_its_pitch() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let clip = a_clip(&mut session, &source_path);
    assert_eq!(pitch_of(&session, clip), Some(0.0), "the pitch is the source's");
}

/// A pitch is **set** on a clip.
#[test]
fn a_pitch_is_set_on_the_clip() {
    let mut session = session();
    let directory = directory("set");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": 12.0 }))
        .expect("the pitch is set");
    assert_eq!(pitch_of(&session, clip), Some(12.0), "the pitch is set");
}

/// A pitch that is **out of range** is refused: the tree's shift runs
/// to twenty-four semitones either way, and a shift outside that is
/// refused and not pulled to the edge.
#[test]
fn a_pitch_that_is_out_of_range_is_refused() {
    let mut session = session();
    let directory = directory("out-of-range");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let clip = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": 100.0 }));
    assert!(refused.is_err(), "a pitch is bounded");
    assert_eq!(pitch_of(&session, clip), Some(0.0), "and the pitch did not move");
}

/// A pitch that is **not a number** is refused.
#[test]
fn a_pitch_that_is_not_a_number_is_refused() {
    let mut session = session();
    let directory = directory("not-a-number");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let clip = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": "high" }));
    assert!(refused.is_err(), "a pitch is a number");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let directory = directory("not-a-clip");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let clip = a_clip(&mut session, &source_path);
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    let refused = session.execute("sound.clip.pitch", &json!({ "clip": track, "semitones": 12.0 }));
    assert!(refused.is_err(), "a track is not a clip");
    assert!(session.document().node(clip).is_some(), "the clip is still there");
}

/// A clip whose **content** is locked is finished, and it is not
/// shifted.
#[test]
fn a_locked_clip_is_not_shifted() {
    let mut session = session();
    let directory = directory("locked");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clip, "locked": true }))
        .expect("the clip is locked");
    let refused = session.execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": 12.0 }));
    assert!(refused.is_err(), "a locked clip is finished");
}

/// The pitch is **heard**: the clip is an octave up and it is as long
/// as it was.
#[test]
fn the_pitch_is_heard_and_the_length_is_the_same() {
    let directory = directory("heard");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": 12.0 }))
        .expect("the pitch is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 48_000, "the clip is as long as it says: {}", sound.frames);
    let period = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 48_000.0 / 880.0;
    assert!((period - expected).abs() < expected * 0.02, "the octave above: {period} and {expected}");
}

/// A pitch **down** is heard too, and the length is the same.
#[test]
fn a_pitch_down_is_heard_and_the_length_is_the_same() {
    let directory = directory("down");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": -12.0 }))
        .expect("the pitch is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 48_000, "the clip is as long as it says: {}", sound.frames);
    let period = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 48_000.0 / 220.0;
    assert!((period - expected).abs() < expected * 0.02, "the octave below: {period} and {expected}");
}

/// A pitch of **zero** is the sound of the source.
#[test]
fn a_pitch_of_zero_is_the_sound_of_the_source() {
    let directory = directory("zero");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": 0.0 }))
        .expect("the pitch is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let period = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 48_000.0 / 440.0;
    assert!((period - expected).abs() < expected * 0.02, "the same note: {period} and {expected}");
}

/// The pitch and the **stretch** are two things: a clip that is both
/// stretched and shifted is longer in pitch and the same in length.
#[test]
fn the_pitch_and_the_stretch_are_two_things() {
    let directory = directory("both");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session
        .execute("sound.clip.pitch", &json!({ "clip": clip, "semitones": 12.0 }))
        .expect("the pitch is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 48_000, "the clip is as long as it says: {}", sound.frames);
    // The stretch halved the pitch and the shift raised it again, so
    // the note is the one the source started on.
    let period = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 48_000.0 / 440.0;
    assert!((period - expected).abs() < expected * 0.03, "the same note: {period} and {expected}");
}
