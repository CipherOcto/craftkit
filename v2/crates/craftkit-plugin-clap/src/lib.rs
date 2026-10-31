//! `craftkit-plugin-clap` — a **CLAP plugin** in the document.
//!
//! # What is ported
//!
//! The family has plugins in its mixer: an insert holds a plugin by name,
//! a set of parameter values, and — since the port of the mixer — the
//! plugin's own **state**, so that a project remembers what a plugin was
//! doing when it was closed.
//!
//! - a **plugin** — a node whose payload holds the plugin's identity and
//!   its state;
//! - the **state** — the plugin's own bytes, carried whole and read by
//!   nobody but the plugin;
//! - the **parameters** — what the plugin says it holds, and their
//!   values.
//!
//! # The three things the port found
//!
//! - **A plugin is a program, and the document carries its state and not
//!   the program.** The library is a file on disk, like a picture: the
//!   document names it and the bytes stay where they are. So the node
//!   says which plugin it is and where the library is, and the **state**
//!   travels with it.
//! - **The state is opaque, and that is what makes it carryable.** The
//!   host never looks inside it: the plugin writes it and the plugin
//!   reads it. The document cannot interpret it either, so it travels as
//!   **base64 in the payload** — the same answer the PDF battery gives
//!   for a stream.
//! - **The instance is not the document.** A CLAP instance holds raw
//!   addresses into a library that is open, and it cannot be sent
//!   between threads, and it is destroyed when the host is finished with
//!   it. So no instance is ever kept in a session: a command **opens the
//!   plugin, hands it the state, does its work, and closes it**. The
//!   document is left with what the plugin said.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Canvas, Document, EngineError, Node, NodeRef, Number, Outcome, Point, Session, Text};

use craftkit_clap::{ClapBlock, ClapDescriptor, ClapParameter, ClapPlugin};

/// The layer of the CLAP plugins.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "clap", name = "CLAP", requires("core"))]
pub struct ClapLayer;

/// A **plugin**: an identity and a state.
///
/// The payload holds the plugin's name, the library it came from, the
/// parameters it says it has, and its state as base64. Nothing else: the
/// instance that made the state is gone, and the bytes are the plugin's
/// own.
#[derive(NodeKind)]
#[node_kind(kind = "clap.plugin", label = "Plugin", layer = "clap", evaluates)]
pub struct PluginKind;

impl PluginKind {
    /// Not drawn.
    ///
    /// A plugin is a program, and the kit cannot draw a program: what it
    /// can do is say what the plugin is and what it holds.
    fn paint(_node: &Node, _canvas: &mut dyn Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: Point) -> bool {
        false
    }

    /// What a plugin **means**: who it is, and what it holds.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let payload = PluginPayload::from_json(&node.data).ok()?;
        Some(json!({
            "id": payload.id,
            "name": payload.name,
            "vendor": payload.vendor,
            "version": payload.version,
            "library": payload.library,
            "params": payload.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
            "state_bytes": payload.state.len(),
        }))
    }
}

/// One **parameter** of a plugin, as the document carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterPayload {
    /// The parameter's id: the name a host asks by.
    pub id: u32,
    /// Its name, for a person.
    pub name: String,
    /// What it is set to.
    pub value: f64,
    /// The smallest value it takes.
    pub min_value: f64,
    /// The largest value it takes.
    pub max_value: f64,
    /// The value it starts at.
    pub default_value: f64,
}

impl ParameterPayload {
    /// The parameter as it is written.
    pub fn to_json(&self) -> Json {
        json!({
            "id": self.id,
            "name": self.name,
            "value": self.value,
            "min_value": self.min_value,
            "max_value": self.max_value,
            "default_value": self.default_value,
        })
    }

