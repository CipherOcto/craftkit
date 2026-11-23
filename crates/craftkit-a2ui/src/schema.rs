//! The **shapes** of the layer's published catalogue.
//!
//! a2ui's component schemas are a *public specification*, and this module
//! re-derives them: the layer publishes the protocol's own vocabulary —
//! the same property names, the same definitions, the same enum values and
//! the same defaults — but the wording and the spelling here are
//! craftkit's own, and every reference is **local**
//! (`"#/$defs/DynamicString"`, never a reference into another file), so the
//! published catalogue is one self-contained document.
//!
//! # Who owns what
//!
//! | | owns |
//! |---|---|
//! | the `defs` module | the property **names** |
//! | this module | the property **shapes** |
//! | the plugin's row | the resolved craftkit values |
//!
//! # The published schema
//!
//! [`published`] takes the component's a2ui name, the row's property
//! names and the row's per-property a2ui definitions, and writes the
//! schema an a2ui agent reads:
//!
//! * the `"component"` discriminator is stamped, always;
//! * a property with a definition becomes a local `$ref` to it;
//! * a property without one becomes the layer's **inline** shape for it
//!   (an enum, or the narrowed `Icon.name`);
//! * the `"required"` list is written from the layer's own knowledge of
//!   which properties may be left out.
//!
//! # The `required` list
//!
//! The list holds the discriminator **and** every property that must be
//! present. A property is **optional** when:
//!
//! * its inline shape carries a `"default"` — an enum with a default may
//!   be left out, and the agent's omission means the default; or
//! * it is one of the three properties a2ui itself leaves out:
//!   `weight` (on every component), `Image`'s `description` and
//!   `TextField`'s `placeholder`.
//!
//! With that rule the published lists are exactly the ones a2ui's own
//! catalogue carries: `Text` requires `["component", "text"]`, `Divider`
//! requires `["component"]`, `Button` requires
//! `["component", "child", "action"]`, and so on.
//!
//! # The known narrowing
//!
//! The layer narrows a row's optionality to *what the layer can see*: a
//! property that carries a `$ref` definition is required unless the layer
//! names it as optional above. A row that made such a property optional
//! would publish it as required — the two differ only for `weight`,
//! `Image`'s `description` and `TextField`'s `placeholder`, and the layer
//! knows all three.
//!
//! # The five narrowings
//!
//! They are all one rule: *a branch the layer cannot execute is a branch
//! it must not publish.*
//!
//! 1. **`Icon`'s built-in name enum** — craftkit has no icon vocabulary,
//!    so the branch is gone and `Icon.name` is the `svgPath` object or a
//!    binding.
//! 2. **`ChildList`'s template branch** — a template is rendered per item
//!    with the reserved `@index`, and the layer refuses every reserved
//!    function.
//! 3. **`FunctionCall`'s `@index` branch** — the same refusal.
//! 4. **`Checkable`'s `@checks`** — the layer evaluates no check rule.
//! 5. **`TextField`'s `value`** — only its binding branch survives: a
//!    literal has nowhere to live, and a call's answer has nowhere to
//!    live either. The narrowing is on the *property*, not on
//!    `DynamicString`, which other components still use in full.
//!
//! # `anyFunction`
//!
//! The catalogue's own `anyFunction` is **not** here: its `oneOf` spans
//! the whole catalogue's function table, which no single component's
//! derivation can know. [`definition`] returns `None` for it, and the
//! projection writes it.

use craftkit_sdk::serde_json::{Map, Value as Json, json};

/// The names of the definitions the layer publishes, in order.
///
/// `anyFunction` is on the list because the published catalogue's `$defs`
/// carries it, but [`definition`] does not build it: the projection
/// writes that one, because its `oneOf` spans the whole catalogue.
pub const DEFINITIONS: &[&str] = &[
    "DynamicString",
    "DynamicBoolean",
    "DataBinding",
    "FunctionCall",
    "FunctionCommon",
    "Child",
    "ChildList",
    "ComponentId",
    "Checkable",
    "Action",
    "DynamicValue",
    "Weight",
    "anyFunction",
];

/// The shape of one `$defs` entry, by name.
///
/// `None` for a name the layer does not build — including `anyFunction`,
/// which the projection writes.
pub fn definition(name: &str) -> Option<Json> {
    match name {
        "DataBinding" => Some(data_binding()),
        "ComponentId" => Some(component_id()),
        "Child" => Some(child()),
        "ChildList" => Some(child_list()),
        "FunctionCommon" => Some(function_common()),
        "FunctionCall" => Some(function_call()),
        "DynamicString" => Some(dynamic_string()),
        "DynamicBoolean" => Some(dynamic_boolean()),
        "DynamicValue" => Some(dynamic_value()),
        "Weight" => Some(weight()),
        "Checkable" => Some(checkable()),
        "Action" => Some(action()),
        _ => None,
    }
}

