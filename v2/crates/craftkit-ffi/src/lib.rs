//! `craftkit-ffi` — the **door**: a host that opens a shared library and
//! calls the plugin table inside it.
//!
//! # Why it is here
//!
//! A plugin that is not part of the program is a **shared library**. The
//! program cannot know what is inside it, so the two sides agree on a
//! **table** — [`craftkit_ffi_abi::PluginVtable`] — and the host finds it
//! **by name**.
//!
//! This is the same door the outer tree's Android package uses: the
//! loader opens the library and looks up `android_main` by name. It is
//! also the door the real plugin formats would come through: a CLAP, a
//! VST3 or an AU plugin would arrive as a **shim** — a thin piece of
//! that format's own language, exposing this table — and nothing above
//! the door would know the difference.
//!
//! # Why it is `unsafe`
//!
//! Opening a library, finding a symbol in it and calling it is `unsafe`
//! by definition: the host is calling code it has not read, at an
//! address it did not choose. So this crate is the **second** exception
//! in v2, and it is held to the same rules as the first:
//!
//! - it is **one crate**, and it does one thing;
//! - the crate root says `deny(unsafe_code)` and **not** `forbid`, so
//!   the exception is explicit;
//! - **every** `unsafe` in it is inside **one module** — `foreign` —
//!   that carries **one** `allow(unsafe_code)`, and every unsafe block in
//!   it says, in a comment, why it is sound;
//! - a **gate** in the tree scans every crate and refuses `unsafe`
//!   anywhere else, so the list of exceptions is one list and it is
//!   short.
//!
//! # What it does **not** do
//!
//! It does not know what a plugin *is*. It does not have a plugin
//! format in it, and it does not pretend to: the formats are a much
//! larger piece of work, and they would sit **above** this door.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
// **Not** `forbid`. There is an exception below, it is marked, and the
// gate in `craftkit-alloc` holds the list: it reads the whole workspace
// and refuses `unsafe` in any crate that is not on it.
#![deny(unsafe_code)]

use craftkit_ffi_abi::{ABI_VERSION, PluginVtable};

/// What went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfiError {
    /// What happened, in words.
    pub message: String,
}

impl FfiError {
    /// A failure with a message.
    fn new(message: impl Into<String>) -> FfiError {
        FfiError { message: message.into() }
    }
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for FfiError {}

/// The result of a call across the door.
pub type Result<T> = std::result::Result<T, FfiError>;

/// A **shared library**, open.
///
/// The library is closed when this is dropped, and it is closed exactly
/// once: the drop is the only place that closes it.
#[derive(Debug)]
pub struct Library {
    /// The handle the operating system gave us.
    ///
    /// It is an **address** and not a pointer: the loader's handle is
    /// opaque, and keeping it as a number means the library is `Send`
    /// and `Sync` like any other piece of session state, with no
    /// `unsafe impl` anywhere to say so.
    handle: usize,
}

impl Library {
    /// Open a shared library.
    ///
    /// The path is the library's own path: a file on disk, or a name the
    /// loader searches for.
    pub fn open(path: &str) -> Result<Library> {
        let handle = foreign::open(path).map_err(FfiError::new)?;
        Ok(Library { handle })
    }

    /// The **plugin table** inside this library.
    ///
    /// The table is found **by name**, and its version is checked before
    /// anything in it is called: a table this host does not understand is
    /// refused, not used.
    pub fn plugins(&self) -> Result<Plugins<'_>> {
        let (vtable, version) = foreign::vtable(self.handle).map_err(FfiError::new)?;
        if version != ABI_VERSION {
            return Err(FfiError::new(format!(
                "the plugin speaks version {version} and this host speaks version {ABI_VERSION}"
            )));
        }
        Ok(Plugins { vtable, _library: std::marker::PhantomData })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        foreign::close(self.handle);
    }
}