    /// The parameter as it is read.
    pub fn from_json(value: &Json) -> ParameterPayload {
        ParameterPayload {
            id: value.get("id").and_then(|id| id.as_u64()).unwrap_or(0) as u32,
            name: value.get("name").and_then(|name| name.as_str()).unwrap_or("").to_string(),
            value: value.get("value").and_then(|value| value.as_f64()).unwrap_or(0.0),
            min_value: value.get("min_value").and_then(|value| value.as_f64()).unwrap_or(0.0),
            max_value: value.get("max_value").and_then(|value| value.as_f64()).unwrap_or(0.0),
            default_value: value.get("default_value").and_then(|value| value.as_f64()).unwrap_or(0.0),
        }
    }
}

/// What a **plugin node** carries.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginPayload {
    /// Where the library is.
    pub library: String,
    /// The plugin's id: the name a host asks for by.
    pub id: String,
    /// Its name, for a person.
    pub name: String,
    /// Who made it.
    pub vendor: String,
    /// Its own version.
    pub version: String,
    /// The parameters it holds.
    pub params: Vec<ParameterPayload>,
    /// Its state, as base64: the plugin's own bytes.
    pub state: String,
}

impl PluginPayload {
    /// The plugin as it is written.
    pub fn to_json(&self) -> Json {
        json!({
            "library": self.library,
            "id": self.id,
            "name": self.name,
            "vendor": self.vendor,
            "version": self.version,
            "params": self.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
            "state": self.state,
        })
    }

    /// The plugin as it is read.
    pub fn from_json(value: &Json) -> Result<PluginPayload, String> {
        let library = value.get("library").and_then(|library| library.as_str()).unwrap_or("").to_string();
        if library.is_empty() {
            return Err("a plugin says which library it is in".to_string());
        }
        let id = value.get("id").and_then(|id| id.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            return Err("a plugin says which plugin it is".to_string());
        }
        let params = value
            .get("params")
            .and_then(|params| params.as_array())
            .map(|params| params.iter().map(ParameterPayload::from_json).collect())
            .unwrap_or_default();
        Ok(PluginPayload {
            library,
            id,
            name: value.get("name").and_then(|name| name.as_str()).unwrap_or("").to_string(),
            vendor: value.get("vendor").and_then(|vendor| vendor.as_str()).unwrap_or("").to_string(),
            version: value.get("version").and_then(|version| version.as_str()).unwrap_or("").to_string(),
            params,
            state: value.get("state").and_then(|state| state.as_str()).unwrap_or("").to_string(),
        })
    }
}

/// The **plugins** in a library.
///
/// The library is opened, the factory is asked what it holds, and the
/// library is closed again: nothing here is kept.
fn plugins_of(path: &str) -> Result<Vec<ClapDescriptor>, EngineError> {
    let plugin = ClapPlugin::open(path).map_err(|error| fail("clap.list", error.to_string()))?;
    plugin.init().map_err(|error| fail("clap.list", error.to_string()))?;
    let factory = plugin.factory().map_err(|error| fail("clap.list", error.to_string()))?;
    let mut found = Vec::new();
    for index in 0..factory.count() {
        let descriptor = factory.descriptor(index).map_err(|error| fail("clap.list", error.to_string()))?;
        found.push(descriptor);
    }
    Ok(found)
}

/// What a plugin said when it was asked.
struct Report {
    /// Who it is.
    descriptor: ClapDescriptor,
    /// The parameters it holds.
    parameters: Vec<ClapParameter>,
    /// Its state, as bytes.
    state: Vec<u8>,
}

