//! The **copy** of a node: a new thing with the same shape.
//!
//! The kit had no command to duplicate a node, and the tree this ports
//! has one for each kind of thing. This arc brings the one command that
//! copies a node and everything under it.

use craftkit_sdk::serde_json::json;

/// A session over the kit.
fn session() -> craftkit_sdk::Session {
    craftkit_studio::ecosystem().expect("the ecosystem is built").session()
}

/// A node, with the id it was given.
fn a_node(session: &mut craftkit_sdk::Session, name: &str, kind: &str) -> u64 {
    session
        .execute("node.add", &json!({ "name": name, "kind": kind }))
        .expect("the node is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id")
}

/// A node is **duplicated**, and the copy is a new thing.
#[test]
fn a_node_is_duplicated() {
    let mut session = session();
    let id = a_node(&mut session, "box", "core.group");
    let outcome = session.execute("node.duplicate", &json!({ "node": id })).expect("the node is duplicated");
    let copy = outcome.data.get("id").and_then(|id| id.as_u64()).expect("there is an id");
    assert_ne!(copy, id, "the copy is a new node");
    let node = session.document().node(copy).expect("the copy is there");
    assert_eq!(node.kind, "core.group", "the copy is the same kind of thing");
    assert_eq!(node.name, "box.dup", "and the copy is named after the node");
    // The node that was copied is still there.
    assert!(session.document().node(id).is_some(), "the node is still there");
}

/// What is **under** the node is copied too.
#[test]
fn what_is_under_it_is_copied() {
    let mut session = session();
    let group = a_node(&mut session, "box", "core.group");
    let first = session
        .execute("node.add", &json!({ "name": "lid", "kind": "core.group", "parent": group }))
        .expect("the child is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let second = session
        .execute("node.add", &json!({ "name": "base", "kind": "core.group", "parent": group }))
        .expect("the child is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let copy = session
        .execute("node.duplicate", &json!({ "node": group }))
        .expect("the node is duplicated")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let copied = session.document().node(copy).expect("the copy is there");
    assert_eq!(copied.children.len(), 2, "the copy has what the node had");
    for child in &copied.children {
        assert_ne!(child.id, first, "the child is a new node");
        assert_ne!(child.id, second, "the child is a new node");
    }
    let names: Vec<String> = copied.children.iter().map(|child| child.name.clone()).collect();
    assert!(names.contains(&"lid".to_string()), "the child kept its name: {names:?}");
    assert!(names.contains(&"base".to_string()), "the child kept its name: {names:?}");
}

/// The **edges** are copied: a node on a layer is copied onto the same
/// layer.
#[test]
fn the_edges_are_copied() {
    let mut session = session();
    let layer = session
        .execute("core.layer.add", &json!({ "name": "paper" }))
        .expect("the layer is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let id = a_node(&mut session, "box", "core.group");
    session
        .execute("core.layer.on", &json!({ "node": id, "layer": layer }))
        .expect("the node is on the layer");
    let copy = session
        .execute("node.duplicate", &json!({ "node": id }))
        .expect("the node is duplicated")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let layers: Vec<u64> = session
        .document()
        .relations_from(copy, Some("core.on_layer"))
        .into_iter()
        .map(|relation| relation.to)
        .collect();
    assert_eq!(layers, vec![layer], "the copy is on the same layer");
}

/// The copy may be **named**.
#[test]
fn the_copy_may_be_named() {
    let mut session = session();
    let id = a_node(&mut session, "box", "core.group");
    let copy = session
        .execute("node.duplicate", &json!({ "node": id, "name": "lid" }))
        .expect("the node is duplicated")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let node = session.document().node(copy).expect("the copy is there");
    assert_eq!(node.name, "lid");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused =
        session.execute("node.duplicate", &json!({ "node": 999 })).expect_err("there is no node 999");
    assert!(refused.to_string().contains("999"), "the error says why: {refused}");
}

/// A **track** is duplicated with its clips, and the copy's clips play
/// the same source.
#[cfg(feature = "sound")]
#[test]
fn a_track_is_duplicated_with_its_clips() {
    let mut session = session();
    let source = session
        .execute("sound.source.add", &json!({ "path": "/tmp/craftkit-v2-copy.wav" }))
        .expect("the source is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let track = session
        .execute("sound.track.add", &json!({ "name": "drums" }))
        .expect("the track is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let clip = session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let copy = session
        .execute("node.duplicate", &json!({ "node": track }))
        .expect("the track is duplicated")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let node = session.document().node(copy).expect("the copy is there");
    assert_eq!(node.kind, "sound.track", "the copy is a track");
    let playlist = node.children.first().map(|child| child.id).expect("the copy has a playlist");
    let copied = session.document().node(playlist).expect("the playlist is there");
    assert_eq!(copied.children.len(), 1, "the copy has the clip the track had");
    let copied_clip = copied.children[0].id;
    assert_ne!(copied_clip, clip, "the clip is a new node");
    // And the copy's clip still plays the same source.
    let sources: Vec<u64> = session
        .document()
        .relations_from(copied_clip, Some("source"))
        .into_iter()
        .map(|relation| relation.to)
        .collect();
    assert_eq!(sources, vec![source], "the copy's clip plays the same source");
}
