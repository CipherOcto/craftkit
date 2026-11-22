//! The **name** of a node.
//!
//! A node's name is what a person calls it, and a document whose names
//! cannot be changed is a document whose names are wrong forever.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem of the kit itself.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session with one node in it.
///
/// The node is a **layer**: the kit's own command that makes a node,
/// and the document is only ever changed by a command.
fn a_document() -> Session {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session.execute("core.layer.add", &json!({ "name": "the shot" })).expect("the layer is added");
    session
}

/// A node may be **renamed**, and the name is the node's own.
#[test]
fn a_node_may_be_renamed() {
    let mut session = a_document();
    session.execute("node.name.set", &json!({ "node": 1, "name": "the shot" })).expect("the node is renamed");
    let node = session.document().node(1).expect("the node is there");
    assert_eq!(node.name, "the shot", "the node is called what it was told");
}

/// A node may be found **by its name**.
#[test]
fn a_node_may_be_found_by_its_name() {
    let mut session = a_document();
    session.execute("node.name.set", &json!({ "node": 1, "name": "the shot" })).expect("the node is renamed");
    let found = session.document().node_by_name("the shot").expect("the node is found");
    assert_eq!(found.id, 1, "and it is the node that was renamed");
}

/// An **empty name** is refused: an empty name is not a name.
#[test]
fn an_empty_name_is_refused() {
    let mut session = a_document();
    let error = session
        .execute("node.name.set", &json!({ "node": 1, "name": "   " }))
        .expect_err("an empty name is not a name");
    assert!(error.to_string().contains("a node has a name"), "the refusal says what is wrong: {error}");
    let node = session.document().node(1).expect("the node is there");
    assert_ne!(node.name, "", "and the node kept the name it had");
}

/// A node that is **not there** may not be renamed.
#[test]
fn a_node_that_is_not_there_may_not_be_renamed() {
    let mut session = a_document();
    let error = session
        .execute("node.name.set", &json!({ "node": 9999, "name": "the shot" }))
        .expect_err("there is no node 9999");
    assert!(error.to_string().contains("there is no node"), "the refusal says what is wrong: {error}");
}
