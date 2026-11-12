//! MCP prompts: the workflows a client can offer a user.
//!
//! A tool **acts**; a resource **is**; a prompt **says**. A prompt is
//! a templated message that tells an agent what to do, which is what
//! a client offers as a slash command. The server had tools (§171)
//! and resources (§176) and no prompts, so a client that read the
//! protocol's third part was told the method did not exist.
//!
//! # A prompt names a command the tree has
//!
//! A prompt is an instruction. An instruction that names a command
//! the tree does not have is a broken instruction — and it is broken
//! *silently*: the agent follows it and calls something that is not
//! there.
//!
//! So a prompt does not write a command's name. It names the
//! **command**, and the message is built from the registry: the id,
//! the label and the description come from the same `CommandSpec` the
//! tools use. A prompt cannot describe a command differently from the
//! registry, because there is only one copy of the description.
//!
//! And a prompt that names a command this server does not have is
//! **not advertised**: the list holds the prompts whose command is in
//! the registry, so a client never sees a broken instruction.
//!
//! # One list
//!
//! `prompts/list` advertises from [`PROMPTS`] and `prompts/get`
//! dispatches through the same list, so a prompt cannot be advertised
//! without being fetchable.
//!
//! # The suggestions are live
//!
//! An argument's suggested values are read from the tree **now** — the
//! command ids from the registry, the nodes from the session. A fixed
//! list would be a copy of the truth, and a copy goes stale the day a
//! node is renamed.

use craftkit_sdk::Session;
use craftkit_sdk::serde_json::{Value as Json, json};

/// The most suggestions a client is told about.
///
/// The protocol's own cap. It is applied **after** the filter, so the
/// count the client is told is the count of everything that matched.
pub const MAX_SUGGESTIONS: usize = 100;

/// A catalogue the suggestions are read from, so they follow the tree
/// instead of a copy of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Live {
    /// Every command id in the registry.
    Commands,
    /// The nodes of the session, by id.
    NodeIds,
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
    fn resolve(self, session: &Session) -> Vec<String> {
        match self {
            Values::Fixed(values) => values.iter().map(|value| (*value).to_string()).collect(),
            Values::Live(live) => live_values(live, session),
        }
    }
}

/// The values of one live catalogue.
fn live_values(live: Live, session: &Session) -> Vec<String> {
    match live {
        Live::Commands => session.registry().all().iter().map(|spec| spec.id.to_string()).collect(),
        Live::NodeIds => session.document().nodes().map(|node| node.id.to_string()).collect(),
    }
}

/// One **argument** of a prompt.
pub struct PromptArg {
    /// The argument's name.
    pub name: &'static str,
    /// What it is for.
    pub description: &'static str,
    /// Whether the caller has to give it.
    pub required: bool,
    /// What it is, when the caller gives nothing.
    pub default: &'static str,
    /// What it may be.
    pub values: Values,
}

/// One **prompt**: a name, a command and a message.
pub struct PromptDef {
    /// The prompt's name.
    pub name: &'static str,
    /// The title a client shows.
    pub title: &'static str,
    /// What it does, in one line.
    pub description: &'static str,
    /// The command the prompt names.
    pub command: &'static str,
    /// The arguments the message takes.
    pub arguments: &'static [PromptArg],
    /// The message, with its placeholders.
    pub text: &'static str,
}

/// Every prompt, in `prompts/list` order.
///
/// **One list.** The advertisement and the dispatch both read it, so
/// they cannot disagree about which prompts exist.
pub static PROMPTS: &[PromptDef] = &[
    PromptDef {
        name: "new-document",
        title: "New document",
        description: "Make a document and put a first paragraph in it.",
        command: "prose.document.add",
        arguments: &[PromptArg {
            name: "name",
            description: "What the document is called.",
            required: false,
            default: "the document",
            values: Values::Fixed(&[]),
        }],
        text: "\
Make a document, and then put a paragraph in it.

The document is called: {name}

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

The document's id is what the answer says, and that id is what the
next command takes.",
    },
    PromptDef {
        name: "add-paragraph",
        title: "Add a paragraph",
        description: "Put a paragraph in a document.",
        command: "prose.block.paragraph",
        arguments: &[
            PromptArg {
                name: "document",
                description: "The document the paragraph goes in.",
                required: true,
                default: "",
                values: Values::Live(Live::NodeIds),
            },
            PromptArg {
                name: "text",
                description: "What the paragraph says.",
                required: false,
                default: "",
                values: Values::Fixed(&[]),
            },
        ],
        text: "\
Put a paragraph in a document.

Document: {document}
Text: {text}

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

The document is named by its id.",
    },
    PromptDef {
        name: "plan-the-document",
        title: "Plan the document",
        description: "Say what the document means.",
        command: "doc.plan",
        arguments: &[],
        text: "\
Say what the document means.

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

The answer is the plan: what every node of the document is for.",
    },
    PromptDef {
        name: "undo-the-last",
        title: "Undo the last change",
        description: "Take the last change back.",
        command: "doc.undo",
        arguments: &[],
        text: "\
Take the last change back.

Call the command `{command}` — {label}.

{label} does this: {doc}

Its parameters:
{params}

The answer says what was taken back.",
    },
];

/// Why a prompt could not be fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptError {
    /// No prompt has that name.
    NotFound(String),
    /// A required argument was left out.
    Missing {
        /// The prompt's name.
        prompt: String,
        /// The argument's name.
        argument: String,
    },
    /// The prompt names a command this server does not have.
    NoCommand(String),
}

