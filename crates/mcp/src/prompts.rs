//! MCP prompts: the workflows a client can offer a user.
//!
//! A tool **acts**; a resource **is**; a prompt **says**. A prompt is a
//! templated message that tells an agent what to do, which is what a client
//! offers as a slash command. The server had tools (§171) and resources
//! (§176) and no prompts, so a client that read the protocol's third part
//! was told the method did not exist.
//!
//! The family serves them: **VC** five, and the rest one to three each. §178.
//!
//! # A prompt names a command the tree has
//!
//! A prompt is an instruction. An instruction that names a command the tree
//! does not have is a broken instruction — and it is broken *silently*: the
//! agent follows it and calls something that is not there.
//!
//! So a prompt does not write a command's name. It names the **command**, and
//! the message is built from the registry: the id, the label and the
//! description come from the same `CommandSpec` the tools and the docs use.
//! A prompt cannot describe a command differently from the registry, because
//! there is only one copy of the description.
//!
//! # One list
//!
//! `prompts/list` advertises from [`PROMPTS`] and `prompts/get` dispatches
//! through the same list, so a prompt cannot be advertised without being
//! fetchable. §171's rule, applied to the protocol's third part.
//!
//! # The suggestions are live
//!
//! An argument's suggested values are read from the tree **now** — the node
//! kinds from the model, the command ids from the registry, the nodes and
//! the property paths from the open design. A fixed list would be a copy of
//! the truth, and a copy goes stale the day a node is renamed.

use std::sync::Mutex;

use craftkit_engine::Session;
use serde_json::{Value as Json, json};

/// How many values one argument's suggestions may carry.
///
/// The protocol's maximum for a completion reply is a hundred; the tree's
/// own rule is that a list a client reads is small enough to read.
const MAX_VALUES: usize = 100;

/// A catalogue the suggestions are read from, so they follow the tree instead
/// of a copy of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Live {
    /// The kinds of node the model knows (`NodeKind::all`).
    NodeKinds,
    /// Every command id in the registry.
    Commands,
    /// The nodes of the open design, by id and by name.
    Nodes,
    /// The property paths of the open design.
    PropertyPaths,
}

/// Where one argument's suggestions come from.
#[derive(Clone, Copy)]
pub enum Values {
    /// A fixed list, written out here.
    Fixed(&'static [&'static str]),
    /// The live catalogue.
    Live(Live),
}

impl Values {
    /// The suggestions, read now.
    fn resolve(self, session: &Mutex<Session>) -> Vec<String> {
        let mut out = match self {
            Values::Fixed(values) => values.iter().map(|v| (*v).to_string()).collect(),
            Values::Live(live) => live_values(live, session),
        };
        out.truncate(MAX_VALUES);
        out
    }
}

/// The values of one live catalogue.
fn live_values(live: Live, session: &Mutex<Session>) -> Vec<String> {
    match live {
        Live::NodeKinds => craftkit_model::NodeKind::all().iter().map(|k| k.label().to_string()).collect(),
        Live::Commands => craftkit_engine::command::ids().into_iter().map(|id| id.to_string()).collect(),
        Live::Nodes => {
            let Ok(guard) = session.lock() else { return Vec::new() };
            let mut out = Vec::new();
            for node in &guard.document().nodes {
                out.push(node.id.get().to_string());
                out.push(node.name.clone());
            }
            out
        }
        Live::PropertyPaths => {
            let Ok(guard) = session.lock() else { return Vec::new() };
            let mut out = Vec::new();
            for node in &guard.document().nodes {
                for (path, _) in node.props.flatten() {
                    out.push(path.to_string());
                }
            }
            out
        }
    }
}

/// One prompt argument.
pub struct PromptArg {
    /// The name the caller gives it.
    pub name: &'static str,
    /// What it means, in one line.
    pub description: &'static str,
    /// Whether the caller must give it.
    pub required: bool,
    /// Substituted for `{name}` when the caller leaves the argument out.
    pub default: &'static str,
    /// Where its suggestions come from.
    pub values: Values,
}

/// One prompt: a workflow with templated arguments.
pub struct PromptDef {
    /// The name a client offers it by.
    pub name: &'static str,
    /// A human title.
    pub title: &'static str,
    /// What it does, in one line.
    pub description: &'static str,
    /// The command this prompt drives.
    ///
    /// The message is built from the registry's entry for it, so the prompt
    /// cannot describe a command differently from the tree does.
    pub command: &'static str,
    /// Its arguments.
    pub arguments: &'static [PromptArg],
    /// The message body. `{argument}` is replaced with the argument's value,
    /// and `{command}`, `{label}`, `{doc}` and `{params}` are replaced with
    /// the registry's own words for the command.
    pub text: &'static str,
}

