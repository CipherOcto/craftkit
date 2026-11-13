//! The **separation** of a clip: one clip becomes two.
//!
//! The tree's editor separates a take at a place on the track: every
//! clip that crosses that place becomes two clips, and the two are the
//! same sound with a line between them. Here one clip is separated at
//! one place, and the two pieces are the same sound.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-separate-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **ramp** of that many frames: a sound that goes up, so that a
/// cut in it is visible.
fn a_ramp(path: &str, frames: usize) {
    let samples: Vec<f32> = (0..frames).map(|index| index as f32 / frames as f32).collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a clip of that length, and its id.
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

/// The clips of the document, by their start.
fn clips_of(session: &Session) -> Vec<(i64, i64, i64, u64)> {
    let mut clips: Vec<(i64, i64, i64, u64)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| {
            let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            let length = node.data.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
            let offset = node.data.get("offset").and_then(|offset| offset.as_i64()).unwrap_or(0);
            (start, length, offset, node.id)
        })
        .collect();
    clips.sort();
    clips
}

/// A clip is separated into **two** clips.
#[test]
fn a_clip_is_separated_into_two() {
    let mut session = session();
    let directory = directory("two");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 2, "the clip is in two: {clips:?}");
    assert_eq!(clips[0].0, 0, "the first piece starts where the clip did");
    assert_eq!(clips[0].1, 240, "and runs to the cut");
    assert_eq!(clips[1].0, 240, "the second piece starts at the cut");
    assert_eq!(clips[1].1, 240, "and runs to the end");
}

/// The two pieces are the **same sound**: the second starts later in
/// the source by the distance the cut made.
#[test]
fn the_second_piece_starts_later_in_the_source() {
    let mut session = session();
    let directory = directory("later");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let clips = clips_of(&session);
    assert_eq!(clips[0].2, 0, "the first piece starts at the head of the source");
    assert_eq!(clips[1].2, 240, "and the second at the cut");
}

/// A **stretched** clip's source is read more slowly, so the content
/// moves by the delta **divided by the stretch**.
#[test]
fn a_stretched_clip_is_separated_by_the_stretch() {
    let mut session = session();
    let directory = directory("stretched");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.stretch", &json!({ "clip": clip, "stretch": 2.0 }))
        .expect("the clip is stretched");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let clips = clips_of(&session);
    assert_eq!(clips[1].2, 120, "the content moved by the delta over the stretch");
}

/// The two pieces are **heard** as one: the sound of the two together
/// is the sound of the clip before it was separated.
#[test]
fn the_two_pieces_are_heard_as_one() {
    let directory = directory("heard");
    let source_path = format!("{directory}/ramp.wav");
    let whole_path = format!("{directory}/whole.wav");
    let cut_path = format!("{directory}/cut.wav");
    a_ramp(&source_path, 480);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 480);
    session.execute("sound.render", &json!({ "path": whole_path })).expect("the sound is rendered");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
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

/// A place that is **not inside** the clip is refused: there is
/// nothing to separate.
#[test]
fn a_place_that_is_not_inside_the_clip_is_refused() {
    let mut session = session();
    let directory = directory("outside");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused = session.execute("sound.clip.separate", &json!({ "clip": clip, "at": 900 }));
    assert!(refused.is_err(), "the clip does not run to 900");
    assert_eq!(clips_of(&session).len(), 1, "and nothing was separated");
}

/// A place at the **edge** of the clip is refused too.
#[test]
fn a_place_at_the_edge_of_the_clip_is_refused() {
    let mut session = session();
    let directory = directory("edge");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused = session.execute("sound.clip.separate", &json!({ "clip": clip, "at": 0 }));
    assert!(refused.is_err(), "the clip starts at 0");
    assert_eq!(clips_of(&session).len(), 1, "and nothing was separated");
}

/// A clip whose **content** is locked is finished, and it is not
/// separated.
#[test]
fn a_locked_clip_is_not_separated() {
    let mut session = session();
    let directory = directory("locked");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clip, "locked": true }))
        .expect("the clip is locked");
    let refused = session.execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }));
    assert!(refused.is_err(), "a locked clip is finished");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let directory = directory("not-a-clip");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused = session.execute("sound.clip.separate", &json!({ "clip": clip + 1_000_000, "at": 240 }));
    assert!(refused.is_err(), "there is no such clip");
}

/// The **beat** of a clip moves with the audio it is in: the second
/// piece's beat is nearer its head.
#[test]
fn the_beat_moves_with_the_audio() {
    let mut session = session();
    let directory = directory("beat");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 300 })).expect("the beat is set");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let clips = clips_of(&session);
    let beat_of = |id: u64| -> Option<i64> {
        session.document().node(id).and_then(|node| node.data.get("sync").and_then(|sync| sync.as_i64()))
    };
    assert_eq!(beat_of(clips[0].3), None, "the beat was in the second piece");
    assert_eq!(beat_of(clips[1].3), Some(60), "and it is nearer the head of its piece");
}
