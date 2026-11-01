//! The **stems**: one file for each track.
//!
//! A stem is the mix with **one track in it**. The tree's bounce solos
//! the track and renders the whole mix, so a stem carries the master
//! faders and the metronome with it — and here the mix is worked out
//! **once** and the stem is that same mix with the other tracks out,
//! so a stem is not a second way of mixing.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-stems-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of that level: one frame, so that a stem is one frame
/// and the two tracks can be told apart.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with two tracks, `kick` and `snare`, each with a sound of
/// its own.
fn two_tracks(session: &mut Session, directory: &str) {
    for (name, level) in [("kick", 0.5_f32), ("snare", 0.25_f32)] {
        let path = format!("{directory}/{name}.wav");
        a_sound(&path, level);
        session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
        let sources: Vec<u64> = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .collect();
        let source = sources.last().copied().expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
        let track = session.document().node_by_name(name).expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
            .expect("the clip is added");
    }
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The **files** in a directory, by name.
fn files_in(directory: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("the directory is there")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// Each track is bounced into a file of its own.
#[test]
fn each_track_is_bounced_into_a_file_of_its_own() {
    let mut session = session();
    let directory = directory("own");
    two_tracks(&mut session, &directory);
    let stems = format!("{directory}/stems");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");
    assert_eq!(files_in(&stems), vec!["kick.wav".to_string(), "snare.wav".to_string()]);
}

/// A stem is the mix with **one track in it**: the track's own sound
/// and not the other track's.
#[test]
fn a_stem_is_the_mix_with_one_track_in_it() {
    let mut session = session();
    let directory = directory("one-track");
    two_tracks(&mut session, &directory);
    let stems = format!("{directory}/stems");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");

    let kick = rendered(&format!("{stems}/kick.wav"));
    let snare = rendered(&format!("{stems}/snare.wav"));
    assert!((kick.samples[0] - 0.5).abs() < 1e-4, "the kick is the kick's own: {}", kick.samples[0]);
    assert!((snare.samples[0] - 0.25).abs() < 1e-4, "the snare is the snare's own: {}", snare.samples[0]);
}

/// The **mix** of the two tracks is both of them: a stem is one of
/// them and not the other.
#[test]
fn the_mix_is_both_of_them() {
    let mut session = session();
    let directory = directory("mix");
    two_tracks(&mut session, &directory);
    let mix_path = format!("{directory}/mix.wav");
    session.execute("sound.render", &json!({ "path": mix_path })).expect("the mix is rendered");
    let mix = rendered(&mix_path);
    assert!((mix.samples[0] - 0.75).abs() < 1e-4, "the mix is both: {}", mix.samples[0]);
}

/// The directory is **made** if it is not there: a stem that is not
/// written is a stem nobody asked for.
#[test]
fn the_directory_is_made_if_it_is_not_there() {
    let mut session = session();
    let directory = directory("made");
    two_tracks(&mut session, &directory);
    let stems = format!("{directory}/not/there/yet");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");
    assert_eq!(files_in(&stems).len(), 2, "the directory was made and the stems are in it");
}

/// Two tracks with the **same name** are two files and not one file
/// written twice.
#[test]
fn two_tracks_with_the_same_name_are_two_files() {
    let mut session = session();
    let directory = directory("same-name");
    for level in [0.5_f32, 0.25_f32] {
        let path = format!("{directory}/source.wav");
        a_sound(&path, level);
        session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
        let sources: Vec<u64> = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .collect();
        let source = sources.last().copied().expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
            .expect("the clip is added");
    }
    let stems = format!("{directory}/stems");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");
    assert_eq!(files_in(&stems), vec!["drums-2.wav".to_string(), "drums.wav".to_string()]);
}

/// A **muted** track is not bounced: a stem of silence is a file
/// nobody asked for.
#[test]
fn a_muted_track_is_not_bounced() {
    let mut session = session();
    let directory = directory("muted");
    two_tracks(&mut session, &directory);
    let snare = session.document().node_by_name("snare").expect("the track is there").id;
    session
        .execute("sound.lane.add", &json!({ "track": snare, "param": "mute", "points": [[0, 1.0]] }))
        .expect("the mute is there");
    let stems = format!("{directory}/stems");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");
    assert_eq!(files_in(&stems), vec!["kick.wav".to_string()]);
}

/// A directory that is **not a directory** is refused.
#[test]
fn a_directory_that_is_not_a_directory_is_refused() {
    let mut session = session();
    let directory = directory("not-a-directory");
    two_tracks(&mut session, &directory);
    let refused = session.execute("sound.render.stems", &json!({ "directory": "  " }));
    assert!(refused.is_err(), "a directory is a path");
}

/// The stems are the **render** of the document: the same sound, and
/// the same number of frames.
#[test]
fn the_stems_are_the_render_of_the_document() {
    let mut session = session();
    let directory = directory("same-render");
    two_tracks(&mut session, &directory);
    let stems = format!("{directory}/stems");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");
    let kick = rendered(&format!("{stems}/kick.wav"));
    assert_eq!(kick.frames, 1, "the stem is as long as the mix: {}", kick.frames);
}
