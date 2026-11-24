//! The **steps** of the file format: what each migration says.
//!
//! A migration is a rule the kit publishes: it says what a file of one
//! version becomes when it is read by a program that knows a later one.
//! Nine steps carry a file from the first version to the tenth, and a
//! step that no test names is a step that nothing proves.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::format::read_document;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{MigrationSpec, Registry};

/// The ecosystem of the kit itself.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// The **migrations** of the kit.
fn migrations() -> Registry<MigrationSpec> {
    ecosystem().migrations
}

/// The step of an id, or nothing.
fn step(id: &str) -> MigrationSpec {
    let migrations = migrations();
    *migrations.get(id).unwrap_or_else(|| panic!("there is a step `{id}`"))
}

/// A **file** of the first version: a list of nodes, and nothing else.
fn a_file_of_the_first_version() -> Json {
    json!({
        "format": "craftkit/1",
        "version": 1,
        "name": "the old file",
        "nodes": [
            { "id": 1, "name": "the node", "kind": "core.layer",
              "rect": { "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0 } }
        ],
    })
}

/// The step from the first version to the second gives every node the
/// flag the second version writes.
#[test]
fn the_step_from_the_first_version_gives_every_node_the_muted_flag() {
    let mut file = a_file_of_the_first_version();
    (step("craftkit.1to2").apply)(&mut file).expect("the step runs");
    let nodes = file["nodes"].as_array().expect("the file has nodes");
    assert_eq!(nodes[0]["muted"], json!(false), "the node is not muted");
}

/// The step from the second version to the third gives every node the
/// children the third version writes.
#[test]
fn the_step_from_the_second_version_gives_every_node_its_children() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in ["craftkit.1to2", "craftkit.2to3"] {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    let nodes = file["nodes"].as_array().expect("the file has nodes");
    assert_eq!(nodes[0]["children"], json!([]), "the node holds nothing");
}

/// The step from the third version to the fourth gives the document the
/// payload the fourth version writes.
#[test]
fn the_step_from_the_third_version_gives_the_document_its_payload() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in ["craftkit.1to2", "craftkit.2to3", "craftkit.3to4"] {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    assert_eq!(file["data"], Json::Null, "the document carries no payload");
}

/// The step from the fourth version to the fifth gives the document the
/// relations the fifth version writes.
#[test]
fn the_step_from_the_fourth_version_gives_the_document_its_relations() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in ["craftkit.1to2", "craftkit.2to3", "craftkit.3to4", "craftkit.4to5"] {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    assert_eq!(file["relations"], json!([]), "the document has no relations");
}

/// The step from the fifth version to the sixth gives the document the
/// assets the sixth version writes.
#[test]
fn the_step_from_the_fifth_version_gives_the_document_its_assets() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in ["craftkit.1to2", "craftkit.2to3", "craftkit.3to4", "craftkit.4to5", "craftkit.5to6"] {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    assert_eq!(file["assets"], json!([]), "the document names no files");
}

/// The step from the sixth version to the seventh gives every node the
/// stream the seventh version writes.
#[test]
fn the_step_from_the_sixth_version_gives_every_node_its_stream() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in
        ["craftkit.1to2", "craftkit.2to3", "craftkit.3to4", "craftkit.4to5", "craftkit.5to6", "craftkit.6to7"]
    {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    let nodes = file["nodes"].as_array().expect("the file has nodes");
    assert_eq!(nodes[0]["stream"], Json::Null, "the node has no stream");
}

/// The step from the seventh version to the eighth gives every relation
/// the payload the eighth version writes.
#[test]
fn the_step_from_the_seventh_version_gives_every_relation_its_payload() {
    let mut file = json!({
        "format": "craftkit/7",
        "version": 7,
        "name": "the old file",
        "data": null,
        "nodes": [],
        "relations": [ { "from": 1, "to": 2, "kind": "core.related" } ],
        "assets": [],
    });
    (step("craftkit.7to8").apply)(&mut file).expect("the step runs");
    let relations = file["relations"].as_array().expect("the file has relations");
    assert_eq!(relations[0]["payload"], Json::Null, "the relation says nothing");
}

/// The step from the eighth version to the ninth gives every node the
/// `hidden` flag, and **not hidden** is what it gets.
#[test]
fn the_step_from_the_eighth_version_gives_every_node_the_hidden_flag() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in [
        "craftkit.1to2",
        "craftkit.2to3",
        "craftkit.3to4",
        "craftkit.4to5",
        "craftkit.5to6",
        "craftkit.6to7",
        "craftkit.7to8",
        "craftkit.8to9",
    ] {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    let nodes = file["nodes"].as_array().expect("the file has nodes");
    assert_eq!(nodes[0]["hidden"], json!(false), "the node is shown");
}

/// The step from the ninth version to the tenth gives every node the
/// `locked` flag, and **not locked** is what it gets.
#[test]
fn the_step_from_the_ninth_version_gives_every_node_the_locked_flag() {
    let mut file = a_file_of_the_first_version();
    let migrations = migrations();
    for id in [
        "craftkit.1to2",
        "craftkit.2to3",
        "craftkit.3to4",
        "craftkit.4to5",
        "craftkit.5to6",
        "craftkit.6to7",
        "craftkit.7to8",
        "craftkit.8to9",
        "craftkit.9to10",
    ] {
        (migrations.get(id).expect("there is a step").apply)(&mut file).expect("the step runs");
    }
    let nodes = file["nodes"].as_array().expect("the file has nodes");
    assert_eq!(nodes[0]["locked"], json!(false), "the node can be changed");
}

/// And the **chain** carries a file of the first version all the way to
/// the tenth, and the file it becomes says what the newest version says.
#[test]
fn the_chain_carries_a_file_of_the_first_version_to_the_tenth() {
    let ecosystem = ecosystem();
    let file = a_file_of_the_first_version();
    let document = read_document(&file, &ecosystem.formats, &ecosystem.migrations)
        .expect("the file is read and walked forward");
    assert_eq!(document.name, "the old file", "the document keeps its name");
    assert_eq!(document.nodes.len(), 1, "and its node");
    let node = &document.nodes[0];
    assert_eq!(node.kind, "core.layer", "the node keeps its kind");
    assert!(!node.muted, "and it is not muted");
    assert!(!node.hidden, "and it is shown");
    assert!(!node.locked, "and it can be changed");
    assert_eq!(node.children.len(), 0, "and it holds nothing");
}
