//! The **bounds** of a document.
//!
//! A document is a tree of nodes with relations, and the shape of a
//! document is the kit's business: how deep the tree may be, how many
//! nodes it may hold and how many relations it may hold. These are the
//! tests that hold those numbers.

use craftkit_sdk::Document;
use craftkit_sdk::document::{MAX_DEPTH, MAX_NODES, MAX_RELATIONS};
use serde_json::json;

/// A node with a child under it, `depth` levels deep.
fn a_node(depth: usize) -> serde_json::Value {
    let mut value = json!({ "id": 1, "name": "leaf", "kind": "shape.rect" });
    for level in 0..depth {
        value = json!({
            "id": level + 2,
            "name": "group",
            "kind": "shape.group",
            "children": [value],
        });
    }
    value
}

/// A document with `count` nodes at the top level.
fn a_document_with(count: usize) -> Document {
    let mut document = Document::new("many");
    for index in 0..count {
        document.add_node(format!("node {index}"), "shape.rect", None);
    }
    document
}

/// A node nested deeper than the kit allows is **refused** — and the
/// refusal is a value, not a crash: the reader is recursive, and a
/// recursion that runs out of stack is a crash the panic guard cannot
/// catch.
#[test]
fn a_document_that_is_too_deep_is_refused() {
    let value = json!({
        "name": "deep",
        "nodes": [a_node(MAX_DEPTH + 5)],
    });
    let error = Document::from_json(&value).expect_err("that document is too deep");
    assert!(error.contains("deeper"), "{error}");
}

/// And a document that fits is read.
#[test]
fn a_document_that_fits_is_read() {
    let value = json!({
        "name": "deep enough",
        "nodes": [a_node(MAX_DEPTH - 5)],
    });
    let document = Document::from_json(&value).expect("that document is read");
    assert_eq!(document.name, "deep enough");
}

/// A document with more **nodes** than the kit allows is refused.
#[test]
fn a_document_with_too_many_nodes_is_refused() {
    let document = a_document_with(MAX_NODES + 1);
    let error = document.check_bounds().expect_err("that document holds too many nodes");
    assert!(error.contains("nodes"), "{error}");
}

/// And a document that fits is fine.
#[test]
fn a_document_with_enough_nodes_is_fine() {
    let document = a_document_with(10);
    document.check_bounds().expect("that document is fine");
}

/// A document with more **relations** than the kit allows is refused.
#[test]
fn a_document_with_too_many_relations_is_refused() {
    let mut document = a_document_with(2);
    for index in 0..MAX_RELATIONS + 1 {
        document.add_relation(craftkit_sdk::Relation::new(1, 2, format!("edge.{index}")));
    }
    let error = document.check_bounds().expect_err("that document holds too many relations");
    assert!(error.contains("relations"), "{error}");
}

/// A node's children are read, and the document's own tree is what the
/// bound is about: a node under a node is one level deeper, not one
/// more node.
#[test]
fn a_child_is_a_level_deeper() {
    let mut document = Document::new("deep");
    let group = document.add_node("group", "shape.group", None);
    let leaf = document.add_node("leaf", "shape.rect", None);
    document.add_node_under(Some(group), "leaf", "shape.rect", None);
    document.check_bounds().expect("a group with a child is fine");
    assert!(leaf > 0);
}

/// The **door** refuses too: a file with more nodes than the kit
/// allows is not read, and the refusal says which bound it broke.
#[test]
fn the_reader_refuses_a_document_with_too_many_nodes() {
    let document = a_document_with(MAX_NODES + 1);
    let formats = craftkit_sdk::contribution::Registry::from_inventory().expect("the formats are there");
    let newest = craftkit_sdk::format::newest_format(&formats, "craftkit").expect("there is a format");
    let mut value = document.to_json();
    value["format"] = json!(newest.tag);
    let migrations = craftkit_sdk::contribution::Registry::<craftkit_sdk::format::MigrationSpec>::new();
    let error = craftkit_sdk::format::read_document(&value, &formats, &migrations)
        .expect_err("that document holds too many nodes");
    assert!(error.contains("nodes"), "{error}");
}
