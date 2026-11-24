//! The **kinds** of the prose battery: what an equation and a list mean.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> Json {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// An **equation** is a kind, and it means its text and its parts.
///
/// The text is **derived** from the parts: it is never written down.
#[test]
fn an_equation_means_its_text_and_its_parts() {
    let mut session = session();
    let document = a_document(&mut session);
    let parts = json!({ "kind": "fraction", "top": { "kind": "text", "text": "a" }, "bottom": { "kind": "text", "text": "b" } });
    let outcome = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect("the equation is added");
    let equation = outcome.data["id"].as_u64().expect("the equation has an id");
    let node = session.document().node(equation).expect("the equation is there");
    assert_eq!(node.kind, "prose.block.equation", "the node is an equation");
    let said = said_of(&session, equation);
    assert_eq!(said["text"], json!("a/b"), "the equation says its text");
    assert_eq!(said["parts"]["kind"], json!("fraction"), "and the parts it is made of");
}

/// A **list** is a kind, and it means its kind and the blocks that name
/// it.
///
/// The blocks are **derived**: they are the blocks of the document that
/// name the list.
#[test]
fn a_list_means_its_kind_and_the_blocks_that_name_it() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.list.add", &json!({ "document": document, "kind": "bullet" }))
        .expect("the list is added");
    let list = outcome.data["id"].as_u64().expect("the list has an id");
    let node = session.document().node(list).expect("the list is there");
    assert_eq!(node.kind, "prose.list", "the node is a list");
    let said = said_of(&session, list);
    assert_eq!(said["kind"], json!("bullet"), "the list says what kind of list it is");
    assert_eq!(said["blocks"], json!([]), "and which blocks name it");
}
