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

use craftkit_clap::{ClapDescriptor, ClapLayout, ClapParameter, ClapPlugin, ClapTransport};

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
            "channels_in": payload.channels_in,
            "channels_out": payload.channels_out,
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
    /// The channels going in, as the plugin said.
    pub channels_in: u32,
    /// The channels coming out, as the plugin said.
    pub channels_out: u32,
    /// Where the music is, in the kit's own time.
    pub transport: TransportPayload,
}

/// Where the music is, and how fast it goes.
///
/// The position is a **tick** — the kit's own unit — and the tempo and
/// the meter are a **map**: a list of events, because the tempo of a
/// piece changes and a single number cannot say when.
///
/// What the plugin is told is **derived** from all of that: the beats,
/// the seconds, the tempo and the meter at that tick. Nothing here is
/// stored twice.
#[derive(Debug, Clone, PartialEq)]
pub struct TransportPayload {
    /// Where the music is, in ticks.
    pub tick: i64,
    /// Whether the music is playing.
    pub playing: bool,
    /// The tempo events, in tick order.
    pub tempos: Vec<TempoPayload>,
    /// The meter events, in tick order.
    pub meters: Vec<MeterPayload>,
}

/// One **tempo** event: a tempo from a tick onwards.
#[derive(Debug, Clone, PartialEq)]
pub struct TempoPayload {
    /// The tick the change happens at.
    pub tick: i64,
    /// The tempo, in beats a minute.
    pub bpm: f64,
}

/// One **meter** event: a bar from a tick onwards.
#[derive(Debug, Clone, PartialEq)]
pub struct MeterPayload {
    /// The tick the change happens at.
    pub tick: i64,
    /// How many beats are in a bar.
    pub numerator: u32,
    /// What a beat is worth, as the denominator of a whole note.
    pub denominator: u32,
}

impl Default for TransportPayload {
    /// The same as [`TransportPayload::new`]: the music at the
    /// beginning, at a hundred and twenty beats a minute.
    fn default() -> TransportPayload {
        TransportPayload::new()
    }
}

impl TempoPayload {
    /// The tempo event as it is written.
    pub fn to_json(&self) -> Json {
        json!({ "tick": self.tick, "bpm": self.bpm })
    }

    /// The tempo event as it is read.
    pub fn from_json(value: &Json) -> TempoPayload {
        TempoPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            bpm: value.get("bpm").and_then(|bpm| bpm.as_f64()).unwrap_or(120.0),
        }
    }
}

impl MeterPayload {
    /// The meter event as it is written.
    pub fn to_json(&self) -> Json {
        json!({ "tick": self.tick, "numerator": self.numerator, "denominator": self.denominator })
    }

    /// The meter event as it is read.
    pub fn from_json(value: &Json) -> MeterPayload {
        MeterPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            numerator: value.get("numerator").and_then(|count| count.as_u64()).unwrap_or(4) as u32,
            denominator: value.get("denominator").and_then(|count| count.as_u64()).unwrap_or(4) as u32,
        }
    }
}

impl TransportPayload {
    /// A transport at the start of the music, at 120 beats a minute.
    pub fn new() -> TransportPayload {
        TransportPayload {
            tick: 0,
            playing: true,
            tempos: vec![TempoPayload { tick: 0, bpm: 120.0 }],
            meters: vec![MeterPayload { tick: 0, numerator: 4, denominator: 4 }],
        }
    }

    /// The transport as it is written.
    pub fn to_json(&self) -> Json {
        json!({
            "tick": self.tick,
            "playing": self.playing,
            "tempos": self.tempos.iter().map(|tempo| tempo.to_json()).collect::<Vec<_>>(),
            "meters": self.meters.iter().map(|meter| meter.to_json()).collect::<Vec<_>>(),
        })
    }

