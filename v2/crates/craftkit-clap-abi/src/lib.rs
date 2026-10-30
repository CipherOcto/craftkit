//! `craftkit-clap-abi` — the **CLAP ABI**, written down as shapes.
//!
//! # What this is
//!
//! CLAP is a **published, open plugin format**: an audio plugin ABI that
//! a host and a plugin agree on, described in public documentation and
//! implemented in open source. This crate writes that ABI down as
//! **data** — the structs a host reads out of a plugin, and the two
//! names a host looks for.
//!
//! It holds **no `unsafe` at all**, and that is the point of the
//! design: a binary contract is a set of shapes, and a set of shapes is
//! safe to write down. The unsafe is on the two sides of it — the host
//! that opens the library and calls through the shapes
//! (`craftkit-clap`), and the plugin that exports them
//! (`craftkit-clap-fixture`).
//!
//! # Why the shapes are here and not in the host
//!
//! The host and the plugin must agree on the **layout** of these
//! structs, byte for byte. If the host had its own copy and the plugin
//! had another, the two would drift apart silently, and a mismatch
//! would be a crash in the middle of an audio callback. So there is
//! **one** copy of the shapes, and both sides use it.
//!
//! # The one thing that is different from a function table
//!
//! craftkit's own door ([`craftkit_ffi_abi`]) exports a **function**
//! that gives back the table's address. CLAP exports a **data**
//! symbol: `clap_entry` is a variable whose value is the address of
//! the entry struct. The host reads the pointer *at* the symbol's
//! address, and the pointer *in* it. That is the difference the host
//! has to know about, and it is why CLAP needs its own door rather than
//! a reuse of the other one.
//!
//! # The version
//!
//! CLAP 1.2 is the version these shapes describe. The compatibility
//! rule is the format's own: a host that speaks major version 1 can
//! use a plugin that speaks major version 1, whatever the minor
//! versions are. The host checks it and refuses a plugin it does not
//! understand, rather than calling into it and hoping.
//!
//! ```text
//! struct ClapPluginEntry {
//!     ClapVersion         clap_version;
//!     fn(char*) -> bool   init;
//!     fn()                deinit;
//!     fn(char*) -> void*  get_factory;
//! }
//!
//! struct ClapPluginFactory {
//!     fn(factory) -> u32                 get_plugin_count;
//!     fn(factory, u32) -> descriptor*     get_plugin_descriptor;
//!     fn(factory, host*, char*) -> plugin* create_plugin;
//! }
//!
//! struct ClapPluginDescriptor {
//!     ClapVersion clap_version;
//!     char*       id;
//!     char*       name;
//!     char*       vendor;
//!     char*       url;
//!     char*       manual_url;
//!     char*       support_url;
//!     char*       version;
//!     char*       description;
//!     char**      features;
//! }
//! ```

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::ffi::{c_char, c_void};

/// The name of the **entry symbol** a CLAP plugin exports.
///
/// It is a **data** symbol, not a function: the value at that address
/// is a pointer to the plugin's entry struct.
pub const ENTRY_SYMBOL: &str = "clap_entry";

/// The id of the **plugin factory**, the one factory a host asks for.
pub const PLUGIN_FACTORY_ID: &str = "clap.plugin-factory";

/// The major version of the format these shapes describe.
pub const CLAP_MAJOR_VERSION: u32 = 1;

/// The minor version of the format these shapes describe.
pub const CLAP_MINOR_VERSION: u32 = 2;

/// The revision of the format these shapes describe.
pub const CLAP_REVISION: u32 = 2;

/// A version, as the format writes it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClapVersion {
    /// The major version: the one that decides compatibility.
    pub major: u32,
    /// The minor version.
    pub minor: u32,
    /// The revision.
    pub revision: u32,
}

impl ClapVersion {
    /// The version these shapes describe.
    pub const CURRENT: ClapVersion =
        ClapVersion { major: CLAP_MAJOR_VERSION, minor: CLAP_MINOR_VERSION, revision: CLAP_REVISION };

