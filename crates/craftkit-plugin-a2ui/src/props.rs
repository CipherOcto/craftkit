//! The **dynamic** props, and where they are resolved.
//!
//! a2ui's dynamic values are a literal **or** a binding to the data
//! model, and the component holds the value **as it was sent** — because
//! the model is the *surface's*, and the surface is in the document.
//!
//! # Why the resolution is not in the reader
//!
//! A prop is read by the **typed view**, and the typed view has the
//! node and not the document — and the model is in the document. So the
//! resolution happens where the model is: in the **painter**, which has
//! both.
//!
//! Without this a bound prop would be read as the literal it is not,
//! and the component would draw nothing — a picture that is silently
//! empty, which is the one thing this tree will not do.
//!
//! A **call** is not resolved: a call is a round trip, and a round trip
//! is not made in a painter.

use craftkit_a2ui::bindings::{self, Resolved};
use craftkit_a2ui::model::Model;
use craftkit_sdk::ParamError;
use craftkit_sdk::params::{Doc, ParamKind};
use craftkit_sdk::serde_json::Value as Json;

/// A **dynamic** value: a literal, a binding, or a call — as it was
/// sent.
///
/// The value is read as it is and **not** resolved: the model is in the
/// document, and the document is not here.
#[derive(Debug, Clone, PartialEq)]
pub struct Dynamic(pub Json);

impl ParamKind for Dynamic {
    const DOC: Doc = Doc::Json;
    const REQUIRED: bool = true;

    fn from_json(value: &Json) -> Result<Self, ParamError> {
        Ok(Dynamic(value.clone()))
    }
}

/// The **text** of a dynamic value, resolved against the model.
///
/// A binding to a path that is not there is **refused**, and the caller
/// draws nothing: the value is missing, and a missing value is not a
/// value to invent.
pub fn text_of(model: &Model, value: &Json) -> Option<String> {
    bindings::resolve_text(model, value).ok()
}

/// The **boolean** of a dynamic value, resolved against the model.
pub fn bool_of(model: &Model, value: &Json) -> Option<bool> {
    bindings::resolve_bool(model, value).ok()
}

/// The **number** of a dynamic value, resolved against the model.
pub fn number_of(model: &Model, value: &Json) -> Option<f64> {
    bindings::resolve_number(model, value).ok()
}

/// The **path** of an icon, resolved against the model.
///
/// The value is a path written as a string, or an object with an
/// `svgPath` — the shape a2ui's own catalogue publishes — and the
/// fifty-nine built-in names are gone: craftkit has no icon vocabulary,
/// and a vocabulary of names the tree cannot draw is a vocabulary that
/// lies.
pub fn icon_path_of(model: &Model, value: &Json) -> Option<String> {
    match bindings::resolve(model, value).ok()? {
        Resolved::Value(value) => path_of(&value),
        Resolved::Call { .. } => None,
    }
}

/// The path in a value, if the value holds one.
fn path_of(value: &Json) -> Option<String> {
    match value {
        Json::String(path) => Some(path.clone()),
        Json::Object(object) => object.get("svgPath").and_then(Json::as_str).map(|path| path.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_sdk::serde_json::json;

    /// A model with one value.
    fn model() -> Model {
        let mut model = Model::new();
        model.set("/title", json!("hello")).expect("the value is written");
        model
    }

    /// A literal is its own text.
    #[test]
    fn a_literal_is_its_own_text() {
        assert_eq!(text_of(&model(), &json!("hello")), Some("hello".to_string()));
    }

    /// A binding is looked up in the model.
    #[test]
    fn a_binding_is_looked_up_in_the_model() {
        let value = json!({ "@path": "/title" });
        assert_eq!(text_of(&model(), &value), Some("hello".to_string()));
    }

    /// A binding to a path that is not there has **no** text.
    #[test]
    fn a_binding_to_nothing_has_no_text() {
        let value = json!({ "@path": "/missing" });
        assert!(text_of(&model(), &value).is_none());
    }

    /// An icon's path is its own text, or its `svgPath`.
    #[test]
    fn an_icons_path_is_read() {
        assert_eq!(icon_path_of(&model(), &json!("M 0 0")), Some("M 0 0".to_string()));
        let value = json!({ "svgPath": "M 1 1" });
        assert_eq!(icon_path_of(&model(), &value), Some("M 1 1".to_string()));
    }
}
