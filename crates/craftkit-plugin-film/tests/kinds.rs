//! The **kinds** of the film battery: what a caption cue means.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **caption track**, and its id.
fn a_track(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// A **caption** is a kind, and it means the two times and the text.
#[test]
fn a_caption_means_its_times_and_its_text() {
    let mut session = session();
    let track = a_track(&mut session);
    let outcome = session
        .execute("film.caption.add", &json!({ "track": track, "start": 12, "end": 36, "text": "the words" }))
        .expect("the cue is added");
    let caption = outcome.data["id"].as_u64().expect("the cue has an id");
    let node = session.document().node(caption).expect("the cue is there");
    assert_eq!(node.kind, "film.caption", "the node is a caption");
    let said = said_of(&session, caption);
    assert_eq!(said["start"], json!(12), "the caption says when it starts");
    assert_eq!(said["end"], json!(36), "and when it ends");
    assert_eq!(said["text"], json!("the words"), "and what it says");
}
