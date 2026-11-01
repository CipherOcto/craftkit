//! The **taking away** of a link: the route, the reading of a bus, the
//! send and the source.
//!
//! A link in a sound document is a **relation** and not a node, and a
//! relation that cannot be taken away is a mistake that cannot be
//! undone.

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

/// A session with a track and a bus.
fn a_track_and_a_bus(session: &mut Session) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    session.execute("sound.bus.add", &json!({ "name": "reverb" })).expect("the bus is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    let bus = session.document().node_by_name("reverb").expect("the bus is there").id;
    (track, bus)
}

/// The **route** between a track and a bus is taken away.
#[test]
fn a_route_is_taken_away() {
    let mut session = session();
    let (track, bus) = a_track_and_a_bus(&mut session);
    session.execute("sound.route.add", &json!({ "track": track, "bus": bus })).expect("the route is made");
    assert_eq!(session.document().relations.len(), 1, "the route is there");
    session
        .execute("sound.route.remove", &json!({ "track": track, "bus": bus }))
        .expect("the route is taken away");
    assert!(session.document().relations.is_empty(), "and it is gone");
}

/// A route that is **not there** is refused: a link that was never
/// made is not a link that was taken away.
#[test]
fn a_route_that_is_not_there_is_refused() {
    let mut session = session();
    let (track, bus) = a_track_and_a_bus(&mut session);
    let refused = session.execute("sound.route.remove", &json!({ "track": track, "bus": bus }));
    assert!(refused.is_err(), "there was no route to take away");
}

/// The **reading** of a bus is taken away.
#[test]
fn a_reading_is_taken_away() {
    let mut session = session();
    let (track, bus) = a_track_and_a_bus(&mut session);
    session
        .execute("sound.track.input", &json!({ "track": track, "bus": bus }))
        .expect("the track reads the bus");
    assert_eq!(session.document().relations.len(), 1, "the reading is there");
    session
        .execute("sound.track.input.remove", &json!({ "track": track, "bus": bus }))
        .expect("the reading is taken away");
    assert!(session.document().relations.is_empty(), "and it is gone");
}

/// A **send** is taken away, and its link goes with it.
#[test]
fn a_send_is_taken_away() {
    let mut session = session();
    let (track, bus) = a_track_and_a_bus(&mut session);
    session.execute("sound.send.add", &json!({ "track": track, "bus": bus })).expect("the send is made");
    let send =
        session.document().walk().find(|node| node.kind == "sound.send").expect("the send is there").id;
    session.execute("sound.send.remove", &json!({ "send": send })).expect("the send is taken away");
    assert!(session.document().walk().all(|node| node.kind != "sound.send"), "the send is gone");
    assert!(session.document().relations.is_empty(), "and its link is gone");
}

/// A **source** is taken away, and the file it brought goes with it: a
/// document does not need a file that nothing plays.
#[test]
fn a_source_is_taken_away() {
    let mut session = session();
    session.execute("sound.source.add", &json!({ "path": "kit/kick.wav" })).expect("the source is added");
    let source =
        session.document().walk().find(|node| node.kind == "sound.source").expect("the source is there").id;
    assert_eq!(session.document().assets.len(), 1, "the document needs the file");
    let removed = session
        .execute("sound.source.remove", &json!({ "source": source }))
        .expect("the source is taken away");
    assert_eq!(removed.data["assets"], 1, "and the asset went with it");
    assert!(session.document().assets.is_empty(), "the document does not need it");
    assert!(session.document().walk().all(|node| node.kind != "sound.source"), "and the source is gone");
}

/// A node that is **not** a send or a source is refused.
#[test]
fn a_node_that_is_not_a_send_is_refused() {
    let mut session = session();
    let (track, _bus) = a_track_and_a_bus(&mut session);
    let refused = session.execute("sound.send.remove", &json!({ "send": track }));
    assert!(refused.is_err(), "the track is not a send");
    let refused = session.execute("sound.source.remove", &json!({ "source": track }));
    assert!(refused.is_err(), "and it is not a source either");
}
