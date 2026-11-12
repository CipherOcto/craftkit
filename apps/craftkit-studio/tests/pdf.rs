//! The PDF battery: the **object model** of a PDF document.
//!
//! A PDF file is an object graph, not a tree: indirect objects, each one
//! a dictionary of pairs, some with a stream of bytes, and some referring
//! to other objects.

use craftkit_sdk::Ecosystem;

// The tests that use it are the ones for the battery, and the battery is
// optional: the import is with them.
#[cfg(feature = "pdf")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "pdf")]
#[test]
fn the_pdf_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("pdf.object.add").is_some(), "the battery is in");
    assert!(ecosystem.commands.get("pdf.ref.add").is_some());
    assert!(ecosystem.kinds.get("pdf.object").is_some());
    assert!(ecosystem.kinds.get("pdf.stream").is_some());
    assert!(ecosystem.evaluators.get("pdf.graph").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "pdf"))]
#[test]
fn the_pdf_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("pdf.object.add").is_none(), "the battery is out");
}

/// An indirect object is a node with a dictionary.
#[cfg(feature = "pdf")]
#[test]
fn an_indirect_object_is_a_node_with_a_dictionary() {
    let mut session = ecosystem().session();
    session
        .execute(
            "pdf.object.add",
            &json!({
                "object": 12,
                "entries": [["Type", "Page"], ["MediaBox", [0, 0, 595, 842]]],
            }),
        )
        .expect("the object is added");
    let node = session.document().node(1).expect("the node is there");
    assert_eq!(node.kind, "pdf.object");
    assert_eq!(node.data["object"], json!(12), "the object number is kept");
    // The dictionary is a **list of pairs**, because a JSON object does
    // not keep the order of its keys.
    assert_eq!(node.data["entries"][0], json!(["Type", "Page"]));
    assert_eq!(node.data["entries"][1], json!(["MediaBox", [0, 0, 595, 842]]));
}

/// The dictionary is a list of pairs, and anything else is refused.
#[cfg(feature = "pdf")]
#[test]
fn a_dictionary_that_is_not_a_list_of_pairs_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("pdf.object.add", &json!({ "object": 3, "entries": { "Type": "Page" } }))
        .expect_err("the dictionary is not a list of pairs");
    assert!(error.to_string().contains("pairs"), "{error}");
}

/// A reference says which key it is under, and which generation.
#[cfg(feature = "pdf")]
#[test]
fn a_reference_says_which_key_it_is_under() {
    let mut session = ecosystem().session();
    for object in [1, 2] {
        session
            .execute("pdf.object.add", &json!({ "object": object, "entries": [] }))
            .expect("the object is added");
    }
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Resources", "to": 2, "generation": 0 }))
        .expect("the reference is added");
    let relation = &session.document().relations[0];
    assert_eq!(relation.kind, "ref");
    assert_eq!(relation.payload["key"], json!("Resources"), "the key is in the payload");
    assert_eq!(relation.payload["generation"], json!(0), "the generation is in the payload");
}

/// The graph is built in **two passes**: a reference names two objects
/// that are in the document, and a PDF may refer forward.
#[cfg(feature = "pdf")]
#[test]
fn the_graph_is_built_in_two_passes() {
    let mut session = ecosystem().session();
    session.execute("pdf.object.add", &json!({ "object": 7, "entries": [] })).expect("the object is added");
    // The reference names an object that is not there yet.
    let error = session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Kids", "to": 2 }))
        .expect_err("the object is not there yet");
    assert!(error.to_string().contains("no object"), "{error}");
    // The second pass: the object arrives, and the reference is made.
    session.execute("pdf.object.add", &json!({ "object": 8, "entries": [] })).expect("the object is added");
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Kids", "to": 2 }))
        .expect("the reference is added");
    assert_eq!(session.document().relations.len(), 1);
}

/// An object with a stream: the bytes are beside the document.
#[cfg(feature = "pdf")]
#[test]
fn an_object_with_a_stream_carries_its_bytes() {
    let mut session = ecosystem().session();
    session
        .execute(
            "pdf.stream.add",
            &json!({
                "object": 4,
                "entries": [["Type", "XObject"], ["Subtype", "Image"]],
                "stream": "streams/object-4.bin",
            }),
        )
        .expect("the stream is added");
    let node = session.document().node(1).expect("the node is there");
    assert_eq!(node.kind, "pdf.stream");
    assert_eq!(node.stream.as_deref(), Some("streams/object-4.bin"), "the stream is named");
}

/// The evaluator counts the references that point at each object.
///
/// The count is **derived**: it is never stored, and it changes when a
/// reference is made.
#[cfg(feature = "pdf")]
#[test]
fn the_evaluator_counts_the_references() {
    let mut session = ecosystem().session();
    for object in [1, 2, 3] {
        session
            .execute("pdf.object.add", &json!({ "object": object, "entries": [] }))
            .expect("the object is added");
    }
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "A", "to": 3 }))
        .expect("the reference is added");
    session
        .execute("pdf.ref.add", &json!({ "from": 2, "key": "B", "to": 3 }))
        .expect("the reference is added");

    let plan = session.evaluate();
    let graph: Vec<_> = plan.items.iter().filter(|item| item.kind == "pdf.graph").collect();
    assert_eq!(graph.len(), 3, "every object is in the graph: {plan:?}");
    let third = graph.iter().find(|item| item.node == 3).expect("the third object is there");
    assert_eq!(third.value["referenced_by"], json!(2), "two references point at it");
    let first = graph.iter().find(|item| item.node == 1).expect("the first object is there");
    assert_eq!(first.value["referenced_by"], json!(0), "nothing points at it");
}

/// The references can be asked for.
#[cfg(feature = "pdf")]
#[test]
fn the_references_can_be_asked_for() {
    let mut session = ecosystem().session();
    for object in [1, 2] {
        session
            .execute("pdf.object.add", &json!({ "object": object, "entries": [] }))
            .expect("the object is added");
    }
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Parent", "to": 2 }))
        .expect("the reference is added");
    let outcome = session.execute("pdf.refs", &json!({})).expect("the references are read");
    assert_eq!(outcome.data["references"][0]["payload"]["key"], json!("Parent"));
}
