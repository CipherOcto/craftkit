//! The third door: a plugin that is a **separate program**.
//!
//! # The three doors
//!
//! - a **linked** plugin is a crate the application depends on. The linker
//!   keeps it, the registry is filled from it, and the compiler checks it.
//!   This is the door the tree v2 replaces used, and it is the best one:
//!   everything is in one address space and everything is checked.
//! - a **loaded** plugin is a program that is **started while the
//!   application runs**. It is not in the address space, so it cannot be
//!   checked by the compiler, and it needs a wire.
//! - a **scripted** plugin is text.
//!
//! This module is the second door: a **wire**, and the two halves of it.
//!
//! # The wire
//!
//! Newline-delimited JSON, in both directions. One JSON value per line, and
//! a line is a message. The protocol is three messages:
//!
//! ```text
//! plugin → host   {"kind":"catalogue","commands":[ … ]}
//! host → plugin   {"kind":"run","id":"hello.greet","params":{ … },"document":{ … }}
//! plugin → host   {"kind":"result","ok":true,"message":"…","data":null,"document":{ … }}
//! ```
//!
//! The **catalogue** is the plugin's commands: their ids, their labels,
//! their parameters. The host turns each one into a command in its own
//! registry, so a command from a subprocess is the **same thing** as a
//! command from a linked plugin: the same door, the same metadata, the same
//! refusal of an unknown parameter.
//!
//! The **document travels with the request**, and the answer may bring a
//! new one back. That is what lets a plugin in another process change the
//! document: the host hands it the document, the plugin hands back the
//! document it would have. The host's session judges the change exactly as
//! it judges a linked plugin's — by comparing before and after.
//!
//! # Why the document travels
//!
//! Because the alternative is worse: a shared document would need shared
//! memory, and shared memory between two processes that do not know each
//! other is a protocol with no rules. A document that is written down and
//! read back has one rule: it is the same shape on both sides, and the
//! shape is the file format.
//!
//! # What this does not buy
//!
//! A subprocess plugin is **not checked by the compiler**. Its catalogue is
//! data, and a catalogue with a typo in it is refused at load time — not
//! at compile time. That is the price of the door, and it is stated here
//! rather than discovered later.

use std::io::{BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::Value as Json;

use crate::command::{CommandRun, CommandSpec};
use crate::contribution::Registry;
use crate::error::{EngineError, Outcome, ParamDoc};
use crate::params::Doc;
use crate::session::Session;

/// Why a plugin could not be loaded, or refused.
#[derive(Debug)]
pub enum PluginError {
    /// The program could not be started.
    Spawn(std::io::Error),
    /// The wire broke.
    Io(std::io::Error),
    /// The plugin said something the host does not understand.
    Protocol(String),
    /// A catalogue entry is wrong.
    BadCommand(String),
    /// The plugin's answer was not an answer.
    BadResult(String),
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        self.stop();
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::Spawn(error) => write!(formatter, "the plugin did not start: {error}"),
            PluginError::Io(error) => write!(formatter, "the plugin's wire broke: {error}"),
            PluginError::Protocol(message) => write!(formatter, "the plugin said something odd: {message}"),
            PluginError::BadCommand(message) => {
                write!(formatter, "the plugin's catalogue is wrong: {message}")
            }
            PluginError::BadResult(message) => {
                write!(formatter, "the plugin's answer was not an answer: {message}")
            }
        }
    }
}

impl std::error::Error for PluginError {}

/// One parameter of a plugin's command, as it travels.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginParam {
    /// The parameter's name.
    pub name: String,
    /// What kind of value it takes.
    pub kind: Doc,
    /// Whether the caller has to give it.
    pub required: bool,
    /// One line about it.
    pub help: String,
    /// The other spelling of this parameter, if it has one.
    pub alternative: Option<String>,
    /// The kind's **own name**, when it has one.
    ///
    /// It travels over the door because the door is where a plugin's
    /// own kinds have to survive: a plugin that is not part of the
    /// program still has to be able to say that its parameter is a
    /// length.
    pub type_name: Option<String>,
    /// The **unit** the value is in, when the kind has one.
    pub unit: Option<String>,
}

