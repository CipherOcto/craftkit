//! `craftkit-mcp` — the **MCP door**: the command registry, spoken to
//! an agent.
//!
//! # What makes this different
//!
//! The other applications in the family put a **façade** in front of
//! their registries: a handful of tools that take a command id and a bag
//! of parameters. Seventeen to twenty-eight tools over three hundred and
//! sixty-three to six hundred and thirty-two commands, and an agent has
//! to learn the façade *and* the ids.
//!
//! This server has **one tool per command**, and the tool list **is** the
//! command list. There is no second vocabulary, and there is nothing to
//! learn twice:
//!
//! - the tool's name is the command's id, with the dots turned into
//!   underscores and a prefix: `doc.open` is `craftkit_doc_open`;
//! - the tool's schema is built from the command's **typed** parameters,
//!   so the agent is told what each one takes and whether it is
//!   required;
//! - the tool's description is the command's own documentation;
//! - calling the tool runs the command, in the session, and returns what
//!   it said.
//!
//! So the registry **is** the protocol. A command that is added is a tool
//! that is added, with nothing else to write.
//!
//! # The wire
//!
//! JSON-RPC 2.0 over **newline-delimited stdio**, which is what the
//! family's servers speak, and what an MCP client expects. A request is
//! one JSON object on one line; the answer is one JSON object on one
//! line. A **notification** has no id and gets no answer.
//!
//! The methods are the ones a client uses: `initialize`, `ping`,
//! `tools/list`, `tools/call`, and the `notifications/initialized`
//! notification.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Doc, EngineError, Outcome, Session};

/// The protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The protocol versions this server understands.
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// The prefix every tool's name carries.
///
/// The prefix is what keeps a tool from colliding with a tool of another
/// server in the same session, and it says what the tool is: a
/// craftkit command.
pub const TOOL_PREFIX: &str = "craftkit_";

/// The name of the tool for a command id.
///
/// The dots become underscores, because a tool name is an identifier and
/// a dot is not one.
pub fn tool_name(id: &str) -> String {
    format!("{TOOL_PREFIX}{}", id.replace('.', "_"))
}

/// The command id a tool name stands for.
pub fn command_id(name: &str) -> Option<String> {
    let rest = name.strip_prefix(TOOL_PREFIX)?;
    Some(rest.replace('_', "."))
}

/// One tool: a command, as an agent sees it.
#[derive(Debug, Clone)]
pub struct Tool {
    /// The tool's name.
    pub name: String,
    /// The command it runs.
    pub command: String,
    /// What it does, in words.
    pub description: String,
    /// Its parameters, as a JSON schema.
    pub schema: Json,
}

impl Tool {
    /// The tool, as the protocol wants it.
    pub fn to_json(&self) -> Json {
        json!({
            "name": self.name,
            "title": self.command,
            "description": self.description,
            "inputSchema": self.schema,
            "annotations": { "title": self.command, "readOnlyHint": false, "openWorldHint": false },
        })
    }
}

/// The **server**: a session, and the doors into it.
///
/// It owns the session, because a command is a thing that runs **in** a
/// session and changes what the session holds.
pub struct Server {
    /// The session the commands run in.
    session: Session,
    /// The tools: one per command.
    tools: Vec<Tool>,
}

impl Server {
    /// A server over a session.
    ///
    /// The tools are made from the session's registry, so a command that
    /// is in the registry is a tool, and a command that is not is not.
    pub fn new(session: Session) -> Server {
        let mut tools = Vec::new();
        for spec in session.registry().all() {
            tools.push(Tool {
                name: tool_name(spec.id),
                command: spec.id.to_string(),
                description: describe(spec.label, spec.doc),
                schema: schema_of(spec.params),
            });
        }
        Server { session, tools }
    }

