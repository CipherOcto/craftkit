//! The **validation** of a field.
//!
//! A field may say what it accepts: the file carries the script on the
//! field's **additional actions** — the `/AA` dictionary — and the
//! **validate** event of that dictionary (`/V`) holds it.
//!
//! The script that a form writes is the one the tree knows —
//! `AFRange_Validate(bGreater, nGreater, bLess, nLess)` — and it says
//! **the range** the field accepts: whether there is a smallest value
//! and what it is, and whether there is a largest one and what it is.
//!
//! And a script that is not that one is **not understood**: it is
//! kept as it is and reported as unsupported, because a script that
//! needs a JavaScript engine is not one this tree may pretend to run.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// The **validation** of a field: what its script says.
#[derive(Debug, Clone, PartialEq)]
pub enum Validation {
    /// The field validates nothing.
    Nothing,
    /// The field accepts a range of values.
    Range {
        /// Whether there is a smallest value.
        greater: bool,
        /// The smallest value the field accepts.
        min: Option<f64>,
        /// Whether there is a largest value.
        less: bool,
        /// The largest value the field accepts.
        max: Option<f64>,
    },
    /// The script is one this tree does not understand.
    Unsupported(String),
}

/// The **flag** an argument holds: a truth or a lie.
fn flag_of(argument: &str) -> bool {
    argument.trim().eq_ignore_ascii_case("true")
}

/// The **number** an argument holds.
fn number_of(argument: &str) -> Option<f64> {
    let number = argument.trim().parse::<f64>().ok()?;
    if number.is_finite() { Some(number) } else { None }
}

/// The **validation** a script says.
pub fn parse(script: &str) -> Validation {
    let Some(arguments) = crate::scripts::arguments_of(script, "AFRange_Validate") else {
        return Validation::Unsupported(script.to_string());
    };
    let flag = |index: usize| arguments.get(index).map(|argument| flag_of(argument)).unwrap_or(false);
    let number = |index: usize| arguments.get(index).and_then(|argument| number_of(argument));
    Validation::Range { greater: flag(0), min: number(1), less: flag(2), max: number(3) }
}

/// The **validation** of a field.
pub fn validation_of(document: &Document, node: &Node) -> Validation {
    match crate::scripts::script_of(document, node, "V") {
        Some(script) => parse(&script),
        None => Validation::Nothing,
    }
}

/// **Read** the **validation** of a field.
///
/// A query: it does not change the document. A field that says nothing
/// about what it accepts validates nothing, and the answer says so.
#[derive(Command)]
#[command(id = "pdf.field.validate", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadValidation {
    /// The field to read.
    pub field: NodeRef,
}

impl ReadValidation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.validate".to_string();
        let document = session.document();
        let node = document.node(self.field.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.field.0),
        })?;
        let validation = validation_of(document, node);
        let name = crate::fields::qualified_name(document, node);
        let (kind, greater, min, less, max) = match &validation {
            Validation::Nothing => ("nothing", false, Json::Null, false, Json::Null),
            Validation::Range { greater, min, less, max } => (
                "range",
                *greater,
                min.map(|value| json!(value)).unwrap_or(Json::Null),
                *less,
                max.map(|value| json!(value)).unwrap_or(Json::Null),
            ),
            Validation::Unsupported(script) => {
                ("unsupported", false, json!(script.clone()), false, Json::Null)
            }
        };
        Ok(Outcome::new(match &validation {
            Validation::Nothing => "the field validates nothing".to_string(),
            Validation::Range { .. } => "the field accepts a range of values".to_string(),
            Validation::Unsupported(_) => "the field has a script this tree does not understand".to_string(),
        })
        .with_data(json!({
            "field": name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
            "kind": kind,
            "greater": greater,
            "min": min,
            "less": less,
            "max": max,
        })))
    }
}
