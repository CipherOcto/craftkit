//! The **example** of a battery, and the batteries that have one.
//!
//! "How is a command written in this battery?" was the largest of the
//! searches an arc made. The answer is now in the battery: one command per
//! battery is marked `example`, and the marker is part of the declaration.
//!
//! These tests say that the marker is there, that there is **one** of them,
//! and that the tool prints what it says it prints.

use std::path::{Path, PathBuf};

use xtask::example::{example_of, the_source};
use xtask::index;

/// The batteries the README counts.
const BATTERIES: [&str; 9] = ["grid", "prose", "pdf", "film", "layout", "cad", "fx", "sound", "vector"];

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

#[test]
fn every_battery_marks_exactly_one_example() {
    let index = index::build(&root()).expect("the index is built");
    for battery in BATTERIES {
        let examples: Vec<&index::Command> =
            index.commands.iter().filter(|c| c.battery == battery && c.example).collect();
        assert_eq!(
            examples.len(),
            1,
            "the `{battery}` battery marks {} examples, and it marks one",
            examples.len()
        );
    }
}

#[test]
fn the_example_is_the_command_the_battery_names() {
    let index = index::build(&root()).expect("the index is built");
    let example = example_of(&index, "grid").expect("the grid battery has an example");
    assert_eq!(example.id, "grid.workbook.add");
    assert!(example.doc.is_some(), "the example says what it is");
}

#[test]
fn the_source_of_the_example_is_the_whole_declaration() {
    let index = index::build(&root()).expect("the index is built");
    let example = example_of(&index, "grid").expect("the grid battery has an example");
    let source = the_source(&root(), example).expect("the source is read");
    assert!(source.contains("grid.workbook.add"), "the id is there:\n{source}");
    assert!(source.contains("example"), "the marker is there:\n{source}");
    // The documentation is above the declaration, so it is in the source too.
    assert!(source.contains("///"), "the documentation is there:\n{source}");
    // The declaration is whole, in **whichever spelling the battery
    // uses**: a struct with an attribute, or a row of a table.
    let struct_form = source.contains("#[derive(Command)]") && source.contains("pub struct");
    let table_form = source.contains("params: {") && source.contains("run: {");
    assert!(struct_form || table_form, "the declaration is whole:\n{source}");
}

#[test]
fn a_battery_with_no_example_is_said_to_have_none() {
    // The rule is one per battery; a battery that has not marked one is a
    // battery that has not said what a command of it looks like — and the
    // tool says so rather than picking one for it.
    let index = index::build(&root()).expect("the index is built");
    let shared = "shared";
    assert!(
        example_of(&index, shared).is_none(),
        "the `{shared}` battery is a fixture, and it has no example"
    );
}
