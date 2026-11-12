//! `craftkit-plugin-vst3` — a **VST3 plugin** in the document.
//!
//! # What is ported
//!
//! The family has plugins in its mixer: an insert holds a plugin by
//! name, a set of parameter values, and — since the port of the mixer —
//! the plugin's own **state**, so that a project remembers what a plugin
//! was doing when it was closed.
//!
//! - a **plugin** — a node whose payload holds the plugin's identity
//!   and its state;
//! - the **state** — the plugin's own bytes, carried whole and read by
//!   nobody but the plugin;
//! - the **parameters** — what the plugin says it holds, and their
//!   values.
//!
//! # The three things the port found
//!
//! - **A plugin is a program, and the document carries its state and
//!   not the program.** The library is a file on disk, like a picture:
//!   the document names it and the bytes stay where they are. So the
//!   node says which plugin it is and where the library is, and the
//!   **state** travels with it.
//! - **The state is opaque, and that is what makes it carryable.** The
//!   host never looks inside it: the plugin writes it and the plugin
//!   reads it. The document cannot interpret it either, so it travels
//!   as **base64 in the payload**.
//! - **The instance is not the document.** A VST3 instance holds raw
//!   addresses into a library that is open, and it cannot be sent
//!   between threads, and it is destroyed when the host is finished
//!   with it. So no instance is ever kept in a session: a command
//!   **opens the plugin, hands it the state, does its work, and closes
//!   it**. The document is left with what the plugin said.
//!
//! # The two things that are **not** the same as CLAP
//!
//! - **A parameter is named, not numbered.** A CLAP parameter is asked
//!   for by a number and a VST3 parameter is asked for by a **name**,
//!   so the command here takes a name and not an index. The document
//!   keeps the name too, because the name is what the plugin says and
//!   the document does not invent.
//! - **A value is between zero and one.** The format's own value is a
//!   **normalized** one, and the plugin says the range it takes in its
//!   own units. So the document carries both: the value as the plugin
//!   holds it, and the value in the units a person sees. The command
//!   takes the one a person sees, because that is what a person has.

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

use craftkit_vst3::{Vst3Class, Vst3Interface, Vst3Param, Vst3Plugin};

/// The layer of the VST3 plugins.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "vst3", name = "VST3", requires("core"))]
pub struct Vst3Layer;

/// A **plugin**: an identity and a state.
///
/// The payload holds the plugin's name, the library it came from, the
/// parameters it says it has, and its state as base64. Nothing else:
/// the instance that made the state is gone, and the bytes are the
/// plugin's own.
#[derive(NodeKind)]
#[node_kind(kind = "vst3.plugin", label = "Plugin", layer = "vst3", evaluates)]
pub struct PluginKind;

impl PluginKind {
    /// Not drawn.
    ///
    /// A plugin is a program, and the kit cannot draw a program: what
    /// it can do is say what the plugin is and what it holds.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What a plugin **means**: who it is, and what it holds.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let payload = PluginPayload::from_json(&node.data).ok()?;
        Some(json!({
            "cid": payload.cid,
            "name": payload.name,
            "vendor": payload.vendor,
            "version": payload.version,
            "library": payload.library,
            "params": payload.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
            "state_bytes": payload.state.len(),
            "channels_in": payload.channels_in,
            "channels_out": payload.channels_out,
        }))
    }
}

/// One **parameter** of a plugin, as the document carries it.
///
/// The **value** is the plugin's own: between zero and one. The
/// **plain** value is the same value in the units the plugin says the
/// parameter takes, and the range is the plugin's too.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterPayload {
    /// The parameter's name: the name a host asks by.
    pub id: String,
    /// Its name, for a person.
    pub title: String,
    /// Its value, between zero and one.
    pub value: f64,
    /// Its value, in the units the person sees.
    pub plain: f64,
    /// The smallest value it takes.
    pub min: f64,
    /// The largest value it takes.
    pub max: f64,
}

impl ParameterPayload {
    /// The parameter as it is written.
    pub fn to_json(&self) -> Json {
        json!({
            "id": self.id,
            "title": self.title,
            "value": self.value,
            "plain": self.plain,
            "min": self.min,
            "max": self.max,
        })
    }

