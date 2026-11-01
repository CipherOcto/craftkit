//! The **group of tracks**: a link between the tracks in it.
//!
//! The tree's sound editor has a group of tracks, and the tracks in it
//! are mixed together. Here the group is a node with a name, the tracks
//! point at it, and the group's flags say whether it links.

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

/// Two tracks, and their ids.
fn two_tracks(session: &mut Session) -> Vec<u64> {
    let mut tracks = Vec::new();
    for name in ["kick", "snare"] {
        session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
        let id = session.document().node_by_name(name).expect("the track is there").id;
        tracks.push(id);
    }
    tracks
}

/// The fader a track carries.
///
/// The fader is the one thing the group links: the tree's group links
/// the **volume**, and the volume is the fader.
fn fader_of(session: &Session, track: u64) -> Option<f64> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("fader_db").and_then(|fader| fader.as_f64())
}

/// The group node of a name, if it is there.
fn group_named(session: &Session, name: &str) -> Option<u64> {
    session.document().walk().find(|node| node.kind == "sound.group" && node.name == name).map(|node| node.id)
}

/// The tracks are put in a **group**, and the group holds them.
#[test]
fn tracks_are_put_in_a_group() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": tracks }))
        .expect("the tracks are grouped");
    let group = group_named(&session, "drums").expect("the group is there");
    let members: Vec<u64> = session
        .document()
        .relations_to(group, Some("sound.grouped"))
        .into_iter()
        .map(|relation| relation.from)
        .collect();
    assert_eq!(members.len(), 2, "the group has the tracks in it");
    assert!(members.contains(&tracks[0]), "the first track is in it");
    assert!(members.contains(&tracks[1]), "the second track is in it");
}

/// A group of tracks **mixes** them: a change to one track's level is
/// a change to every track in the group.
#[test]
fn a_group_of_tracks_mixes_them() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": tracks }))
        .expect("the tracks are grouped");
    session
        .execute("sound.track.fader", &json!({ "track": tracks[0], "db": -6.0 }))
        .expect("the fader is set");
    assert_eq!(fader_of(&session, tracks[0]), Some(-6.0), "the track is set");
    assert_eq!(fader_of(&session, tracks[1]), Some(-6.0), "and the group is set with it");
}

/// A group that is **off** links nothing.
#[test]
fn a_group_that_is_off_links_nothing() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": tracks }))
        .expect("the tracks are grouped");
    let group = group_named(&session, "drums").expect("the group is there");
    session
        .execute("sound.track.group.set", &json!({ "group": group, "active": false }))
        .expect("the group is off");
    session
        .execute("sound.track.fader", &json!({ "track": tracks[0], "db": -6.0 }))
        .expect("the fader is set");
    assert_eq!(fader_of(&session, tracks[0]), Some(-6.0), "the track is set");
    assert_eq!(fader_of(&session, tracks[1]), Some(0.0), "and the group is not");
}

/// The **mix** can be taken off, and a group that does not mix links
/// nothing even while it is on.
#[test]
fn the_mix_can_be_taken_off() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": tracks }))
        .expect("the tracks are grouped");
    let group = group_named(&session, "drums").expect("the group is there");
    session
        .execute("sound.track.group.set", &json!({ "group": group, "mix": false }))
        .expect("the group does not mix");
    session
        .execute("sound.track.fader", &json!({ "track": tracks[0], "db": -6.0 }))
        .expect("the fader is set");
    assert_eq!(fader_of(&session, tracks[1]), Some(0.0), "the group is not linked");
}

/// A group is taken away and the **tracks stay**: a group is a link
/// between them and not a box they are in.
#[test]
fn a_group_is_taken_away_and_the_tracks_stay() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": tracks }))
        .expect("the tracks are grouped");
    let group = group_named(&session, "drums").expect("the group is there");
    session.execute("sound.track.ungroup", &json!({ "group": group })).expect("the group is taken away");
    assert!(group_named(&session, "drums").is_none(), "the group is gone");
    assert!(session.document().node(tracks[0]).is_some(), "the track is still there");
    session
        .execute("sound.track.fader", &json!({ "track": tracks[0], "db": -6.0 }))
        .expect("the fader is set");
    assert_eq!(fader_of(&session, tracks[1]), Some(0.0), "and nothing is linked any more");
}

/// A group of **one** track is refused, the way a group of one clip is.
#[test]
fn a_group_of_one_track_is_refused() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    let refused = session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": [tracks[0]] }))
        .expect_err("one track is not a group");
    assert!(refused.to_string().contains("two tracks"), "the error says why: {refused}");
    assert!(group_named(&session, "drums").is_none(), "no group was made");
}

/// Only a **track** is grouped.
#[test]
fn only_a_track_is_grouped() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.clip.add", &json!({ "track": tracks[0], "start": 0, "length": 500 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let refused = session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": [tracks[0], clip] }))
        .expect_err("a clip is not a track");
    assert!(refused.to_string().contains("not a track"), "the error says why: {refused}");
}

/// The group is in the **plan**: its members and its flags are said.
#[test]
fn the_group_is_in_the_plan() {
    let mut session = session();
    let tracks = two_tracks(&mut session);
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": tracks }))
        .expect("the tracks are grouped");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "sound.group").expect("the group is in the plan");
    assert_eq!(item.value.get("name").and_then(|name| name.as_str()), Some("drums"));
    assert_eq!(item.value.get("mix").and_then(|mix| mix.as_bool()), Some(true));
    assert_eq!(item.value.get("active").and_then(|active| active.as_bool()), Some(true));
    assert_eq!(
        item.value.get("size").and_then(|size| size.as_u64()),
        Some(2),
        "the group has two tracks in it"
    );
}
