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
