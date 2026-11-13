//! The **split** of a track into **mono** tracks.
//!
//! A stereo track is two channels that travel together; a track of one
//! channel is two things that can be heard apart.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-split-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stereo** sound of a second: the left channel says one thing and
/// the right channel another, so that the two can be told apart.
fn a_stereo_sound(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.wav");
    let mut samples: Vec<f32> = Vec::with_capacity(48_000 * 2);
    for frame in 0..48_000_u32 {
        samples.push(frame as f32 / 48_000.0);
        samples.push(-(frame as f32) / 48_000.0);
    }
    let bytes = craftkit_wav::write(2, 48_000, &samples);
    std::fs::write(&path, bytes).expect("the file is on the disk");
    path
}

/// A session with a **stereo** track and a clip of a stereo sound.
fn a_stereo_track(session: &mut Session, name: &str) -> (u64, u64) {
    let path = a_stereo_sound(name);
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 2, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": name, "channels": 2 })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// The **names** of the tracks of the document.
fn track_names(session: &Session) -> Vec<String> {
    let mut names: Vec<String> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.track")
        .map(|node| node.name.clone())
        .collect();
    names.sort();
    names
}

/// A stereo track is split into **two** tracks, one for each channel,
/// and they are named after the channel they hold.
#[test]
fn a_stereo_track_is_split_into_two() {
    let mut session = session();
    let (track, _) = a_stereo_track(&mut session, "stereo");
    let directory = directory("split-out");
    let outcome = session
        .execute("sound.track.split.mono", &json!({ "track": track, "directory": directory }))
        .expect("the track is split");
    assert_eq!(outcome.data["count"], 2);
    assert_eq!(
        track_names(&session),
        vec!["stereo".to_string(), "stereo.L".to_string(), "stereo.R".to_string()]
    );
}

/// The two new tracks are **panned to the sides**, so that the pair
/// says what the one track said.
#[test]
fn the_two_tracks_are_panned_to_the_sides() {
    let mut session = session();
    let (track, _) = a_stereo_track(&mut session, "panned");
    let directory = directory("panned-out");
    session
        .execute("sound.track.split.mono", &json!({ "track": track, "directory": directory }))
        .expect("the track is split");
    let pan_of = |name: &str| {
        session
            .document()
            .walk()
            .find(|node| node.kind == "sound.track" && node.name == name)
            .map(|node| node.data["pan"].as_f64().expect("the pan is there"))
            .expect("the track is there")
    };
    assert_eq!(pan_of("panned.L"), -1.0, "the left track is hard left");
    assert_eq!(pan_of("panned.R"), 1.0, "and the right one hard right");
}

/// Each new track plays **one channel** of the material: the file it
/// plays is mono and holds the channel of its own.
#[test]
fn each_track_plays_one_channel_of_the_material() {
    let mut session = session();
    let (track, _) = a_stereo_track(&mut session, "channels");
    let directory = directory("channels-out");
    session
        .execute("sound.track.split.mono", &json!({ "track": track, "directory": directory }))
        .expect("the track is split");
    // The **files** that were written: one for each channel.
    let mut files: Vec<String> = std::fs::read_dir(&directory)
        .expect("the directory is there")
        .filter_map(|entry| entry.ok().map(|entry| entry.path().to_string_lossy().to_string()))
        .filter(|path| path.ends_with(".wav"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 2, "there is a file for each channel: {files:?}");
    // The **left** file holds the left channel: the ramp, and not its
    // opposite.
    let bytes = std::fs::read(&files[0]).expect("the file is on the disk");
    let sound = craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the file is a sound");
    assert_eq!(sound.channels, 1, "the file is mono");
    assert!(sound.samples[0] >= 0.0, "the left channel says the ramp: {:?}", &sound.samples[..4]);
}

/// The **original** track is left where it is: it is the caller's to
/// mute, and the tree leaves it too.
#[test]
fn the_original_track_is_left_where_it_is() {
    let mut session = session();
    let (track, _) = a_stereo_track(&mut session, "original");
    let directory = directory("original-out");
    session
        .execute("sound.track.split.mono", &json!({ "track": track, "directory": directory }))
        .expect("the track is split");
    assert!(
        session.document().walk().any(|node| node.kind == "sound.track" && node.name == "original"),
        "the original track is still there"
    );
}

/// A track that is **not** stereo is refused: there is nothing to
/// split.
#[test]
fn a_mono_track_is_refused() {
    let mut session = session();
    session
        .execute("sound.track.add", &json!({ "name": "mono", "channels": 1 }))
        .expect("the track is added");
    let track = session.document().node_by_name("mono").expect("the track is there").id;
    let directory = directory("mono-out");
    let error = session
        .execute("sound.track.split.mono", &json!({ "track": track, "directory": directory }))
        .expect_err("the track is refused");
    assert!(error.to_string().contains("nothing to split"), "the error says why: {error}");
}