    /// Whether a host that speaks **this** version can use a plugin
    /// that speaks `version`.
    ///
    /// The rule is the format's own: the major versions must agree.
    /// A plugin that speaks a newer major version is refused — the
    /// shapes may have changed — and a plugin that speaks an older one
    /// is refused too.
    pub fn is_compatible_with(&self, version: ClapVersion) -> bool {
        version.major == self.major
    }
}

/// The **entry** of a CLAP plugin: what a host finds first.
///
/// The function pointers are written as **safe** `extern "C" fn`
/// types, and that is the same choice craftkit's own ABI makes. A
/// table of function pointers is a safe thing to write down: writing
/// one down performs no unsafe operation. What makes a call safe is
/// the **contract** — the format says what each function does, and
/// the host reads the table and then obeys it. The unsafe is on the
/// two sides of the table: the host that reads the plugin's memory,
/// and the plugin that exports it.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapPluginEntry {
    /// The version the plugin speaks.
    pub clap_version: ClapVersion,
    /// Called once, with the plugin's own path, before anything else.
    pub init: Option<extern "C" fn(plugin_path: *const c_char) -> bool>,
    /// Called once, when the host is finished with the plugin.
    pub deinit: Option<extern "C" fn()>,
    /// Asked for a factory, by the factory's id.
    pub get_factory: Option<extern "C" fn(factory_id: *const c_char) -> *const c_void>,
}

/// The **plugin factory**: the thing that holds the plugins.
///
/// The factory's own methods take the factory back as their first
/// argument, which is how the format's C code does it: the factory is
/// the receiver.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapPluginFactory {
    /// How many plugins the factory holds.
    pub get_plugin_count: Option<extern "C" fn(factory: *const ClapPluginFactory) -> u32>,
    /// The descriptor of the plugin at that index.
    pub get_plugin_descriptor:
        Option<extern "C" fn(factory: *const ClapPluginFactory, index: u32) -> *const ClapPluginDescriptor>,
    /// Make the plugin with that id.
    pub create_plugin: Option<
        extern "C" fn(
            factory: *const ClapPluginFactory,
            host: *const c_void,
            id: *const c_char,
        ) -> *const c_void,
    >,
}

/// The **host**: what a plugin sees of the program that loaded it.
///
/// This is the other half of the contract. The plugin is given a
/// pointer to one of these when it is made, and it reads the name off
/// it and calls back through it when it wants something.
///
/// The **extensions** the format has — the state, the log, the
/// parameters, the audio ports — are not here: they are reached
/// through `get_extension`, and a host that has none of them answers
/// with nothing. That is a later arc, and the shape of the answer is
/// the same either way.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapHost {
    /// The version the host speaks.
    pub clap_version: ClapVersion,
    /// The host's own state, which the host's functions read.
    pub host_data: *mut c_void,
    /// The host's name, for a plugin that wants to say who it is to.
    pub name: *const c_char,
    /// Who made the host.
    pub vendor: *const c_char,
    /// Where the host lives.
    pub url: *const c_char,
    /// The host's own version.
    pub version: *const c_char,
    /// Ask for an extension of the host, by its id.
    pub get_extension:
        Option<extern "C" fn(host: *const ClapHost, extension_id: *const c_char) -> *const c_void>,
    /// Ask the host to restart the plugin.
    pub request_restart: Option<extern "C" fn(host: *const ClapHost)>,
    /// Ask the host to call the plugin back.
    pub request_process: Option<extern "C" fn(host: *const ClapHost)>,
    /// Ask the host to call the plugin on its main thread.
    pub request_callback: Option<extern "C" fn(host: *const ClapHost)>,
}

/// What the plugin says it did with a block of audio.
///
/// The format's enum, and an enum in C is a number: the four values
/// are the four the format names.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClapProcessStatus {
    /// The plugin processed the block.
    Continue = 0,
    /// The plugin did not process the block, and the host may carry on.
    ContinueIfNotProcessed = 1,
    /// The plugin is still processing a tail.
    Tail = 2,
    /// The plugin has nothing to do.
    Sleep = 3,
}