/// **Open** the plugin, hand it the state it had, and ask it what it holds.
///
/// The instance lives for the length of this function and not one moment
/// longer: it holds addresses into a library that is closed when this
/// returns, and it cannot be sent between threads, and the document has
/// no place for it. So the plugin is opened, given its state, asked, and
/// closed — and what it said is what the document keeps.
fn talk(id: &str, path: &str, state: Option<&[u8]>, set: Option<(u32, f64)>) -> Result<Report, EngineError> {
    let plugin = ClapPlugin::open(path).map_err(|error| fail("clap", error.to_string()))?;
    plugin.init().map_err(|error| fail("clap", error.to_string()))?;
    let factory = plugin.factory().map_err(|error| fail("clap", error.to_string()))?;
    let instance = factory.create(id).map_err(|error| fail("clap", error.to_string()))?;
    instance.init().map_err(|error| fail("clap", error.to_string()))?;
    instance.activate(SAMPLE_RATE, 1, MAX_FRAMES).map_err(|error| fail("clap", error.to_string()))?;
    instance.start_processing().map_err(|error| fail("clap", error.to_string()))?;

    // The plugin is given the state it had, so that what it says now is
    // what it would say if it had never been closed.
    if let Some(bytes) = state {
        instance.load_state(bytes).map_err(|error| fail("clap", error.to_string()))?;
    }
    // A parameter is set by **offering an event**, and the plugin reads
    // the event while it works — so it has to work once for the value to
    // arrive.
    if let Some((parameter, value)) = set {
        instance.set_parameter(parameter, value);
        let mut block = ClapBlock::new(1, 1);
        instance.process(1, &mut block).map_err(|error| fail("clap", error.to_string()))?;
    }

    let descriptor = instance.descriptor().map_err(|error| fail("clap", error.to_string()))?;
    let parameters = instance.parameters().map_err(|error| fail("clap", error.to_string()))?;
    let saved = instance.save_state().map_err(|error| fail("clap", error.to_string()))?;

    // The instance is finished with: it is stopped, deactivated and
    // destroyed, and only then is the library closed.
    instance.stop_processing().map_err(|error| fail("clap", error.to_string()))?;
    instance.deactivate().map_err(|error| fail("clap", error.to_string()))?;
    instance.destroy();

    Ok(Report { descriptor, parameters, state: saved })
}

/// An error, in the kit's own shape.
fn fail(id: &str, message: String) -> EngineError {
    EngineError::Failed { id: id.to_string(), message }
}

/// The rate a plugin is activated at.
///
/// The kit has no audio thread and no sample rate of its own, so the
/// rate is this program's, and it is the one every CLAP plugin accepts.
const SAMPLE_RATE: f64 = 48_000.0;

/// The most frames in the block that carries a parameter to a plugin.
///
/// One frame is enough: the block is not here for the audio, it is here
/// so that the plugin reads the event the host offered.
const MAX_FRAMES: u32 = 512;

/// The most bytes of **state** a node may carry.
///
/// The state is the plugin's own, and a plugin may have a lot of it. The
/// limit is this program's, and it is generous: a state that is bigger
/// than this is not a state a document should carry.
const MAX_STATE_BYTES: usize = 16 * 1024 * 1024;

/// The **plugins** in a library.
///
/// A measurement: it changes nothing, so it is not an undo step.
#[derive(Command)]
#[command(id = "clap.list", area = "clap", layer = "clap", icon = "list", menu("CLAP"), no_journal)]
pub struct ListPlugins {
    /// The shared library.
    pub path: Text,
}

impl ListPlugins {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(fail("clap.list", "a library has a path".to_string()));
        }
        let plugins = plugins_of(&path)?;
        let count = plugins.len();
        Ok(Outcome::new(format!("{count} plugin(s) in `{path}`")).with_data(json!({
            "path": path,
            "plugins": plugins
                .iter()
                .map(|descriptor| json!({
                    "id": descriptor.id,
                    "name": descriptor.name,
                    "vendor": descriptor.vendor,
                    "version": descriptor.version,
                }))
                .collect::<Vec<_>>(),
        })))
    }
}

/// **Attach** a plugin to the document.
///
/// The node holds the plugin's identity and its state, and the state is
/// the plugin's own bytes: the document cannot read them, and it does
/// not try to.
#[derive(Command)]
#[command(id = "clap.attach", area = "clap", layer = "clap", icon = "plus", menu("CLAP"), mutates, undoable)]
pub struct AttachPlugin {
    /// The shared library the plugin is in.
    pub path: Text,
    /// Which plugin it is. The only one in the library, if the library
    /// has one.
    pub plugin: Option<Text>,
}

