//! The **processes** on a clip: what is done to its sound.
//!
//! The tree's audio suite does four things to a clip's sound: it
//! reverses it, turns it upside down, brings it to a level, and takes
//! the direct current away. The tree does each of them by rendering
//! the material and replacing the clip's content with the rendered
//! file; here the process is on the clip, and the sound is worked out
//! when it is played.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-process-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **ramp** of that many frames: a sound that goes up and comes
/// back, so that a reversal is visible.
fn a_ramp(path: &str, frames: usize) {
    let samples: Vec<f32> = (0..frames).map(|index| index as f32 / frames as f32).collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and the sound on it.
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

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The processes a clip carries.
fn processes_of(session: &Session, clip: u64) -> Vec<String> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data
        .get("processes")
        .and_then(|processes| processes.as_array())
        .map(|processes| {
            processes
                .iter()
                .filter_map(|process| {
                    process.get("process").and_then(|name| name.as_str()).map(|name| name.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A clip carries a **process**, and it is done when the clip is
/// played.
#[test]
fn a_process_is_on_the_clip() {
    let mut session = session();
    let directory = directory("on-the-clip");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the process is added");
    assert_eq!(processes_of(&session, clip), vec!["reverse".to_string()], "the clip is processed");
}

/// A process the battery does not know is **refused**: a process that
/// is quietly ignored is worse than no process at all.
#[test]
fn a_process_that_is_not_known_is_refused() {
    let mut session = session();
    let directory = directory("not-known");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused = session.execute("sound.clip.process", &json!({ "clip": clip, "process": "levitate" }));
    assert!(refused.is_err(), "a process is one this battery knows");
    assert!(processes_of(&session, clip).is_empty(), "and nothing was added");
}

/// A process with **no name** is refused.
#[test]
fn a_process_with_no_name_is_refused() {
    let mut session = session();
    let directory = directory("no-name");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused = session.execute("sound.clip.process", &json!({ "clip": clip, "process": "  " }));
    assert!(refused.is_err(), "a process is a name");
}

/// A **level** outside its range is refused, and the process is not
/// added.
#[test]
fn a_level_outside_its_range_is_refused() {
    let mut session = session();
    let directory = directory("level");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused = session.execute(
        "sound.clip.process",
        &json!({ "clip": clip, "process": "normalize", "params": { "target_db": 40.0 } }),
    );
    assert!(refused.is_err(), "a level is bounded");
    assert!(processes_of(&session, clip).is_empty(), "and nothing was added");
}

/// A clip whose **content** is locked is finished, and it is not
/// processed.
#[test]
fn a_locked_clip_is_not_processed() {
    let mut session = session();
    let directory = directory("locked");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clip, "locked": true }))
        .expect("the clip is locked");
    let refused = session.execute("sound.clip.process", &json!({ "clip": clip, "process": "reverse" }));
    assert!(refused.is_err(), "a locked clip is finished");
}

/// The sound is **reversed**: the ramp comes down instead of up.
#[test]
fn the_sound_is_reversed() {
    let directory = directory("reversed");
    let source_path = format!("{directory}/ramp.wav");
    let output_path = format!("{directory}/out.wav");
    a_ramp(&source_path, 480);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the process is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let first = sound.samples[0];
    let last = sound.samples[sound.frames as usize - 1];
    assert!(first > last, "the ramp comes down: {first} and {last}");
}

/// The sound is **upside down**: the sign is flipped and the shape is
/// the same.
#[test]
fn the_sound_is_upside_down() {
    let directory = directory("inverted");
    let source_path = format!("{directory}/ramp.wav");
    let output_path = format!("{directory}/out.wav");
    a_ramp(&source_path, 480);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "invert" }))
        .expect("the process is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!(sound.samples[1] < 0.0, "the sign is flipped: {}", sound.samples[1]);
}

/// The sound is at a **level**: its loudest sample is at the level
/// that was asked for.
#[test]
fn the_sound_is_at_a_level() {
    let directory = directory("normalized");
    let source_path = format!("{directory}/ramp.wav");
    let output_path = format!("{directory}/out.wav");
    a_ramp(&source_path, 480);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute(
            "sound.clip.process",
            &json!({ "clip": clip, "process": "normalize", "params": { "target_db": 0.0 } }),
        )
        .expect("the process is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let loudest = sound.samples.iter().fold(0.0_f32, |most, sample| most.max(sample.abs()));
    assert!((loudest - 1.0).abs() < 1e-3, "the loudest sample is at zero decibels: {loudest}");
}

/// The sound has **no direct current**: its mean is taken away.
#[test]
fn the_sound_has_no_direct_current() {
    let directory = directory("dc");
    let source_path = format!("{directory}/ramp.wav");
    let output_path = format!("{directory}/out.wav");
    // A ramp that sits above zero: the sound is offset and not
    // louder.
    let frames = 480;
    let samples: Vec<f32> = (0..frames).map(|index| 0.5 + index as f32 / frames as f32).collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(&source_path, bytes).expect("the sound is written");

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, frames as i64);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "dc_offset" }))
        .expect("the process is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let mean = sound.samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / sound.frames as f64;
    assert!(mean.abs() < 1e-4, "the mean is gone: {mean}");
}

/// The processes are done **in order**, and a clip may have the same
/// one twice: the same process asked for twice is done twice.
#[test]
fn the_processes_are_done_in_order() {
    let mut session = session();
    let directory = directory("order");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the first process is added");
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "invert" }))
        .expect("the second process is added");
    assert_eq!(
        processes_of(&session, clip),
        vec!["reverse".to_string(), "invert".to_string()],
        "the processes are in the order they were added"
    );
}

/// A process is **taken off**: the last one of that name goes.
#[test]
fn a_process_is_taken_off() {
    let mut session = session();
    let directory = directory("removed");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the process is added");
    session
        .execute("sound.clip.process.remove", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the process is taken off");
    assert!(processes_of(&session, clip).is_empty(), "the clip is not processed");
}

/// A process that is **not there** cannot be taken off.
#[test]
fn a_process_that_is_not_there_cannot_be_taken_off() {
    let mut session = session();
    let directory = directory("not-there");
    let source_path = format!("{directory}/ramp.wav");
    a_ramp(&source_path, 480);
    let clip = a_clip(&mut session, &source_path, 480);
    let refused =
        session.execute("sound.clip.process.remove", &json!({ "clip": clip, "process": "reverse" }));
    assert!(refused.is_err(), "there is no process to take off");
}

/// The sound of a clip with the process taken off is the sound of the
/// source again.
#[test]
fn the_sound_is_the_source_again_when_the_process_is_taken_off() {
    let directory = directory("again");
    let source_path = format!("{directory}/ramp.wav");
    let output_path = format!("{directory}/out.wav");
    a_ramp(&source_path, 480);

    let mut session = session();
    let clip = a_clip(&mut session, &source_path, 480);
    session
        .execute("sound.clip.process", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the process is added");
    session
        .execute("sound.clip.process.remove", &json!({ "clip": clip, "process": "reverse" }))
        .expect("the process is taken off");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!(sound.samples[0] < sound.samples[sound.frames as usize - 1], "the ramp goes up again");
}
