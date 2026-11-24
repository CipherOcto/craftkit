//! The **evaluators** of the pdf battery: what the whole document means.
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
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

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

/// The **graph** evaluator is there, and it is the pdf layer's.
#[test]
fn the_graph_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("pdf.graph").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Object graph", "the evaluator is called Object graph");
    assert_eq!(evaluator.layer, "pdf", "and the pdf layer owns it");
}

/// The evaluator counts what points at each **object**.
#[test]
fn the_graph_evaluator_counts_the_references_to_an_object() {
    let mut session = session();
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Catalog"]] }))
        .expect("the object is added");
    let object = outcome.data["id"].as_u64().expect("the object has an id");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "pdf.graph").expect("the plan has the graph");
    assert_eq!(item.node, object, "the item is about the object");
    assert_eq!(item.value["object"], json!(1), "the item says which object it is: {item:?}");
    assert_eq!(item.value["referenced_by"], json!(0), "and nothing points at it yet");
}