    /// The transport as it is read.
    pub fn from_json(value: &Json) -> TransportPayload {
        let tempos = value
            .get("tempos")
            .and_then(|tempos| tempos.as_array())
            .map(|tempos| tempos.iter().map(TempoPayload::from_json).collect())
            .unwrap_or_default();
        let meters = value
            .get("meters")
            .and_then(|meters| meters.as_array())
            .map(|meters| meters.iter().map(MeterPayload::from_json).collect())
            .unwrap_or_default();
        TransportPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            playing: value.get("playing").and_then(|playing| playing.as_bool()).unwrap_or(true),
            tempos,
            meters,
        }
    }

    /// The **map** the payload describes.
    ///
    /// The map is the kit's own model — a tempo map — and the payload
    /// is read into it. An event that the model refuses is an event the
    /// payload does not have: the model's rules are the kit's rules.
    pub fn to_map(&self) -> Result<craftkit_sound_time::TempoMap, String> {
        let mut map = craftkit_sound_time::TempoMap::new();
        for tempo in &self.tempos {
            map.set_tempo(tempo.tick, tempo.bpm)?;
        }
        for meter in &self.meters {
            map.set_meter(meter.tick, meter.numerator, meter.denominator)?;
        }
        Ok(map)
    }

    /// What the plugin is told: where the music is, right now.
    ///
    /// Everything here is **derived**: the beats and the seconds from
    /// the tick, and the tempo and the meter from the map at that tick.
    pub fn to_transport(&self, sample_rate: u32) -> Result<ClapTransport, String> {
        let map = self.to_map()?;
        let tempo = map.tempo_at_tick(self.tick);
        let (numerator, denominator) = map.meter_at_tick(self.tick);
        let beats = self.tick as f64 / craftkit_sound_time::TICKS_PER_QUARTER as f64;
        let samples = map.ticks_to_samples(self.tick, sample_rate);
        let seconds = samples as f64 / sample_rate as f64;
        Ok(ClapTransport {
            song_pos_beats: beats,
            song_pos_seconds: seconds,
            tempo,
            tsig_num: numerator as u16,
            tsig_denom: denominator as u16,
            playing: self.playing,
            recording: false,
        })
    }
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
            "channels_in": self.channels_in,
            "channels_out": self.channels_out,
            "transport": self.transport.to_json(),
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
            channels_in: value.get("channels_in").and_then(|count| count.as_u64()).unwrap_or(0) as u32,
            channels_out: value.get("channels_out").and_then(|count| count.as_u64()).unwrap_or(0) as u32,
            transport: value.get("transport").map(TransportPayload::from_json).unwrap_or_default(),
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
    /// The shape of its audio, as the plugin said.
    layout: Option<ClapLayout>,
    /// Whether the plugin said its state had changed.
    dirty: bool,
}

/// **Open** the plugin, hand it the state it had, and ask it what it holds.
///
/// The instance lives for the length of this function and not one moment
/// longer: it holds addresses into a library that is closed when this
/// returns, and it cannot be sent between threads, and the document has
/// no place for it. So the plugin is opened, given its state, asked, and
/// closed — and what it said is what the document keeps.
fn talk(
    id: &str,
    path: &str,
    state: Option<&[u8]>,
    set: Option<(u32, f64)>,
    transport: ClapTransport,
) -> Result<Report, EngineError> {
    let plugin = ClapPlugin::open(path).map_err(|error| fail("clap", error.to_string()))?;
    plugin.init().map_err(|error| fail("clap", error.to_string()))?;
    let factory = plugin.factory().map_err(|error| fail("clap", error.to_string()))?;
    let instance = factory.create(id).map_err(|error| fail("clap", error.to_string()))?;
    instance.init().map_err(|error| fail("clap", error.to_string()))?;
    instance.activate(SAMPLE_RATE, 1, MAX_FRAMES).map_err(|error| fail("clap", error.to_string()))?;
    instance.start_processing().map_err(|error| fail("clap", error.to_string()))?;

    // Where the music is, is offered with every block: the plugin is
    // told where it is, and it is told again and again.
    instance.set_transport(transport);
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
        // The block is the plugin's own shape: the host does not guess
        // how many channels a plugin has.
        let mut block = instance.block(1).map_err(|error| fail("clap", error.to_string()))?;
        instance.process(1, &mut block).map_err(|error| fail("clap", error.to_string()))?;
    }

    let descriptor = instance.descriptor().map_err(|error| fail("clap", error.to_string()))?;
    let parameters = instance.parameters().map_err(|error| fail("clap", error.to_string()))?;
    // The plugin's **word**, taken before the state is saved: saving is
    // what answers it.
    let dirty = instance.state_dirty();
    let saved = instance.save_state().map_err(|error| fail("clap", error.to_string()))?;
    let layout = instance.layout();

    // The instance is finished with: it is stopped, deactivated and
    // destroyed, and only then is the library closed.
    instance.stop_processing().map_err(|error| fail("clap", error.to_string()))?;
    instance.deactivate().map_err(|error| fail("clap", error.to_string()))?;
    instance.destroy();

    Ok(Report { descriptor, parameters, state: saved, layout, dirty })
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