/// Every prompt, in `prompts/list` order.
///
/// **One list.** The advertisement and the dispatch both read it, so they
/// cannot disagree about which prompts exist. §178.
pub static PROMPTS: &[PromptDef] = &[
    PromptDef {
        name: "add-node",
        title: "Add a node",
        description: "Add a node to the open design, of a kind the tree knows.",
        command: "node.add",
        arguments: &[
            PromptArg {
                name: "kind",
                description: "What the node is for: a source, a filter, a sink, a cue or a comment.",
                required: true,
                default: "filter",
                values: Values::Live(Live::NodeKinds),
            },
            PromptArg {
                name: "name",
                description: "What the node is called. A name has to be unique in the design.",
                required: false,
                default: "New node",
                values: Values::Fixed(&[]),
            },
        ],
        text: "\
Add a node to the open design.

Kind: {kind}
Name: {name}

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

The node's id comes back in the command's data; use it for anything the node
does next.",
    },
    PromptDef {
        name: "set-property",
        title: "Set a property",
        description: "Set one property of one node to a value.",
        command: "prop.set",
        arguments: &[
            PromptArg {
                name: "node",
                description: "The node, by its id or by its name.",
                required: true,
                default: "",
                values: Values::Live(Live::Nodes),
            },
            PromptArg {
                name: "path",
                description: "The property, by its path.",
                required: true,
                default: "",
                values: Values::Live(Live::PropertyPaths),
            },
            PromptArg {
                name: "value",
                description: "The value the property takes.",
                required: true,
                default: "1",
                values: Values::Fixed(&[]),
            },
        ],
        text: "\
Set one property of one node.

Node: {node}
Property: {path}
Value: {value}

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

If the command refuses the path, call `{command}` with no arguments to see
what the design holds, and take the path from there.",
    },
    PromptDef {
        name: "key-property",
        title: "Key a property",
        description: "Key one property of one node at an instant, so it moves over time.",
        command: "key.set",
        arguments: &[
            PromptArg {
                name: "node",
                description: "The node, by its id or by its name.",
                required: true,
                default: "",
                values: Values::Live(Live::Nodes),
            },
            PromptArg {
                name: "path",
                description: "The property, by its path.",
                required: true,
                default: "",
                values: Values::Live(Live::PropertyPaths),
            },
            PromptArg {
                name: "time",
                description: "When the key is, in seconds from the start of the design.",
                required: true,
                default: "0",
                values: Values::Fixed(&[]),
            },
            PromptArg {
                name: "value",
                description: "What the property holds at that instant.",
                required: true,
                default: "1",
                values: Values::Fixed(&[]),
            },
        ],
        text: "\
Key one property of one node, so it moves over time.

Node: {node}
Property: {path}
At: {time} seconds
Value: {value}

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

A key outside the design's range is allowed, and the design says so in the
command's message: the playhead can never reach it.",
    },
];

/// Why a prompt could not be fetched.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PromptError {
    /// No prompt has that name.
    #[error("no prompt named `{0}`; call prompts/list")]
    NotFound(String),
    /// A required argument was left out.
    #[error("the prompt `{prompt}` needs its `{argument}`")]
    Missing {
        /// The prompt's name.
        prompt: String,
        /// The argument's name.
        argument: String,
    },
}