    /// The parameter as it is read.
    pub fn from_json(value: &Json) -> Result<ParameterPayload, String> {
        let id = value.get("id").and_then(|id| id.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            return Err("a parameter says which parameter it is".to_string());
        }
        Ok(ParameterPayload {
            id,
            title: value.get("title").and_then(|title| title.as_str()).unwrap_or("").to_string(),
            value: value.get("value").and_then(|value| value.as_f64()).unwrap_or(0.0),
            plain: value.get("plain").and_then(|plain| plain.as_f64()).unwrap_or(0.0),
            min: value.get("min").and_then(|min| min.as_f64()).unwrap_or(0.0),
            max: value.get("max").and_then(|max| max.as_f64()).unwrap_or(1.0),
        })
    }
}

/// A **plugin**, as the document carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginPayload {
    /// The library the plugin is in.
    pub library: String,
    /// The class's identifier: the name a host asks by.
    pub cid: String,
    /// Its name, for a person.
    pub name: String,
    /// Who makes it.
    pub vendor: String,
    /// Its version.
    pub version: i32,
    /// The parameters it holds.
    pub params: Vec<ParameterPayload>,
    /// Its state, as base64.
    pub state: String,
    /// How many channels come in.
    pub channels_in: i32,
    /// How many channels go out.
    pub channels_out: i32,
}

impl PluginPayload {
    /// The plugin as it is written.
    pub fn to_json(&self) -> Json {
        json!({
            "library": self.library,
            "cid": self.cid,
            "name": self.name,
            "vendor": self.vendor,
            "version": self.version,
            "params": self.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
            "state": self.state,
            "channels_in": self.channels_in,
            "channels_out": self.channels_out,
        })
    }

    /// The plugin as it is read.
    pub fn from_json(value: &Json) -> Result<PluginPayload, String> {
        let library = value.get("library").and_then(|library| library.as_str()).unwrap_or("").to_string();
        if library.is_empty() {
            return Err("a plugin says which library it is in".to_string());
        }
        let cid = value.get("cid").and_then(|cid| cid.as_str()).unwrap_or("").to_string();
        if cid.is_empty() {
            return Err("a plugin says which class it is".to_string());
        }
        let params = value
            .get("params")
            .and_then(|params| params.as_array())
            .map(|params| params.iter().filter_map(|param| ParameterPayload::from_json(param).ok()).collect())
            .unwrap_or_default();
        Ok(PluginPayload {
            library,
            cid,
            name: value.get("name").and_then(|name| name.as_str()).unwrap_or("").to_string(),
            vendor: value.get("vendor").and_then(|vendor| vendor.as_str()).unwrap_or("").to_string(),
            version: value.get("version").and_then(|version| version.as_u64()).unwrap_or(0) as i32,
            params,
            state: value.get("state").and_then(|state| state.as_str()).unwrap_or("").to_string(),
            channels_in: value.get("channels_in").and_then(|count| count.as_u64()).unwrap_or(0) as i32,
            channels_out: value.get("channels_out").and_then(|count| count.as_u64()).unwrap_or(0) as i32,
        })
    }
}

/// The **classes** in a library.
///
/// The library is opened, the factory is asked what it holds, and the
/// library is closed again: nothing here is kept.
fn classes_of(path: &str) -> Result<Vec<Vst3Class>, EngineError> {
    let plugin = Vst3Plugin::open(path).map_err(|error| fail("vst3.list", error.to_string()))?;
    plugin.factory().classes().map_err(|error| fail("vst3.list", error.to_string()))
}

/// What a plugin said when it was asked.
struct Report {
    /// Who it is.
    class: Vst3Class,
    /// The parameters it holds.
    params: Vec<ParameterPayload>,
    /// Its state, as bytes.
    state: Vec<u8>,
    /// How many channels come in.
    channels_in: i32,
    /// How many channels go out.
    channels_out: i32,
    /// What came out of the block, if a block was run.
    output: Vec<f32>,
}

