//! The **function table** — what the layer does, and what it expects the
//! agent to have.
//!
//! # The three namespaces
//!
//! An a2ui **function name** (`surfaceApply`), an a2ui **message name**
//! (`createSurface`) and a craftkit **command id** (`a2ui.surface.apply`)
//! are three different strings in three different places, and this table
//! keeps them apart on purpose: a function name that reads like a message
//! name would be ambiguous to an agent generating `{"@call": "…"}`.
//!
//! # The two kinds of entry
//!
//! | the entry | what it says | `allowedCallers` |
//! |---|---|---|
//! | a **local** function | one the layer **implements** | `"rendererOnly"` |
//! | an **agent-side** function | one the layer **expects the agent to have** and does not implement | `"rendererOnly"` |
//!
//! **Both** are `"rendererOnly"`: that is the value that lets the
//! **renderer** dispatch the call, and it only forbids the **agent** from
//! calling it back (`specification/v1_0/docs/a2ui_protocol.md:1484`).
//!
//! # The three rules for a name
//!
//! | the name | what happens |
//! |---|---|
//! | in the **local table** | it is a craftkit **command id**, and it runs through `Session::execute` |
//! | **declared agent-side** in the catalogue's `functions` | it is dispatched to the agent as `callAgentFunction` |
//! | **neither** | it is **refused** |
//!
//! **The third row is a narrowing, and it is deliberate.** The protocol's
//! own default is *fallback* — a name the catalogue does not declare at
//! all is assumed to be the agent's. craftkit's rule is that an unknown
//! name is refused, so the layer refuses it rather than sending it on.
//!
//! # The reserved functions
//!
//! A **reserved** `@`-prefixed function is refused, not forwarded: the
//! protocol says it is always local and must never go to the agent, and
//! the layer implements no reserved functions — so a call to one is
//! **refused**, with the reason.

/// The **kind** of a function in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The layer **implements** it: the name is a craftkit command id.
    Local,
    /// The layer **expects the agent to have** it, and does not implement
    /// it: the name is dispatched to the agent.
    Agent,
}

/// One function of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Function {
    /// The a2ui **function name**: `surfaceApply`.
    pub name: &'static str,
    /// What it does, in one line.
    pub doc: &'static str,
    /// Whether the layer implements it or expects the agent to have it.
    pub kind: Kind,
    /// The **return type** the function's shape carries.
    ///
    /// a2ui's `FunctionDefinition` requires a `returnType` from its own
    /// enum of eight, and a function whose return shape is not known is
    /// **not declared**.
    pub return_type: &'static str,
    /// **Who may call it**: the boundary the declaration carries.
    ///
    /// The protocol says a function declares its own boundary
    /// (`specification/v1_0/docs/a2ui_protocol.md:629`), and a function
    /// that does not say is the renderer's alone. The layer says
    /// **`rendererOnly`** for every function it declares, and the value
    /// is the one that **permits the renderer's own dispatch** — the
    /// layer resolves a call itself, or hands it to the agent, and
    /// `rendererOnly` is what lets the hand happen
    /// (`a2ui_protocol.md:1484`; `a2ui_custom_functions.md:71`).
    ///
    /// It is the **agent** that `rendererOnly` forbids: the agent may
    /// not call a function back. So the local functions are the
    /// renderer's, and the agent-side ones are the agent's to answer —
    /// and neither is the agent's to call.
    pub allowed_callers: &'static str,
    /// The **arguments** the function takes: the a2ui name and the
    /// craftkit parameter name.
    ///
    /// The two vocabularies are not the same — an a2ui call says
    /// `surfaceId` and a craftkit command says `surface_id` — so the
    /// table says which is which, and the door translates before it
    /// runs the command. An agent that follows the catalogue can call
    /// a function, and the layer does not guess at an argument's name.
    pub args: &'static [(&'static str, &'static str)],
    /// The arguments the function **must** be given: their a2ui names.
    ///
    /// The catalogue says which arguments a call cannot leave out, and a
    /// call that leaves one out is a **wrong request** — and the door
    /// says so before it runs anything. An argument that is not in this
    /// list is optional, and one that is not in `args` at all is
    /// **refused**.
    pub required: &'static [&'static str],
}

