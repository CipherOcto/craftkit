//! The **cheap undo**: the history's states share what they have in
//! common.
//!
//! What is proved here: a state and the document it came from **share**
//! the nodes that were not changed, a state keeps what it was when the
//! document moves on, and the cost of an edit does **not** grow with the
//! size of the document.
//!
//! The last one is a **measurement**: the counting allocator is
//! installed in this test binary, and the numbers are the difference
//! between two readings.

use std::sync::Arc;

use craftkit_sdk::serde_json::json;

/// The counting allocator, installed in this test binary.
///
/// Declaring it is safe code: the `unsafe` is in the **impl**, and that
/// is in the crate that has the exception.
#[global_allocator]
static ALLOCATOR: craftkit_alloc::Counting = craftkit_alloc::Counting;

/// The ecosystem the tests drive.
fn ecosystem() -> craftkit_sdk::Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with a document of `nodes` nodes.
fn a_document(nodes: usize) -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    for index in 0..nodes {
        session
            .execute("node.add", &json!({ "name": format!("box {index}"), "kind": "geometry.rect" }))
            .expect("the node is added");
    }
    session
}

/// A state and the document **share** the node they have in common.
///
/// This is the whole of copy-on-write, and it is a fact about the
/// pointers and not about the time: the two hold the **same** node.
#[test]
fn a_state_shares_the_nodes_it_did_not_change() {
    let session = a_document(3);
    let state_id = session.history().current();
    // The state the document came from, and the document itself.
    let state = session.history().node(state_id).expect("the state is there");
    let shared = Arc::ptr_eq(&session.document().roots()[0], &state.document.roots()[0]);
    assert!(shared, "the state and the document share the node");
}

/// And the state **keeps what it was** when the document moves on.
#[test]
fn a_state_keeps_what_it_was() {
    let mut session = a_document(1);
    let state_id = session.history().current();
    session.execute("node.mute", &json!({ "node": 1 })).expect("the node is muted");
    let state = session.history().node(state_id).expect("the state is there");
    assert!(!state.document.roots()[0].muted, "the state says the node was not muted");
    assert!(session.document().roots()[0].muted, "and the document says it is");
    // The two are different nodes now: the changed one was copied.
    let copied = !Arc::ptr_eq(&session.document().roots()[0], &state.document.roots()[0]);
    assert!(copied, "the changed node was copied, so the state kept its own");
}

/// The cost of an edit does **not** grow with the size of the document.
///
/// Before this, every edit copied the whole document, so an edit cost
/// more the bigger the document was. Now the copy is shared, and the
/// measurement says so: the same number of edits on a document twenty
/// times the size allocates no more than a handful of times as much.
#[test]
fn the_cost_of_an_edit_does_not_grow_with_the_document() {
    /// The allocations made by `edits` edits on a document of `nodes`.
    fn allocations(nodes: usize, edits: usize) -> u64 {
        let mut session = a_document(nodes);
        // The build is not part of the measurement.
        let before = craftkit_alloc::counts().allocations;
        for index in 0..edits {
            session
                .execute("node.add", &json!({ "name": format!("more {index}"), "kind": "geometry.rect" }))
                .expect("the node is added");
        }
        let after = craftkit_alloc::counts().allocations;
        after - before
    }

    let small = allocations(10, 20);
    let large = allocations(400, 20);
    assert!(small > 0, "the measurement measured something: {small}");
    // Twenty times the document, and the cost has not gone up by much:
    // the copy is shared, so what an edit costs does not depend on how
    // big the document is.
    let allowed = small * 2 + 64;
    assert!(
        large <= allowed,
        "twenty times the document cost {large} allocations and ten times it cost {small}: the copy is not shared"
    );
}

/// And the undo still puts the document back.
#[test]
fn the_undo_puts_the_document_back() {
    let mut session = a_document(2);
    session.execute("node.mute", &json!({ "node": 1 })).expect("the node is muted");
    assert!(session.document().roots()[0].muted);
    session.undo().expect("the undo works");
    assert!(!session.document().roots()[0].muted, "the node is not muted any more");
    assert_eq!(session.document().roots().len(), 2, "and the document is as it was");
}
