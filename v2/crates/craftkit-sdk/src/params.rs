//! Parameters: the caller's values, and the types that read them.
//!
//! # The rule
//!
//! **A parameter is read by a type, not by a string.**
//!
//! In the tree v2 replaces, a command declared its parameters as a table of
//! names and kinds and then read them one by one through a string-keyed
//! reader. Two gates held that arrangement together: one that compared the
//! declared table with what the body read, and one that compared it with
//! what the caller passed. Both are gone here, and they are gone *structurally*:
//!
//! - a command's parameters are **fields of a struct**, so the compiler
//!   knows what they are and cannot let two of them share a name;
//! - the reader is **generated from those fields**, so a command cannot read
//!   a parameter it did not declare — it is handed a struct, not a bag of
//!   values, so there is nothing to read an undeclared one *out of*;
//! - a field that is [`Option`] is optional, and a field that is not is
//!   required. "Required" is a property of the **type**, so it cannot drift
//!   from the declaration.
//!
//! What is left is a single decision at the boundary: an unknown key is
//! refused, because the engine knows the names and the caller does not get
//! to invent one.

use serde_json::Value as Json;

use crate::error::ParamError;

/// The documented type of a parameter.
///
/// This is what an agent is told: the shape of the value, not a Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doc {
    Number,
    Text,
    Bool,
    NodeRef,
    NodeKind,
    /// A value of any shape.
    ///
    /// **The door out.** Every other kind is a type, and a type is checked.
    /// This one is not checked, and it exists because a plugin that was
    /// loaded at run time cannot offer a Rust type: it offers a schema and a
    /// value, and the schema says `json`.
    ///
    /// It is named, and it is not the default: a command that wants the raw
    /// value says so, and the person reading the declaration sees it.
    Json,
}

impl Doc {
    /// The name used in a schema and in a generated document.
    pub fn json_name(self) -> &'static str {
        match self {
            Doc::Number => "number",
            Doc::Text => "text",
            Doc::Bool => "bool",
            Doc::NodeRef => "node-ref",
            Doc::NodeKind => "node-kind",
            Doc::Json => "json",
        }
    }
}

/// The caller's values, read by name.
///
/// The one door into a parameter object, and it is a door that only opens by
/// name: there is no indexing, no iteration and no way to reach a value that
/// was not asked for.
pub struct Params<'a> {
    value: &'a Json,
}

impl<'a> Params<'a> {
    /// A view of a caller's values.
    pub fn new(value: &'a Json) -> Self {
        Params { value }
    }

    /// The value at a name, if there is one.
    pub fn get(&self, name: &str) -> Option<&Json> {
        match self.value.get(name) {
            Some(Json::Null) => None,
            other => other,
        }
    }

    /// Refuse a key that is not one of the names.
    ///
    /// This is the whole of "no undeclared parameter": the engine knows the
    /// names from the command's declaration, and a caller that invents one is
    /// told so instead of being quietly ignored.
    pub fn deny_unknown(&self, allowed: &[&str]) -> Result<(), ParamError> {
        let Some(object) = self.value.as_object() else {
            // A call with no parameters at all is a call with no parameters.
            return Ok(());
        };
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(ParamError::new(key, format!("`{key}` is not a parameter of this command")));
            }
        }
        Ok(())
    }
}

/// A type that knows how to read itself, and what it is.
///
/// This is the trait the derive macro leans on. Implementing it is what makes
/// a type usable as a parameter, and it says three things: the documented
/// kind, whether the parameter is required, and how to read a value.
pub trait ParamKind: Sized {
    /// The kind an agent is told about.
    const DOC: Doc;

    /// Whether the caller has to supply it.
    const REQUIRED: bool;

    /// Read one value.
    fn from_json(value: &Json) -> Result<Self, ParamError>;