impl AttachPlugin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(fail("clap.attach", "a library has a path".to_string()));
        }
        // The plugin is named, or the library has only one and it is
        // that one. A library with several and no name is refused: a
        // host that guessed would attach the wrong plugin.
        let id = match &self.plugin {
            Some(plugin) if !plugin.0.trim().is_empty() => plugin.0.trim().to_string(),
            Some(_) => return Err(fail("clap.attach", "a plugin has an id".to_string())),
            None => {
                let plugins = plugins_of(&path)?;
                match plugins.len() {
                    0 => return Err(fail("clap.attach", format!("there is no plugin in `{path}`"))),
                    1 => plugins[0].id.clone(),
                    _ => {
                        let ids =
                            plugins.iter().map(|plugin| plugin.id.clone()).collect::<Vec<_>>().join(", ");
                        return Err(fail(
                            "clap.attach",
                            format!("`{path}` holds several plugins, and one of them has to be named: {ids}"),
                        ));
                    }
                }
            }
        };
        let report = talk(&id, &path, None, None)?;
        let payload = PluginPayload {
            library: path.clone(),
            id: report.descriptor.id.clone(),
            name: report.descriptor.name.clone(),
            vendor: report.descriptor.vendor.clone(),
            version: report.descriptor.version.clone(),
            params: report.parameters.iter().map(parameter_of).collect(),
            state: encode_state(&report.state)?,
        };
        let node = session.document_mut()?.add_node(payload.name.clone(), "clap.plugin", None);
        if let Some(node) = session.document_mut()?.node_mut(node) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("attached the plugin `{}`", payload.name)).with_data(json!({
            "id": node,
            "kind": "clap.plugin",
            "plugin": payload.id,
            "name": payload.name,
            "params": payload.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
        })))
    }
}

/// **Detach** a plugin: take it out of the document.
///
/// The state goes with it, because the state is the plugin's.
#[derive(Command)]
#[command(id = "clap.detach", area = "clap", layer = "clap", icon = "minus", menu("CLAP"), mutates, undoable)]
pub struct DetachPlugin {
    /// The plugin node.
    pub node: NodeRef,
}

impl DetachPlugin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let Some(existing) = session.document().node(node) else {
            return Err(fail("clap.detach", format!("there is no node `{node}`")));
        };
        if existing.kind != "clap.plugin" {
            return Err(fail("clap.detach", format!("the node `{node}` is not a plugin")));
        }
        let name = existing.name.clone();
        session.document_mut()?.remove_node(node);
        Ok(Outcome::changed(format!("detached the plugin `{name}`")).with_data(json!({ "id": node })))
    }
}

/// **Set** a parameter of a plugin, and keep the state that follows.
///
/// The value is offered to the plugin as an event, the plugin reads it
/// while it works, and the state it saves afterwards is what the
/// document keeps. So the document's copy of the state always follows
/// what the plugin did.
#[derive(Command)]
#[command(
    id = "clap.parameter.set",
    area = "clap",
    layer = "clap",
    icon = "list",
    menu("CLAP"),
    mutates,
    undoable
)]
pub struct SetParameter {
    /// The plugin node.
    pub node: NodeRef,
    /// Which parameter.
    pub parameter: Number,
    /// What it is set to.
    pub value: Number,
}

impl SetParameter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let payload = read_payload(session, node, "clap.parameter.set")?;
        let parameter = self.parameter.0;
        if !parameter.is_finite() || parameter < 0.0 || parameter.fract() != 0.0 {
            return Err(fail(
                "clap.parameter.set",
                "a parameter is numbered with a whole number".to_string(),
            ));
        }
        let value = self.value.0;
        if !value.is_finite() {
            return Err(fail("clap.parameter.set", "a value is a number".to_string()));
        }
        let state = decode_state(&payload.state, "clap.parameter.set")?;
        let report = talk(&payload.id, &payload.library, Some(&state), Some((parameter as u32, value)))?;
        let mut payload = payload;
        payload.params = report.parameters.iter().map(parameter_of).collect();
        payload.state = encode_state(&report.state)?;
        write_payload(session, node, &payload)?;
        Ok(Outcome::changed(format!("set the parameter {parameter} of `{}`", payload.name)).with_data(
            json!({
                "id": node,
                "parameter": parameter,
                "value": value,
                "params": payload.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
            }),
        ))
    }
}