/// The **plugin**: the instance the factory makes.
///
/// This is the instance, not the file: a file may hold many plugins,
/// and each one that is made is an instance of its own, with its own
/// state, its own life and its own destruction.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapPlugin {
    /// What the plugin says about itself.
    pub desc: *const ClapPluginDescriptor,
    /// The plugin's own state, which the plugin reads and writes.
    pub plugin_data: *mut c_void,
    /// Called once, when the instance is ready to work.
    pub init: Option<extern "C" fn(plugin: *const ClapPlugin) -> bool>,
    /// Called once, when the instance is finished with.
    pub destroy: Option<extern "C" fn(plugin: *const ClapPlugin)>,
    /// Called when the plugin is to work at that rate.
    pub activate: Option<
        extern "C" fn(
            plugin: *const ClapPlugin,
            sample_rate: f64,
            min_frames_count: u32,
            max_frames_count: u32,
        ) -> bool,
    >,
    /// Called when the plugin is to stop working.
    pub deactivate: Option<extern "C" fn(plugin: *const ClapPlugin)>,
    /// Called when the host starts to send audio.
    pub start_processing: Option<extern "C" fn(plugin: *const ClapPlugin) -> bool>,
    /// Called when the host stops sending audio.
    pub stop_processing: Option<extern "C" fn(plugin: *const ClapPlugin)>,
    /// Called when the plugin is to forget its audio.
    pub reset: Option<extern "C" fn(plugin: *const ClapPlugin)>,
    /// Called with a block of audio.
    pub process:
        Option<extern "C" fn(plugin: *const ClapPlugin, process: *const ClapProcess) -> ClapProcessStatus>,
    /// Ask the plugin for an extension of its own.
    pub get_extension: Option<extern "C" fn(plugin: *const ClapPlugin, id: *const c_char) -> *const c_void>,
    /// Called on the host's main thread, when the plugin asks for it.
    pub on_main_thread: Option<extern "C" fn(plugin: *const ClapPlugin)>,
}

/// The **space** the core events live in.
pub const CORE_EVENT_SPACE_ID: u16 = 0;

/// The type of a **parameter value** event.
pub const EVENT_PARAM_VALUE: u32 = 5;

/// The type of a **MIDI** event.
pub const EVENT_MIDI: u32 = 10;

/// The type of a **transport** event.
pub const EVENT_TRANSPORT: u32 = 2;

/// The **head** of every event.
///
/// Every event begins with this, so that a list of events can be read
/// without knowing what is in it: the size says how long the event is,
/// and the type says what it is.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapEventHeader {
    /// How long the whole event is, in bytes.
    pub size: u32,
    /// When the event happened, in samples.
    pub time: u32,
    /// Which space the event is in.
    pub space_id: u16,
    /// What the event is.
    pub type_: u32,
    /// What the event is like: live, or not to be recorded.
    pub flags: u32,
}

/// A **parameter value** event: the host telling the plugin that a
/// parameter has moved.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapEventParamValue {
    /// The head every event has.
    pub header: ClapEventHeader,
    /// The parameter that moved.
    pub param_id: u32,
    /// The host's own state for the parameter.
    pub cookie: *mut c_void,
    /// The note the parameter belongs to, if it belongs to one.
    pub note_id: i32,
    /// The port the parameter belongs to, if it belongs to one.
    pub port_index: i16,
    /// The channel the parameter belongs to, if it belongs to one.
    pub channel: i16,
    /// The key the parameter belongs to, if it belongs to one.
    pub key: i16,
    /// The value the parameter is now.
    pub value: f64,
}

/// How many beats a beat-time is in.
///
/// The format writes a position in beats as a **fixed-point** number:
/// the position multiplied by this, so that a position keeps its
/// fraction of a beat.
pub const BEATTIME_FACTOR: i64 = 1 << 31;

/// How many seconds a second-time is in.
pub const SECTIME_FACTOR: i64 = 1 << 31;

/// The transport says there is a tempo.
pub const TRANSPORT_HAS_TEMPO: u32 = 1 << 0;

/// The transport says where the music is, in beats.
pub const TRANSPORT_HAS_BEATS_TIMELINE: u32 = 1 << 1;

