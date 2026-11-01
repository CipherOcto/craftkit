//! The **name** of a node: what a person calls it by.
//!
//! A node has a name, and until this arc nothing could change it. The
//! tree this ports has a rename command for each kind of thing; the kit
//! has one command for every node.

use craftkit_sdk::serde_json::json;

/// A session over the kit.
fn session() -> craftkit_sdk::Session {
    craftkit_studio::ecosystem().expect("the ecosystem is built").session()
}

/// A node to rename.
fn a_node(session: &mut craftkit_sdk::Session, name: &str) -> u64 {
    session
        .execute("core.appearance.add", &json!({ "name": name }))
        .expect("the node is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id")
}

/// A node is **renamed**, and the new name is the node's name.
#[test]
fn a_node_is_renamed() {
    let mut session = session();
    let id = a_node(&mut session, "ink");
    session.execute("node.rename", &json!({ "node": id, "name": "wash" })).expect("the node is renamed");
    let node = session.document().node(id).expect("the node is there");
    assert_eq!(node.name, "wash");
}

/// The name is **trimmed**: a name with spaces around it is the name
/// without them.
#[test]
fn the_name_is_trimmed() {
    let mut session = session();
    let id = a_node(&mut session, "ink");
    session.execute("node.rename", &json!({ "node": id, "name": "  wash  " })).expect("the node is renamed");
    let node = session.document().node(id).expect("the node is there");
    assert_eq!(node.name, "wash");
}

/// A name that is **nothing** is refused: a node with no name cannot be
/// found.
#[test]
fn a_name_that_is_nothing_is_refused() {
    let mut session = session();
    let id = a_node(&mut session, "ink");
    let refused =
        session.execute("node.rename", &json!({ "node": id, "name": "   " })).expect_err("a node has a name");
    assert!(refused.to_string().contains("name"), "the error says why: {refused}");
    let node = session.document().node(id).expect("the node is there");
    assert_eq!(node.name, "ink", "the node kept its name");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session
        .execute("node.rename", &json!({ "node": 999, "name": "wash" }))
        .expect_err("there is no node 999");
    assert!(refused.to_string().contains("999"), "the error says why: {refused}");
}

/// Renaming a node to the name it **already has** changes nothing, and
/// a command that changes nothing leaves no undo step behind.
#[test]
fn renaming_to_the_same_name_changes_nothing() {
    let mut session = session();
    let id = a_node(&mut session, "ink");
    let outcome =
        session.execute("node.rename", &json!({ "node": id, "name": "ink" })).expect("the node is renamed");
    assert!(!outcome.changed, "nothing changed");
}