/// The **inline** shape of a plain prop, if the layer has one.
///
/// A plain prop is one the row declares without an a2ui definition: an
/// enum, or the narrowed `Icon.name`.
pub fn inline(component: &str, prop: &str) -> Option<Json> {
    match (component, prop) {
        ("Text", "variant") => Some(json!({
            "type": "string",
            "description": "A hint for the base text style.",
            "enum": ["caption", "body"],
            "default": "body"
        })),
        // The **icon**: a shape, and nothing else.
        //
        // a2ui's own `Icon.name` is a `oneOf` of three branches — the
        // fifty-nine built-in names, an object with an `svgPath`, and a
        // binding. The **first** branch is removed: craftkit has no icon
        // vocabulary, and a vocabulary of names the tree cannot draw is a
        // vocabulary that lies.
        ("Icon", "name") => Some(json!({
            "description": "The shape to draw.",
            "oneOf": [
                {
                    "type": "object",
                    "description": "A path, drawn as it is.",
                    "properties": {
                        "svgPath": { "$ref": "#/$defs/DynamicString" }
                    },
                    "required": ["svgPath"],
                    "additionalProperties": false
                },
                { "$ref": "#/$defs/DataBinding" }
            ]
        })),
        // The **url** is a path to a file, and the schema says so: a
        // narrowing the catalogue does not say is a narrowing an agent
        // cannot see.
        ("Image", "url") => Some(json!({
            "description": "The path to a PNG file to draw. The tree fetches nothing over a \
                            network: the path names a file on this machine.",
            "allOf": [{ "$ref": "#/$defs/DynamicString" }],
        })),
        ("Image", "fit") => Some(json!({
            "type": "string",
            "description": "How the image is resized to fit its box.",
            "enum": ["contain", "cover", "fill", "none", "scaleDown"],
            "default": "fill"
        })),
        ("Image", "variant") => Some(json!({
            "type": "string",
            "description": "A hint for the size and style of the image.",
            "enum": ["icon", "avatar", "smallFeature", "mediumFeature", "largeFeature", "header"],
            "default": "mediumFeature"
        })),
        ("Row", "justify") => Some(json!({
            "type": "string",
            "description": "How the children are arranged along the row.",
            "enum": ["center", "end", "spaceAround", "spaceBetween", "spaceEvenly", "start", "stretch"],
            "default": "start"
        })),
        ("Row", "align") => Some(json!({
            "type": "string",
            "description": "How the children are aligned across the row.",
            "enum": ["start", "center", "end", "stretch"],
            "default": "stretch"
        })),
        ("Column", "justify") => Some(json!({
            "type": "string",
            "description": "How the children are arranged along the column.",
            "enum": ["start", "center", "end", "spaceBetween", "spaceAround", "spaceEvenly", "stretch"],
            "default": "start"
        })),
        ("Column", "align") => Some(json!({
            "type": "string",
            "description": "How the children are aligned across the column.",
            "enum": ["center", "end", "start", "stretch"],
            "default": "stretch"
        })),
        ("List", "direction") => Some(json!({
            "type": "string",
            "description": "The way the list's items are laid out.",
            "enum": ["vertical", "horizontal"],
            "default": "vertical"
        })),
        ("List", "align") => Some(json!({
            "type": "string",
            "description": "How the items are aligned across the list.",
            "enum": ["start", "center", "end", "stretch"],
            "default": "stretch"
        })),
        ("Divider", "axis") => Some(json!({
            "type": "string",
            "description": "The orientation of the divider.",
            "enum": ["horizontal", "vertical"],
            "default": "horizontal"
        })),
        ("Button", "variant") => Some(json!({
            "type": "string",
            "description": "A hint for the button's style.",
            "enum": ["default", "primary", "borderless"],
            "default": "default"
        })),
        ("TextField", "variant") => Some(json!({
            "type": "string",
            "description": "The kind of input the field shows.",
            "enum": ["longText", "number", "shortText", "obscured"],
            "default": "shortText"
        })),
        _ => None,
    }
}

/// The published schema of a component.
///
/// The **row** owns the names and the definitions; the layer owns the
/// shapes. So this takes the component's a2ui name, the row's property
/// names, and the row's per-property a2ui definitions (the `#[a2ui_prop]`
/// values — `None` for a plain prop, whose shape the layer owns).
///
/// It stamps the `"component"` discriminator, writes the local `$ref`s,
/// and writes the `required` list.
///
/// # The **checkable** components keep their `allOf`
///
/// a2ui's `Button`, `CheckBox` and `TextField` are an `allOf` of
/// `Checkable` and their own object, and the published schema keeps that
/// shape — because `Checkable` is a **definition in the closure**, and a
/// definition that nothing reaches is a definition that could be dropped
/// without anybody noticing.
///
/// The narrowing is in the **definition**: `Checkable` is published with
/// **no properties**, because the layer evaluates no check rule and a
/// rule it cannot run must not be published. The `allOf` itself stays.
pub fn published(name: &str, names: &[&str], definitions: &[Option<&str>]) -> Json {
    published_with(name, names, definitions, &[], &[])
}