/// The transport says where the music is, in seconds.
pub const TRANSPORT_HAS_SECONDS_TIMELINE: u32 = 1 << 2;

/// The transport says what the time signature is.
pub const TRANSPORT_HAS_TIME_SIGNATURE: u32 = 1 << 3;

/// The transport says the music is playing.
pub const TRANSPORT_IS_PLAYING: u32 = 1 << 4;

/// The transport says the music is being recorded.
pub const TRANSPORT_IS_RECORDING: u32 = 1 << 5;

/// The transport says the loop is running.
pub const TRANSPORT_IS_LOOP_ACTIVE: u32 = 1 << 6;

/// A **transport** event: where the music is.
///
/// This is the event that puts a plugin **in time**: the position of
/// the music in beats and in seconds, the tempo, the time signature,
/// and whether it is playing.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapEventTransport {
    /// The head every event has.
    pub header: ClapEventHeader,
    /// What the transport says: which of the fields below are there.
    pub flags: u32,
    /// Where the music is, in beats, as a fixed-point number.
    pub song_pos_beats: i64,
    /// Where the music is, in seconds, as a fixed-point number.
    pub song_pos_seconds: i64,
    /// The tempo, in beats a minute.
    pub tempo: f64,
    /// How much the tempo is changing, in beats a minute a second.
    pub tempo_inc: f64,
    /// Where the loop starts, in beats.
    pub loop_start_beats: i64,
    /// Where the loop ends, in beats.
    pub loop_end_beats: i64,
    /// Where the loop starts, in seconds.
    pub loop_start_seconds: i64,
    /// Where the loop ends, in seconds.
    pub loop_end_seconds: i64,
    /// Where the bar starts, in beats.
    pub bar_start: i64,
    /// Which bar it is.
    pub bar_number: i32,
    /// The top of the time signature.
    pub tsig_num: u16,
    /// The bottom of the time signature.
    pub tsig_denom: u16,
}

/// The id of the **parameters** extension.
pub const PARAMS_EXTENSION: &str = "clap.params";

/// How long a parameter's name may be, in bytes.
pub const NAME_SIZE: usize = 256;

/// How long a parameter's module path may be, in bytes.
pub const PATH_SIZE: usize = 1024;

/// What one parameter of a plugin is.
///
/// The name and the module are **fixed-size** arrays, not pointers: the
/// format says a parameter's name is a short, fixed thing, and a
/// host that has to allocate for every parameter it reads is a host
/// that allocates on the audio thread.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapParamInfo {
    /// The parameter's id: the name a host asks by.
    pub id: u32,
    /// What the parameter is: stepped, hidden, read-only, and so on.
    pub flags: u32,
    /// The plugin's own state for this parameter.
    pub cookie: *mut c_void,
    /// The parameter's name, for a person.
    pub name: [c_char; NAME_SIZE],
    /// The module the parameter belongs to.
    pub module: [c_char; PATH_SIZE],
    /// The smallest value the parameter takes.
    pub min_value: f64,
    /// The largest value the parameter takes.
    pub max_value: f64,
    /// The value the parameter starts at.
    pub default_value: f64,
}

/// The **parameters** a plugin holds.
///
/// The plugin says how many there are, what each of them is, and what
/// each of them is set to. Setting one is not here: the format sets a
/// parameter by **offering an event**, and that is a later arc.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapPluginParams {
    /// How many parameters the plugin holds.
    pub count: Option<extern "C" fn(plugin: *const ClapPlugin) -> u32>,
    /// What the parameter at that index is.
    pub get_info: Option<
        extern "C" fn(plugin: *const ClapPlugin, param_index: u32, param_info: *mut ClapParamInfo) -> bool,
    >,
    /// What the parameter with that id is set to.
    pub get_value:
        Option<extern "C" fn(plugin: *const ClapPlugin, param_id: u32, out_value: *mut f64) -> bool>,
    /// The parameter's value as text.
    pub value_to_text: Option<
        extern "C" fn(
            plugin: *const ClapPlugin,
            param_id: u32,
            value: f64,
            out_buffer: *mut c_char,
            out_buffer_capacity: u32,
        ) -> bool,
    >,
    /// The value the text means.
    pub text_to_value: Option<
        extern "C" fn(
            plugin: *const ClapPlugin,
            param_id: u32,
            param_value_text: *const c_char,
            out_value: *mut f64,
        ) -> bool,
    >,
    /// The plugin has new values to give.
    pub flush: Option<
        extern "C" fn(plugin: *const ClapPlugin, in_: *const ClapInputEvents, out: *const ClapOutputEvents),
    >,
}

