//! The **bindings** — a2ui's dynamic values, resolved.
//!
//! # The three forms
//!
//! a2ui's `DynamicString` and its friends are **one of three things**
//! (`specification/v1_0/json/common_types.json`'s `$defs/DynamicString`):
//!
//! - a **literal** — a value;
//! - a **binding** — `{"@path": "/title"}`, a JSON Pointer into the data
//!   model; and
//! - a **function call** — `{"@call": "…"}`, which the renderer resolves.
//!
//! # What resolves here, and what does not
//!
//! The **literal** and the **binding** resolve here: a literal is used,
//! and a binding is looked up in the model. A path that is not there is
//! **refused** — the house rule is that an unknown name is refused, and a
//! path that names nothing is an unknown name.
//!
//! The **function call** does **not** resolve here: a call is a round
//! trip, and the round trip is the door's business. What this module
//! does is **tell the three forms apart**, so that the caller knows
//! which one it has.

use craftkit_sdk::serde_json::Value as Json;

use crate::model::Model;

/// The **form** of a dynamic value.
#[derive(Debug, Clone, PartialEq)]
pub enum Dynamic {
    /// A literal: the value itself.
    Literal(Json),
    /// A binding: a path into the data model.
    Binding(String),
    /// A function call: a name, and the arguments.
    Call {
        /// The name of the function.
        name: String,
        /// The arguments, as they were sent.
        args: Json,
    },
}

impl Dynamic {
    /// The **form** of a value, if it is one of the three.
    ///
    /// A value that is not a binding and not a call is a literal — a
    /// string, a number, a boolean, or anything else.
    pub fn of(value: &Json) -> Dynamic {
        if let Json::Object(map) = value {
            if let Some(Json::String(path)) = map.get("@path") {
                return Dynamic::Binding(path.clone());
            }
            if let Some(Json::String(name)) = map.get("@call") {
                let args = map.get("args").cloned().unwrap_or(Json::Null);
                return Dynamic::Call { name: name.clone(), args };
            }
        }
        Dynamic::Literal(value.clone())
    }

    /// Whether a value is a **binding**.
    pub fn is_binding(value: &Json) -> bool {
        matches!(Dynamic::of(value), Dynamic::Binding(_))
    }

    /// Whether a value is a **call**.
    pub fn is_call(value: &Json) -> bool {
        matches!(Dynamic::of(value), Dynamic::Call { .. })
    }
}

/// What a dynamic value **resolved to**.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved {
    /// The value itself.
    Value(Json),
    /// A **call** — the caller must make the round trip.
    Call {
        /// The name of the function.
        name: String,
        /// The arguments, as they were sent.
        args: Json,
    },
}

/// Resolve a dynamic value against the model.
///
/// A **literal** resolves to itself and a **binding** to the value at its
/// path. A path that is not there is an **error**, and the error says
/// which path.
///
/// A **call** does not resolve: it comes back as [`Resolved::Call`], and
/// the caller makes the round trip.
pub fn resolve(model: &Model, value: &Json) -> Result<Resolved, String> {
    match Dynamic::of(value) {
        Dynamic::Literal(literal) => Ok(Resolved::Value(literal)),
        Dynamic::Binding(path) => match model.get(&path) {
            Some(found) => Ok(Resolved::Value(found.clone())),
            None => Err(format!("the binding `{path}` names a path that is not there")),
        },
        Dynamic::Call { name, args } => Ok(Resolved::Call { name, args }),
    }
}

/// Resolve a dynamic value to its **text**.
///
/// A string is a string; a number and a boolean are their own text; a
/// tree is not text, and it is refused.
pub fn resolve_text(model: &Model, value: &Json) -> Result<String, String> {
    match resolve(model, value)? {
        Resolved::Call { name, .. } => Err(format!("the call `{name}` has no text: it is a round trip")),
        Resolved::Value(Json::String(text)) => Ok(text),
        Resolved::Value(Json::Number(number)) => Ok(number.to_string()),
        Resolved::Value(Json::Bool(value)) => Ok(value.to_string()),
        Resolved::Value(other) => Err(format!("the value is a {}, and it has no text", kind_of(&other))),
    }
}