    /// The tools, one per command.
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// The session the commands run in.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The session, to change.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Answer one request.
    ///
    /// A **notification** gets no answer, and that is what `None` means.
    pub fn handle(&mut self, request: &Json) -> Option<Json> {
        let id = request.get("id").cloned();
        let Some(method) = request.get("method").and_then(|method| method.as_str()) else {
            // Not a request: it is an answer to something of ours, and
            // there is nothing to do with it.
            return None;
        };
        let params = request.get("params").cloned().unwrap_or(Json::Null);
        let outcome = self.request(method, &params);
        let id = id?;
        Some(match outcome {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
            }
        })
    }

    /// Answer one line of the wire.
    ///
    /// A line that is not JSON is an error, and the error is the answer:
    /// the client sent something that is not a request, and it is told
    /// so.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let request: Json = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(error) => {
                let answer = json!({
                    "jsonrpc": "2.0",
                    "id": Json::Null,
                    "error": { "code": -32700, "message": format!("that is not JSON: {error}") },
                });
                return Some(answer.to_string());
            }
        };
        self.handle(&request).map(|answer| answer.to_string())
    }

    /// The methods, and what they answer.
    fn request(&mut self, method: &str, params: &Json) -> Result<Json, (i64, String)> {
        match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(|version| version.as_str());
                let version = match asked {
                    Some(asked) if SUPPORTED_VERSIONS.contains(&asked) => asked,
                    _ => PROTOCOL_VERSION,
                };
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "craftkit",
                        "title": "craftkit",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "instructions": format!(
                        "Every command in the registry is a tool of its own, named after the command: \
                         `doc.open` is `{}`. The tool's schema is the command's typed parameters. \
                         There are {} commands, and there is no other vocabulary.",
                        tool_name("doc.open"),
                        self.tools.len()
                    ),
                }))
            }
            "ping" => Ok(json!({})),
            "notifications/initialized" | "notifications/cancelled" => Ok(json!({})),
            "tools/list" => {
                Ok(json!({ "tools": self.tools.iter().map(|tool| tool.to_json()).collect::<Vec<_>>() }))
            }
            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(|name| name.as_str())
                    .ok_or((-32602, "the tool has no name".to_string()))?;
                let arguments = params.get("arguments").cloned().unwrap_or(Json::Null);
                Ok(self.call(name, &arguments).to_json())
            }
            other => Err((-32601, format!("there is no method `{other}`"))),
        }
    }

    /// Run a command, and say what it said.
    fn call(&mut self, name: &str, arguments: &Json) -> ToolResult {
        let Some(command) = command_id(name) else {
            return ToolResult::error(format!("`{name}` is not a craftkit command"));
        };
        if !self.tools.iter().any(|tool| tool.name == name) {
            return ToolResult::error(format!("there is no command `{command}`"));
        }
        match self.session.execute(&command, arguments) {
            Ok(outcome) => ToolResult::of(outcome),
            Err(error) => ToolResult::error(describe_error(&error)),
        }
    }
}

/// What a tool call answered.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolResult {
    /// The content blocks.
    pub content: Vec<Json>,
    /// Whether it is an error.
    pub is_error: bool,
}

impl ToolResult {
    /// The result of a command that ran.
    fn of(outcome: Outcome) -> ToolResult {
        let mut content = vec![json!({ "type": "text", "text": outcome.message })];
        if !outcome.data.is_null() {
            let written = serde_json::to_string_pretty(&outcome.data).unwrap_or_default();
            content.push(json!({ "type": "text", "text": written }));
        }
        ToolResult { content, is_error: false }
    }

    /// A result that says something went wrong.
    fn error(message: impl Into<String>) -> ToolResult {
        ToolResult { content: vec![json!({ "type": "text", "text": message.into() })], is_error: true }
    }

    /// The result, as the protocol wants it.
    pub fn to_json(&self) -> Json {
        json!({ "content": self.content, "isError": self.is_error })
    }
}

/// What a command said, in the words of the protocol.
fn describe_error(error: &EngineError) -> String {
    match error {
        EngineError::Unknown(id) => format!("there is no command `{id}`"),
        EngineError::Disabled { id, reason } => format!("`{id}` is not available: {reason}"),
        other => other.to_string(),
    }
}

/// A command's description: its label and its own words.
fn describe(label: &str, doc: &str) -> String {
    let doc = doc.trim();
    if doc.is_empty() {
        return label.to_string();
    }
    format!("{label}: {doc}")
}

/// A command's parameters, as a JSON schema.
///
/// The types come from the command's own declaration, so the schema
/// cannot disagree with the code that reads the values.
fn schema_of(params: &[craftkit_sdk::ParamDoc]) -> Json {
    let mut properties = serde_json::Map::new();
    let mut required: Vec<Json> = Vec::new();
    for param in params {
        let mut schema = match param.kind {
            Doc::Number => json!({ "type": "number" }),
            Doc::Text => json!({ "type": "string" }),
            Doc::Bool => json!({ "type": "boolean" }),
            Doc::NodeRef => json!({ "type": "integer", "minimum": 0 }),
            Doc::NodeKind => json!({ "type": "string" }),
            Doc::Json => json!({}),
        };
        let description = param.help.trim();
        if !description.is_empty() {
            schema["description"] = Json::String(description.to_string());
        }
        properties.insert(param.name.to_string(), schema);
        if let Some(alternative) = param.alternative {
            properties
                .insert(alternative.to_string(), json!({ "type": "string", "description": description }));
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
