//! The **kinds** of the pdf battery: what the information means.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// The **information** is a kind, and it means the object it names.
///
/// There is **one** information dictionary in a file, and a second one
/// would be a second answer to one question.
#[test]
fn the_information_means_the_object_it_names() {
    let mut session = session();
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 3, "entries": [["Title", "the file"]] }))
        .expect("the object is added");
    let object = outcome.data["id"].as_u64().expect("the object has an id");
    let outcome =
        session.execute("pdf.info.set", &json!({ "object": object })).expect("the information is set");
    let information = outcome.data["id"].as_u64().expect("the information has an id");
    let node = session.document().node(information).expect("the information is there");
    assert_eq!(node.kind, "pdf.info", "the node is the information");
    let said = said_of(&session, information);
    assert_eq!(said["object"], json!(object), "the information says which object it is");
}