/// The prompts, as `prompts/list` answers.
pub fn list_json() -> Json {
    Json::Array(
        PROMPTS
            .iter()
            .map(|p| {
                json!({
                    "name": p.name,
                    "title": p.title,
                    "description": p.description,
                    "arguments": p.arguments.iter().map(|a| json!({
                        "name": a.name,
                        "description": a.description,
                        "required": a.required,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// Fetch one prompt's message, with its arguments substituted.
///
/// The session is **not** read: the message is built from the registry and
/// from the arguments the caller gave, so the same prompt reads the same way
/// whatever the open design holds. The *suggestions* read the design; the
/// message does not.
pub fn message(name: &str, arguments: &Json, _session: &Mutex<Session>) -> Result<Json, PromptError> {
    let prompt =
        PROMPTS.iter().find(|p| p.name == name).ok_or_else(|| PromptError::NotFound(name.to_string()))?;
    let mut text = prompt.text.to_string();

    for argument in prompt.arguments {
        let given = arguments.get(argument.name).and_then(Json::as_str);
        let value = match given {
            Some(value) => value.to_string(),
            None if argument.required => {
                return Err(PromptError::Missing {
                    prompt: prompt.name.to_string(),
                    argument: argument.name.to_string(),
                });
            }
            None => argument.default.to_string(),
        };
        text = text.replace(&format!("{{{}}}", argument.name), &value);
    }

    // The command's own words, from the registry: the id, the label, the
    // description and the parameters, all from the same `CommandSpec` the
    // tools and the docs use. A prompt cannot describe a command
    // differently from the tree does, because there is only one copy.
    let spec = craftkit_engine::command::get(prompt.command);
    let (id, label, doc, params) = match spec {
        Some(spec) => (
            spec.id.to_string(),
            spec.label.to_string(),
            spec.doc.to_string(),
            spec.params
                .iter()
                .map(|p| format!("  {} ({:?}): {}", p.name, p.kind, p.help))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        None => {
            // The registry does not have the command. That is a bug in the
            // tree, and the gate in `xtask/src/lists.rs` looks for it; the
            // server does not pretend the prompt is fine.
            return Err(PromptError::NotFound(format!(
                "{} (the command `{}` is not in the registry)",
                prompt.name, prompt.command
            )));
        }
    };
    text = text.replace("{command}", &id);
    text = text.replace("{label}", &label);
    text = text.replace("{doc}", &doc);
    text = text.replace("{params}", &params);

    Ok(json!({
        "description": prompt.description,
        "messages": [{ "role": "user", "content": { "type": "text", "text": text } }],
    }))
}

/// The suggestions for one prompt argument.
///
/// The values come from the argument's own declaration, so a suggestion can
/// never be for an argument the prompt does not have.
pub fn complete(
    prompt_name: &str,
    argument_name: &str,
    session: &Mutex<Session>,
) -> Result<Vec<String>, PromptError> {
    let prompt = PROMPTS
        .iter()
        .find(|p| p.name == prompt_name)
        .ok_or_else(|| PromptError::NotFound(prompt_name.to_string()))?;
    let argument = prompt.arguments.iter().find(|a| a.name == argument_name).ok_or_else(|| {
        PromptError::Missing { prompt: prompt.name.to_string(), argument: argument_name.to_string() }
    })?;
    Ok(argument.values.resolve(session))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Mutex<Session> {
        Mutex::new(Session::sample())
    }

    /// Every prompt names a command the registry has.
    ///
    /// A prompt is an instruction; an instruction that names a command the
    /// tree does not have is broken silently — the agent follows it and calls
    /// something that is not there. §178.
    #[test]
    fn every_prompt_names_a_command_the_registry_has() {
        for prompt in PROMPTS {
            assert!(
                craftkit_engine::command::get(prompt.command).is_some(),
                "the prompt `{}` names the command `{}`, which the registry does not have",
                prompt.name,
                prompt.command
            );
        }
    }

    /// Every prompt that is advertised can be fetched.
    ///
    /// The advertisement and the dispatch are one list, so a prompt cannot be
    /// advertised without being fetchable. §171's rule. §178.
    #[test]
    fn every_advertised_prompt_can_be_fetched() {
        let s = session();
        for prompt in PROMPTS {
            let arguments = json!({
                "kind": "filter",
                "name": "Probe",
                "node": "1",
                "path": "amount",
                "value": "1",
                "time": "0",
            });
            let reply = message(prompt.name, &arguments, &s)
                .unwrap_or_else(|e| panic!("the prompt `{}` cannot be fetched: {e}", prompt.name));
            let text = reply["messages"][0]["content"]["text"].as_str().unwrap_or_default();
            assert!(!text.is_empty(), "the prompt `{}` holds nothing", prompt.name);
        }
    }

    /// The message is built from the registry's own words for the command.
    ///
    /// A prompt cannot describe a command differently from the tree does,
    /// because there is only one copy of the description. §178.
    #[test]
    fn the_message_carries_the_registry_s_words() {
        let s = session();
        let spec = craftkit_engine::command::get("node.add").expect("the command is in the registry");
        let reply = message("add-node", &json!({ "kind": "filter" }), &s).expect("the prompt is fetched");
        let text = reply["messages"][0]["content"]["text"].as_str().unwrap_or_default();
        assert!(text.contains(spec.id), "the message does not name the command: {text}");
        assert!(text.contains(spec.label), "the message does not carry the label: {text}");
        assert!(text.contains(spec.doc), "the message does not carry the description: {text}");
        assert!(!text.contains("{command}"), "a placeholder was left in: {text}");
        assert!(!text.contains("{params}"), "a placeholder was left in: {text}");
    }

    /// A required argument that is left out is refused.
    #[test]
    fn a_required_argument_must_be_given() {
        let s = session();
        assert_eq!(
            message("add-node", &json!({}), &s),
            Err(PromptError::Missing { prompt: "add-node".to_string(), argument: "kind".to_string() }),
            "a prompt without its required argument was fetched"
        );
        assert!(matches!(message("nope", &json!({}), &s), Err(PromptError::NotFound(_))));
    }

    /// The suggestions are the tree's, not a copy's.
    #[test]
    fn the_suggestions_are_live() {
        let s = session();
        let kinds = complete("add-node", "kind", &s).expect("the argument is known");
        let model: Vec<String> =
            craftkit_model::NodeKind::all().iter().map(|k| k.label().to_string()).collect();
        assert_eq!(kinds, model, "the suggestions are not the model's own");

        let commands = complete("add-node", "kind", &s).expect("the argument is known");
        assert!(!commands.is_empty());

        // A prompt that has no such argument says so, rather than answering
        // with nothing.
        assert!(complete("add-node", "nope", &s).is_err(), "an unknown argument was answered");
        assert!(complete("nope", "kind", &s).is_err(), "an unknown prompt was answered");
    }
}
