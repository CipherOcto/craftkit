//! Errors and outcomes.
//!
//! # Two kinds of failure, and they are not the same thing
//!
//! An **error** is a refusal: the command did not happen, and the document
//! is as it was. An **outcome** is what a command did: a message, some data
//! for the caller, and whether anything changed.
//!
//! The engine distinguishes them because an agent has to be able to tell
//! them apart: a refusal means "not like that", and an outcome means "done,
//! and this is what happened".

use crate::params::Doc;
use crate::prompt::Question;

/// Why a parameter was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamError {
    /// The parameter at fault.
    pub key: String,
    /// What was wrong with it.
    pub message: String,
    /// Whether the value was not there at all, as opposed to being
    /// there and being wrong.
    ///
    /// The two are different questions: a value that is wrong is the
    /// caller's business, and a value that is missing is a **host's**,
    /// because the host may be the one who can get it (§ the prompt
    /// seam).
    pub missing: bool,
}

impl ParamError {
    /// An error about a named parameter.
    pub fn new(key: impl Into<String>, message: impl Into<String>) -> Self {
        ParamError { key: key.into(), message: message.into(), missing: false }
    }

    /// The error for a parameter that was not there.
    pub fn missing(key: &str) -> Self {
        ParamError { key: key.to_string(), message: format!("`{key}` is required"), missing: true }
    }

    /// Whether the value was not there at all.
    pub fn is_missing(&self) -> bool {
        self.missing
    }
}

impl std::fmt::Display for ParamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParamError {}

/// Why a command was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// There is no command with that id.
    Unknown(String),
    /// The command exists and cannot run right now.
    Disabled { id: String, reason: String },
    /// A parameter was wrong, missing, or not one the command has.
    Param { id: String, source: ParamError },
    /// The command failed on purpose.
    Failed { id: String, message: String },
    /// The command panicked.
    ///
    /// A crash is not a result: the document was put back and the failure
    /// is reported as what it is. §94 of the tree this replaces says the
    /// same thing about a panic in code craftkit does not own.
    Internal { id: String, message: String },
    /// The command is **waiting**: it asked the host something while it
    /// ran, and the host has to answer before it can go on.
    ///
    /// This is not a refusal. The command did not fail; it stopped to
    /// ask, and the session turns this into an outcome that says what is
    /// wanted. See [`crate::resume`].
    NeedsInput { id: String, question: Question },
}

impl EngineError {
    /// The command's id, when the error is about one.
    pub fn id(&self) -> Option<&str> {
        match self {
            EngineError::Unknown(id) => Some(id),
            EngineError::Disabled { id, .. } => Some(id),
            EngineError::Param { id, .. } => Some(id),
            EngineError::Failed { id, .. } => Some(id),
            EngineError::Internal { id, .. } => Some(id),
            EngineError::NeedsInput { id, .. } => Some(id),
        }
    }

    /// The question the command is waiting for, if it is waiting.
    pub fn question(&self) -> Option<&Question> {
        match self {
            EngineError::NeedsInput { question, .. } => Some(question),
            _ => None,
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Unknown(id) => write!(f, "there is no command `{id}`"),
            EngineError::Disabled { id, reason } => write!(f, "`{id}` is not available: {reason}"),
            EngineError::Param { id, source } => write!(f, "`{id}` was refused: {source}"),
            EngineError::Failed { id, message } => write!(f, "`{id}` failed: {message}"),
            EngineError::Internal { id, message } => write!(f, "`{id}` failed inside: {message}"),
            EngineError::NeedsInput { id, question } => {
                write!(f, "`{id}` is waiting for `{}`: {}", question.name, question.help)
            }
        }
    }
}

impl std::error::Error for EngineError {}