/// The **local** functions: the layer's own commands.
///
/// The names are a2ui's style — the protocol's function names are UAX #31
/// names (`specification/v1_0/docs/a2ui_protocol.md:546-548`) — and the
/// commands they name are craftkit's dotted ids. **The key and the value
/// are never the same string.**
pub const LOCAL: &[Function] = &[
    Function {
        name: "surfaceCreate",
        doc: "Make a surface, and the node it draws into.",
        kind: Kind::Local,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("surfaceId", "surface_id")],
        required: &["surfaceId"],
    },
    Function {
        name: "surfaceApply",
        doc: "Draw a component tree onto a surface.",
        kind: Kind::Local,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("surfaceId", "surface_id"), ("components", "components")],
        required: &["surfaceId", "components"],
    },
    Function {
        name: "surfaceDelete",
        doc: "Remove a surface's nodes.",
        kind: Kind::Local,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("surfaceId", "surface_id")],
        required: &["surfaceId"],
    },
    Function {
        name: "dataSet",
        doc: "Write a value into the data model, and mirror it into craftkit.",
        kind: Kind::Local,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("surfaceId", "surface_id"), ("path", "path"), ("value", "value")],
        required: &["surfaceId", "path", "value"],
    },
    Function {
        name: "componentInvoke",
        doc: "Run a component's action.",
        kind: Kind::Local,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("surfaceId", "surface_id"), ("componentId", "component_id"), ("action", "action")],
        required: &["surfaceId", "componentId", "action"],
    },
];

/// The **agent-side** functions: the ones the layer expects the agent to
/// have, and does not implement.
///
/// The protocol says that *"a renderer that rejects payloads naming
/// functions absent from its catalogues MUST declare the agent-side
/// functions it expects, without implementing them"*
/// (`specification/v1_0/docs/a2ui_protocol.md:1488`). So the declaration
/// is in the catalogue's `functions` map — and that is what makes the
/// second row of the table above possible.
pub const AGENT_SIDE: &[Function] = &[
    Function {
        name: "required",
        doc: "Check that a value is not null, undefined or empty.",
        kind: Kind::Agent,
        return_type: "validationResult",
        allowed_callers: "rendererOnly",
        args: &[("value", "value")],
        required: &["value"],
    },
    Function {
        name: "regex",
        doc: "Check that a value matches a regular expression.",
        kind: Kind::Agent,
        return_type: "validationResult",
        allowed_callers: "rendererOnly",
        args: &[("value", "value"), ("pattern", "pattern")],
        required: &["value", "pattern"],
    },
    Function {
        name: "length",
        doc: "Check a string's length.",
        kind: Kind::Agent,
        return_type: "validationResult",
        allowed_callers: "rendererOnly",
        args: &[("value", "value"), ("min", "min"), ("max", "max")],
        required: &["value"],
    },
    Function {
        name: "numeric",
        doc: "Check that a value is a number.",
        kind: Kind::Agent,
        return_type: "validationResult",
        allowed_callers: "rendererOnly",
        args: &[("value", "value"), ("min", "min"), ("max", "max")],
        required: &["value"],
    },
    Function {
        name: "email",
        doc: "Check that a value is an email address.",
        kind: Kind::Agent,
        return_type: "validationResult",
        allowed_callers: "rendererOnly",
        args: &[("value", "value")],
        required: &["value"],
    },
    Function {
        name: "formatString",
        doc: "Format a value as a string.",
        kind: Kind::Agent,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("value", "value")],
        required: &["value"],
    },
    Function {
        name: "formatNumber",
        doc: "Format a value as a number.",
        kind: Kind::Agent,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("value", "value"), ("decimals", "decimals"), ("grouping", "grouping")],
        required: &["value"],
    },
    Function {
        name: "formatCurrency",
        doc: "Format a value as an amount of money.",
        kind: Kind::Agent,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[
            ("value", "value"),
            ("currency", "currency"),
            ("decimals", "decimals"),
            ("grouping", "grouping"),
        ],
        required: &["value", "currency"],
    },
    Function {
        name: "formatDate",
        doc: "Format a value as a date.",
        kind: Kind::Agent,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[("value", "value"), ("format", "format")],
        required: &["value", "format"],
    },
    Function {
        name: "pluralize",
        doc: "Choose the singular or the plural of a word.",
        kind: Kind::Agent,
        return_type: "string",
        allowed_callers: "rendererOnly",
        args: &[
            ("value", "value"),
            ("zero", "zero"),
            ("one", "one"),
            ("two", "two"),
            ("few", "few"),
            ("many", "many"),
            ("other", "other"),
        ],
        required: &["value", "other"],
    },
    Function {
        name: "openUrl",
        doc: "Open a URL.",
        kind: Kind::Agent,
        return_type: "void",
        allowed_callers: "rendererOnly",
        args: &[("url", "url")],
        required: &["url"],
    },
    Function {
        name: "and",
        doc: "Both of two values must hold.",
        kind: Kind::Agent,
        return_type: "boolean",
        allowed_callers: "rendererOnly",
        args: &[("values", "values")],
        required: &["values"],
    },
    Function {
        name: "or",
        doc: "Either of two values must hold.",
        kind: Kind::Agent,
        return_type: "boolean",
        allowed_callers: "rendererOnly",
        args: &[("values", "values")],
        required: &["values"],
    },
    Function {
        name: "not",
        doc: "A value must not hold.",
        kind: Kind::Agent,
        return_type: "boolean",
        allowed_callers: "rendererOnly",
        args: &[("value", "value")],
        required: &["value"],
    },
];

