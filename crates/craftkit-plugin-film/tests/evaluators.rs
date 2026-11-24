//! The **evaluators** of the film battery: what the whole document means.
//!
//! An evaluator is a gesture about the **whole document**: it may look at
//! every node and say something about all of them. It is a rule the kit
//! publishes as much as a command or a kind is, and an evaluator that is
//! only proved in another battery's tests is a rule that does not travel
//! with the thing it is about.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

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

/// The **timeline** evaluator is there, and it is the film layer's.
#[test]
fn the_timeline_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("film.timeline").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Timeline", "the evaluator is called Timeline");
    assert_eq!(evaluator.layer, "film", "and the film layer owns it");
}

/// The evaluator says how long every **track** is: the end of its last
/// clip.
#[test]
fn the_timeline_evaluator_says_the_end_of_every_track() {
    let mut session = session();
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": "the clip", "bin": bin, "item": 1, "duration": 100, "path": "/media/clip.mov" }),
        )
        .expect("the item is added");
    let sequence = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": sequence, "start": 10, "duration": 50, "source_in": 0 }),
        )
        .expect("the clip is added");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "film.timeline").expect("the plan has the timeline");
    assert_eq!(item.node, track, "the item is about the track");
    assert_eq!(item.value["end"], json!(60), "the clip ends at sixty: {item:?}");
}