impl PluginParam {
    /// To and from the wire.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "name": self.name,
            "kind": self.kind.json_name(),
            "required": self.required,
            "help": self.help,
            "alternative": self.alternative,
            "type_name": self.type_name,
            "unit": self.unit,
        })
    }

    /// Read one from the wire.
    pub fn from_json(value: &Json) -> Result<Self, PluginError> {
        let name = value
            .get("name")
            .and_then(|name| name.as_str())
            .ok_or_else(|| PluginError::BadCommand("a parameter has no name".to_string()))?
            .to_string();
        let kind = value
            .get("kind")
            .and_then(|kind| kind.as_str())
            .and_then(Doc::from_json_name)
            .ok_or_else(|| PluginError::BadCommand(format!("the parameter `{name}` has no kind")))?;
        let required = value.get("required").and_then(|flag| flag.as_bool()).unwrap_or(false);
        let help = value.get("help").and_then(|help| help.as_str()).unwrap_or_default().to_string();
        let alternative = value
            .get("alternative")
            .and_then(|alternative| alternative.as_str())
            .map(|alternative| alternative.to_string());
        let type_name = value.get("type_name").and_then(|name| name.as_str()).map(|name| name.to_string());
        let unit = value.get("unit").and_then(|unit| unit.as_str()).map(|unit| unit.to_string());
        Ok(PluginParam { name, kind, required, help, alternative, type_name, unit })
    }
}

/// One command of a plugin's catalogue.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginCommand {
    /// The command's id: `<area>.<thing>`.
    pub id: String,
    /// What the button says.
    pub label: String,
    /// Which area it belongs to.
    pub area: String,
    /// What the command is for.
    pub capability: String,
    /// The layer it belongs to.
    pub layer: String,
    /// The icon's name.
    pub icon: String,
    /// One line about what it does.
    pub doc: String,
    /// What it takes.
    pub params: Vec<PluginParam>,
    /// Whether it is supposed to change the document.
    pub mutates: bool,
}

impl PluginCommand {
    /// To the wire.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "area": self.area,
            "capability": self.capability,
            "layer": self.layer,
            "icon": self.icon,
            "doc": self.doc,
            "params": self.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
            "mutates": self.mutates,
        })
    }

    /// Read one from the wire.
    pub fn from_json(value: &Json) -> Result<Self, PluginError> {
        let text =
            |name: &str| value.get(name).and_then(|value| value.as_str()).unwrap_or_default().to_string();
        let id = value
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| PluginError::BadCommand("a command has no id".to_string()))?
            .to_string();
        if crate::contribution::malformed_dotted_id(&id).is_some() {
            return Err(PluginError::BadCommand(format!("`{id}` is not a command id")));
        }
        let params = match value.get("params") {
            Some(params) => params
                .as_array()
                .ok_or_else(|| PluginError::BadCommand(format!("`{id}`'s parameters are not a list")))?
                .iter()
                .map(PluginParam::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        Ok(PluginCommand {
            id,
            label: text("label"),
            area: text("area"),
            capability: text("capability"),
            layer: text("layer"),
            icon: text("icon"),
            doc: text("doc"),
            params,
            mutates: value.get("mutates").and_then(|flag| flag.as_bool()).unwrap_or(false),
        })
    }
}

/// The catalogue: a plugin's commands.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Catalogue {
    /// The commands.
    pub commands: Vec<PluginCommand>,
}

impl Catalogue {
    /// An empty catalogue.
    pub fn new() -> Self {
        Catalogue { commands: Vec::new() }
    }