/// The **published** schema, with the **row's** own optionality.
///
/// The layer knows the **names** and the **shapes**, and the row knows
/// whether a prop is **optional** — and the row is the one that reads
/// it: `Option<T>` is optional, and `T` is not. So the published
/// `required` list is the layer's rule **and** the row's, and a prop
/// either of them calls optional is optional.
///
/// A row that says nothing — the layer's own tests — leaves the layer's
/// rule alone.
pub fn published_with(
    name: &str,
    names: &[&str],
    definitions: &[Option<&str>],
    optionals: &[bool],
    alternatives: &[Option<&str>],
) -> Json {
    let mut properties = Map::new();
    properties.insert("component".to_string(), json!({ "const": name }));
    // Every component carries an **id**: a2ui's `ComponentCommon` says so,
    // and the id is how a container's children are named.
    properties.insert("id".to_string(), json!({ "$ref": "#/$defs/ComponentId" }));
    let mut required: Vec<&str> = vec!["component", "id"];

    for (index, prop) in names.iter().enumerate() {
        let definition = definitions.get(index).copied().flatten();
        let shape = prop_shape(name, prop, definition);
        properties.insert((*prop).to_string(), shape.clone());
        // A prop written **two ways** — an `Either` — has another
        // spelling, and the schema must name it: a schema that forbids
        // the other spelling is a schema that forbids half the prop.
        let alternative = alternatives.get(index).copied().flatten();
        if let Some(alternative) = alternative {
            properties.insert(alternative.to_string(), shape);
        }
        let row_says_optional = optionals.get(index).copied().unwrap_or(false);
        if !optional(name, prop) && !row_says_optional && alternative.is_none() {
            required.push(prop);
        }
    }

    let mut own = Map::new();
    own.insert("type".to_string(), json!("object"));
    own.insert("properties".to_string(), Json::Object(properties));
    let listed = required.iter().map(|prop| Json::String((*prop).to_string())).collect();
    own.insert("required".to_string(), Json::Array(listed));
    // A prop the component does not declare is **refused**: the schema
    // says what the component takes, and the layer refuses the rest.
    own.insert("additionalProperties".to_string(), json!(false));

    if !is_checkable(name) {
        return Json::Object(own);
    }

    // The **allOf**: the checkable part and the component's own.
    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert(
        "allOf".to_string(),
        json!([
            { "$ref": "#/$defs/Checkable" },
            Json::Object(own),
        ]),
    );
    Json::Object(schema)
}

/// The three components a2ui gives `Checkable` to.
fn is_checkable(component: &str) -> bool {
    matches!(component, "Button" | "CheckBox" | "TextField")
}

/// The **properties** of a published schema, wherever they are.
///
/// A schema that is an `allOf` holds its properties in its **own** part,
/// and a check walks the parts: the *own* object's properties are the
/// row's, and `Checkable`'s is the exempted `@checks`.
pub fn properties_of(schema: &Json) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    collect_properties(schema, &mut out);
    out
}

/// The property names of a schema, in the order they are declared.
fn collect_properties(schema: &Json, out: &mut Vec<String>) {
    if let Some(properties) = schema.get("properties").and_then(Json::as_object) {
        for name in properties.keys() {
            out.push(name.clone());
        }
    }
    if let Some(parts) = schema.get("allOf").and_then(Json::as_array) {
        for part in parts {
            collect_properties(part, out);
        }
    }
}

/// The shape of one property.
fn prop_shape(component: &str, prop: &str, definition: Option<&str>) -> Json {
    // Narrowing five: the field's value is a binding and nothing else.
    if component == "TextField" && prop == "value" {
        return json!({ "$ref": "#/$defs/DataBinding" });
    }
    match definition {
        Some(name) => json!({ "$ref": format!("#/$defs/{name}") }),
        None => inline(component, prop).unwrap_or_else(|| json!({})),
    }
}

/// Whether the property may be left out of the component.
fn optional(component: &str, prop: &str) -> bool {
    if prop == "weight" {
        return true;
    }
    if component == "Image" && prop == "description" {
        return true;
    }
    if component == "TextField" && prop == "placeholder" {
        return true;
    }
    match inline(component, prop) {
        Some(shape) => shape.get("default").is_some(),
        None => false,
    }
}

