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
    ///
    /// **Not cut short here.** The protocol's cap of a hundred is applied
    /// after the filter, so that the count the client is told is the count
    /// of everything that matched. §179.
    fn resolve(self, session: &Mutex<Session>) -> Vec<String> {
        match self {
            Values::Fixed(values) => values.iter().map(|v| (*v).to_string()).collect(),
            Values::Live(live) => live_values(live, session),
        }
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
    // The values the template is filled with, and the order they are read in.
    //
    // The arguments come first, so an argument named `command` is the
    // caller's and not the registry's; the registry's own words come second
    // and fill the names no argument claimed.
    let mut values: Vec<(&str, String)> = Vec::new();

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
        values.push((argument.name, value));
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
    for (name, value) in [("command", id), ("label", label), ("doc", doc), ("params", params)] {
        if !values.iter().any(|(claimed, _)| *claimed == name) {
            values.push((name, value));
        }
    }

    // The template is read **once**, and a value is put into it as it is.
    //
    // The old way replaced one placeholder at a time, in the text the last
    // replacement had already made: so a caller's value that itself looked
    // like a placeholder was filled in again, and the registry's own words
    // were substituted into the caller's text. A value is text. §255.
    let text = fill(prompt.text, &values);

    Ok(json!({
        "description": prompt.description,
        "messages": [{ "role": "user", "content": { "type": "text", "text": text } }],
    }))
}

/// Fill a template's placeholders, once.
///
/// The placeholders are read from the **template**, and a value goes into
/// the result as it is: a value that itself looks like a placeholder is
/// text, not a placeholder. §255.
fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find('}') else {
            // No closing brace: the rest of the template is text.
            out.push_str(after);
            return out;
        };
        let name = &after[1..end];
        match values.iter().find(|(claimed, _)| *claimed == name) {
            Some((_, value)) => out.push_str(value),
            // A placeholder nobody claims is left as it is: the tree does not
            // invent a word for it.
            None => out.push_str(&after[..=end]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The suggestions for one prompt argument.
///
/// The values come from the argument's own declaration, so a suggestion can
/// never be for an argument the prompt does not have — and they are
/// **filtered by what the caller has typed**, because that is what the
/// method is for. §179.
pub fn complete(
    prompt_name: &str,
    argument_name: &str,
    typed: &str,
    session: &Mutex<Session>,
) -> Result<crate::rpc::Suggestions, PromptError> {
    let prompt = PROMPTS
        .iter()
        .find(|p| p.name == prompt_name)
        .ok_or_else(|| PromptError::NotFound(prompt_name.to_string()))?;
    let argument = prompt.arguments.iter().find(|a| a.name == argument_name).ok_or_else(|| {
        PromptError::Missing { prompt: prompt.name.to_string(), argument: argument_name.to_string() }
    })?;
    let matching: Vec<String> =
        argument.values.resolve(session).into_iter().filter(|v| crate::rpc::starts_with(v, typed)).collect();
    let total = matching.len();
    Ok(crate::rpc::Suggestions { values: matching.into_iter().take(crate::rpc::MAX_VALUES).collect(), total })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Mutex<Session> {
        Mutex::new(Session::sample())
    }

    /// **A value is text, not a placeholder.** §255.
    ///
    /// The old way replaced one placeholder at a time, in the text the last
    /// replacement had already made: so a caller's value that itself looked
    /// like a placeholder was filled in again — and the registry's own words
    /// were substituted into the caller's text. An agent reading the message
    /// would see text the caller never sent, in the field of another
    /// argument.
    #[test]
    fn a_value_that_looks_like_a_placeholder_is_left_alone() {
        let s = session();
        // The caller's `kind` is the other argument's placeholder, and the
        // caller's `name` is the registry's own word for the command's
        // description.
        let arguments = json!({"kind": "{name}", "name": "{doc}"});
        let reply = message("add-node", &arguments, &s).expect("the message");
        let text = reply["messages"][0]["content"]["text"].as_str().expect("the text");
        assert!(text.contains("Kind: {name}"), "the caller's value was replaced: {text}");
        assert!(text.contains("Name: {doc}"), "the caller's value was replaced: {text}");
        // And the registry's own words are still in the message, where the
        // template asked for them.
        assert!(text.contains("Call the command `node.add`"), "{text}");
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
        let kinds = complete("add-node", "kind", "", &s).expect("the argument is known");
        let model: Vec<String> =
            craftkit_model::NodeKind::all().iter().map(|k| k.label().to_string()).collect();
        assert_eq!(kinds.values, model, "the suggestions are not the model's own");
        assert_eq!(kinds.total, model.len(), "the count is not the model's own");

        // A prompt that has no such argument says so, rather than answering
        // with nothing.
        assert!(complete("add-node", "nope", "", &s).is_err(), "an unknown argument was answered");
        assert!(complete("nope", "kind", "", &s).is_err(), "an unknown prompt was answered");
    }

    /// The suggestions are filtered by what the caller has typed.
    ///
    /// That is the whole point of the method: a list of every value, every
    /// time, is not a suggestion. §179.
    #[test]
    fn the_suggestions_are_filtered_by_what_was_typed() {
        let s = session();
        let all = complete("add-node", "kind", "", &s).expect("the argument is known");
        let some = complete("add-node", "kind", "s", &s).expect("the argument is known");
        assert!(some.total < all.total, "the filter did not narrow the list");
        assert!(
            some.values.iter().all(|v| v.to_ascii_lowercase().starts_with('s')),
            "a suggestion does not begin with what was typed: {:?}",
            some.values
        );
        // And the match is case-insensitive, the way the tree matches names.
        let upper = complete("add-node", "kind", "S", &s).expect("the argument is known");
        assert_eq!(upper.values, some.values, "the match was case-sensitive");
    }
}
