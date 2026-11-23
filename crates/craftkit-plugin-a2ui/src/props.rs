//! The **resolved** props.
//!
//! a2ui's dynamic values are a literal **or** a binding to the data
//! model, and the component holds the **resolved** value: the layer owns
//! the rules of a binding — what a binding is and how it is looked up —
//! and the plugin owns the model, so the resolution is here, in the
//! prop's own reader.
//!
//! Without this a bound prop would be read as the literal it is not, and
//! the component would draw nothing — a picture that is silently empty,
//! which is the one thing this tree will not do.
//!
//! A **call** is not resolved: a call is a round trip, and a round trip
//! is not made in a painter.

use craftkit_a2ui::bindings::{self, Resolved};
use craftkit_sdk::ParamError;
use craftkit_sdk::params::{Doc, ParamKind};
use craftkit_sdk::serde_json::Value as Json;

/// A **text** prop: a literal, or a binding to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundText(pub String);

impl ParamKind for BoundText {
    const DOC: Doc = Doc::Text;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match crate::model::resolve(|model| bindings::resolve_text(model, value)) {
            Ok(text) => Ok(BoundText(text)),
            Err(reason) => Err(ParamError::new("", reason)),
        }
    }
}

/// A **boolean** prop: a literal, or a binding to the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundBool(pub bool);

impl ParamKind for BoundBool {
    const DOC: Doc = Doc::Bool;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match crate::model::resolve(|model| bindings::resolve_bool(model, value)) {
            Ok(flag) => Ok(BoundBool(flag)),
            Err(reason) => Err(ParamError::new("", reason)),
        }
    }
}

/// A **number** prop: a literal, or a binding to the model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundNumber(pub f64);

impl ParamKind for BoundNumber {
    const DOC: Doc = Doc::Number;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        match crate::model::resolve(|model| bindings::resolve_number(model, value)) {
            Ok(number) => Ok(BoundNumber(number)),
            Err(reason) => Err(ParamError::new("", reason)),
        }
    }
}

/// An **icon's** path: the shape to draw.
///
/// a2ui's `Icon.name` is one of three things — one of fifty-nine
/// built-in names, an object with an `svgPath`, or a binding. The
/// built-in names are gone (the tree has no icon vocabulary), so what is
/// left is the object and the binding, and the **resolved** value is the
/// path itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconPath(pub String);

impl ParamKind for IconPath {
    const DOC: Doc = Doc::Text;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        let resolved = match crate::model::resolve(|model| bindings::resolve(model, value)) {
            Ok(resolved) => resolved,
            Err(reason) => return Err(ParamError::new("", reason)),
        };
        match resolved {
            Resolved::Call { name, .. } => {
                Err(ParamError::new("", format!("the call `{name}` has no path: it is a round trip")))
            }
            Resolved::Value(value) => match path_of(&value) {
                Some(path) => Ok(IconPath(path)),
                None => {
                    Err(ParamError::new("", "the icon is neither a path nor an object with an `svgPath`"))
                }
            },
        }
    }
}

/// The path in a value, if the value holds one.
///
/// The value is a path written as a string, or an object with an
/// `svgPath` — which is the shape a2ui's own catalogue publishes.
fn path_of(value: &Json) -> Option<String> {
    match value {
        Json::String(path) => Some(path.clone()),
        Json::Object(object) => object.get("svgPath").and_then(Json::as_str).map(|path| path.to_string()),
        _ => None,
    }
}