/// **Read** the state of a plugin, as the plugin reads it.
///
/// The host cannot read a state: the bytes are the plugin's own. So the
/// plugin is given the state the document holds, and is asked what it
/// makes of it — and what it says is the answer.
#[derive(Command)]
#[command(id = "clap.state", area = "clap", layer = "clap", icon = "list", menu("CLAP"), no_journal)]
pub struct ReadState {
    /// The plugin node.
    pub node: NodeRef,
}

impl ReadState {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let payload = read_payload(session, node, "clap.state")?;
        let state = decode_state(&payload.state, "clap.state")?;
        let report = talk(&payload.id, &payload.library, Some(&state), None)?;
        let bytes = state.len();
        Ok(Outcome::new(format!("`{}` holds {bytes} bytes of state", payload.name)).with_data(json!({
            "id": node,
            "plugin": payload.id,
            "state_bytes": bytes,
            "params": report.parameters.iter().map(parameter_of_json).collect::<Vec<_>>(),
        })))
    }
}

/// The panel that shows the plugins.
#[derive(Panel)]
#[panel(id = "panel.clap", label = "CLAP", layer = "clap", commands("clap.list", "clap.attach"))]
pub struct ClapPanel;

/// The payload of a node, read.
fn read_payload(session: &Session, node: u64, id: &str) -> Result<PluginPayload, EngineError> {
    let Some(existing) = session.document().node(node) else {
        return Err(fail(id, format!("there is no node `{node}`")));
    };
    if existing.kind != "clap.plugin" {
        return Err(fail(id, format!("the node `{node}` is not a plugin")));
    }
    PluginPayload::from_json(&existing.data).map_err(|message| fail(id, message))
}

/// The payload of a node, written.
fn write_payload(session: &mut Session, node: u64, payload: &PluginPayload) -> Result<(), EngineError> {
    let document = session.document_mut()?;
    if let Some(node) = document.node_mut(node) {
        node.data = payload.to_json();
    }
    Ok(())
}

/// One of the plugin's parameters, as the document carries it.
fn parameter_of(parameter: &ClapParameter) -> ParameterPayload {
    ParameterPayload {
        id: parameter.id,
        name: parameter.name.clone(),
        value: parameter.value,
        min_value: parameter.min_value,
        max_value: parameter.max_value,
        default_value: parameter.default_value,
    }
}

/// One of the plugin's parameters, as the answer says it.
fn parameter_of_json(parameter: &ClapParameter) -> Json {
    parameter_of(parameter).to_json()
}

/// The state, as base64.
fn encode_state(state: &[u8]) -> Result<String, EngineError> {
    if state.len() > MAX_STATE_BYTES {
        return Err(fail(
            "clap",
            format!("the state is {} bytes, and this program carries at most {MAX_STATE_BYTES}", state.len()),
        ));
    }
    Ok(craftkit_sdk::bytes::encode(state))
}

/// The state, from base64.
fn decode_state(state: &str, id: &str) -> Result<Vec<u8>, EngineError> {
    let bytes = craftkit_sdk::bytes::decode(state).map_err(|message| fail(id, message))?;
    if bytes.len() > MAX_STATE_BYTES {
        return Err(fail(
            id,
            format!("the state is {} bytes, and this program carries at most {MAX_STATE_BYTES}", bytes.len()),
        ));
    }
    Ok(bytes)
}

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    ClapLayer,
    PluginKind,
    ListPlugins,
    AttachPlugin,
    DetachPlugin,
    SetParameter,
    ReadState,
    ClapPanel
);