/// The same rate, as a whole number of cycles a second.
///
/// The kit has no audio thread, so the rate is this program's — and
/// the transport's seconds are worked out with it.
const SAMPLE_RATE_HZ: u32 = 48_000;

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
        let transport = TransportPayload::new();
        let transport =
            transport.to_transport(SAMPLE_RATE_HZ).map_err(|message| fail("clap.attach", message))?;
        let report = talk(&id, &path, None, None, transport)?;
        let payload = PluginPayload {
            library: path.clone(),
            id: report.descriptor.id.clone(),
            name: report.descriptor.name.clone(),
            vendor: report.descriptor.vendor.clone(),
            version: report.descriptor.version.clone(),
            params: report.parameters.iter().map(parameter_of).collect(),
            state: encode_state(&report.state)?,
            channels_in: report.layout.map(|layout| layout.inputs).unwrap_or(0),
            channels_out: report.layout.map(|layout| layout.outputs).unwrap_or(0),
            transport: TransportPayload::new(),
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
        let report = talk(
            &payload.id,
            &payload.library,
            Some(&state),
            Some((parameter as u32, value)),
            payload
                .transport
                .to_transport(SAMPLE_RATE_HZ)
                .map_err(|message| fail("clap.parameter.set", message))?,
        )?;
        let mut payload = payload;
        payload.params = report.parameters.iter().map(parameter_of).collect();
        payload.state = encode_state(&report.state)?;
        write_payload(session, node, &payload)?;
        Ok(Outcome::changed(format!("set the parameter {parameter} of `{}`", payload.name)).with_data(
            json!({
                "id": node,
                "parameter": parameter,
                "value": value,
                "dirty": report.dirty,
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
        let report = talk(
            &payload.id,
            &payload.library,
            Some(&state),
            None,
            payload.transport.to_transport(SAMPLE_RATE_HZ).map_err(|message| fail("clap.state", message))?,
        )?;
        let bytes = state.len();
        Ok(Outcome::new(format!("`{}` holds {bytes} bytes of state", payload.name)).with_data(json!({
            "id": node,
            "plugin": payload.id,
            "state_bytes": bytes,
            "dirty": report.dirty,
            "params": report.parameters.iter().map(parameter_of_json).collect::<Vec<_>>(),
        })))
    }
}

/// **Move** the music: where it is, and whether it is playing.
///
/// The position is a **tick** — the kit's own unit — and the transport
/// the plugin is told about is worked out from it: the beats, the
/// seconds, the tempo and the meter at that tick.
#[derive(Command)]
#[command(
    id = "clap.transport.set",
    area = "clap",
    layer = "clap",
    icon = "list",
    menu("CLAP"),
    mutates,
    undoable
)]
pub struct SetTransport {
    /// The plugin node.
    pub node: NodeRef,
    /// Where the music is, in ticks.
    pub tick: Number,
    /// Whether the music is playing.
    pub playing: Option<craftkit_sdk::Bool>,
}

impl SetTransport {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let mut payload = read_payload(session, node, "clap.transport.set")?;
        let tick = tick_of(self.tick.0, "clap.transport.set")?;
        payload.transport.tick = tick;
        if let Some(playing) = self.playing {
            payload.transport.playing = playing.0;
        }
        let transport = payload
            .transport
            .to_transport(SAMPLE_RATE_HZ)
            .map_err(|message| fail("clap.transport.set", message))?;
        write_payload(session, node, &payload)?;
        Ok(Outcome::changed(format!("moved the music of `{}` to tick {tick}", payload.name))
            .with_data(json!({ "id": node, "tick": tick, "transport": transport_data(&transport) })))
    }
}

/// Set the **tempo** from a tick onwards.
///
/// The tempo of a piece changes, so the tempo is a **map** and not a
/// number: this command says what the tempo is from a tick onwards, and
/// the map keeps the rest.
#[derive(Command)]
#[command(
    id = "clap.transport.tempo",
    area = "clap",
    layer = "clap",
    icon = "list",
    menu("CLAP"),
    mutates,
    undoable
)]
pub struct SetTempo {
    /// The plugin node.
    pub node: NodeRef,
    /// The tick the change happens at.
    pub tick: Number,
    /// The tempo, in beats a minute.
    pub bpm: Number,
}

