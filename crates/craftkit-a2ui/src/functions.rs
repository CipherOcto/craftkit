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
    },
    Function {
        name: "surfaceApply",
        doc: "Draw a component tree onto a surface.",
        kind: Kind::Local,
        return_type: "string",
    },
    Function {
        name: "surfaceDelete",
        doc: "Remove a surface's nodes.",
        kind: Kind::Local,
        return_type: "string",
    },
    Function {
        name: "dataSet",
        doc: "Write a value into the data model, and mirror it into craftkit.",
        kind: Kind::Local,
        return_type: "string",
    },
    Function {
        name: "componentInvoke",
        doc: "Run a component's action.",
        kind: Kind::Local,
        return_type: "string",
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
    },
    Function {
        name: "regex",
        doc: "Check that a value matches a regular expression.",
        kind: Kind::Agent,
        return_type: "validationResult",
    },
    Function {
        name: "length",
        doc: "Check a string's length.",
        kind: Kind::Agent,
        return_type: "validationResult",
    },
    Function {
        name: "numeric",
        doc: "Check that a value is a number.",
        kind: Kind::Agent,
        return_type: "validationResult",
    },
    Function {
        name: "email",
        doc: "Check that a value is an email address.",
        kind: Kind::Agent,
        return_type: "validationResult",
    },
    Function {
        name: "formatString",
        doc: "Format a value as a string.",
        kind: Kind::Agent,
        return_type: "string",
    },
    Function {
        name: "formatNumber",
        doc: "Format a value as a number.",
        kind: Kind::Agent,
        return_type: "string",
    },
    Function {
        name: "formatCurrency",
        doc: "Format a value as an amount of money.",
        kind: Kind::Agent,
        return_type: "string",
    },
    Function {
        name: "formatDate",
        doc: "Format a value as a date.",
        kind: Kind::Agent,
        return_type: "string",
    },
    Function {
        name: "pluralize",
        doc: "Choose the singular or the plural of a word.",
        kind: Kind::Agent,
        return_type: "string",
    },
    Function { name: "openUrl", doc: "Open a URL.", kind: Kind::Agent, return_type: "void" },
    Function { name: "and", doc: "Both of two values must hold.", kind: Kind::Agent, return_type: "boolean" },
    Function {
        name: "or",
        doc: "Either of two values must hold.",
        kind: Kind::Agent,
        return_type: "boolean",
    },
    Function { name: "not", doc: "A value must not hold.", kind: Kind::Agent, return_type: "boolean" },
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
/// `required` holding `"@call"` — plus a **`returnType`**, with
/// `unevaluatedProperties: false`.
pub fn declaration(function: &Function) -> craftkit_sdk::serde_json::Value {
    use craftkit_sdk::serde_json::json;
    json!({
        "type": "object",
        "description": function.doc,
        "returnType": function.return_type,
        "properties": {
            "@call": { "const": function.name },
            "args": {
                "type": "object",
                "properties": {},
                "unevaluatedProperties": false,
            },
        },
        "required": ["@call"],
        "unevaluatedProperties": false,
    })
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
        assert_eq!(declaration["required"], crate::serde_json::json!(["@call"]));
        assert!(declaration["returnType"].is_string(), "and it says what it returns");
        assert_eq!(declaration["unevaluatedProperties"], crate::serde_json::json!(false));
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
