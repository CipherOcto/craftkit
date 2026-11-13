//! The **parameters** of the tree: every one of them says what it is.
//!
//! # Why this test exists
//!
//! A parameter's **help** is what the schema publishes, what the
//! catalogue shows and what the generated documents print. It is written
//! by the derive from the field's **doc comment**, so it cannot disagree
//! with the code that reads the parameter — but a field with **no** doc
//! comment has a parameter with **no** help, and nothing said so.
//!
//! A parameter with no help is a hole in the published spec: an agent
//! reads the schema, sees a parameter it has no way of understanding, and
//! guesses. The tree's rule is that a reader can look a thing up — and a
//! parameter the reader cannot look up is a parameter that is not
//! published.
//!
//! # What is checked
//!
//! - every parameter of every command **this build links** has a help;
//! - the help is **one line**: the schema, the catalogue and the documents
//!   all print it on one line;
//! - a parameter with **two spellings** names the other one, so the
//!   schema can say both.
//!
//! The build decides the scope: a build without the batteries' features
//! links fewer commands, and the test says of the ones it has.

use craftkit_sdk::ecosystem::Ecosystem;

/// The commands of this build, in the order of their ids.
fn the_commands() -> Vec<craftkit_sdk::command::CommandSpec> {
    let ecosystem: Ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<craftkit_sdk::command::CommandSpec> = ecosystem.commands.all().to_vec();
    commands.sort_by(|left, right| left.id.cmp(right.id));
    commands
}

/// Every parameter of every command in this build says what it is.
#[test]
fn every_parameter_has_a_help() {
    let commands = the_commands();
    assert!(!commands.is_empty(), "the build links no commands");
    let mut missing: Vec<String> = Vec::new();
    for command in &commands {
        for parameter in command.params {
            if parameter.help.trim().is_empty() {
                missing.push(format!("{} :: {}", command.id, parameter.name));
            }
        }
    }
    assert!(missing.is_empty(), "these parameters say nothing: {}", missing.join(", "));
}

/// The help is **one line**: the schema, the catalogue and the documents
/// print it on one line, and a help that is two is a help that is cut in
/// half by whatever prints it.
#[test]
fn every_help_is_one_line() {
    let commands = the_commands();
    let mut broken: Vec<String> = Vec::new();
    for command in &commands {
        for parameter in command.params {
            if parameter.help.contains('\n') || parameter.help.contains('\r') {
                broken.push(format!("{} :: {}", command.id, parameter.name));
            }
        }
    }
    assert!(broken.is_empty(), "these helps are more than one line: {}", broken.join(", "));
}

/// A parameter with **two spellings** names the other one, so the schema
/// can say both.
#[test]
fn every_alternative_is_named() {
    let commands = the_commands();
    for command in &commands {
        for parameter in command.params {
            let Some(alternative) = parameter.alternative else { continue };
            assert!(
                !alternative.trim().is_empty(),
                "{} :: {} has an empty second spelling",
                command.id,
                parameter.name
            );
        }
    }
}