/// Every function of the table: the local ones and the agent-side ones.
pub fn all() -> Vec<Function> {
    let mut out: Vec<Function> = LOCAL.to_vec();
    out.extend_from_slice(AGENT_SIDE);
    out
}

/// The function of a **name**, if the table has one.
pub fn by_name(name: &str) -> Option<Function> {
    all().into_iter().find(|function| function.name == name)
}

/// The **command id** a local function runs.
///
/// A local function's name is a2ui's and its command is craftkit's, and
/// the two are never the same string.
pub fn command_of(name: &str) -> Option<&'static str> {
    match name {
        "surfaceCreate" => Some("a2ui.surface.create"),
        "surfaceApply" => Some("a2ui.surface.apply"),
        "surfaceDelete" => Some("a2ui.surface.delete"),
        "dataSet" => Some("a2ui.data.set"),
        "componentInvoke" => Some("a2ui.component.invoke"),
        _ => None,
    }
}

/// Whether a name is **reserved**.
///
/// A reserved function is `@`-prefixed, and the protocol says it is always
/// local and must never go to the agent. The layer implements no reserved
/// functions, so a call to one is **refused**.
pub fn is_reserved(name: &str) -> bool {
    name.starts_with('@')
}

/// The **declaration** of a function, in a2ui's `catalog_definition` shape.
///
/// a2ui validates this shape: a `FunctionDefinition` requires a **`@call`**
/// envelope — `type`, `properties` with a `"@call"` `const`, and a
/// `required` holding `"@call"` — plus a **`returnType`**, and the
/// definition **refuses** a property it does not name.
///
/// And that last part is the reason the declaration carries no
/// `unevaluatedProperties` of its own: the keyword belongs to a
/// **schema**, and the declaration is an **instance**. The keyword is
/// inside the `args` — which is a schema — and nowhere else, so the
/// catalogue a2ui's own schema checks is a catalogue that passes.
pub fn declaration(function: &Function) -> craftkit_sdk::serde_json::Value {
    use craftkit_sdk::serde_json::json;
    // The **arguments** the function takes: the a2ui names, and the
    // shapes they are. A function that takes arguments says which, so
    // an agent that follows the catalogue can call it.
    let mut properties = craftkit_sdk::serde_json::Map::new();
    for (name, _) in function.args {
        properties.insert((*name).to_string(), json!({}));
    }
    // And the ones it **must** be given.
    let required: Vec<craftkit_sdk::serde_json::Value> =
        function.required.iter().map(|name| json!(name)).collect();
    let mut args = craftkit_sdk::serde_json::Map::new();
    args.insert("type".to_string(), json!("object"));
    args.insert("properties".to_string(), craftkit_sdk::serde_json::Value::Object(properties));
    if !required.is_empty() {
        args.insert("required".to_string(), craftkit_sdk::serde_json::Value::Array(required));
    }
    args.insert("unevaluatedProperties".to_string(), json!(false));
    json!({
        "type": "object",
        "description": function.doc,
        "returnType": function.return_type,
        "allowedCallers": function.allowed_callers,
        "properties": {
            "@call": { "const": function.name },
            "args": craftkit_sdk::serde_json::Value::Object(args),
        },
        "required": ["@call", "args"],
    })
}

