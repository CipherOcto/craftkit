//! The **separation on the grid**: a clip cut at every line it
//! crosses.
//!
//! The tree's editor separates a take on the grid: the clip is cut at
//! every line of the grid it crosses, and the pieces are the bars or
//! the beats the grid says they are. The grid is the session's and the
//! lines are musical, so they move with the tempo.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-separategrid-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **ramp** of that many frames, so that a cut in it is visible.
fn a_ramp(path: &str, frames: usize) {
    let samples: Vec<f32> = (0..frames).map(|index| index as f32 / frames as f32).collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A quarter note at 120 beats a minute and 48 kHz is half a second,
/// and half a second is twenty-four thousand samples.
const BEAT: i64 = 24_000;

/// A session with a grid of quarter notes and a clip of four beats.
fn a_clip(session: &mut Session, source_path: &str) -> u64 {
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 0, "length": 4 * BEAT, "source": source }),
        )
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// The clips of the document, by their start: their start and their
/// length.
fn pieces_of(session: &Session) -> Vec<(i64, i64)> {
    let mut pieces: Vec<(i64, i64)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| {
            let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            let length = node.data.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
            (start, length)
        })
        .collect();
    pieces.sort();
    pieces
}

/// A clip is cut at every **line** it crosses: a clip of four beats on
/// a grid of quarter notes is four pieces.
#[test]
fn a_clip_is_cut_at_every_line_it_crosses() {
    let mut session = session();
    let directory = directory("lines");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, (4 * BEAT) as usize);
    let clip = a_clip(&mut session, &source_path);
    session.execute("sound.clip.separate.grid", &json!({ "clip": clip })).expect("the clip is separated");
    let pieces = pieces_of(&session);
    assert_eq!(pieces.len(), 4, "the clip is in four: {pieces:?}");
    for (index, (start, length)) in pieces.iter().enumerate() {
        assert_eq!(*start, BEAT * index as i64, "the piece {index} starts on the beat");
        assert_eq!(*length, BEAT, "and it is a beat long");
    }
}

/// The pieces are the **same sound**: the four of them together are
/// the clip that was cut.
#[test]
fn the_pieces_are_the_same_sound() {
    let directory = directory("sound");
    let source_path = format!("{directory}/ramp.wav");
    let whole_path = format!("{directory}/whole.wav");
    let cut_path = format!("{directory}/cut.wav");
    a_ramp(&source_path, (4 * BEAT) as usize);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path);
    session.execute("sound.render", &json!({ "path": whole_path })).expect("the sound is rendered");
    session.execute("sound.clip.separate.grid", &json!({ "clip": clip })).expect("the clip is separated");
    session.execute("sound.render", &json!({ "path": cut_path })).expect("the sound is rendered");

    let written = std::fs::read(&whole_path).expect("the sound is on the disk");
    let whole = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    let written = std::fs::read(&cut_path).expect("the sound is on the disk");
    let cut = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    assert_eq!(whole.frames, cut.frames, "the two are as long as the one");
    let mut worst = 0.0_f32;
    for (before, after) in whole.samples.iter().zip(cut.samples.iter()) {
        worst = worst.max((before - after).abs());
    }
    assert!(worst < 1e-4, "the same sound: the worst difference is {worst}");
}

/// A clip that crosses **no line** is not cut: there is nothing to
/// separate it at.
#[test]
fn a_clip_that_crosses_no_line_is_not_cut() {
    let mut session = session();
    let directory = directory("no-line");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, BEAT as usize);
    let clip = a_clip(&mut session, &source_path);
    // A clip of half a beat: it does not reach the next line.
    session
        .execute("sound.clip.trim", &json!({ "clip": clip, "from": 0, "to": BEAT / 2 }))
        .expect("the clip is trimmed");
    session.execute("sound.clip.separate.grid", &json!({ "clip": clip })).expect("the clip is separated");
    assert_eq!(pieces_of(&session).len(), 1, "the clip is whole");
}

/// A clip whose **content** is locked is finished, and it is not cut.
#[test]
fn a_locked_clip_is_not_cut() {
    let mut session = session();
    let directory = directory("locked");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, (4 * BEAT) as usize);
    let clip = a_clip(&mut session, &source_path);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clip, "locked": true }))
        .expect("the clip is locked");
    let refused = session.execute("sound.clip.separate.grid", &json!({ "clip": clip }));
    assert!(refused.is_err(), "a locked clip is finished");
    assert_eq!(pieces_of(&session).len(), 1, "and nothing was cut");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let directory = directory("not-a-clip");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, (4 * BEAT) as usize);
    let clip = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.clip.separate.grid", &json!({ "clip": clip + 1_000_000 }));
    assert!(refused.is_err(), "there is no such clip");
}