    /// To the wire.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "kind": "catalogue",
            "commands": self.commands.iter().map(|command| command.to_json()).collect::<Vec<_>>(),
        })
    }

    /// Read one from the wire.
    pub fn from_json(value: &Json) -> Result<Self, PluginError> {
        let commands = match value.get("commands") {
            Some(commands) => commands
                .as_array()
                .ok_or_else(|| PluginError::BadCommand("the commands are not a list".to_string()))?
                .iter()
                .map(PluginCommand::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        Ok(Catalogue { commands })
    }
}

/// The wire: something the host can send to and hear from a plugin.
///
/// A trait and not a concrete process, so that the protocol can be tested
/// without starting one: the test drives it with two buffers.
pub trait Transport: Send {
    /// Send one line.
    fn send(&mut self, line: &str) -> Result<(), PluginError>;
    /// Hear one line.
    ///
    /// `None` means the other end has gone.
    fn receive(&mut self) -> Result<Option<String>, PluginError>;
}

/// A transport over a child process's pipes.
struct ChildTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// The most bytes a line of the plugin's wire may hold.
///
/// A command is a JSON object and an answer is a JSON object, and a
/// megabyte is more than either needs.
const MAX_LINE_BYTES: crate::lines::LineLimit = 1024 * 1024;

impl Transport for ChildTransport {
    fn send(&mut self, line: &str) -> Result<(), PluginError> {
        writeln!(self.stdin, "{line}").map_err(PluginError::Io)?;
        self.stdin.flush().map_err(PluginError::Io)
    }

    fn receive(&mut self) -> Result<Option<String>, PluginError> {
        // The line is read **within a limit**: a plugin that never ends
        // its line must not be able to make the host allocate.
        crate::lines::read_line_bounded(&mut self.stdout, MAX_LINE_BYTES).map_err(PluginError::Protocol)
    }
}

/// A loaded plugin: a program, and its catalogue.
///
/// The wire is behind a lock and behind an `Arc`, because the commands
/// that are made from the catalogue have to be able to use it, and a
/// command is a value that may be cloned.
pub struct LoadedPlugin {
    transport: std::sync::Arc<std::sync::Mutex<Box<dyn Transport>>>,
    child: Option<Child>,
    catalogue: Catalogue,
}

impl LoadedPlugin {
    /// Start a plugin and read its catalogue.
    ///
    /// The plugin says hello with its catalogue, and the host reads it. A
    /// plugin that does not say hello, or says something the host cannot
    /// read, is refused here — before any of its commands are registered.
    pub fn spawn(program: &str, args: &[&str]) -> Result<Self, PluginError> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(PluginError::Spawn)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PluginError::Protocol("the plugin has no way to be spoken to".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PluginError::Protocol("the plugin has no way to speak".to_string()))?;
        let mut transport: Box<dyn Transport> =
            Box::new(ChildTransport { stdin, stdout: BufReader::new(stdout) });
        let catalogue = read_catalogue(transport.as_mut())?;
        Ok(LoadedPlugin {
            transport: std::sync::Arc::new(std::sync::Mutex::new(transport)),
            child: Some(child),
            catalogue,
        })
    }

    /// Start a plugin over a transport the caller made.
    ///
    /// This is the door the test uses: the protocol is exercised without a
    /// process, so the test is fast and does not depend on a binary.
    pub fn over(transport: Box<dyn Transport>) -> Result<Self, PluginError> {
        let mut transport = transport;
        let catalogue = read_catalogue(transport.as_mut())?;
        Ok(LoadedPlugin {
            transport: std::sync::Arc::new(std::sync::Mutex::new(transport)),
            child: None,
            catalogue,
        })
    }

    /// The plugin's commands.
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    /// Stop the plugin.
    ///
    /// A plugin is a program, and a program that is not wanted any more
    /// should not be left running.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// The plugin's command ids.
    pub fn ids(&self) -> Vec<&str> {
        self.catalogue.commands.iter().map(|command| command.id.as_str()).collect()
    }

    /// Register the plugin's commands in a registry.
    ///
    /// Each one becomes a command with the **same metadata** a linked
    /// plugin's command has, and a body that runs in the other process.
    ///
    /// The metadata is **leaked** once, here: a command's labels are
    /// `&'static str`, and a subprocess plugin's labels are strings that
    /// arrive at run time. The leak is bounded — one per command of one
    /// plugin — and it is the price of not making every command's metadata
    /// owned.
    ///
    /// # The command the host already has
    ///
    /// A plugin program is a **whole craftkit program**, so its catalogue
    /// carries the core's own commands as well as its own. The host has
    /// those commands too — they are the same commands — so a catalogue
    /// command the host already has is **skipped**: the host keeps its
    /// own, and the plugin's copy is not a second one.
    ///
    /// A catalogue command whose id the host has and whose **capability**
    /// is different is refused: that is two different commands with one
    /// id, and the registry is right to refuse it.
    pub fn register(&mut self, registry: &mut Registry<CommandSpec>) -> Result<(), PluginError> {
        for command in self.catalogue.commands.clone() {
            let spec = self.as_spec(&command)?;
            if let Some(mine) = registry.get(spec.id) {
                if mine.capability == spec.capability {
                    continue;
                }
                return Err(PluginError::BadCommand(format!(
                    "`{}` is a command here already, and not the same one",
                    spec.id
                )));
            }
            registry.register(spec).map_err(|error| PluginError::BadCommand(error.to_string()))?;
        }
        Ok(())
    }

    /// A plugin's command, as a command in this process.
    fn as_spec(&self, command: &PluginCommand) -> Result<CommandSpec, PluginError> {
        let params: Vec<ParamDoc> = command
            .params
            .iter()
            .map(|param| ParamDoc {
                name: leak(param.name.clone()),
                kind: param.kind,
                required: param.required,
                help: leak(param.help.clone()),
                alternative: param.alternative.as_ref().map(|name| leak(name.clone())),
                type_name: param.type_name.as_ref().map(|name| leak(name.clone())),
                unit: param.unit.as_ref().map(|unit| leak(unit.clone())),
            })
            .collect();
        let params: &'static [ParamDoc] = Box::leak(params.into_boxed_slice());
        let id = leak(command.id.clone());
        let label = leak(command.label.clone());
        let area = leak(command.area.clone());
        let capability = leak(command.capability.clone());
        let layer = leak(command.layer.clone());
        let icon = leak(command.icon.clone());
        let doc = leak(command.doc.clone());
        let mutates = command.mutates;
        // The transport is shared: the closure is `Send + Sync`, and the
        // wire is one line at a time.
        let transport = self.transport.clone();
        let run_id = id.to_string();
        let run = CommandRun::Dynamic(std::sync::Arc::new(move |session, params| {
            let request = serde_json::json!({
                "kind": "run",
                "id": run_id,
                "params": params.value().clone(),
                "document": session.document().to_json(),
            });
            let mut guard = transport.lock().map_err(|_| EngineError::Internal {
                id: run_id.clone(),
                message: "the plugin's wire is poisoned".to_string(),
            })?;
            guard
                .send(&request.to_string())
                .map_err(|error| EngineError::Internal { id: run_id.clone(), message: error.to_string() })?;
            let Some(line) = guard
                .receive()
                .map_err(|error| EngineError::Internal { id: run_id.clone(), message: error.to_string() })?
            else {
                return Err(EngineError::Internal {
                    id: run_id.clone(),
                    message: "the plugin went away".to_string(),
                });
            };
            let answer: Json = serde_json::from_str(&line).map_err(|error| EngineError::Internal {
                id: run_id.clone(),
                message: format!("the plugin's answer was not JSON: {error}"),
            })?;
            let ok = answer.get("ok").and_then(|ok| ok.as_bool()).unwrap_or(false);
            if !ok {
                let message = answer
                    .get("error")
                    .and_then(|error| error.as_str())
                    .unwrap_or("the plugin refused the call")
                    .to_string();
                return Err(EngineError::Failed { id: run_id.clone(), message });
            }
            // The document the plugin would have. The host's session
            // judges the change by comparing before and after, exactly as
            // it judges a linked plugin's.
            if let Some(document) = answer.get("document") {
                let document = crate::document::Document::from_json(document).map_err(|error| {
                    EngineError::Internal {
                        id: run_id.clone(),
                        message: format!("the plugin sent a document nobody can read: {error}"),
                    }
                })?;
                *session.document_mut()? = document;
            }
            let message = answer
                .get("message")
                .and_then(|message| message.as_str())
                .unwrap_or("the plugin did it")
                .to_string();
            Ok(Outcome::new(message))
        }));
        Ok(CommandSpec {
            id,
            label,
            area,
            capability,
            layer,
            icon,
            doc,
            shortcut: None,
            // The menu path of a plugin's command is its area: a
            // subprocess plugin's catalogue says what area a command is
            // in, and the path is leaked with the rest of its metadata.
            menu: Box::leak(Box::new([area])),
            params,
            mutates,
            undoable: mutates,
            journal: true,
            enabled: crate::always,
            run,
        })
    }
}