/// The **arguments** a function takes, by its a2ui name.
///
/// The pairs are the a2ui name and the craftkit parameter name: the
/// door translates before it runs the command, so an agent's own
/// spelling is the one that is written down.
pub fn arguments_of(name: &str) -> Option<&'static [(&'static str, &'static str)]> {
    all().iter().find(|function| function.name == name).map(|function| function.args)
}

/// The arguments a function **must** be given, by their a2ui names.
///
/// A call that leaves one of these out is a **wrong request**: the
/// catalogue says which they are, and the door says so before it runs
/// anything.
pub fn required_of(name: &str) -> Option<&'static [&'static str]> {
    all().iter().find(|function| function.name == name).map(|function| function.required)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The **five** local functions are the layer's own commands.
    #[test]
    fn the_five_local_functions_are_the_layers_own_commands() {
        assert_eq!(LOCAL.len(), 5);
        let names: Vec<&str> = LOCAL.iter().map(|function| function.name).collect();
        assert_eq!(
            names,
            vec!["surfaceCreate", "surfaceApply", "surfaceDelete", "dataSet", "componentInvoke"]
        );
        for function in LOCAL {
            let command = command_of(function.name).expect("the command is there");
            assert_ne!(command, function.name, "the key and the value are never the same string");
        }
    }

    /// The agent-side functions are the ones a2ui's own catalogue has.
    #[test]
    fn the_agent_side_functions_are_the_ones_a2ui_has() {
        assert_eq!(AGENT_SIDE.len(), 14);
        for function in AGENT_SIDE {
            assert_eq!(function.kind, Kind::Agent);
            assert!(command_of(function.name).is_none(), "the layer does not implement it");
        }
    }

    /// A name that is in neither is **refused**.
    #[test]
    fn a_name_that_is_in_neither_is_refused() {
        assert!(by_name("surfaceApply").is_some(), "a local one is there");
        assert!(by_name("required").is_some(), "an agent-side one is there");
        assert!(by_name("somethingElse").is_none(), "and an unknown one is not");
    }

    /// A **reserved** name is recognised.
    #[test]
    fn a_reserved_name_is_recognised() {
        assert!(is_reserved("@index"));
        assert!(!is_reserved("surfaceApply"));
    }

    /// The declaration carries the `@call` envelope a2ui requires.
    #[test]
    fn the_declaration_carries_the_envelope() {
        let function = by_name("surfaceApply").expect("the function is there");
        let declaration = declaration(&function);
        assert_eq!(declaration["properties"]["@call"], crate::serde_json::json!({ "const": "surfaceApply" }));
        assert_eq!(declaration["required"], crate::serde_json::json!(["@call", "args"]));
        assert!(declaration["returnType"].is_string(), "and it says what it returns");
        // And the **arguments** it takes, and the ones it must be given.
        let args = &declaration["properties"]["args"];
        assert!(args["properties"]["surfaceId"].is_object(), "the argument is named");
        assert_eq!(args["required"], crate::serde_json::json!(["surfaceId", "components"]));
        // The keyword belongs to a **schema**: the `args` are a schema
        // and carry it, and the declaration — an instance — does not.
        assert_eq!(args["unevaluatedProperties"], crate::serde_json::json!(false));
        assert!(declaration.get("unevaluatedProperties").is_none(), "and the instance carries none");
    }

    /// The **agent-side** functions carry the arguments a2ui's own
    /// catalogue names, and the ones it requires.
    ///
    /// An agent that follows a2ui's catalogue sends `args.value` — and a
    /// declaration that forbids it would forbid the agent's own call.
    #[test]
    fn the_agent_side_functions_carry_their_arguments() {
        let function = by_name("required").expect("the function is there");
        let declaration = declaration(&function);
        let args = &declaration["properties"]["args"];
        assert!(args["properties"]["value"].is_object(), "the argument is named");
        assert_eq!(args["required"], crate::serde_json::json!(["value"]));
        // And the layer's own table says the same.
        assert_eq!(required_of("required"), Some(&["value"][..]));
    }

    /// The two kinds of entry do not overlap.
    #[test]
    fn the_two_kinds_of_entry_do_not_overlap() {
        let mut names: Vec<&str> = all().iter().map(|function| function.name).collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "every name appears once");
    }
}
