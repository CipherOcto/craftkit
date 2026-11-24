//! One JSON Schema generator, and every door that publishes a schema uses it.
//!
//! # What this is
//!
//! The mapping from a command's **declared parameters** to the **JSON
//! Schema** an agent is told. It used to live in the MCP door, which meant
//! that only the MCP door published a JSON Schema — every other door had
//! its own idea of what a schema was, or had none at all.
//!
//! Now the mapping lives **here**, in the SDK, and the doors call it. That
//! is what "one source" means in practice: one function, not one document.
//!
//! # Two things, and they are not the same
//!
//! The SDK has two views of a command's shape, and both of them survive:
//!
//! - the **descriptor** — [`crate::Projection::schema`] — which is `id`,
//!   `label`, `area`, `capability`, `doc` and the parameter list. This is
//!   what the bridge publishes, and it is a **description**, not a schema.
//! - the **JSON Schema** — [`to_json_schema`] and
//!   [`crate::Projection::json_schema`] — which is what an agent door
//!   publishes, and it is a **schema**.
//!
//! A door that publishes a schema publishes this one. A door that publishes
//! a descriptor publishes the descriptor. Neither is re-expressed as the
//! other, because the descriptor's envelope is what the bridge reads its
//! names from.

use crate::error::ParamDoc;
use crate::params::Doc;
use crate::serde_json::{Value as Json, json};

/// A command's parameters, as a JSON Schema.
///
/// The types come from the command's own declaration, so the schema cannot
/// disagree with the code that reads the values.
///
/// This is the **one** mapping: the MCP door calls it, the SDK's own
/// [`crate::Projection::json_schema`] calls it, and any door that publishes
/// a schema publishes what this returns.
pub fn to_json_schema(params: &[ParamDoc]) -> Json {
    let mut properties = serde_json::Map::new();
    let mut required: Vec<Json> = Vec::new();
    for param in params {
        let mut schema = match param.kind {
            Doc::Number => json!({ "type": "number" }),
            // A **text** is bounded, and the schema says so: the bound is
            // the SDK's own (`MAX_TEXT_CHARACTERS`), read here and not
            // restated, so the schema and the reader cannot disagree.
            // Telling the model the bound before it proposes a value is
            // the whole difference — a refusal after the fact is a
            // refusal too late.
            Doc::Text => json!({ "type": "string", "maxLength": crate::params::MAX_TEXT_CHARACTERS }),
            Doc::Bool => json!({ "type": "boolean" }),
            Doc::NodeRef => json!({ "type": "integer", "minimum": 0 }),
            Doc::NodeKind => json!({ "type": "string" }),
            Doc::Json => json!({}),
        };
        let description = param.help.trim();
        if !description.is_empty() {
            schema["description"] = Json::String(description.to_string());
        }
        // A kind that is a plugin's own carries its **name** and its
        // **unit**: `title` is the name a schema gives a thing, and the
        // unit says what the number means. The type is unchanged: a
        // length is a number, and the schema says number.
        if let Some(type_name) = param.type_name {
            schema["title"] = Json::String(type_name.to_string());
        }
        if let Some(unit) = param.unit {
            schema["unit"] = Json::String(unit.to_string());
        }
        if let Some(alternative) = param.alternative {
            // The **alternative** is the same prop written another way,
            // so it is published with the **same type** — and not as a
            // string whatever the prop is. A number written two ways is
            // two numbers, and a schema that says otherwise is a schema
            // that lies.
            let mut other = schema.clone();
            other["description"] =
                Json::String(format!("{description} (the other spelling of `{}`)", param.name));
            properties.insert(alternative.to_string(), other);
        }
        properties.insert(param.name.to_string(), schema);
        if let Some(alternative) = param.alternative {
            // The pair is required, and neither half is: the schema says
            // so in the words of the description, because JSON Schema
            // cannot say "one of these two".
            properties[param.name]["description"] =
                Json::String(format!("{description} (or `{alternative}`)"));
        }
        if param.required {
            required.push(Json::String(param.name.to_string()));
        }
    }
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    });
    if !required.is_empty() {
        schema["required"] = Json::Array(required);
    }
    schema
}
