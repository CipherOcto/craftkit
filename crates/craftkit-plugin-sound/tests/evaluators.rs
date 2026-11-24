//! The **evaluators** of the sound battery: what the whole document
//! means.
//!
//! An evaluator is a gesture about the **whole document**: it may look at
//! every node and say something about all of them. It is a rule the kit
//! publishes as much as a command or a kind is, and an evaluator that is
//! only proved in another battery's tests is a rule that does not travel
//! with the thing it is about.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands linked
/// into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// The **timeline** evaluator is there, and it is the sound layer's.
#[test]
fn the_timeline_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("sound.timeline").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Timeline", "the evaluator is called Timeline");
    assert_eq!(evaluator.layer, "sound", "and the sound layer owns it");
}

/// The evaluator says what the document's **tracks** are.
#[test]
fn the_timeline_evaluator_says_the_tracks_of_the_document() {
    let mut session = session();
    let outcome =
        session.execute("sound.track.add", &json!({ "name": "snare" })).expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "sound.timeline").expect("the plan has the timeline");
    let tracks = item.value["tracks"].as_array().expect("the tracks are a list");
    assert_eq!(tracks.len(), 1, "the document holds one track: {item:?}");
    assert_eq!(tracks[0]["id"], json!(track), "and it is that track");
}
