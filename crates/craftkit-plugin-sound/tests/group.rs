//! The **group** a clip is in: a set of clips that belong together.
//!
//! The tree's sound editor keeps a group on a clip as a shared number.
//! Here the group is a node with a name, and the clips in it point at
//! it.

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

/// A track with three clips on it, and the ids of the clips.
fn three_clips(session: &mut Session) -> (u64, Vec<u64>) {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let mut clips = Vec::new();
    for start in [0, 1000, 2000] {
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": start, "length": 500 }))
            .expect("the clip is added");
        let id = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.clip")
            .last()
            .map(|node| node.id)
            .expect("the clip is there");
        clips.push(id);
    }
    (track, clips)
}

/// The group a clip is in, read from the plan.
fn group_in_plan(session: &Session, clip: u64) -> Option<u64> {
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == clip && item.kind == "sound.clip")
        .expect("the clip is in the plan");
    item.value.get("group").and_then(|group| group.as_u64())
}

/// The group node of a name, if it is there.
fn group_named(session: &Session, name: &str) -> Option<u64> {
    session.document().walk().find(|node| node.kind == "sound.group" && node.name == name).map(|node| node.id)
}

/// Clips are put in a **group**, and the group holds them.
#[test]
fn clips_are_put_in_a_group() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    let outcome = session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[1]] }))
        .expect("the clips are grouped");
    assert!(outcome.changed, "the document changed");
    let group = group_named(&session, "Drums").expect("the group is there");
    assert_eq!(group_in_plan(&session, clips[0]), Some(group), "the first clip is in the group");
    assert_eq!(group_in_plan(&session, clips[1]), Some(group), "the second clip is in the group");
    assert_eq!(group_in_plan(&session, clips[2]), None, "the third clip is in no group");
}

/// The group is a **node**, and the clips stay where they are: a group
/// is not a folder.
#[test]
fn a_group_holds_nothing_and_the_clips_stay() {
    let mut session = session();
    let (track, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[1]] }))
        .expect("the clips are grouped");
    let group = group_named(&session, "Drums").expect("the group is there");
    let node = session.document().node(group).expect("the group is there");
    assert!(node.children.is_empty(), "the group holds nothing");
    // The clips are still on their track, and where they were.
    let playlist =
        session.document().walk().find(|node| node.kind == "sound.playlist").expect("the playlist is there");
    let on_the_playlist = |clip: u64| playlist.children.iter().any(|child| child.id == clip);
    assert!(on_the_playlist(clips[0]), "the clip is still on the playlist");
    assert!(on_the_playlist(clips[1]), "the clip is still on the playlist");
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("start").and_then(|start| start.as_i64()), Some(0));
    let _ = track;
}

/// A group is **named**, and the name is the handle: grouping clips
/// under the same name again puts them back in the same group. This is
/// what the tree's `clip.regroup` does by counting ids.
#[test]
fn the_name_is_the_handle() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[1]] }))
        .expect("the clips are grouped");
    let group = group_named(&session, "Drums").expect("the group is there");
    session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[2]] }))
        .expect("the clips are grouped again");
    assert_eq!(group_named(&session, "Drums"), Some(group), "the same group is used");
    assert_eq!(group_in_plan(&session, clips[2]), Some(group), "the third clip joined it");
    assert_eq!(group_in_plan(&session, clips[1]), Some(group), "the second clip is still in it");
    assert_eq!(group_in_plan(&session, clips[0]), Some(group), "the first clip is still in it");
}

/// A clip is in **one** group: a clip that is put in a group leaves the
/// group it was in.
#[test]
fn a_clip_is_in_one_group() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[1]] }))
        .expect("the clips are grouped");
    session
        .execute("sound.clip.group", &json!({ "name": "Bass", "clips": [clips[0], clips[2]] }))
        .expect("the clips are grouped again");
    let bass = group_named(&session, "Bass").expect("the second group is there");
    let drums = group_named(&session, "Drums").expect("the first group is still there");
    assert_eq!(group_in_plan(&session, clips[0]), Some(bass), "the clip is in the new group");
    assert_ne!(group_in_plan(&session, clips[0]), Some(drums), "and not in the old one");
    assert_eq!(
        group_in_plan(&session, clips[1]),
        Some(drums),
        "the clip that was not moved is still in the old group"
    );
    assert_eq!(group_in_plan(&session, clips[2]), Some(bass), "the third clip is in the new group");
}
/// A group of **one** clip is refused: a group is a set of clips that
/// belong together, and one clip belongs to itself.
#[test]
fn a_group_of_one_clip_is_refused() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    let refused = session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0]] }))
        .expect_err("one clip is not a group");
    assert!(refused.to_string().contains("two clips"), "the error says why: {refused}");
    assert!(group_named(&session, "Drums").is_none(), "no group was made");
}

/// A clip is taken out of its group, and a group that is left with
/// nothing is **taken away**: an empty group is not a group.
#[test]
fn a_clip_leaves_its_group() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[1]] }))
        .expect("the clips are grouped");
    session
        .execute("sound.clip.ungroup", &json!({ "clips": [clips[0]] }))
        .expect("the clip leaves the group");
    assert_eq!(group_in_plan(&session, clips[0]), None, "the clip is in no group");
    assert!(group_named(&session, "Drums").is_some(), "the group still has a clip in it");
    session
        .execute("sound.clip.ungroup", &json!({ "clips": [clips[1]] }))
        .expect("the last clip leaves the group");
    assert!(group_named(&session, "Drums").is_none(), "the empty group is gone");
}

/// A clip that is in no group is **left alone**: the command changes
/// nothing, and a command that changes nothing leaves no undo step
/// behind.
#[test]
fn a_clip_in_no_group_is_left_alone() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    let outcome = session
        .execute("sound.clip.ungroup", &json!({ "clips": [clips[0]] }))
        .expect("the clip is left alone");
    assert!(!outcome.changed, "nothing changed");
}

/// The group is **read**: the group of a clip is derived from the edge
/// it carries, and the members of a group are the edges that point at
/// it.
#[test]
fn the_group_is_read() {
    let mut session = session();
    let (_, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], clips[1]] }))
        .expect("the clips are grouped");
    let outcome =
        session.execute("sound.clip.group.read", &json!({ "clip": clips[0] })).expect("the group is read");
    let members = outcome.data.get("members").and_then(|members| members.as_array());
    let members = members.expect("the members are there");
    assert_eq!(members.len(), 2, "the group has two clips in it");
    assert_eq!(outcome.data.get("name").and_then(|name| name.as_str()), Some("Drums"), "the group is named");
    let outcome =
        session.execute("sound.clip.group.read", &json!({ "clip": clips[2] })).expect("the clip is read");
    assert!(outcome.data.get("group").map(|group| group.is_null()).unwrap_or(false));
}

/// Only a **clip** is grouped: a track is not a clip, and a group of
/// tracks is not a thing here.
#[test]
fn only_a_clip_can_be_grouped() {
    let mut session = session();
    let (track, clips) = three_clips(&mut session);
    let refused = session
        .execute("sound.clip.group", &json!({ "name": "Drums", "clips": [clips[0], track] }))
        .expect_err("a track is not a clip");
    assert!(refused.to_string().contains("not a clip"), "the error says why: {refused}");
}
