//! `craftkit-plugin-shared` — a plugin that is **not part of the
//! program**: a shared library with an entry point.
//!
//! # What it is
//!
//! This crate is the **other side of the door**. It is built as a
//! `cdylib`, it exports the table by name — `craftkit_plugin_vtable` —
//! and the host finds it with `dlsym` and calls it.
//!
//! It is the shape every out-of-process plugin has: the outer tree's
//! Android package exports `android_main` and the loader looks it up by
//! name; a CLAP, a VST3 or an AU plugin would export its own factory the
//! same way. What this crate adds to that shape is the **version**: the
//! table says which contract it speaks, and the host refuses a table it
//! does not understand rather than calling it and hoping.
//!
//! # Why it is `unsafe`
//!
//! One attribute: `#[unsafe(no_mangle)]`. The host looks the entry point
//! up **by name**, so the symbol must have that name and no other, and a
//! name that is not mangled is the one thing `unsafe` is used for here.
//! It is the second kind of exception in the tree — the first was the
//! counting allocator, the second is the loader, and this is the third:
//! an exported name.
//!
//! Everything else in this crate is ordinary safe Rust: the table is
//! data, and the functions behind it are ordinary functions.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
// **Not** `forbid`: the export below needs one `unsafe` attribute, and
// the gate in `craftkit-alloc` holds the list of crates that may have
// one.
#![deny(unsafe_code)]

use std::ffi::{CString, c_char};

use craftkit_ffi_abi::{ABI_VERSION, PluginVtable};

/// The plugins this library holds.
const PLUGINS: [(&str, &str); 2] =
    [("the demo", r#"{"kind":"demo","channels":2}"#), ("the second", r#"{"kind":"demo","channels":1}"#)];

/// How many plugins there are.
extern "C" fn plugin_count() -> u32 {
    PLUGINS.len() as u32
}

/// The name of one of them.
///
/// The string is **built once** and kept: the host copies it out, and
/// the plugin does not free it, because the plugin's memory outlives
/// the call and the host does not own it.
extern "C" fn plugin_name(index: u32) -> *const c_char {
    name_at(index).map(|name| name.as_ptr()).unwrap_or(std::ptr::null())
}

/// The parameters of one of them, as JSON.
extern "C" fn plugin_params(index: u32) -> *const c_char {
    params_at(index).map(|params| params.as_ptr()).unwrap_or(std::ptr::null())
}

/// The names, kept alive for as long as the library is.
fn names() -> &'static Vec<CString> {
    use std::sync::OnceLock;
    static NAMES: OnceLock<Vec<CString>> = OnceLock::new();
    NAMES.get_or_init(|| PLUGINS.iter().map(|(name, _)| CString::new(*name).unwrap_or_default()).collect())
}

/// The parameters, kept alive for as long as the library is.
fn params() -> &'static Vec<CString> {
    use std::sync::OnceLock;
    static PARAMS: OnceLock<Vec<CString>> = OnceLock::new();
    PARAMS
        .get_or_init(|| PLUGINS.iter().map(|(_, params)| CString::new(*params).unwrap_or_default()).collect())
}

/// The name at that index.
fn name_at(index: u32) -> Option<&'static CString> {
    names().get(index as usize)
}

/// The parameters at that index.
fn params_at(index: u32) -> Option<&'static CString> {
    params().get(index as usize)
}

/// The table: the whole contract.
static VTABLE: PluginVtable =
    PluginVtable { abi_version: ABI_VERSION, plugin_count, plugin_name, plugin_params };

/// The **exports**: the one part of this crate that is `unsafe`.
///
/// The name is not mangled because the host cannot know what the
/// compiler would have called it, and the host looks the entry point up
/// **by name**.
#[allow(unsafe_code)]
mod exports {
    use craftkit_ffi_abi::PluginVtable;

    /// The entry point.
    #[unsafe(no_mangle)]
    pub extern "C" fn craftkit_plugin_vtable() -> *const PluginVtable {
        &super::VTABLE as *const PluginVtable
    }
}