/// **Open** the plugin, hand it the state it had, and ask it what it holds.
///
/// The instance lives for the length of this function and not one moment
/// longer: it holds addresses into a library that is closed when this
/// returns, and it cannot be sent between threads, and the document has
/// no place for it. So the plugin is opened, given its state, asked, and
/// closed — and what it said is what the document keeps.
fn talk(
    cid: &str,
    path: &str,
    state: Option<&[u8]>,
    set: Option<(&str, f64)>,
    block: Option<&[f32]>,
) -> Result<Report, EngineError> {
    let plugin = Vst3Plugin::open(path).map_err(|error| fail("vst3", error.to_string()))?;
    let factory = plugin.factory();
    let classes = factory.classes().map_err(|error| fail("vst3", error.to_string()))?;
    let Some(class) = classes.into_iter().find(|class| class.cid == cid) else {
        return Err(fail("vst3", format!("the library holds no class called `{cid}`")));
    };
    let instance =
        factory.create(cid, Vst3Interface::Component).map_err(|error| fail("vst3", error.to_string()))?;
    instance.initialize().map_err(|error| fail("vst3", error.to_string()))?;

    // The plugin is given the state it had, so that what it says now is
    // what it would say if it had never been closed.
    if let Some(bytes) = state {
        instance.set_state(bytes).map_err(|error| fail("vst3", error.to_string()))?;
    }

    // The parameters are the plugin's own: it says how many it holds
    // and what each of them is.
    let mut params = Vec::new();
    for index in 0..instance.param_count() {
        let info = instance.param_info(index).map_err(|error| fail("vst3", error.to_string()))?;
        let value = instance.param_value(index).map_err(|error| fail("vst3", error.to_string()))?;
        params.push(parameter_of(&info, value as f64));
    }

    // A value is set by **name**, because that is what the format names
    // a parameter by.
    if let Some((name, plain)) = set {
        let Some(index) = params.iter().position(|param| param.id == name) else {
            return Err(fail("vst3", format!("the plugin has no parameter called `{name}`")));
        };
        let normalized = normalized(&params[index], plain);
        instance
            .set_param_value(index as i32, normalized as f32)
            .map_err(|error| fail("vst3", error.to_string()))?;
        // The plugin's own copy is read back: the document keeps what
        // the plugin says, and not what the host asked for.
        let value = instance.param_value(index as i32).map_err(|error| fail("vst3", error.to_string()))?;
        params[index].value = value as f64;
        params[index].plain = plain_of(&params[index], value as f64);
    }

    // The audio goes through, if a block was asked for.
    let mut output = Vec::new();
    if let Some(input) = block {
        let mut samples = vec![0.0f32; input.len()];
        instance
            .process(&[input], &[&mut samples], input.len())
            .map_err(|error| fail("vst3", error.to_string()))?;
        output = samples;
    }

    // The buses are the plugin's own: the host asks and does not guess.
    let channels_in =
        instance.bus_info(craftkit_vst3::BUS_DIRECTION_INPUT, 0).map(|bus| bus.channels).unwrap_or(0);
    let channels_out =
        instance.bus_info(craftkit_vst3::BUS_DIRECTION_OUTPUT, 0).map(|bus| bus.channels).unwrap_or(0);

    // The state is taken **last**: saving is what answers it.
    let saved = instance.state().map_err(|error| fail("vst3", error.to_string()))?;

    Ok(Report { class, params, state: saved, channels_in, channels_out, output })
}

/// A plain value as the plugin's own normalized one.
///
/// The plugin says the range it takes in its own units, and the value
/// the format holds is between zero and one. A value outside the range
/// is **held at the end of it**: the plugin would clip it anyway, and a
/// value that says what was asked for is not what the plugin did.
fn normalized(parameter: &ParameterPayload, plain: f64) -> f64 {
    let span = parameter.max - parameter.min;
    if span <= 0.0 {
        return 0.0;
    }
    ((plain - parameter.min) / span).clamp(0.0, 1.0)
}

/// A normalized value as the plain one.
fn plain_of(parameter: &ParameterPayload, value: f64) -> f64 {
    parameter.min + value * (parameter.max - parameter.min)
}

/// One of the plugin's parameters, as the document carries it.
fn parameter_of(parameter: &Vst3Param, value: f64) -> ParameterPayload {
    ParameterPayload {
        id: parameter.id.clone(),
        title: parameter.title.clone(),
        value,
        plain: parameter.min + value * (parameter.max - parameter.min),
        min: parameter.min,
        max: parameter.max,
    }
}

/// An error, in the kit's own shape.
fn fail(id: &str, message: String) -> EngineError {
    EngineError::Failed { id: id.to_string(), message }
}

/// The most bytes of **state** a node may carry.
///
/// The state is the plugin's own, and a plugin may have a lot of it. The
/// limit is this program's, and it is generous: a state that is bigger
/// than this is not a state a document should carry.
const MAX_STATE_BYTES: usize = 16 * 1024 * 1024;

/// The most samples one block may hold.
///
/// The kit has no audio thread, and a block here is a measurement
/// rather than a stream: the limit is this program's.
const MAX_BLOCK_SAMPLES: usize = 64 * 1024;

/// The **classes** in a library.
///
/// A measurement: it changes nothing, so it is not an undo step.
#[derive(Command)]
#[command(id = "vst3.list", area = "vst3", layer = "vst3", icon = "list", menu("VST3"), no_journal)]
pub struct ListPlugins {
    /// The shared library.
    pub path: Text,
}

