//! The **music** that comes with a MIDI file: its tempo map and its
//! keys.
//!
//! The tree's import reads the file's tempos and meters into the
//! session's tempo map, and throws the file's key signatures away.
//! Here the keys come in as well, and every event lands where the
//! notes land.

use craftkit_midi::{Note, Sequence, Smf, SmfTrack, write_smf};
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-importmusic-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A MIDI file with a **tempo**, a **meter** and a **key** in it, and
/// two notes.
fn a_file(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.mid");
    let file = Smf {
        format: 1,
        tracks: vec![SmfTrack {
            name: "the notes".to_string(),
            sequence: Sequence {
                notes: vec![
                    Note {
                        pitch: 60,
                        velocity: 100,
                        release_velocity: 64,
                        channel: 0,
                        start: 0,
                        length: 960,
                    },
                    Note {
                        pitch: 64,
                        velocity: 100,
                        release_velocity: 64,
                        channel: 0,
                        start: 960,
                        length: 960,
                    },
                ],
                ctrls: Vec::new(),
            },
        }],
        tempos: vec![(0, 90.0)],
        meters: vec![(0, 3, 4)],
        key_sigs: vec![(0, 1, false)],
        ..Smf::default()
    };
    std::fs::write(&path, write_smf(&file).expect("the file is written")).expect("the file is on the disk");
    path
}

/// The **ticks** of the events of a kind, in the order they happen.
fn ticks_of(session: &Session, kind: &str) -> Vec<i64> {
    let mut ticks: Vec<i64> = session
        .document()
        .walk()
        .filter(|node| node.kind == kind)
        .filter_map(|node| node.data.get("tick").and_then(|tick| tick.as_i64()))
        .collect();
    ticks.sort_unstable();
    ticks
}

/// The file's tempo map comes in with it, and its events land where
/// the notes land.
#[test]
fn the_music_of_the_file_comes_in_with_it() {
    let mut session = session();
    let path = a_file("themap");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    let outcome = session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    assert_eq!(outcome.data["tempos"], 1);
    assert_eq!(outcome.data["meters"], 1);
    assert_eq!(outcome.data["keys"], 1);
    assert_eq!(ticks_of(&session, "sound.tempo"), vec![0]);
    assert_eq!(ticks_of(&session, "sound.meter"), vec![0]);
    assert_eq!(ticks_of(&session, "sound.key"), vec![0]);
}

/// The file's **key** comes in as a key of the document, with the
/// sharps and the mode the file carried.
#[test]
fn the_key_of_the_file_comes_in_as_a_key() {
    let mut session = session();
    let path = a_file("thekey");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    let key = session.document().walk().find(|node| node.kind == "sound.key").expect("the key is there");
    assert_eq!(key.data["sharps"], 1);
    assert_eq!(key.data["minor"], false);
    let read = session.execute("sound.key.read", &json!({})).expect("the keys are read");
    assert_eq!(read.data["keys"][0]["name"], "G major");
}

/// The file is imported at a place, and its events land **there** and
/// not at the file's own ticks: the tree's import leaves the tempos
/// behind at the file's ticks while its notes move.
#[test]
fn the_events_of_the_file_land_where_the_notes_land() {
    let mut session = session();
    let path = a_file("theplace");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    // Five seconds in, at the document's own tempo of a hundred and
    // twenty beats a minute: ten quarters of music.
    let start = 240_000i64;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": start, "path": path }))
        .expect("the file is imported");
    let ticks = ticks_of(&session, "sound.tempo");
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0], 9600, "the tempo is at the tick the clip is at");
}

/// A caller who says **no** gets the notes at the document's own tempo,
/// and the document's tempo map is not touched.
#[test]
fn the_music_of_the_file_may_be_left_behind() {
    let mut session = session();
    let path = a_file("nomap");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    let outcome = session
        .execute(
            "sound.clip.midi.import",
            &json!({ "track": track, "start": 0, "path": path, "tempo_map": false }),
        )
        .expect("the file is imported");
    assert_eq!(outcome.data["tempos"], 0);
    assert_eq!(outcome.data["meters"], 0);
    assert_eq!(outcome.data["keys"], 0);
    assert!(ticks_of(&session, "sound.tempo").is_empty(), "the document's tempo map is untouched");
    assert!(ticks_of(&session, "sound.key").is_empty(), "and there is no key");
}

/// The exported file carries the **music** with the notes: the
/// document's tempo map, its meters and its keys go into the file, so
/// that the file plays as it was written.
#[test]
fn the_exported_file_carries_the_music() {
    let mut session = session();
    let path = a_file("exportmusic");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");

    let directory = directory("exportmusic");
    let out = format!("{directory}/out.mid");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    let outcome = session
        .execute("sound.clip.midi.export", &json!({ "clip": clip, "path": out }))
        .expect("the file is written");
    assert_eq!(outcome.data["tempos"], 1);
    assert_eq!(outcome.data["meters"], 1);
    assert_eq!(outcome.data["keys"], 1);

    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let file = craftkit_midi::read_smf(&bytes).expect("the file is read");
    // The tempo of a file is written in microseconds a quarter note, so
    // it comes back very nearly the tempo it went in as.
    assert_eq!(file.tempos.len(), 1);
    assert_eq!(file.tempos[0].0, 0);
    assert!((file.tempos[0].1 - 90.0).abs() < 0.001, "the tempo is 90: {file:?}");
    assert_eq!(file.meters, vec![(0, 3, 4)]);
    assert_eq!(file.key_sigs, vec![(0, 1, false)]);
}

/// The **markers** of the document go into the file too, as the text
/// markers a file carries.
#[test]
fn the_exported_file_carries_the_markers() {
    let mut session = session();
    let path = a_file("exportmarkers");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    session
        .execute("sound.marker.add", &json!({ "name": "the chorus", "start": 96000 }))
        .expect("the marker is set");

    let directory = directory("exportmarkers");
    let out = format!("{directory}/out.mid");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    let outcome = session
        .execute("sound.clip.midi.export", &json!({ "clip": clip, "path": out }))
        .expect("the file is written");
    assert_eq!(outcome.data["markers"], 1);

    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let file = craftkit_midi::read_smf(&bytes).expect("the file is read");
    assert_eq!(file.markers.len(), 1);
    assert_eq!(file.markers[0].1, "the chorus");
}

/// The exported notes are at the **ticks they are at** in the
/// document, and not at the ticks they are at inside the clip.
#[test]
fn the_exported_notes_are_at_their_place_in_the_document() {
    let mut session = session();
    let path = a_file("exportticks");
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    let start = 240_000i64;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": start, "path": path }))
        .expect("the file is imported");

    let directory = directory("exportticks");
    let out = format!("{directory}/out.mid");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    session
        .execute("sound.clip.midi.export", &json!({ "clip": clip, "path": out }))
        .expect("the file is written");

    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let file = craftkit_midi::read_smf(&bytes).expect("the file is read");
    let first = file.tracks[0].sequence.notes[0];
    assert_eq!(first.start, 9600, "the first note is at the clip's own place");
}