/// Read a catalogue off a transport.
fn read_catalogue(transport: &mut dyn Transport) -> Result<Catalogue, PluginError> {
    let Some(line) = transport.receive()? else {
        return Err(PluginError::Protocol("the plugin said nothing at all".to_string()));
    };
    let value: Json = serde_json::from_str(&line)
        .map_err(|error| PluginError::Protocol(format!("the plugin's first line was not JSON: {error}")))?;
    if value.get("kind").and_then(|kind| kind.as_str()) != Some("catalogue") {
        return Err(PluginError::Protocol("the plugin's first line was not a catalogue".to_string()));
    }
    Catalogue::from_json(&value)
}

/// Keep a string for the life of the program.
///
/// The one leak in this tree, and it is here on purpose: a command's
/// metadata is `&'static str`, and a subprocess plugin's metadata arrives
/// at run time. The leak is bounded by the number of commands the plugin
/// has, and it happens once, when the plugin is loaded.
pub fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// The plugin's side of the wire.
///
/// A plugin program is a **full craftkit program**: it has its own
/// ecosystem, its own commands, its own session. It serves them over the
/// wire, and the document travels with each request.
pub fn serve(ecosystem: &crate::ecosystem::Ecosystem) -> Result<(), PluginError> {
    // The catalogue first: the host reads it before anything else.
    let catalogue = Catalogue {
        commands: ecosystem
            .commands
            .all()
            .iter()
            .map(|command| PluginCommand {
                id: command.id.to_string(),
                label: command.label.to_string(),
                area: command.area.to_string(),
                capability: command.capability.to_string(),
                layer: command.layer.to_string(),
                icon: command.icon.to_string(),
                doc: command.doc.to_string(),
                params: command
                    .params
                    .iter()
                    .map(|param| PluginParam {
                        name: param.name.to_string(),
                        kind: param.kind,
                        required: param.required,
                        help: param.help.to_string(),
                        alternative: param.alternative.map(|name| name.to_string()),
                        type_name: param.type_name.map(|name| name.to_string()),
                        unit: param.unit.map(|unit| unit.to_string()),
                    })
                    .collect(),
                mutates: command.mutates,
            })
            .collect(),
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "{}", catalogue.to_json()).map_err(PluginError::Io)?;
    out.flush().map_err(PluginError::Io)?;

    let stdin = std::io::stdin();
    let lines = crate::lines::lines_bounded(stdin.lock(), MAX_LINE_BYTES);
    for line in lines {
        let line = line.map_err(PluginError::Protocol)?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Json = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(_) => continue,
        };
        if request.get("kind").and_then(|kind| kind.as_str()) != Some("run") {
            continue;
        }
        let answer = answer(ecosystem, &request);
        writeln!(out, "{answer}").map_err(PluginError::Io)?;
        out.flush().map_err(PluginError::Io)?;
    }
    Ok(())
}