/// The **plugins** inside a library.
///
/// The table is the plugin's, and the calls are the plugin's functions:
/// they are safe to call because the contract says what they do, and
/// the strings they return are the plugin's to keep.
#[derive(Debug)]
pub struct Plugins<'a> {
    /// The table.
    vtable: *const PluginVtable,
    /// The library it is in, so that it cannot outlive it.
    _library: std::marker::PhantomData<&'a Library>,
}

impl Plugins<'_> {
    /// How many plugins there are.
    pub fn count(&self) -> u32 {
        foreign::plugin_count(self.vtable)
    }

    /// The name of the plugin at that index.
    pub fn name(&self, index: u32) -> Result<String> {
        let pointer = foreign::plugin_name(self.vtable, index);
        foreign::read_string(pointer).map_err(FfiError::new)
    }

    /// The parameters of the plugin at that index, as JSON.
    pub fn params(&self, index: u32) -> Result<String> {
        let pointer = foreign::plugin_params(self.vtable, index);
        foreign::read_string(pointer).map_err(FfiError::new)
    }
}

/// The **foreign** part: the only `unsafe` in this crate.
///
/// Everything the operating system and the plugin's own strings go
/// through is here, behind **one** `allow(unsafe_code)`. The rest of the
/// crate is safe Rust that calls into this module.
///
/// The rules this module keeps:
///
/// - a name that goes to the operating system is a **C string**: no zero
///   byte in it, and it is freed by nobody;
/// - a handle that comes back is either null (which is an error) or a
///   handle, and it is closed exactly once, by the drop;
/// - a string that comes back from the plugin is **copied out** at once,
///   so that nothing in this program points into the plugin's memory
///   after the call.
#[cfg(unix)]
#[allow(unsafe_code)]
mod foreign {
    use std::ffi::{CStr, c_char, c_void};

    use craftkit_ffi_abi::{ENTRY_POINT, EntryPoint, PluginVtable};

    /// Resolve the library completely before it is used.
    pub const RTLD_NOW: i32 = 2;
    /// Do not offer this library's symbols to anything else.
    pub const RTLD_LOCAL: i32 = 0;

    // The loader's doors. They are `unsafe` to call because the
    // operating system trusts the caller: a name that is not a string,
    // or a handle that is not a handle, is a crash.
    unsafe extern "C" {
        /// Open a shared library.
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        /// Find a symbol in an open library.
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        /// Close an open library.
        fn dlclose(handle: *mut c_void) -> i32;
        /// The last error the loader had.
        fn dlerror() -> *mut c_char;
    }

    /// Open a shared library and give back its handle.
    pub fn open(path: &str) -> Result<usize, String> {
        let name = match std::ffi::CString::new(path) {
            Ok(name) => name,
            Err(_) => return Err(format!("the path `{path}` has a zero byte in it")),
        };
        // SAFETY: the name is a valid C string with no zero byte in it,
        // and the flags are `RTLD_NOW` and `RTLD_LOCAL`: the library is
        // resolved completely before it is used, and its symbols are not
        // offered to anything else that is loaded.
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        if handle.is_null() {
            return Err(format!("the library `{path}` did not open: {}", last_error()));
        }
        Ok(handle as usize)
    }

    /// Close a library.
    pub fn close(handle: usize) {
        // SAFETY: the handle came from `dlopen` and has not been
        // closed, and this is called once, from the drop.
        unsafe { dlclose(handle as *mut c_void) };
    }