/// Resolve a dynamic value to its **boolean**.
pub fn resolve_bool(model: &Model, value: &Json) -> Result<bool, String> {
    match resolve(model, value)? {
        Resolved::Call { name, .. } => Err(format!("the call `{name}` has no boolean: it is a round trip")),
        Resolved::Value(Json::Bool(value)) => Ok(value),
        Resolved::Value(Json::String(text)) => match text.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("the value `{text}` is not a boolean")),
        },
        Resolved::Value(other) => Err(format!("the value is a {}, and it is not a boolean", kind_of(&other))),
    }
}

/// Resolve a dynamic value to its **number**.
pub fn resolve_number(model: &Model, value: &Json) -> Result<f64, String> {
    match resolve(model, value)? {
        Resolved::Call { name, .. } => Err(format!("the call `{name}` has no number: it is a round trip")),
        Resolved::Value(Json::Number(number)) => {
            number.as_f64().ok_or_else(|| "the number is not a number this tree can hold".to_string())
        }
        Resolved::Value(Json::String(text)) => {
            text.parse().map_err(|_| format!("the value `{text}` is not a number"))
        }
        Resolved::Value(other) => Err(format!("the value is a {}, and it is not a number", kind_of(&other))),
    }
}

/// What a value **is**, for a message.
fn kind_of(value: &Json) -> &'static str {
    match value {
        Json::Null => "nothing",
        Json::Bool(_) => "boolean",
        Json::Number(_) => "number",
        Json::String(_) => "string",
        Json::Array(_) => "list",
        Json::Object(_) => "tree",
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

    /// A literal is a literal.
    #[test]
    fn a_literal_is_a_literal() {
        assert_eq!(Dynamic::of(&json!("hello")), Dynamic::Literal(json!("hello")));
        assert_eq!(Dynamic::of(&json!(3)), Dynamic::Literal(json!(3)));
        assert_eq!(Dynamic::of(&json!(true)), Dynamic::Literal(json!(true)));
    }

    /// A binding is a binding.
    #[test]
    fn a_binding_is_a_binding() {
        assert_eq!(Dynamic::of(&json!({ "@path": "/title" })), Dynamic::Binding("/title".to_string()));
        assert!(Dynamic::is_binding(&json!({ "@path": "/title" })));
    }

    /// A call is a call.
    #[test]
    fn a_call_is_a_call() {
        let value = json!({ "@call": "surfaceApply", "args": { "surfaceId": "s" } });
        assert_eq!(
            Dynamic::of(&value),
            Dynamic::Call { name: "surfaceApply".to_string(), args: json!({ "surfaceId": "s" }) }
        );
        assert!(Dynamic::is_call(&value));
    }

    /// A literal resolves to itself.
    #[test]
    fn a_literal_resolves_to_itself() {
        let model = model();
        assert_eq!(resolve(&model, &json!("hello")), Ok(Resolved::Value(json!("hello"))));
    }

    /// A binding resolves to the value at its path.
    #[test]
    fn a_binding_resolves_to_the_value_at_its_path() {
        let model = model();
        assert_eq!(resolve(&model, &json!({ "@path": "/title" })), Ok(Resolved::Value(json!("hello"))));
        assert_eq!(resolve_text(&model, &json!({ "@path": "/title" })), Ok("hello".to_string()));
    }

    /// A binding to a path that is not there is **refused**.
    #[test]
    fn a_binding_to_nothing_is_refused() {
        let model = model();
        let error = resolve(&model, &json!({ "@path": "/missing" })).expect_err("the path is refused");
        assert!(error.contains("/missing"), "{error}");
    }

    /// A call does **not** resolve — it comes back as a call.
    #[test]
    fn a_call_does_not_resolve() {
        let model = model();
        let value = json!({ "@call": "surfaceApply" });
        assert_eq!(
            resolve(&model, &value),
            Ok(Resolved::Call { name: "surfaceApply".to_string(), args: Json::Null })
        );
        let error = resolve_text(&model, &value).expect_err("the call has no text");
        assert!(error.contains("round trip"), "{error}");
    }

    /// A number and a boolean resolve to their own kinds.
    #[test]
    fn a_number_and_a_boolean_resolve_to_their_own_kinds() {
        let model = model();
        assert_eq!(resolve_number(&model, &json!(3.5)), Ok(3.5));
        assert_eq!(resolve_bool(&model, &json!(true)), Ok(true));
    }

    /// A tree has no text, and it is refused.
    #[test]
    fn a_tree_has_no_text() {
        let model = model();
        let error = resolve_text(&model, &json!({ "a": 1 })).expect_err("the tree has no text");
        assert!(error.contains("tree"), "{error}");
    }
}
