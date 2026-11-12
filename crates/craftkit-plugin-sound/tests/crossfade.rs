//! The **crossfade** of a range of a track.
//!
//! The tree's crossfade: a range that covers the boundary between two
//! clips makes them overlap, and each is extended into the other's
//! territory using its handle.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-crossfade-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **mono** sound of two seconds.
fn a_sound(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.wav");
    let mut samples = Vec::with_capacity(96_000);
    for frame in 0..96_000_u32 {
        samples.push(frame as f32 / 96_000.0);
    }
    std::fs::write(&path, craftkit_wav::write(1, 48_000, &samples)).expect("the file is on the disk");
    path
}

/// A session with a track and **two** clips on it: the first plays the
/// first half of the material and the second plays the second half, and
/// they are **next to each other**.
///
/// The second clip plays the material from its middle, and so the
/// material before it is its **handle**.
fn two_clips(session: &mut Session, name: &str) -> (u64, u64, u64) {
    let path = a_sound(name);
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 1, "frames": 96_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the first clip is added");
    let first =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the first clip is there").id;
    // The second clip is added **long** and trimmed, so that it plays
    // the second half of the material: the material before it is its
    // **handle**.
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 96_000, "source": source }))
        .expect("the second clip is added");
    let second = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip" && node.id != first)
        .expect("the second clip is there")
        .id;
    session
        .execute("sound.clip.trim", &json!({ "clip": second, "from": 48_000, "to": 96_000 }))
        .expect("the second clip is trimmed");
    (track, first, second)
}

/// The **clocks** of a clip: where it starts, how long it is and how
/// far into its source it begins.
fn clocks_of(session: &Session, clip: u64) -> (i64, i64, i64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_i64().unwrap_or(0);
    let length = node.data["length"].as_i64().unwrap_or(0);
    let offset = node.data["offset"].as_i64().unwrap_or(0);
    (start, length, offset)
}

/// The **fade out** of a clip.
fn fade_out_of(session: &Session, clip: u64) -> i64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["fade_out"]["len"].as_i64().unwrap_or(0)
}

/// The **fade in** of a clip.
fn fade_in_of(session: &Session, clip: u64) -> i64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["fade_in"]["len"].as_i64().unwrap_or(0)
}

/// A range that covers the **boundary** between two clips makes them
/// overlap.
#[test]
fn a_range_that_covers_a_boundary_makes_the_clips_overlap() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 24_000, "to": 72_000 }))
        .expect("the range is faded");
    assert_eq!(clocks_of(&session, first), (0, 72_000, 0), "the first clip is longer");
    assert_eq!(clocks_of(&session, second), (24_000, 72_000, 24_000), "and the second starts earlier");
}

/// The two fades meet in the **overlap**: the first fades out over it
/// and the second fades in over it.
#[test]
fn the_fades_meet_in_the_overlap() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 24_000, "to": 72_000 }))
        .expect("the range is faded");
    assert_eq!(fade_out_of(&session, first), 48_000, "the first fades out over the overlap");
    assert_eq!(fade_in_of(&session, second), 48_000, "and the second fades in over it");
}

/// A crossfade is limited by the **handles**: a clip that has no
/// material before it cannot be extended backwards.
#[test]
fn a_crossfade_is_limited_by_the_handles() {
    let mut session = session();
    // The second clip plays the material from its **beginning**, so
    // there is nothing before it.
    let (track, first, second) = two_clips_without_handles(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 24_000, "to": 72_000 }))
        .expect("the range is faded");
    let (_, first_length, _) = clocks_of(&session, first);
    let (second_start, second_length, _) = clocks_of(&session, second);
    assert_eq!(first_length, 72_000, "the first clip has the material after it");
    assert_eq!(second_start, 48_000, "and the second cannot start earlier");
    assert_eq!(second_length, 48_000, "and it is as long as it was");
}

/// A session with a track and **two** clips on it, and the second one
/// has **no handle** before it.
fn two_clips_without_handles(session: &mut Session, name: &str) -> (u64, u64, u64) {
    let path = a_sound(name);
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 1, "frames": 96_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the first clip is added");
    let first =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the first clip is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 48_000, "length": 48_000, "source": source }),
        )
        .expect("the second clip is added");
    let second = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip" && node.id != first)
        .expect("the second clip is there")
        .id;
    (track, first, second)
}

/// The fades of a crossfade may be **taken away**, and the overlap goes
/// with them: the two clips are trimmed back to the midpoint.
#[test]
fn taking_the_fades_away_takes_the_overlap_with_them() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 24_000, "to": 72_000 }))
        .expect("the range is faded");
    session
        .execute("sound.track.fade.delete", &json!({ "track": track, "from": 24_000, "to": 72_000 }))
        .expect("the fades are taken away");
    assert_eq!(fade_out_of(&session, first), 0, "the fade out is gone");
    assert_eq!(fade_in_of(&session, second), 0, "and the fade in too");
    assert_eq!(clocks_of(&session, first), (0, 48_000, 0), "and the first is as long as it was");
    assert_eq!(clocks_of(&session, second), (48_000, 48_000, 48_000), "and the second too");
}

/// Two clips that are **not next to each other** are not a crossfade:
/// there is no boundary between them, and the range fades them apart.
#[test]
fn two_clips_with_a_gap_between_them_are_not_a_crossfade() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session, "the track");
    session
        .execute("sound.clip.move", &json!({ "clips": [second], "delta": 12_000 }))
        .expect("the clip is moved");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 24_000, "to": 72_000 }))
        .expect("the range is faded");
    assert_eq!(clocks_of(&session, first).1, 48_000, "the first clip is as long as it was");
    assert_eq!(clocks_of(&session, second).0, 60_000, "and the second has not moved");
    assert_eq!(fade_out_of(&session, first), 24_000, "the first fades out and no more");
}