impl PromptError {
    /// The **message** the refusal says.
    pub fn message(&self) -> String {
        match self {
            PromptError::NotFound(missing) => {
                format!("no prompt named `{missing}`; call prompts/list")
            }
            PromptError::Missing { prompt, argument } => {
                format!("the prompt `{prompt}` needs its `{argument}`")
            }
            PromptError::NoCommand(command) => {
                format!("the prompt names the command `{command}`, and this server does not have it")
            }
        }
    }

    /// The **code** the refusal is answered with.
    ///
    /// The MCP's own code for a bad request: the client asked for a
    /// prompt it could not have.
    pub fn code(&self) -> i64 {
        -32602
    }
}

/// The **prompts** this server serves, as `prompts/list` answers.
///
/// A prompt whose command is not in the registry is **not** in the
/// list: an instruction that names a command the tree does not have
/// is broken, and it is broken silently.
pub fn list_json(session: &Session) -> Json {
    Json::Array(
        PROMPTS
            .iter()
            .filter(|prompt| session.registry().all().iter().any(|spec| spec.id == prompt.command))
            .map(|prompt| {
                json!({
                    "name": prompt.name,
                    "title": prompt.title,
                    "description": prompt.description,
                    "arguments": prompt.arguments.iter().map(|argument| json!({
                        "name": argument.name,
                        "description": argument.description,
                        "required": argument.required,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// The **suggestions** of one argument, read now and filtered by what
/// the caller has typed.
///
/// The cap is applied **after** the filter, so the count the client is
/// told is the count of everything that matched.
pub fn complete(
    prompt_name: &str,
    argument_name: &str,
    typed: &str,
    session: &Session,
) -> Result<Suggestions, PromptError> {
    let Some(prompt) = PROMPTS.iter().find(|prompt| prompt.name == prompt_name) else {
        return Err(PromptError::NotFound(prompt_name.to_string()));
    };
    let Some(argument) = prompt.arguments.iter().find(|argument| argument.name == argument_name) else {
        return Err(PromptError::Missing {
            prompt: prompt.name.to_string(),
            argument: argument_name.to_string(),
        });
    };
    let matching: Vec<String> =
        argument.values.resolve(session).into_iter().filter(|value| value.starts_with(typed)).collect();
    let total = matching.len();
    Ok(Suggestions { values: matching.into_iter().take(MAX_SUGGESTIONS).collect(), total })
}

/// The **suggestions** of an argument: the values to send, and how many
/// matched before the cap.
pub struct Suggestions {
    /// The values to send, at most [`MAX_SUGGESTIONS`].
    pub values: Vec<String>,
    /// How many matched, before the cap.
    pub total: usize,
}

impl Suggestions {
    /// Whether the list was cut short.
    pub fn has_more(&self) -> bool {
        self.total > self.values.len()
    }

    /// The reply the protocol expects.
    pub fn to_json(&self) -> Json {
        json!({
            "completion": {
                "values": self.values,
                "total": self.total,
                "hasMore": self.has_more(),
            }
        })
    }
}

/// Fill a **template** in, in one pass.
///
/// A value that looks like a placeholder is **left alone**: the
/// template is read once, and what is put in it is not read again. An
/// agent reading the message would otherwise see text the caller never
/// sent, in the field of another argument.
fn substitute(template: &str, words: &[(String, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[at..]);
            return out;
        };
        let name = &after[..end];
        match words.iter().find(|(word, _)| word == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The **message** of a prompt, with its arguments substituted.
pub fn message(name: &str, arguments: &Json, session: &Session) -> Result<Json, PromptError> {
    let Some(prompt) = PROMPTS.iter().find(|prompt| prompt.name == name) else {
        return Err(PromptError::NotFound(name.to_string()));
    };
    let Some(spec) = session.registry().all().iter().find(|spec| spec.id == prompt.command) else {
        return Err(PromptError::NoCommand(prompt.command.to_string()));
    };
    // The **values**: what the caller gave, or the default.
    let mut values: Vec<(String, String)> = Vec::new();
    for argument in prompt.arguments {
        let given = arguments.get(argument.name).and_then(Json::as_str).map(|value| value.to_string());
        match given {
            Some(value) => values.push((argument.name.to_string(), value)),
            None if argument.required => {
                return Err(PromptError::Missing {
                    prompt: prompt.name.to_string(),
                    argument: argument.name.to_string(),
                });
            }
            None => values.push((argument.name.to_string(), argument.default.to_string())),
        }
    }
    // The **text**: the template, with the arguments and the registry's
    // own words in it.
    let params = spec
        .params
        .iter()
        .map(|param| {
            let kind = param.kind.json_name();
            let need = if param.required { "required" } else { "optional" };
            format!("- {} ({}, {need}): {}", param.name, kind, param.help)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut words: Vec<(String, String)> = values;
    words.push(("command".to_string(), spec.id.to_string()));
    words.push(("label".to_string(), spec.label.to_string()));
    words.push(("doc".to_string(), spec.doc.to_string()));
    words.push(("params".to_string(), params));
    let text = substitute(prompt.text, &words);
    Ok(json!({
        "description": prompt.description,
        "messages": [{
            "role": "user",
            "content": { "type": "text", "text": text },
        }],
    }))
}