impl ListPlugins {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(fail("vst3.list", "a library has a path".to_string()));
        }
        let classes = classes_of(&path)?;
        let count = classes.len();
        Ok(Outcome::new(format!("{count} class(es) in `{path}`")).with_data(json!({
            "path": path,
            "classes": classes
                .iter()
                .map(|class| json!({
                    "cid": class.cid,
                    "name": class.name,
                    "vendor": class.vendor,
                    "category": class.category,
                    "version": class.version,
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
#[command(id = "vst3.attach", area = "vst3", layer = "vst3", icon = "plus", menu("VST3"), mutates, undoable)]
pub struct AttachPlugin {
    /// The shared library the plugin is in.
    pub path: Text,
    /// Which class it is. The only one in the library, if the library
    /// has one.
    pub plugin: Option<Text>,
}

impl AttachPlugin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(fail("vst3.attach", "a library has a path".to_string()));
        }
        // The class is named, or the library has only one and it is that
        // one. A library with several and no name is refused: a host
        // that guessed would attach the wrong plugin.
        let cid = match &self.plugin {
            Some(plugin) if !plugin.0.trim().is_empty() => plugin.0.trim().to_string(),
            Some(_) => return Err(fail("vst3.attach", "a class has a name".to_string())),
            None => {
                let classes = classes_of(&path)?;
                match classes.len() {
                    0 => return Err(fail("vst3.attach", format!("there is no class in `{path}`"))),
                    1 => classes[0].cid.clone(),
                    _ => {
                        let names =
                            classes.iter().map(|class| class.cid.clone()).collect::<Vec<_>>().join(", ");
                        return Err(fail(
                            "vst3.attach",
                            format!(
                                "`{path}` holds several classes, and one of them has to be named: {names}"
                            ),
                        ));
                    }
                }
            }
        };
        let report = talk(&cid, &path, None, None, None)?;
        let payload = PluginPayload {
            library: path.clone(),
            cid: report.class.cid.clone(),
            name: report.class.name.clone(),
            vendor: report.class.vendor.clone(),
            version: report.class.version,
            params: report.params.clone(),
            state: encode_state(&report.state)?,
            channels_in: report.channels_in,
            channels_out: report.channels_out,
        };
        let node = session.document_mut()?.add_node(payload.name.clone(), "vst3.plugin", None);
        if let Some(node) = session.document_mut()?.node_mut(node) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("attached the plugin `{}`", payload.name)).with_data(json!({
            "id": node,
            "kind": "vst3.plugin",
            "plugin": payload.cid,
            "name": payload.name,
            "params": payload.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
        })))
    }
}

/// **Detach** a plugin: take it out of the document.
///
/// The state goes with it, because the state is the plugin's.
#[derive(Command)]
#[command(id = "vst3.detach", area = "vst3", layer = "vst3", icon = "minus", menu("VST3"), mutates, undoable)]
pub struct DetachPlugin {
    /// The plugin node.
    pub node: NodeRef,
}

impl DetachPlugin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let Some(existing) = session.document().node(node) else {
            return Err(fail("vst3.detach", format!("there is no node `{node}`")));
        };
        if existing.kind != "vst3.plugin" {
            return Err(fail("vst3.detach", format!("the node `{node}` is not a plugin")));
        }
        let name = existing.name.clone();
        session.document_mut()?.remove_node(node);
        Ok(Outcome::changed(format!("detached the plugin `{name}`")).with_data(json!({ "id": node })))
    }
}

/// **Set** a parameter of a plugin, and keep the state that follows.
///
/// The parameter is named, because that is what the format names it by,
/// and the value is the one a person sees — the plugin's own units.
#[derive(Command)]
#[command(
    id = "vst3.parameter.set",
    area = "vst3",
    layer = "vst3",
    icon = "list",
    menu("VST3"),
    mutates,
    undoable
)]
pub struct SetParameter {
    /// The plugin node.
    pub node: NodeRef,
    /// Which parameter: the name the plugin says.
    pub parameter: Text,
    /// What it is set to, in the units the plugin says.
    pub value: Number,
}