impl SetTempo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let mut payload = read_payload(session, node, "clap.transport.tempo")?;
        let tick = tick_of(self.tick.0, "clap.transport.tempo")?;
        let mut map = payload.transport.to_map().map_err(|message| fail("clap.transport.tempo", message))?;
        map.set_tempo(tick, self.bpm.0).map_err(|message| fail("clap.transport.tempo", message))?;
        payload.transport.tempos =
            map.tempos().iter().map(|tempo| TempoPayload { tick: tempo.tick, bpm: tempo.bpm }).collect();
        let transport = payload
            .transport
            .to_transport(SAMPLE_RATE_HZ)
            .map_err(|message| fail("clap.transport.tempo", message))?;
        write_payload(session, node, &payload)?;
        Ok(Outcome::changed(format!("set the tempo of `{}` at tick {tick}", payload.name)).with_data(
            json!({ "id": node, "tick": tick, "bpm": self.bpm.0, "transport": transport_data(&transport) }),
        ))
    }
}

/// Set the **meter** from a tick onwards.
///
/// A meter is how many beats are in a bar and what a beat is worth, and
/// it changes like the tempo does.
#[derive(Command)]
#[command(
    id = "clap.transport.meter",
    area = "clap",
    layer = "clap",
    icon = "list",
    menu("CLAP"),
    mutates,
    undoable
)]
pub struct SetMeter {
    /// The plugin node.
    pub node: NodeRef,
    /// The tick the change happens at.
    pub tick: Number,
    /// How many beats are in a bar.
    pub numerator: Number,
    /// What a beat is worth, as the denominator of a whole note.
    pub denominator: Number,
}

impl SetMeter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let mut payload = read_payload(session, node, "clap.transport.meter")?;
        let tick = tick_of(self.tick.0, "clap.transport.meter")?;
        let numerator = self.numerator.0;
        let denominator = self.denominator.0;
        if !numerator.is_finite() || !denominator.is_finite() {
            return Err(fail("clap.transport.meter", "a meter is two whole numbers".to_string()));
        }
        let mut map = payload.transport.to_map().map_err(|message| fail("clap.transport.meter", message))?;
        map.set_meter(tick, numerator as u32, denominator as u32)
            .map_err(|message| fail("clap.transport.meter", message))?;
        payload.transport.meters = map
            .meters()
            .iter()
            .map(|meter| MeterPayload {
                tick: meter.tick,
                numerator: meter.numerator,
                denominator: meter.denominator,
            })
            .collect();
        let transport = payload
            .transport
            .to_transport(SAMPLE_RATE_HZ)
            .map_err(|message| fail("clap.transport.meter", message))?;
        write_payload(session, node, &payload)?;
        Ok(Outcome::changed(format!("set the meter of `{}` at tick {tick}", payload.name)).with_data(json!({
            "id": node,
            "tick": tick,
            "numerator": numerator,
            "denominator": denominator,
            "transport": transport_data(&transport),
        })))
    }
}

/// A tick, as the kit's own number.
///
/// A tick is a whole number of them, and it is not negative: the
/// music starts at the beginning and moves forward.
fn tick_of(value: f64, id: &str) -> Result<i64, EngineError> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(fail(id, "a tick is a whole number of them".to_string()));
    }
    if value > craftkit_sound_time::MAX_POSITION as f64 {
        return Err(fail(id, format!("a tick is at most {}", craftkit_sound_time::MAX_POSITION)));
    }
    Ok(value as i64)
}

/// The transport, as the answer says it.
fn transport_data(transport: &ClapTransport) -> Json {
    json!({
        "beats": transport.song_pos_beats,
        "seconds": transport.song_pos_seconds,
        "tempo": transport.tempo,
        "meter": [transport.tsig_num, transport.tsig_denom],
        "playing": transport.playing,
    })
}

/// The panel that shows the plugins.
#[derive(Panel)]
#[panel(
    id = "panel.clap",
    label = "CLAP",
    layer = "clap",
    commands("clap.list", "clap.attach", "clap.transport.set")
)]
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
    SetTransport,
    SetTempo,
    SetMeter,
    ClapPanel
);