    /// The plugin's table, found **by name**, and the version it says.
    pub fn vtable(handle: usize) -> Result<(*const PluginVtable, u32), String> {
        let wanted = match std::ffi::CString::new(ENTRY_POINT) {
            Ok(wanted) => wanted,
            Err(_) => return Err("the entry point's name is not a C string".to_string()),
        };
        // SAFETY: the handle is open, and the name is a valid C string
        // with no zero byte in it. The result is a pointer into the
        // library, which stays open as long as the handle does.
        let symbol = unsafe { dlsym(handle as *mut c_void, wanted.as_ptr()) };
        if symbol.is_null() {
            return Err(format!("the library has no `{ENTRY_POINT}`: {}", last_error()));
        }
        // SAFETY: the symbol is the plugin's entry point, and the
        // contract says it is a function that gives the table's address.
        // A data pointer and a function pointer are the same size on
        // every target this runs on.
        let entry: EntryPoint = unsafe { std::mem::transmute(symbol) };
        // The function is the plugin's, and it gives back the table's
        // address and does nothing else. Calling it is safe: the
        // contract says what it does.
        let vtable = entry();
        if vtable.is_null() {
            return Err("the plugin table is not there".to_string());
        }
        // SAFETY: the symbol is the plugin's entry point, and the
        // contract says it is a pointer to the table. The version is
        // read first, and the table is only used when the version is one
        // this host knows.
        let version = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*vtable).abi_version)) };
        Ok((vtable, version))
    }

    /// How many plugins the table holds.
    pub fn plugin_count(vtable: *const PluginVtable) -> u32 {
        // SAFETY: the table was checked when it was found, and its
        // `plugin_count` is a function pointer the plugin wrote.
        unsafe { ((*vtable).plugin_count)() }
    }

    /// The name of one plugin, as the plugin's own C string.
    pub fn plugin_name(vtable: *const PluginVtable, index: u32) -> *const c_char {
        // SAFETY: the table was checked when it was found, and
        // `plugin_name` is a function pointer the plugin wrote.
        unsafe { ((*vtable).plugin_name)(index) }
    }

    /// The parameters of one plugin, as the plugin's own C string.
    pub fn plugin_params(vtable: *const PluginVtable, index: u32) -> *const c_char {
        // SAFETY: the table was checked when it was found, and
        // `plugin_params` is a function pointer the plugin wrote.
        unsafe { ((*vtable).plugin_params)(index) }
    }

    /// A C string the plugin gave us, as a Rust string.
    pub fn read_string(pointer: *const c_char) -> Result<String, String> {
        if pointer.is_null() {
            return Err("the plugin gave no string".to_string());
        }
        // SAFETY: the plugin promised a C string: a zero byte ends it,
        // and the bytes before it are text. The string is copied out
        // here, so the copy does not point into the plugin's memory.
        let text = unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned();
        Ok(text)
    }

    /// The last error the loader had.
    fn last_error() -> String {
        // SAFETY: `dlerror` returns either a pointer to a message or
        // null, and the message belongs to the loader.
        let pointer = unsafe { dlerror() };
        if pointer.is_null() {
            return "the loader said nothing".to_string();
        }
        // SAFETY: the loader's message is a C string, and it is copied
        // out here, so the copy does not point into the loader's memory.
        unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned()
    }
}

/// The **foreign** part, where there is no foreign part.
///
/// There is no shared library to open on this target, so every door
/// here refuses — and it says so, rather than pretending to work.
#[cfg(not(unix))]
mod foreign {
    use std::ffi::{c_char, c_void};

    use craftkit_ffi_abi::PluginVtable;

    /// Open a shared library: not here.
    pub fn open(path: &str) -> Result<usize, String> {
        Err(format!("a shared library cannot be opened on this target: `{path}`"))
    }

    /// Close a shared library: nothing was opened.
    pub fn close(_handle: usize) {}

    /// The plugin's table: there is no library to look in.
    pub fn vtable(_handle: usize) -> Result<(*const PluginVtable, u32), String> {
        Err("there is no shared library on this target".to_string())
    }

    /// How many plugins: none.
    pub fn plugin_count(_vtable: *const PluginVtable) -> u32 {
        0
    }

    /// The name of a plugin: there is none.
    pub fn plugin_name(_vtable: *const PluginVtable, _index: u32) -> *const c_char {
        std::ptr::null()
    }

    /// The parameters of a plugin: there are none.
    pub fn plugin_params(_vtable: *const PluginVtable, _index: u32) -> *const c_char {
        std::ptr::null()
    }

    /// A string the plugin gave us: there is no plugin.
    pub fn read_string(_pointer: *const c_char) -> Result<String, String> {
        Err("there is no plugin on this target".to_string())
    }
}