    /// Read the value at a name.
    ///
    /// The default is "the value is there or the parameter is missing"; a
    /// kind that can be absent overrides this.
    fn read(params: &Params<'_>, name: &str) -> Result<Self, ParamError> {
        match params.get(name) {
            Some(value) => Self::from_json(value),
            None => Err(ParamError::missing(name)),
        }
    }
}

/// A number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number(pub f64);

impl ParamKind for Number {
    const DOC: Doc = Doc::Number;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match value.as_f64() {
            Some(number) if number.is_finite() => Ok(Number(number)),
            _ => Err(ParamError::new("", "expected a finite number")),
        }
    }
}

/// A string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text(pub String);

impl ParamKind for Text {
    const DOC: Doc = Doc::Text;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match value.as_str() {
            Some(text) => Ok(Text(text.to_string())),
            None => Err(ParamError::new("", "expected a string")),
        }
    }
}

/// A yes or no.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);

impl ParamKind for Bool {
    const DOC: Doc = Doc::Bool;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match value.as_bool() {
            Some(flag) => Ok(Bool(flag)),
            None => Err(ParamError::new("", "expected true or false")),
        }
    }
}

/// The id of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRef(pub u64);

impl ParamKind for NodeRef {
    const DOC: Doc = Doc::NodeRef;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match value.as_u64() {
            Some(id) => Ok(NodeRef(id)),
            None => Err(ParamError::new("", "expected a node id")),
        }
    }
}

/// The raw door.
///
/// A value of any shape: a command that takes one can read anything the
/// caller sent. It is here for the contributions that cannot offer a type —
/// a command from a subprocess, a command from a script — and it is
/// **named** in the declaration, so that a caller knows.
///
/// The implementation is on [`serde_json::Value`] itself, so a plugin that
/// wants the raw value asks for the value and gets it.
impl ParamKind for serde_json::Value {
    const DOC: Doc = Doc::Json;
    const REQUIRED: bool = true;

    fn from_json(value: &serde_json::Value) -> Result<Self, ParamError> {
        Ok(value.clone())
    }
}

/// A parameter that may be absent.
///
/// The type says it: an optional parameter is `Option<T>`, and the
/// declaration the derive macro writes says the same thing, because it asks
/// the type.
impl<T: ParamKind> ParamKind for Option<T> {
    const DOC: Doc = T::DOC;
    const REQUIRED: bool = false;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        T::from_json(value).map(Some)
    }

    fn read(params: &Params<'_>, name: &str) -> Result<Self, ParamError> {
        match params.get(name) {
            Some(value) => T::from_json(value).map(Some),
            None => Ok(None),
        }
    }
}

/// One parameter, two spellings.
///
/// `node` and `node_name` are one parameter written two ways: a caller may
/// give an id or a name, and the two are not the same kind of value. This
/// type holds whichever one arrived, and the command decides what to do with
/// it.
///
/// The pair is **required as a pair and optional as either half**: that is
/// why the declaration says `required: false` with an alternative, and why
/// the engine asks for both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> Either<A, B> {
    /// The left half, if that is what arrived.
    pub fn left(self) -> Option<A> {
        match self {
            Either::Left(value) => Some(value),
            Either::Right(_) => None,
        }
    }

    /// The right half, if that is what arrived.
    pub fn right(self) -> Option<B> {
        match self {
            Either::Left(_) => None,
            Either::Right(value) => Some(value),
        }
    }
}

impl<A: ParamKind, B: ParamKind> Either<A, B> {
    /// Read the first spelling, and the second if the first is not there.
    ///
    /// Neither half is required on its own, so the error names both.
    pub fn read_either(params: &Params<'_>, first: &str, second: &str) -> Result<Self, ParamError> {
        if let Some(value) = params.get(first) {
            return A::from_json(value).map(Either::Left);
        }
        if let Some(value) = params.get(second) {
            return B::from_json(value).map(Either::Right);
        }
        Err(ParamError::new(first, format!("`{first}` is required, or `{second}` instead of it")))
    }
}
