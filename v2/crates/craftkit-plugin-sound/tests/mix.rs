//! The **mix** of a sound document: the sound of the whole thing.
//!
//! The mix is **interleaved**: a frame is a sample of every channel,
//! and the sound of a clip is at the place the clip is at — in frames,
//! and not in samples.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-mix-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stereo** sound of a second, and a ramp: every frame has a
/// different value, so the place of the sound in the file says where
/// it was.
fn a_stereo_sound(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.wav");
    let mut samples = Vec::with_capacity(48_000 * 2);
    for frame in 0..48_000_u32 {
        let value = frame as f32 / 48_000.0;
        samples.push(value);
        samples.push(value);
    }
    let bytes = craftkit_wav::write(2, 48_000, &samples);
    std::fs::write(&path, bytes).expect("the file is on the disk");
    path
}

/// The frames of the rendered file that carry the sound.
fn loud_frames(sound: &craftkit_wav::Sound) -> (usize, usize) {
    let mut first = None;
    let mut last = 0_usize;
    for frame in 0..sound.frames as usize {
        if sound.samples[frame * 2] > 0.0001 {
            if first.is_none() {
                first = Some(frame);
            }
            last = frame;
        }
    }
    (first.unwrap_or(0), last)
}

/// A **stereo** clip of a second is at the place it was put, and the
/// file that is written is as long as the session is.
///
/// The mix is interleaved, and a clip's place is a place in frames:
/// a clip that is put half a second into a stereo session is half a
/// second into the file and not a quarter of a second.
#[test]
fn a_stereo_clip_is_where_it_was_put() {
    let mut session = session();
    let path = a_stereo_sound("stereo");
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 2, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": "stereo" })).expect("the track is added");
    let track = session.document().node_by_name("stereo").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 24_000, "length": 24_000, "source": source }),
        )
        .expect("the clip is added");

    let directory = directory("stereo-out");
    let out = format!("{directory}/out.wav");
    let outcome = session.execute("sound.render", &json!({ "path": out })).expect("the sound is rendered");
    assert_eq!(outcome.data["frames"], 48_000, "the mix is a second long");
    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let sound = craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the file is a sound");
    assert_eq!(sound.channels, 2);
    assert_eq!(sound.frames, 48_000, "the file is a second long");
    let (first, last) = loud_frames(&sound);
    assert!((24_000..24_010).contains(&first), "the sound starts at the clip: {first}");
    assert_eq!(last, 47_999, "and it runs to the end of the clip");
}

/// A **mono** clip is where it was put too, and the file is as long as
/// the session.
#[test]
fn a_mono_clip_is_where_it_was_put() {
    let mut session = session();
    let directory = directory("mono");
    let path = format!("{directory}/mono.wav");
    let mut samples = Vec::with_capacity(48_000);
    for frame in 0..48_000_u32 {
        samples.push(frame as f32 / 48_000.0);
    }
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(&path, bytes).expect("the file is on the disk");
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 1, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": "mono" })).expect("the track is added");
    let track = session.document().node_by_name("mono").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 24_000, "length": 24_000, "source": source }),
        )
        .expect("the clip is added");

    let out = format!("{directory}/out.wav");
    session.execute("sound.render", &json!({ "path": out })).expect("the sound is rendered");
    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let sound = craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the file is a sound");
    assert_eq!(sound.channels, 1);
    assert_eq!(sound.frames, 48_000, "the file is a second long");
    let mut first = None;
    for frame in 0..sound.frames as usize {
        if sound.samples[frame] > 0.0001 {
            first = Some(frame);
            break;
        }
    }
    assert!((24_000..24_010).contains(&first.unwrap_or(0)), "the sound starts at the clip: {first:?}");
}

/// Two clips of a stereo session **overlap** and are heard together:
/// the second one is not swallowed by the first.
#[test]
fn two_stereo_clips_are_heard_together() {
    let mut session = session();
    let path = a_stereo_sound("together");
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 2, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": "together" })).expect("the track is added");
    let track = session.document().node_by_name("together").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 24_000, "source": source }))
        .expect("the first clip is added");
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 12_000, "length": 24_000, "source": source }),
        )
        .expect("the second clip is added");

    let directory = directory("together-out");
    let out = format!("{directory}/out.wav");
    session.execute("sound.render", &json!({ "path": out })).expect("the sound is rendered");
    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let sound = craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the file is a sound");
    assert_eq!(sound.frames, 36_000, "the mix ends with the last clip");
    // Where the two clips overlap the sound is the **sum** of the two:
    // the first clip plays the ramp from its own head (eighteen
    // thousand of forty-eight thousand) and the second plays it from
    // six thousand (six thousand of forty-eight thousand), and the
    // two are added.
    let at = 18_000 * 2;
    let loud = sound.samples[at];
    let first = 18_000.0 / 48_000.0;
    let second = 6_000.0 / 48_000.0;
    let sum = first + second;
    assert!((loud - (first + second)).abs() < 0.01, "the two clips are heard together: {loud} and not {sum}");
}
