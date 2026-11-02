//! The **fade of a clip to a place**: up to it, or down to it.
//!
//! The tree's editor fades a clip to the playhead: "Fade to Start"
//! fades the clip from its head up to the playhead, and "Fade to End"
//! fades it from the playhead down to its tail. The shape is the
//! tree's own — the equal-power curve.

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

/// A session with a track and a clip of ten thousand samples.
fn a_clip(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 1_000, "length": 10_000 }))
        .expect("the clip is added");
    session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id
}

/// The clip's fade in and fade out, as `(length, shape)`.
fn fades_of(session: &Session, clip: u64) -> ((i64, String), (i64, String)) {
    let node = session.document().node(clip).expect("the clip is there");
    let read = |key: &str| {
        (
            node.data[key]["len"].as_i64().expect("the length is there"),
            node.data[key]["shape"].as_str().expect("the shape is there").to_string(),
        )
    };
    (read("fade_in"), read("fade_out"))
}

/// The clip is faded **up to a place**: the fade in reaches it, and
/// the shape is the tree's own.
#[test]
fn the_clip_is_faded_up_to_a_place() {
    let mut session = session();
    let clip = a_clip(&mut session);
    session
        .execute("sound.clip.fade.to", &json!({ "clip": clip, "at": 4_000, "to": "in" }))
        .expect("the clip is faded");
    let (fade_in, fade_out) = fades_of(&session, clip);
    assert_eq!(fade_in, (3_000, "equal_power".to_string()), "the fade in: {fade_in:?}");
    assert_eq!(fade_out.0, 0, "and no fade out: {fade_out:?}");
}

/// The clip is faded **down to a place**: the fade out starts at it.
#[test]
fn the_clip_is_faded_down_to_a_place() {
    let mut session = session();
    let clip = a_clip(&mut session);
    session
        .execute("sound.clip.fade.to", &json!({ "clip": clip, "at": 8_000, "to": "out" }))
        .expect("the clip is faded");
    let (fade_in, fade_out) = fades_of(&session, clip);
    assert_eq!(fade_in.0, 0, "and no fade in: {fade_in:?}");
    // The clip ends at eleven thousand, so the fade out runs from
    // eight thousand to there.
    assert_eq!(fade_out, (3_000, "equal_power".to_string()), "the fade out: {fade_out:?}");
}

/// A place that is **not inside the clip** is refused: a fade that
/// ends outside the clip is a fade of nothing.
#[test]
fn a_place_outside_the_clip_is_refused() {
    let mut session = session();
    let clip = a_clip(&mut session);
    let refused = session.execute("sound.clip.fade.to", &json!({ "clip": clip, "at": 50_000 }));
    assert!(refused.is_err(), "the clip does not run to fifty thousand");
    let (fade_in, _) = fades_of(&session, clip);
    assert_eq!(fade_in.0, 0, "and the clip was not faded");
}

/// A way of fading that this battery does not know is **refused**.
#[test]
fn an_unknown_way_is_refused() {
    let mut session = session();
    let clip = a_clip(&mut session);
    let refused =
        session.execute("sound.clip.fade.to", &json!({ "clip": clip, "at": 4_000, "to": "sideways" }));
    assert!(refused.is_err(), "a fade goes in or out");
    let (fade_in, _) = fades_of(&session, clip);
    assert_eq!(fade_in.0, 0, "and the clip was not faded");
}
