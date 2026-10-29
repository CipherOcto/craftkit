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

/// Why a parameter was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamError {
    /// The parameter at fault.
    pub key: String,
    /// What was wrong with it.
    pub message: String,
}

impl ParamError {
    /// An error about a named parameter.
    pub fn new(key: impl Into<String>, message: impl Into<String>) -> Self {
        ParamError { key: key.into(), message: message.into() }
    }

    /// The error for a parameter that was not there.
    pub fn missing(key: &str) -> Self {
        ParamError::new(key, format!("`{key}` is required"))
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
}

impl EngineError {
    /// The command's id, when the error is about one.
    pub fn id(&self) -> Option<&str> {
        match self {
            EngineError::Unknown(id) => Some(id),
            EngineError::Disabled { id, .. } => Some(id),
            EngineError::Param { id, .. } => Some(id),
            EngineError::Failed { id, .. } => Some(id),
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
}

impl Outcome {
    /// An outcome with a message and no data.
    pub fn new(message: impl Into<String>) -> Self {
        Outcome { message: message.into(), data: serde_json::Value::Null, changed: false }
    }

    /// An outcome that changed the document.
    pub fn changed(message: impl Into<String>) -> Self {
        Outcome { message: message.into(), data: serde_json::Value::Null, changed: true }
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
}

impl ParamDoc {
    /// A required parameter.
    pub const fn required(name: &'static str, kind: Doc, help: &'static str) -> Self {
        ParamDoc { name, kind, required: true, help, alternative: None }
    }

    /// An optional parameter.
    pub const fn optional(name: &'static str, kind: Doc, help: &'static str) -> Self {
        ParamDoc { name, kind, required: false, help, alternative: None }
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
        ParamDoc { name, kind, required: false, help, alternative: Some(alternative) }
    }

    /// Whether a person has to fill this in.
    pub fn needed(&self) -> bool {
        self.required || self.alternative.is_some()
    }
}
