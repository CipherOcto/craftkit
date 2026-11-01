//! The **auto rename** of a clip: the names editing gives the pieces it
//! separates.
//!
//! The tree's sound editor names the pieces that editing makes: a clip
//! with a scheme is cut, and the piece after the cut takes the next
//! name of the scheme. Here a scheme is a node and the clips point at
//! it, and the piece after a cut takes the scheme's next number.

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

/// A session with a clip of that length on a track of that name, and
/// the clip's id.
fn a_clip(session: &mut Session, name: &str, length: i64) -> u64 {
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": length, "name": name }))
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// The clips of the document, by their start, with their names.
fn clips_of(session: &Session) -> Vec<(i64, String)> {
    let mut clips: Vec<(i64, String)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| {
            let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            (start, node.name.clone())
        })
        .collect();
    clips.sort();
    clips
}

/// The piece after a cut takes the next name of the scheme, and the
/// piece before it keeps the name it had.
#[test]
fn the_piece_after_a_cut_takes_the_next_name() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "SFX", "number": 23 }))
        .expect("the scheme is set");
    let clips = clips_of(&session);
    assert_eq!(clips[0].1, "kick", "setting the scheme leaves the clip's name");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 2, "the clip is in two: {clips:?}");
    assert_eq!(clips[0].1, "kick", "the piece before the cut keeps the name");
    assert_eq!(clips[1].1, "SFX23", "and the piece after it takes the next name");
}

/// The name is made of the **name**, the **zeros**, the **number** and
/// the **suffix**, in that order.
#[test]
fn the_name_is_made_of_its_parts() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    session
        .execute(
            "sound.clip.autoname",
            &json!({ "clips": [clip], "name": "SFX", "number": 24, "zeros": 1, "suffix": ".Reel1" }),
        )
        .expect("the scheme is set");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let clips = clips_of(&session);
    assert_eq!(clips[1].1, "SFX024.Reel1", "the zeros go in front of the number");
}

/// The scheme **counts up**: every piece editing makes takes the next
/// number of the same sequence.
#[test]
fn the_scheme_counts_up() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "Hit" }))
        .expect("the scheme is set");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let piece = clips_of(&session)[1].1.clone();
    // The piece is in the scheme too, so a cut of the piece counts on.
    let second = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .max_by_key(|node| node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0))
        .map(|node| node.id)
        .expect("the piece is there");
    session
        .execute("sound.clip.separate", &json!({ "clip": second, "at": 360 }))
        .expect("the piece is separated");
    let clips = clips_of(&session);
    let names: Vec<&str> = clips.iter().map(|(_, name)| name.as_str()).collect();
    assert_eq!(names, ["kick", "Hit1", "Hit2"], "one sequence for all of them");
    assert_eq!(piece, "Hit1");
}

/// Two clips with **one** scheme between them count up **one**
/// sequence: that is what a shared scheme is for.
#[test]
fn two_clips_share_one_sequence() {
    let mut session = session();
    let first = a_clip(&mut session, "kick", 480);
    session.execute("sound.track.add", &json!({ "name": "snare" })).expect("the track is added");
    let track = session.document().node_by_name("snare").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480, "name": "snare" }))
        .expect("the clip is added");
    let second = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .find(|id| *id != first)
        .expect("the second clip is there");
    session
        .execute("sound.clip.autoname", &json!({ "clips": [first, second], "name": "Hit" }))
        .expect("the scheme is set");
    session
        .execute("sound.clip.separate", &json!({ "clip": first, "at": 240 }))
        .expect("the first clip is separated");
    session
        .execute("sound.clip.separate", &json!({ "clip": second, "at": 240 }))
        .expect("the second clip is separated");
    let names: Vec<String> = clips_of(&session).into_iter().map(|(_, name)| name).collect();
    assert!(names.contains(&"Hit1".to_string()), "the first piece: {names:?}");
    assert!(names.contains(&"Hit2".to_string()), "the second piece: {names:?}");
}

/// Without a name the scheme is **taken away**, and the pieces keep the
/// names they have.
#[test]
fn without_a_name_the_scheme_is_taken_away() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "SFX" }))
        .expect("the scheme is set");
    let read =
        session.execute("sound.clip.autoname.read", &json!({ "clip": clip })).expect("the scheme is read");
    assert_eq!(read.data["named"], true, "the clip is named by the scheme");
    let taken = session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "  " }))
        .expect("the scheme is taken away");
    assert_eq!(taken.data["named"], false, "and the command says so");
    let read =
        session.execute("sound.clip.autoname.read", &json!({ "clip": clip })).expect("the scheme is read");
    assert_eq!(read.data["named"], false, "the clip is named by nothing");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 240 }))
        .expect("the clip is separated");
    let names: Vec<String> = clips_of(&session).into_iter().map(|(_, name)| name).collect();
    assert_eq!(names, ["kick", "kick"], "the pieces keep the names they had");
}

/// A scheme that **no clip is in** is not a scheme, and a node that
/// says nothing is litter.
#[test]
fn a_scheme_no_clip_is_in_is_removed() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "SFX" }))
        .expect("the scheme is set");
    assert_eq!(
        session.document().walk().filter(|node| node.kind == "sound.autoname").count(),
        1,
        "the scheme is there"
    );
    session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "" }))
        .expect("the scheme is taken away");
    assert_eq!(
        session.document().walk().filter(|node| node.kind == "sound.autoname").count(),
        0,
        "and it is gone"
    );
}

/// The number, the zeros and the suffix **need** a name: they say what
/// the name is made of.
#[test]
fn the_number_needs_a_name() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    for params in [
        json!({ "clips": [clip], "number": 3 }),
        json!({ "clips": [clip], "zeros": 2 }),
        json!({ "clips": [clip], "suffix": ".a" }),
    ] {
        let refused = session.execute("sound.clip.autoname", &params);
        assert!(refused.is_err(), "{params} needs a name");
    }
    assert_eq!(
        session.document().walk().filter(|node| node.kind == "sound.autoname").count(),
        0,
        "and no scheme was made"
    );
}

/// A number that is not a whole number, or is above the bound, is
/// **refused**: it is not a count of clips.
#[test]
fn a_number_outside_the_bound_is_refused() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    for params in [
        json!({ "clips": [clip], "name": "SFX", "number": 1_000_000_000 }),
        json!({ "clips": [clip], "name": "SFX", "number": 1.5 }),
        json!({ "clips": [clip], "name": "SFX", "zeros": 10 }),
    ] {
        let refused = session.execute("sound.clip.autoname", &params);
        assert!(refused.is_err(), "{params} is refused");
    }
}

/// A node that is **not a clip** is refused, and so is a clip that is
/// not there.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let clip = a_clip(&mut session, "kick", 480);
    let refused =
        session.execute("sound.clip.autoname", &json!({ "clips": [clip + 1_000_000], "name": "SFX" }));
    assert!(refused.is_err(), "there is no such clip");
    let refused = session.execute("sound.clip.autoname", &json!({ "clips": [], "name": "SFX" }));
    assert!(refused.is_err(), "there are no clips to name");
}
