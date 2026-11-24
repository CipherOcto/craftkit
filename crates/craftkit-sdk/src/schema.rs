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

/// The shape of one kind, as a schema says it.
///
/// The **type** comes from the SDK's own table of kinds, and the schema
/// and the engine's refusal read the same table — so the shape an agent
/// is told and the shape the session accepts are one answer.
///
/// A kind that takes a **string** as well — a length, which may be
/// written `25` or `25mm` — says **both** shapes. The schema is what
/// the reader takes, and the reader takes both.
///
/// A **text** is bounded, and the schema says so: the bound is the SDK's
/// own (`MAX_TEXT_CHARACTERS`), read here and not restated, so the schema
/// and the reader cannot disagree. Telling the model the bound before it
/// proposes a value is the whole difference — a refusal after the fact is
/// a refusal too late. And a node's id is a whole number that is not
/// negative, and the schema says so.
fn shape_of(kind: Doc, accepts_text: bool) -> Json {
    let mut schema = match kind.json_type() {
        Some(kind) => json!({ "type": kind }),
        // The raw door has no type: a value of any shape is of it.
        None => json!({}),
    };
    if accepts_text && let Some(shape) = kind.json_type() {
        schema["type"] = json!([shape, "string"]);
    }
    if kind == Doc::Text {
        schema["maxLength"] = json!(crate::params::MAX_TEXT_CHARACTERS);
    }
    if kind == Doc::NodeRef {
        schema["minimum"] = json!(0);
    }
    schema
}

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
    // The pairs: a parameter with two spellings is required as a pair,
    // and each pair is a **group** of two halves.
    let mut pairs: Vec<Vec<Json>> = Vec::new();
    for param in params {
        // The **type** comes from the SDK's own table of kinds, and the
        // schema and the engine's refusal read the same table — so the
        // shape an agent is told and the shape the session accepts are
        // one answer.
        let mut schema = shape_of(param.kind, param.accepts_text);
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
            // The **alternative** is the same parameter written another
            // way, and the two ways are **two kinds**: `node` is a
            // node's id and `node_name` is a name. So the alternative is
            // published with **its own** shape — the one its reader
            // takes — and not with the first spelling's. A schema that
            // says a name is a number is a schema that lies.
            let mut other =
                shape_of(param.alternative_kind.unwrap_or(param.kind), param.alternative_accepts_text);
            other["description"] =
                Json::String(format!("{description} (the other spelling of `{}`)", param.name));
            properties.insert(alternative.to_string(), other);
        }
        properties.insert(param.name.to_string(), schema);
        if let Some(alternative) = param.alternative {
            // The pair is required, and neither half is. The **schema**
            // says so: JSON Schema cannot say "one of these two" in a
            // word, but it can say it in a shape — `anyOf` with a
            // `required` in each half. A schema that permits a call the
            // door refuses is a schema that lies.
            properties[param.name]["description"] =
                Json::String(format!("{description} (or `{alternative}`)"));
            pairs.push(vec![json!({ "required": [param.name] }), json!({ "required": [alternative] })]);
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
    if !pairs.is_empty() {
        // One `anyOf` for **one** pair: each half says which of the two
        // spellings the caller gave. And more than one pair is an
        // `allOf` of them: the caller has to fill in **every** pair, and
        // each pair is satisfied by one of its two spellings.
        let mut groups = pairs.into_iter();
        let first = groups.next();
        match (first, groups.len()) {
            (Some(pair), 0) => schema["anyOf"] = Json::Array(pair),
            (Some(pair), _) => {
                let mut all = vec![json!({ "anyOf": Json::Array(pair) })];
                for pair in groups {
                    all.push(json!({ "anyOf": Json::Array(pair) }));
                }
                schema["allOf"] = Json::Array(all);
            }
            (None, _) => {}
        }
    }
    schema
}