/// A binding to a place in the data model.
fn data_binding() -> Json {
    json!({
        "type": "object",
        "description": "A binding to a value in the data model.",
        "properties": {
            "@path": {
                "type": "string",
                "description": "A JSON Pointer to the value in the data model."
            }
        },
        "required": ["@path"],
        "additionalProperties": false
    })
}

/// The name of a component on a surface.
fn component_id() -> Json {
    json!({
        "type": "string",
        "description": "The name a component is known by on its surface."
    })
}

/// The name of a single child component.
fn child() -> Json {
    json!({
        "$ref": "#/$defs/ComponentId",
        "description": "The name of the one component to show here."
    })
}

/// The names of a container's children.
fn child_list() -> Json {
    json!({
        "oneOf": [
            {
                "type": "array",
                "description": "The children, named.",
                "items": { "$ref": "#/$defs/ComponentId" }
            }
        ]
    })
}

/// The envelope every function call carries.
fn function_common() -> Json {
    json!({
        "type": "object",
        "description": "What every function call carries, whatever the function.",
        "properties": {
            "@call": {
                "type": "string",
                "description": "The name of the function to call."
            },
            "catalogId": {
                "type": "string",
                "description": "The catalogue the function comes from, when the surface's own is not the one to use."
            }
        },
        "required": ["@call"]
    })
}

/// A call to one of the catalogue's functions.
fn function_call() -> Json {
    json!({
        "type": "object",
        "description": "A call to a function the catalogue declares.",
        "allOf": [
            { "$ref": "#/$defs/FunctionCommon" },
            { "$ref": "#/$defs/anyFunction" }
        ],
        "unevaluatedProperties": false
    })
}

/// A string: written down, bound, or computed.
fn dynamic_string() -> Json {
    json!({
        "description": "A string: written down, bound to the data model, or the answer of a call.",
        "oneOf": [
            { "type": "string" },
            { "$ref": "#/$defs/DataBinding" },
            { "$ref": "#/$defs/FunctionCall" }
        ]
    })
}

/// A boolean: written down, bound, or computed.
fn dynamic_boolean() -> Json {
    json!({
        "description": "A boolean: written down, bound to the data model, or the answer of a call.",
        "oneOf": [
            { "type": "boolean" },
            { "$ref": "#/$defs/DataBinding" },
            { "$ref": "#/$defs/FunctionCall" }
        ]
    })
}

/// A value of any kind: written down, bound, or computed.
fn dynamic_value() -> Json {
    json!({
        "description": "A value of any kind: written down, bound to the data model, or the answer of a call.",
        "oneOf": [
            { "type": "string" },
            { "type": "number" },
            { "type": "boolean" },
            { "type": "array" },
            {
                "type": "object",
                "description": "A plain object: its keys do not begin with '@', and it is neither a binding nor a call.",
                "propertyNames": {
                    "not": { "pattern": "^@([^@]|$)" }
                },
                "not": {
                    "anyOf": [
                        { "required": ["@path"] },
                        { "required": ["@call"] }
                    ]
                }
            },
            { "$ref": "#/$defs/DataBinding" },
            { "$ref": "#/$defs/FunctionCall" }
        ]
    })
}

/// How much of a row or column a component takes.
fn weight() -> Json {
    json!({
        "type": "number",
        "description": "How much of the row or column this component takes, like a flex grow factor. It counts only when the component is a direct child of a row or a column."
    })
}

/// What the components that could carry checks would carry.
///
/// The published component schema is **flat** — the discriminator, the
/// properties and the `required` list — so this definition is not
/// referenced from it. It is published anyway, because the catalogue's
/// `$defs` carries it: the shape of a checkable component's checks, with
/// the checks themselves taken out.
fn checkable() -> Json {
    json!({
        "type": "object",
        "description": "The part of a component's schema that could carry checks. The layer runs no check rule, so it publishes no check.",
        "properties": {}
    })
}

/// What a button does when it is pressed.
fn action() -> Json {
    json!({
        "description": "What happens when the component is activated: an event goes back to the agent, or a function runs.",
        "oneOf": [
            {
                "type": "object",
                "description": "An event to send back to the agent.",
                "properties": {
                    "event": {
                        "type": "object",
                        "description": "The event the agent is told about.",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The name of the action that happened."
                            },
                            "userMessage": {
                                "$ref": "#/$defs/DynamicString",
                                "description": "A line about what the user did, for the agent to read."
                            },
                            "context": {
                                "type": "object",
                                "description": "The key and value pairs that go with the event.",
                                "additionalProperties": { "$ref": "#/$defs/DynamicValue" }
                            }
                        },
                        "required": ["name"],
                        "additionalProperties": false
                    }
                },
                "required": ["event"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "description": "A function to run.",
                "properties": {
                    "functionCall": { "$ref": "#/$defs/FunctionCall" }
                },
                "required": ["functionCall"],
                "additionalProperties": false
            }
        ]
    })
}