impl SetParameter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let payload = read_payload(session, node, "vst3.parameter.set")?;
        let name = self.parameter.0.trim().to_string();
        if name.is_empty() {
            return Err(fail("vst3.parameter.set", "a parameter has a name".to_string()));
        }
        let value = self.value.0;
        if !value.is_finite() {
            return Err(fail("vst3.parameter.set", "a value is a number".to_string()));
        }
        let state = decode_state(&payload.state, "vst3.parameter.set")?;
        let report = talk(&payload.cid, &payload.library, Some(&state), Some((&name, value)), None)?;
        let mut payload = payload;
        payload.params = report.params.clone();
        payload.state = encode_state(&report.state)?;
        write_payload(session, node, &payload)?;
        let set =
            payload.params.iter().find(|param| param.id == name).map(|param| param.plain).unwrap_or(value);
        Ok(Outcome::changed(format!("set the parameter {name} of `{}`", payload.name)).with_data(json!({
            "id": node,
            "parameter": name,
            "value": set,
            "params": payload.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
        })))
    }
}

/// **Read** the state of a plugin, as the plugin reads it.
///
/// The host cannot read a state: the bytes are the plugin's own. So the
/// plugin is given the state the document holds, and is asked what it
/// makes of it — and what it says is the answer.
#[derive(Command)]
#[command(id = "vst3.state", area = "vst3", layer = "vst3", icon = "list", menu("VST3"), no_journal)]
pub struct ReadState {
    /// The plugin node.
    pub node: NodeRef,
}

impl ReadState {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let payload = read_payload(session, node, "vst3.state")?;
        let state = decode_state(&payload.state, "vst3.state")?;
        let report = talk(&payload.cid, &payload.library, Some(&state), None, None)?;
        let bytes = report.state.len();
        Ok(Outcome::new(format!("`{}` holds {bytes} bytes of state", payload.name)).with_data(json!({
            "id": node,
            "plugin": payload.cid,
            "state_bytes": bytes,
            "params": report.params.iter().map(|param| param.to_json()).collect::<Vec<_>>(),
        })))
    }
}

/// **Run** a block of audio through the plugin.
///
/// The block is **one channel**: the plugin says how many it has, and
/// the kit asks rather than guesses. What comes out is what the plugin
/// did to what went in.
#[derive(Command)]
#[command(id = "vst3.process", area = "vst3", layer = "vst3", icon = "list", menu("VST3"), no_journal)]
pub struct ProcessBlock {
    /// The plugin node.
    pub node: NodeRef,
    /// The samples that go in: a list of numbers.
    pub input: craftkit_sdk::serde_json::Value,
}

impl ProcessBlock {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let payload = read_payload(session, node, "vst3.process")?;
        let input = samples_of(&self.input, "vst3.process")?;
        let state = decode_state(&payload.state, "vst3.process")?;
        let report = talk(&payload.cid, &payload.library, Some(&state), None, Some(&input))?;
        Ok(Outcome::new(format!("ran {} samples through `{}`", input.len(), payload.name)).with_data(json!({
            "id": node,
            "frames": input.len(),
            "input": input,
            "output": report.output,
        })))
    }
}

/// The samples of a raw value, as numbers.
///
/// A block is a list of numbers, and anything else is refused: a value
/// that is not a number is not a sample.
fn samples_of(value: &Json, id: &str) -> Result<Vec<f32>, EngineError> {
    let Some(items) = value.as_array() else {
        return Err(fail(id, "a block is a list of numbers".to_string()));
    };
    if items.len() > MAX_BLOCK_SAMPLES {
        return Err(fail(
            id,
            format!("a block holds at most {MAX_BLOCK_SAMPLES} samples, and this one holds {}", items.len()),
        ));
    }
    let mut samples = Vec::with_capacity(items.len());
    for item in items {
        let Some(number) = item.as_f64() else {
            return Err(fail(id, "a block is a list of numbers".to_string()));
        };
        if !number.is_finite() {
            return Err(fail(id, "a sample is a number".to_string()));
        }
        samples.push(number as f32);
    }
    Ok(samples)
}

/// The panel that shows the plugins.
#[derive(Panel)]
#[panel(id = "panel.vst3", label = "VST3", layer = "vst3", commands("vst3.list", "vst3.attach"))]
pub struct Vst3Panel;

/// The payload of a node, read.
fn read_payload(session: &Session, node: u64, id: &str) -> Result<PluginPayload, EngineError> {
    let Some(existing) = session.document().node(node) else {
        return Err(fail(id, format!("there is no node `{node}`")));
    };
    if existing.kind != "vst3.plugin" {
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

/// The state, as base64.
fn encode_state(state: &[u8]) -> Result<String, EngineError> {
    if state.len() > MAX_STATE_BYTES {
        return Err(fail(
            "vst3",
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
    Vst3Layer,
    PluginKind,
    ListPlugins,
    AttachPlugin,
    DetachPlugin,
    SetParameter,
    ReadState,
    ProcessBlock,
    Vst3Panel
);