/// What a command did.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// One line for a person.
    pub message: String,
    /// What the caller gets back, if anything.
    pub data: serde_json::Value,
    /// Whether the document changed.
    pub changed: bool,
    /// The question the command is waiting for, if it is waiting.
    ///
    /// A command that asked while it ran stops with this set, and the
    /// document is as it was: a command that asks **before** it changes
    /// anything, so that it can be run again when the answer comes.
    pub needs_input: Option<Question>,
}

impl Outcome {
    /// An outcome with a message and no data.
    pub fn new(message: impl Into<String>) -> Self {
        Outcome { message: message.into(), data: serde_json::Value::Null, changed: false, needs_input: None }
    }

    /// An outcome that changed the document.
    pub fn changed(message: impl Into<String>) -> Self {
        Outcome { message: message.into(), data: serde_json::Value::Null, changed: true, needs_input: None }
    }

    /// An outcome that says the command is **waiting** for an answer.
    ///
    /// It is not a refusal and not a change: the command asked, the host
    /// has to answer, and the document is as it was.
    pub fn needs_input(question: Question) -> Self {
        let message = format!("waiting for `{}`: {}", question.name, question.help);
        let data = serde_json::json!({
            "status": "needs_input",
            "question": question.to_json(),
        });
        Outcome { message, data, changed: false, needs_input: Some(question) }
    }

    /// The data the caller gets back.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Whether the document changed, when the command knows.
    ///
    /// A command that sets a value to what it already was has not changed
    /// anything, and saying so is the difference between an undo entry and
    /// none.
    pub fn changed_if(mut self, changed: bool) -> Self {
        self.changed = changed;
        self
    }
}

/// The documented type of one parameter, as it is published.
///
/// This is what the schema, the catalogue and the generated documents read.
/// It is written by the derive macro from the **types** of the command's
/// fields, so it cannot disagree with the code that reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamDoc {
    /// The parameter's name.
    pub name: &'static str,
    /// What kind of value it takes.
    pub kind: Doc,
    /// Whether the caller has to give it.
    pub required: bool,
    /// One line about it.
    pub help: &'static str,
    /// The other spelling of this parameter, if it has one.
    pub alternative: Option<&'static str>,
    /// The kind's **own name**, when the kind has one.
    ///
    /// A plugin's kind: `length`, `angle`. The shape is in
    /// [`Self::kind`] and the name is here, so that a schema can say
    /// both.
    pub type_name: Option<&'static str>,
    /// The **unit** the value is in, when the kind has one.
    pub unit: Option<&'static str>,
}

impl ParamDoc {
    /// A required parameter.
    pub const fn required(name: &'static str, kind: Doc, help: &'static str) -> Self {
        ParamDoc { name, kind, required: true, help, alternative: None, type_name: None, unit: None }
    }

    /// An optional parameter.
    pub const fn optional(name: &'static str, kind: Doc, help: &'static str) -> Self {
        ParamDoc { name, kind, required: false, help, alternative: None, type_name: None, unit: None }
    }

    /// The parameter with the kind's **own name** on it.
    ///
    /// The shape is unchanged: this says what the number **is**, and the
    /// shape says what it looks like.
    pub const fn with_type_name(mut self, type_name: Option<&'static str>) -> Self {
        self.type_name = type_name;
        self
    }

    /// The parameter with the **unit** on it.
    pub const fn with_unit(mut self, unit: Option<&'static str>) -> Self {
        self.unit = unit;
        self
    }

    /// A parameter with a second spelling.
    ///
    /// The pair is required; neither half is. §116 of the tree this
    /// replaces said it, and it is still true.
    pub const fn either(
        name: &'static str,
        kind: Doc,
        help: &'static str,
        alternative: &'static str,
    ) -> Self {
        ParamDoc {
            name,
            kind,
            required: false,
            help,
            alternative: Some(alternative),
            type_name: None,
            unit: None,
        }
    }

    /// Whether a person has to fill this in.
    pub fn needed(&self) -> bool {
        self.required || self.alternative.is_some()
    }
}
