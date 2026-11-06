//! The **calculation** of a field.
//!
//! A field may say how its value is worked out: the file carries a
//! small script on the field's **additional actions** — the `/AA`
//! dictionary — and the **calculate** event of that dictionary (`/C`)
//! holds the script.
//!
//! The scripts that a form writes are the ones the tree knows:
//!
//! - `AFSimple_Calculate("SUM", new Array("a", "b"))` — the way the
//!   value is worked out from other fields: a **sum**, a **product**,
//!   an **average**, a **minimum** or a **maximum**;
//! - the **simplified field notation**, which the file keeps in a
//!   comment between `BVCALC` and `EVCALC` — an expression over the
//!   fields, such as `Price * Quantity`.
//!
//! And a script that is neither of those is **not understood**: it is
//! kept as it is and reported as unsupported, because a script that
//! needs a JavaScript engine is not one this tree may pretend to run.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// The **way** a value is worked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// The sum of the fields.
    Sum,
    /// The product of the fields.
    Product,
    /// The average of the fields.
    Average,
    /// The smallest of the fields.
    Minimum,
    /// The largest of the fields.
    Maximum,
}

impl Operation {
    /// The name the file says.
    pub fn code(self) -> &'static str {
        match self {
            Operation::Sum => "SUM",
            Operation::Product => "PRD",
            Operation::Average => "AVG",
            Operation::Minimum => "MIN",
            Operation::Maximum => "MAX",
        }
    }

    /// The way, from the name the file says.
    pub fn parse(code: &str) -> Option<Operation> {
        match code.trim() {
            "SUM" => Some(Operation::Sum),
            "PRD" => Some(Operation::Product),
            "AVG" => Some(Operation::Average),
            "MIN" => Some(Operation::Minimum),
            "MAX" => Some(Operation::Maximum),
            _ => None,
        }
    }
}

/// The **calculation** of a field: what its script says.
#[derive(Debug, Clone, PartialEq)]
pub enum Calculation {
    /// The field calculates nothing.
    Nothing,
    /// The value is worked out from other fields, in one way.
    Simple {
        /// The way it is worked out.
        operation: Operation,
        /// The fields it is worked out from.
        fields: Vec<String>,
    },
    /// The value is worked out by an expression the file keeps in a
    /// comment.
    Notation(String),
    /// The script is one this tree does not understand.
    Unsupported(String),
}

/// The **names** of the fields an argument lists: either an array of
/// names, or one text that holds them separated by commas.
fn names_of(argument: &str) -> Vec<String> {
    let trimmed = argument.trim();
    if let Some(text) = crate::scripts::text_of(trimmed) {
        return text.split(',').map(|name| name.trim().to_string()).filter(|name| !name.is_empty()).collect();
    }
    let Some(open) = trimmed.find('(') else { return Vec::new() };
    let Some(close) = trimmed.rfind(')') else { return Vec::new() };
    if close <= open {
        return Vec::new();
    }
    crate::scripts::split_arguments(&trimmed[open + 1..close])
        .into_iter()
        .filter_map(|name| crate::scripts::text_of(&name))
        .collect()
}

/// The **calculation** a script says.
pub fn parse(script: &str) -> Calculation {
    // The **simplified field notation**: the expression is kept in a
    // comment by the file that wrote it.
    if let (Some(start), Some(end)) = (script.find("BVCALC"), script.find("EVCALC"))
        && end > start
    {
        let expression = script[start + 6..end].trim().to_string();
        return Calculation::Notation(expression);
    }
    if let Some(arguments) = crate::scripts::arguments_of(script, "AFSimple_Calculate") {
        let Some(code) = arguments.first().and_then(|argument| crate::scripts::text_of(argument)) else {
            return Calculation::Unsupported(script.to_string());
        };
        let Some(operation) = Operation::parse(&code) else {
            return Calculation::Unsupported(script.to_string());
        };
        let Some(names) = arguments.get(1) else {
            return Calculation::Unsupported(script.to_string());
        };
        return Calculation::Simple { operation, fields: names_of(names) };
    }
    Calculation::Unsupported(script.to_string())
}

/// The **calculation** of a field.
pub fn calculation_of(document: &Document, node: &Node) -> Calculation {
    match crate::scripts::script_of(document, node, "C") {
        Some(script) => parse(&script),
        None => Calculation::Nothing,
    }
}

/// **Read** the **calculation** of a field.
///
/// A query: it does not change the document. A field that says nothing
/// about how its value is worked out calculates nothing, and the
/// answer says so.
#[derive(Command)]
#[command(id = "pdf.field.calculate", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadCalculation {
    /// The field to read.
    pub field: NodeRef,
}

impl ReadCalculation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.calculate".to_string();
        let document = session.document();
        let node = document.node(self.field.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.field.0),
        })?;
        let calculation = calculation_of(document, node);
        let (kind, operation, fields, expression) = match &calculation {
            Calculation::Nothing => ("nothing", Json::Null, Vec::new(), Json::Null),
            Calculation::Simple { operation, fields } => {
                ("simple", json!(operation.code()), fields.clone(), Json::Null)
            }
            Calculation::Notation(expression) => {
                ("notation", Json::Null, Vec::new(), json!(expression.clone()))
            }
            Calculation::Unsupported(script) => {
                ("unsupported", Json::Null, Vec::new(), json!(script.clone()))
            }
        };
        let name = crate::fields::qualified_name(document, node);
        Ok(Outcome::new(match &calculation {
            Calculation::Nothing => "the field calculates nothing".to_string(),
            Calculation::Simple { operation, .. } => {
                format!("the field is calculated by a {}", operation.code())
            }
            Calculation::Notation(_) => "the field is calculated by an expression".to_string(),
            Calculation::Unsupported(_) => "the field has a script this tree does not understand".to_string(),
        })
        .with_data(json!({
            "field": name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
            "kind": kind,
            "operation": operation,
            "fields": fields,
            "expression": expression,
        })))
    }
}
