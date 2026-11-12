//! The **prompt**: a host that has what a command is missing.
//!
//! # The rule this kit keeps
//!
//! A command takes its parameters and never asks. It does not open a
//! dialog, it does not read the terminal and it does not wait: the
//! outer tree's rule (§118) is the same, and every front end that drives
//! this kit obeys it.
//!
//! # What was missing
//!
//! The survey of the family's applications (§40, §43) found a command
//! that **asks**: CADCraft's registry has `interactive: Option<Factory>`,
//! some of its commands want a radius or a point that the caller does
//! not have yet, and its MCP server answers that by exposing the prompt
//! itself as a tool — an agent types at the prompt exactly as a person
//! would.
//!
//! That is a **host** concern, not a command concern, and this kit draws
//! the line in the same place the outer tree does: the command says what
//! it needs, and the host decides how to get it.
//!
//! # What is here
//!
//! - a [`Question`]: what is wanted, of what kind, and one line about it;
//! - a [`Supplier`]: where the answers come from — a person, an agent, a
//!   script, a file;
//! - [`Session::execute_with`](crate::Session::execute_with): run a
//!   command, and ask the host for the parameters that are missing
//!   before it runs.
//!
//! The command does not change, the registry does not change, and the
//! document does not change: the host is simply given a turn before the
//! command runs. A command that is already given everything is run
//! exactly as before, and the supplier is never asked.

use crate::params::Doc;
use crate::serde_json::Value as Json;

/// What the host is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// The parameter's name.
    pub name: String,
    /// What kind of value it takes.
    pub kind: Doc,
    /// One line about it, from the command's own declaration.
    pub help: String,
}

impl Question {
    /// The question, as it is put to a host.
    pub fn to_json(&self) -> Json {
        crate::serde_json::json!({
            "name": self.name,
            "kind": self.kind.json_name(),
            "help": self.help,
        })
    }
}

/// Where the answers come from.
///
/// A host implements this: a person at a prompt, an agent over a
/// protocol, a script reading a file. The kit does not care which, and
/// it does not ask when the answer is already there.
pub trait Supplier {
    /// The value for that question, if there is one.
    ///
    /// `None` means "I cannot answer that", and the command is run
    /// without it — so it fails the way it would have failed anyway.
    fn supply(&mut self, question: &Question) -> Option<Json>;
}

/// A supplier that answers from a **map** of values.
///
/// It is what a host uses when the answers are already known — a
/// script, a test, a front end that has them in hand.
pub struct MapSupplier {
    /// The answers, by parameter name.
    values: crate::serde_json::Map<String, Json>,
}

impl MapSupplier {
    /// A supplier with nothing to say.
    pub fn new() -> MapSupplier {
        MapSupplier { values: crate::serde_json::Map::new() }
    }

    /// A supplier that has these answers.
    pub fn with(values: Json) -> MapSupplier {
        let values = values.as_object().cloned().unwrap_or_default();
        MapSupplier { values }
    }

    /// Add an answer.
    pub fn with_answer(mut self, name: &str, value: Json) -> MapSupplier {
        self.values.insert(name.to_string(), value);
        self
    }
}

impl Default for MapSupplier {
    fn default() -> Self {
        MapSupplier::new()
    }
}

impl Supplier for MapSupplier {
    fn supply(&mut self, question: &Question) -> Option<Json> {
        self.values.get(&question.name).cloned()
    }
}
