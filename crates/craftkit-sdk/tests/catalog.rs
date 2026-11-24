//! The **catalogue entries**: a contribution type like any other.
//!
//! A catalogue entry has an id, a layer and a schema, and the registry
//! checks it the way it checks every other contribution — which is the
//! whole point of making it one.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{CatalogEntry, Contribution, Ecosystem, Registry, RegistryError};

/// A well-formed entry.
fn entry(id: &'static str) -> CatalogEntry {
    CatalogEntry {
        id,
        name: id,
        label: "An entry",
        layer: "core",
        doc: "One line about itself.",
        schema: || json!({ "type": "object" }),
    }
}

/// An id that is not dotted and lower case is refused, like every other
/// contribution.
#[test]
fn a_malformed_id_is_refused() {
    let mut registry: Registry<CatalogEntry> = Registry::new();
    let error = registry.register(entry("NotDotted")).expect_err("the id is refused");
    assert!(matches!(error, RegistryError::Malformed { .. }), "{error:?}");
}

/// Two entries with one id are refused.
#[test]
fn a_duplicate_id_is_refused() {
    let mut registry: Registry<CatalogEntry> = Registry::new();
    registry.register(entry("thing.one")).expect("the first is accepted");
    let error = registry.register(entry("thing.one")).expect_err("the second is refused");
    assert!(matches!(error, RegistryError::Duplicate(_)), "{error:?}");
}

/// An entry in a layer that is not linked in is refused.
#[test]
fn an_entry_in_a_layer_that_is_not_there_is_refused() {
    let mut ecosystem = Ecosystem::new();
    ecosystem.catalog.register(entry("thing.one")).expect("the entry is accepted");
    let error = ecosystem.validate().expect_err("the layer is refused");
    assert!(format!("{error}").contains("not linked in"), "{error}");
}

/// The entry says what it is.
#[test]
fn an_entry_says_what_it_is() {
    let entry = entry("thing.one");
    assert_eq!(entry.id(), "thing.one");
    assert_eq!(entry.capability(), "component");
    assert_eq!(entry.layer(), "core");
}

/// The **published** name is beside the checked one.
#[test]
fn the_published_name_is_beside_the_checked_one() {
    let entry = CatalogEntry {
        id: "a2ui.text",
        name: "Text",
        label: "Text",
        layer: "core",
        doc: "A line of text.",
        schema: || json!({ "type": "object" }),
    };
    assert_eq!(entry.id, "a2ui.text", "the id is craftkit's");
    assert_eq!(entry.name, "Text", "and the name is the published one");
    assert!(entry.id != entry.name, "and the two are never the same string");
}

/// The **entries** view is a view of the registry, with no shape of its own.
#[test]
fn the_entries_view_is_a_view_of_the_registry() {
    let mut ecosystem = Ecosystem::new();
    ecosystem.catalog.register(entry("thing.one")).expect("the entry is accepted");
    let entries = ecosystem.projection().entries();
    assert_eq!(entries.len(), 1, "there is one entry");
    assert_eq!(entries[0]["id"], json!("thing.one"));
    assert_eq!(entries[0]["name"], json!("thing.one"));
    assert_eq!(entries[0]["layer"], json!("core"));
    assert_eq!(entries[0]["doc"], json!("One line about itself."));
    assert_eq!(entries[0]["schema"], json!({ "type": "object" }));
}

/// And the view names nothing that is not registered.
#[test]
fn the_entries_view_names_nothing_else() {
    let ecosystem = Ecosystem::new();
    assert!(ecosystem.projection().entries().is_empty(), "an empty registry has no entries");
}

/// The **vocabulary view** lists every registered command, tool, panel and
/// kind — and nothing else.
#[test]
fn the_vocabulary_view_lists_the_whole_vocabulary() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    let projection = ecosystem.projection();
    let catalog = projection.catalog();

    let mut expected: Vec<String> = Vec::new();
    for command in ecosystem.commands.all() {
        expected.push(command.id.to_string());
    }
    for tool in ecosystem.tools.all() {
        expected.push(tool.id.to_string());
    }
    for panel in ecosystem.panels.all() {
        expected.push(panel.id.to_string());
    }
    for kind in ecosystem.kinds.all() {
        expected.push(kind.kind.to_string());
    }
    assert!(!expected.is_empty(), "the tree has a vocabulary");

    let listed: Vec<String> =
        catalog.iter().map(|entry| entry["id"].as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(listed.len(), expected.len(), "the same number: {catalog:?}");
    for id in &expected {
        assert!(listed.contains(id), "{id} is missing");
    }
}

/// Nothing is listed twice.
#[test]
fn nothing_is_listed_twice() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    let catalog = ecosystem.projection().catalog();
    let mut ids: Vec<String> =
        catalog.iter().map(|entry| entry["id"].as_str().unwrap_or_default().to_string()).collect();
    let count = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), count, "every id appears once");
}

/// A command carries its **schema**, and a kind does not.
#[test]
fn a_command_carries_a_schema_and_a_kind_does_not() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    let catalog = ecosystem.projection().catalog();
    let mut commands = 0;
    let mut kinds = 0;
    for entry in &catalog {
        let capability = entry["capability"].as_str().unwrap_or_default();
        if capability == "command" {
            commands += 1;
            assert_eq!(entry["schema"]["type"], json!("object"), "a command has a schema");
        }
        if capability == "kind" {
            kinds += 1;
            assert!(entry["schema"].is_null(), "a kind carries no schema");
        }
    }
    assert!(commands > 0, "there are commands");
    assert!(kinds > 0, "there are kinds");
}

/// The **vocabulary view** and the **entries view** are different views of
/// different registries, and neither is the other.
#[test]
fn the_vocabulary_view_is_not_the_entries_view() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    let projection = ecosystem.projection();
    let catalog = projection.catalog();
    let entries = projection.entries();
    // The core has no catalogue entries, and a vocabulary of many.
    assert!(entries.is_empty(), "the core contributes no catalogue entries");
    assert!(!catalog.is_empty(), "and the vocabulary is not empty");
}

/// A command in the vocabulary says **what it changes**.
///
/// The promise is the command's own: a command that says it does not
/// change the document does not, and the session keeps that promise —
/// a command that breaks it is refused and the document is put back.
#[test]
fn a_command_says_what_it_changes() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    let catalog = ecosystem.projection().catalog();
    let mut read = 0;
    let mut write = 0;
    for entry in &catalog {
        if entry["capability"] != "command" {
            continue;
        }
        let id = entry["id"].as_str().unwrap_or_default();
        let mutates = entry["mutates"].as_bool().expect("a command says what it changes");
        let spec = ecosystem.commands.get(id).expect("the command is there");
        assert_eq!(mutates, spec.mutates, "{id} says something else in the vocabulary");
        assert_eq!(entry["undoable"], json!(spec.undoable), "{id}");
        if mutates {
            write += 1;
        } else {
            read += 1;
        }
    }
    assert!(read > 0, "there are commands that only read");
    assert!(write > 0, "and commands that change the document");
}