/// The **host's** parameters: what a plugin may ask of the host.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapHostParams {
    /// The plugin's parameters have changed: ask it again.
    pub rescan: Option<extern "C" fn(host: *const ClapHost, flags: u32)>,
    /// Forget the automations of a parameter.
    pub clear: Option<extern "C" fn(host: *const ClapHost, param_id: u32, flags: u32)>,
    /// The host wants the plugin's values.
    pub request_flush: Option<extern "C" fn(host: *const ClapHost)>,
}

/// The **audio buffer**: the channels of one port.
///
/// The channels are **planar**: each one is its own array of samples,
/// and the buffer holds the addresses of those arrays.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapAudioBuffer {
    /// The channels, as thirty-two bit floats.
    pub data32: *mut *mut f32,
    /// The channels, as sixty-four bit floats: the format's other
    /// half, and this kit does not use it.
    pub data64: *mut *mut f64,
    /// How many channels the port has.
    pub channel_count: u32,
    /// How many samples of delay the port has.
    pub latency: u32,
    /// Which channels hold the same value all the way through.
    pub constant_mask: u64,
}

/// The **input events**: what the plugin reads while it works.
///
/// The list is read through two functions: how many events there are,
/// and the event at that index. A host with no events answers zero and
/// nothing.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapInputEvents {
    /// The host's own state, which the two functions read.
    pub ctx: *mut c_void,
    /// How many events there are.
    pub size: Option<extern "C" fn(list: *const ClapInputEvents) -> u32>,
    /// The event at that index.
    pub get: Option<extern "C" fn(list: *const ClapInputEvents, index: u32) -> *const c_void>,
}

/// The **output events**: what the plugin writes while it works.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapOutputEvents {
    /// The host's own state, which the function reads.
    pub ctx: *mut c_void,
    /// Offer an event to the host.
    pub try_push: Option<extern "C" fn(list: *const ClapOutputEvents, event: *const c_void) -> bool>,
}

/// The **process**: one block of audio, going through.
///
/// This is the shape of the call the plugin is made for: the frames,
/// the ports, and the events that go with them.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapProcess {
    /// The host's steady clock, in nanoseconds.
    pub steady_time: i64,
    /// How many frames the block holds.
    pub frames_count: u32,
    /// The transport, if the host has one.
    pub transport: *const c_void,
    /// The input ports.
    pub audio_inputs: *const ClapAudioBuffer,
    /// The output ports.
    pub audio_outputs: *mut ClapAudioBuffer,
    /// How many input ports there are.
    pub audio_inputs_count: u32,
    /// How many output ports there are.
    pub audio_outputs_count: u32,
    /// The events going in.
    pub in_events: *const ClapInputEvents,
    /// The events coming out.
    pub out_events: *const ClapOutputEvents,
}

/// The **descriptor** of a plugin: what the host knows about it before
/// it makes it.
///
/// Every string is the plugin's own C string, and the host copies them
/// out at once: a string that is read later may be gone by then.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ClapPluginDescriptor {
    /// The version the plugin speaks.
    pub clap_version: ClapVersion,
    /// The plugin's id: the name the host asks for by.
    pub id: *const c_char,
    /// The plugin's name, for a person.
    pub name: *const c_char,
    /// Who made it.
    pub vendor: *const c_char,
    /// Where it lives.
    pub url: *const c_char,
    /// Its manual.
    pub manual_url: *const c_char,
    /// Where to ask for help.
    pub support_url: *const c_char,
    /// Its own version.
    pub version: *const c_char,
    /// A line about it.
    pub description: *const c_char,
    /// The features it has: a list of C strings, ending at a null.
    pub features: *const *const c_char,
}