/// Answer one request.
fn answer(ecosystem: &crate::ecosystem::Ecosystem, request: &Json) -> Json {
    let id = request.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string();
    let params = request.get("params").cloned().unwrap_or(Json::Null);
    let document = request.get("document").cloned().unwrap_or(Json::Null);
    let mut session = ecosystem.session();
    if let Ok(document) = crate::document::Document::from_json(&document) {
        session.set_document(document);
    }
    match session.execute(&id, &params) {
        Ok(outcome) => serde_json::json!({
            "kind": "result",
            "ok": true,
            "message": outcome.message,
            "data": outcome.data,
            "document": session.document().to_json(),
        }),
        Err(error) => serde_json::json!({
            "kind": "result",
            "ok": false,
            "error": error.to_string(),
        }),
    }
}

/// A transport over two buffers: what the test drives the protocol with.
///
/// The **inbox** is what the plugin says, and the **outbox** is what the
/// host said. Both are read in order, and the outbox is kept so that the
/// test can look at it.
pub struct MemoryTransport {
    /// What the plugin will say, one line at a time.
    inbox: Vec<String>,
    /// What the host has said.
    pub outbox: Vec<String>,
}

impl MemoryTransport {
    /// A transport that will say these lines, in order.
    pub fn saying(lines: Vec<String>) -> Self {
        MemoryTransport { inbox: lines, outbox: Vec::new() }
    }
}

impl Transport for MemoryTransport {
    fn send(&mut self, line: &str) -> Result<(), PluginError> {
        self.outbox.push(line.to_string());
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<String>, PluginError> {
        if self.inbox.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.inbox.remove(0)))
    }
}

/// A session that has a plugin's commands in it.
///
/// This is what the host does with a loaded plugin: the plugin's commands
/// are in the same registry as the linked ones, and the session runs them
/// the same way.
pub fn session_with(
    ecosystem: &crate::ecosystem::Ecosystem,
    plugin: &mut LoadedPlugin,
) -> Result<Session, PluginError> {
    let mut commands = ecosystem.commands.clone();
    plugin.register(&mut commands)?;
    Ok(Session::with_formats(
        commands,
        ecosystem.kinds.clone(),
        ecosystem.formats.clone(),
        ecosystem.migrations.clone(),
        ecosystem.evaluators.clone(),
    ))
}
