//! A battery as a declaration.
//!
//! The table below is a **family** of commands, and the compiler writes
//! for each row the struct, the reader, the registration and the spec. What
//! these tests are about is the part that cannot be read off the table: that
//! the commands really reach the **registry**, with the ids, the labels and
//! the areas the table says, and that the body of a row is the body's.

use craftkit_derive::commands;
use craftkit_sdk::command::CommandSpec;
use craftkit_sdk::contribution::Registry;
use craftkit_sdk::{Outcome, Session, Text};

commands! {
    /// Add a workbook to the grid.
    grid.workbook.add {
        label: "Add workbook",
        area: "grid",
        mutates,
        undoable,
        params: {
            /// The workbook's name.
            name: Text,
        },
        run: {
            let _ = &self.name;
            Ok(Outcome::changed(format!("added {}", self.name.0)))
        }
    }

    /// Remove a workbook from the grid.
    grid.workbook.remove {
        label: "Remove workbook",
        area: "grid",
        params: {
            /// The workbook's name.
            name: Text,
        },
        run: {
            Ok(Outcome::new(format!("removed {}", self.name.0)))
        }
    }

    /// Count the workbooks.
    ///
    /// The second paragraph is not the command's one line: only the first
    /// paragraph is.
    grid.workbook.count {
        run: {
            Ok(Outcome::new("counted"))
        }
    }
}

/// The registry, as the tree reads it.
fn registry() -> Registry<CommandSpec> {
    Registry::from_inventory().expect("the registry is built")
}

/// The commands of the table reach the registry.
#[test]
fn the_commands_reach_the_registry() {
    let registry = registry();
    for id in ["grid.workbook.add", "grid.workbook.remove", "grid.workbook.count"] {
        assert!(registry.get(id).is_some(), "{id} is not in the registry");
    }
}

/// The labels are the table's.
#[test]
fn the_labels_are_the_tables() {
    let registry = registry();
    let add = registry.get("grid.workbook.add").expect("the command is there");
    assert_eq!(add.label, "Add workbook");
    let remove = registry.get("grid.workbook.remove").expect("the command is there");
    assert_eq!(remove.label, "Remove workbook");
}

/// A row with no label takes the **id's last part**.
#[test]
fn a_row_with_no_label_takes_the_ids_last_part() {
    let registry = registry();
    let count = registry.get("grid.workbook.count").expect("the command is there");
    assert_eq!(count.label, "Count");
}

/// The areas are the table's.
#[test]
fn the_areas_are_the_tables() {
    let registry = registry();
    for id in ["grid.workbook.add", "grid.workbook.remove", "grid.workbook.count"] {
        let command = registry.get(id).expect("the command is there");
        assert_eq!(command.area, "grid", "{id} is in the wrong area");
    }
}

/// The parameters are the table's, and the **type** says what they are.
#[test]
fn the_parameters_are_the_tables() {
    let registry = registry();
    let add = registry.get("grid.workbook.add").expect("the command is there");
    assert_eq!(add.param_names(), vec!["name"], "the parameters are not the table's");
    let parameter = &add.params[0];
    assert!(parameter.required, "the parameter is not required");
    assert_eq!(parameter.help, "The workbook's name.", "the help is not the table's");
}

/// The doc comment is the command's one line.
#[test]
fn the_doc_comment_is_the_commands_one_line() {
    let registry = registry();
    let count = registry.get("grid.workbook.count").expect("the command is there");
    assert_eq!(count.doc, "Count the workbooks.");
}

/// The flags are the table's.
#[test]
fn the_flags_are_the_tables() {
    let registry = registry();
    let add = registry.get("grid.workbook.add").expect("the command is there");
    assert!(add.mutates, "the command does not say it changes the document");
    assert!(add.undoable, "the command is not undoable");
    let count = registry.get("grid.workbook.count").expect("the command is there");
    assert!(!count.mutates, "the command says it changes the document");
}

/// The body of a row is the body's, and the reader reads the parameters.
///
/// This is the whole of the promise: the table wrote the edges, and the
/// middle is the programmer's.
#[test]
fn the_body_of_a_row_is_the_bodys() {
    let mut session = Session::new(registry(), Registry::new());
    let outcome = session
        .execute("grid.workbook.add", &craftkit_sdk::serde_json::json!({ "name": "Book" }))
        .expect("the command ran");
    assert_eq!(outcome.message, "added Book", "the body is not the body's");
}
