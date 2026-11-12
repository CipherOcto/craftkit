//! `craftkit-ffi-abi` — the **C ABI** a craftkit plugin speaks.
//!
//! # What this is
//!
//! A plugin that is not part of the program is a **shared library**, and
//! the program cannot know what is inside it. So the two sides agree on
//! a **table**: a struct of function pointers, with a version at the
//! front, exported under a name the host looks up.
//!
//! This crate is that table, and it is **the whole contract**: it holds
//! no `unsafe` at all, because a table of function pointers is a safe
//! thing to write down. The unsafe is on the two sides of it — the host
//! that opens the library and finds the table (`craftkit-ffi`), and the
//! plugin that exports it (`craftkit-plugin-shared`).
//!
//! # The shape
//!
//! ```text
//! struct PluginVtable {
//!     u32               abi_version;   // the contract's version
//!     fn() -> u32      plugin_count;  // how many plugins
//!     fn(u32) -> char* plugin_name;   // the name of one of them
//!     fn(u32) -> char* plugin_params; // its parameters, as JSON
//! }
//! ```
//!
//! The strings come back as C strings: a `char*` that is **not** a
//! `String`, and the host reads it. That reading is the host's unsafe,
//! and it is the host's business.
//!
//! # Why craftkit's own table, and not CLAP's
//!
//! The real plugin formats — CLAP, VST3, AU — each have their own ABI,
//! and each is a large surface written in a language this kit does not
//! speak. Porting one of them is not a port of a **document model**; it
//! is a port of a **binary contract**, and it is much larger.
//!
//! What is built here is the **door**: a host that opens a shared
//! library, finds the table by name, checks the version, and calls it.
//! A format would arrive as a **shim** — a thin piece of the format's
//! own language that exposes this table — and nothing above the door
//! would know the difference. The door is the same one the Android
//! package uses: a shared library, an entry point, looked up by name.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::ffi::c_char;

/// The version of the contract.
///
/// The host checks it before it calls anything, and refuses a table it
/// does not understand. A plugin built against a different version is
/// **not** called: the numbers would mean different things.
pub const ABI_VERSION: u32 = 1;

/// The name the host looks the entry point up under.
///
/// It is a name and not an address, because the host does not know where
/// the library will be loaded.
pub const ENTRY_POINT: &str = "craftkit_plugin_vtable";

/// The **entry point**: what the host finds by name.
///
/// It is a function and not the table itself, because the host cannot
/// know where in the library the table is. The function gives it the
/// address, and the function is the only thing the two sides must agree
/// on by name.
pub type EntryPoint = extern "C" fn() -> *const PluginVtable;

/// The table: the whole of the contract between a host and a plugin.
///
/// It is a plain old data structure: `repr(C)` says the layout is the C
/// one, so a table written in another language has the same shape, and
/// the two sides agree without agreeing on a compiler.
#[repr(C)]
pub struct PluginVtable {
    /// The version of this table.
    pub abi_version: u32,
    /// How many plugins the library holds.
    pub plugin_count: extern "C" fn() -> u32,
    /// The name of the plugin at that index, as a C string.
    pub plugin_name: extern "C" fn(index: u32) -> *const c_char,
    /// The parameters of the plugin at that index, as a C string holding
    /// JSON.
    pub plugin_params: extern "C" fn(index: u32) -> *const c_char,
}
