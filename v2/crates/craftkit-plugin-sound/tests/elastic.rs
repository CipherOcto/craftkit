//! The **elastic track**: a clip that is longer and the same note.
//!
//! A **varispeed** plays the sound faster and the pitch falls with it.
//! A **stretch** makes the sound longer and the pitch stays, and it is
//! the difference between a record played at the wrong speed and a
//! record played slowly.
//!
//! The tree keeps the difference in two places: an algorithm on the
//! track, and a warp on the clip that is baked into a rendered file.
//! Here the algorithm is on the track and the stretch is one field on
//! the clip, and what plays is derived from the two.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-elastic-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
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

/// A session with a track called `drums`, a tone as its source, and a
/// clip of a second on it.
fn a_clip(session: &mut Session, source_path: &str) -> (u64, u64) {
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
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The **pitch** of the sound: the number of samples between its
/// upward crossings.
///
/// The crossings are counted and the gaps averaged: a tone crosses at
/// a steady rate, and a stretched one crosses at the same rate when
/// the pitch is kept.
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

/// A track carries an **elastic algorithm**.
#[test]
fn a_track_carries_its_elastic_algorithm() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    let data = session.document().node(track).expect("the track is there").data.clone();
    assert_eq!(
        data.get("elastic").and_then(|elastic| elastic.as_str()),
        Some("polyphonic"),
        "the track carries the algorithm"
    );
}

/// The algorithm `none` takes the elastic **away**.
#[test]
fn the_algorithm_none_takes_the_elastic_away() {
    let mut session = session();
    let directory = directory("none");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "none" }))
        .expect("the elastic is taken away");
    let data = session.document().node(track).expect("the track is there").data.clone();
    assert_eq!(data.get("elastic"), Some(&json!(null)), "the track is not elastic");
}

/// An algorithm that is **not a name** is refused: an empty name says
/// nothing, and the name `none` is how the elastic is taken away.
#[test]
fn an_algorithm_that_is_not_a_name_is_refused() {
    let mut session = session();
    let directory = directory("empty");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.elastic", &json!({ "track": track, "algorithm": "  " }));
    assert!(refused.is_err(), "an algorithm is a name");
}

/// A track that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (_, clip) = a_clip(&mut session, &source_path);
    let refused =
        session.execute("sound.track.elastic", &json!({ "track": clip, "algorithm": "polyphonic" }));
    assert!(refused.is_err(), "a clip is not a track");
}

/// An **elastic** clip keeps its **pitch** when it is stretched: the
/// sound is longer and it is the same note.
#[test]
fn an_elastic_clip_keeps_its_pitch() {
    let directory = directory("keeps");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let (track, clip) = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let after = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 48_000.0 / 440.0;
    assert!((after - expected).abs() < expected * 0.02, "the same pitch: {after} and the tone's {expected}");
}

/// An ordinary clip **drops** its pitch when it is stretched: the
/// source is read more slowly and the note falls with it.
#[test]
fn an_ordinary_clip_drops_its_pitch() {
    let directory = directory("drops");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let (_, clip) = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let period = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 2.0 * 48_000.0 / 440.0;
    assert!(
        (period - expected).abs() < expected * 0.02,
        "the pitch fell: {period} and the tone's {expected}"
    );
}

/// The algorithm `varispeed` says that the pitch **falls**: it is the
/// ordinary track with a name on it.
#[test]
fn the_algorithm_varispeed_drops_the_pitch() {
    let directory = directory("varispeed");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let (track, clip) = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "varispeed" }))
        .expect("the track is varispeed");
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let period = pitch(&sound.samples[..sound.frames as usize]);
    let expected = 2.0 * 48_000.0 / 440.0;
    assert!(
        (period - expected).abs() < expected * 0.02,
        "the pitch fell: {period} and the tone's {expected}"
    );
}

/// An elastic stretch that is **too large** is refused: a stretch of a
/// hundredfold would ask for a frame every hundredth of a window, and
/// the sound would be a noise. It is refused and not pulled to the
/// edge, because a caller that asked for a hundredfold would be given
/// a tenthfold one and would not know.
#[test]
fn an_elastic_stretch_that_is_too_large_is_refused() {
    let mut session = session();
    let directory = directory("too-large");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, clip) = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    let refused = session.execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 20.0 }));
    assert!(refused.is_err(), "an elastic stretch is bounded");
}

/// An elastic stretch that is **too small** is refused, for the same
/// reason and at the other end.
#[test]
fn an_elastic_stretch_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("too-small");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, clip) = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    let refused = session.execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 0.05 }));
    assert!(refused.is_err(), "an elastic stretch is bounded");
}

/// The bound is the **elastic** one and not the ordinary one: an
/// ordinary track may be stretched a hundredfold, because a varispeed
/// has no trouble with it.
#[test]
fn an_ordinary_track_may_be_stretched_further() {
    let mut session = session();
    let directory = directory("ordinary");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (_, clip) = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 20.0 }))
        .expect("the clip is stretched");
    let payload = &session.document().node(clip).expect("the clip is there").data;
    assert_eq!(
        payload.get("stretch").and_then(|stretch| stretch.as_f64()),
        Some(20.0),
        "the clip is stretched"
    );
}

/// A clip **added** to an elastic track is refused a stretch outside
/// the bound: the clip would be one that cannot be played.
#[test]
fn a_clip_added_to_an_elastic_track_is_refused_a_stretch_outside_the_bound() {
    let mut session = session();
    let directory = directory("added");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
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
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    let refused = session.execute(
        "sound.clip.add",
        &json!({ "track": track, "start": 0, "length": 48_000, "source": source, "stretch": 20.0 }),
    );
    assert!(refused.is_err(), "an elastic stretch is bounded");
}

/// A clip **added** to an elastic track with a stretch inside the
/// bound is added, and it is stretched.
#[test]
fn a_clip_added_to_an_elastic_track_is_stretched() {
    let mut session = session();
    let directory = directory("added-inside");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
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
        .execute("sound.track.elastic", &json!({ "track": track, "algorithm": "polyphonic" }))
        .expect("the track is elastic");
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 0, "length": 48_000, "source": source, "stretch": 2.0 }),
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
        Some(2.0),
        "the clip is stretched"
    );
}
