//! `craftkit-clap` — the **CLAP host**: a program that opens a CLAP
//! plugin and asks it what it holds.
//!
//! # Why it is here
//!
//! A CLAP plugin is a **shared library** written in C, and the host
//! cannot know what is inside it. The two sides agree on a **binary
//! contract** — the shapes in [`craftkit_clap_abi`] — and the host
//! finds the plugin's entry **by name**.
//!
//! This crate is the host side of that contract: it opens the library,
//! finds `clap_entry`, checks the version, initialises the plugin, asks
//! for the factory, and reads the descriptors out of it.
//!
//! # Why it is `unsafe`
//!
//! Opening a library, reading a symbol out of it and calling through
//! pointers the plugin wrote is `unsafe` by definition: the host is
//! reading memory it did not write, at addresses it did not choose. So
//! this crate is on v2's **list** of exceptions — the fourth entry —
//! and it is held to the same rules as the others:
//!
//! - it is **one crate**, and it does one thing;
//! - the crate root says `deny(unsafe_code)` and **not** `forbid`, so
//!   the exception is explicit;
//! - **every** `unsafe` in it is inside **one module** — `foreign` —
//!   that carries **one** `allow(unsafe_code)`, and every unsafe block
//!   in it says, in a comment, why it is sound;
//! - a **gate** in the tree scans every crate and refuses `unsafe`
//!   anywhere else, so the list of exceptions is one list and it is
//!   short.
//!
//! # The one thing that is different from craftkit's own door
//!
//! Craftkit's own door finds a **function** and calls it. CLAP exports
//! a **data** symbol: `clap_entry` is a *variable* whose value is the
//! address of the entry struct. So the host reads the pointer **at**
//! the symbol's address, and then reads the struct **at** that
//! pointer. Two reads, one address each, and both of them are the
//! host's unsafe.
//!
//! # What it does **not** do
//!
//! It does not make a plugin and it does not run audio. It reads what
//! a plugin says about itself: the version, and the descriptors. The
//! making of a plugin needs a **host** to hand over — a structure this
//! kit does not have yet — and that is a larger piece of work, in the
//! same way that the audio path is.
//!
//! # The bounds
//!
//! A plugin is code the host did not write, so the host reads every
//! string **bounded**: a string longer than `MAX_STRING_BYTES` is
//! refused, and a feature list longer than `MAX_FEATURES` is refused.
//! Both are named in this tree's README, because a bound a reader
//! cannot look up is a bound a reader cannot check.

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

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

use craftkit_clap_abi::{
    ClapHost, ClapPlugin as ClapPluginInstance, ClapPluginEntry, ClapPluginFactory, ClapProcessStatus,
    ClapVersion,
};

/// The host's name, for a plugin that wants to say who it is to.
const HOST_NAME: &[u8] = b"craftkit\0";
/// Who made the host.
const HOST_VENDOR: &[u8] = b"Craftkit\0";
/// Where the host lives.
const HOST_URL: &[u8] = b"https://example.invalid/craftkit\0";
/// The host's own version.
const HOST_VERSION: &[u8] = b"0.2.0\0";

/// What went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClapError {
    /// What happened, in words.
    pub message: String,
}

impl ClapError {
    /// A failure with a message.
    fn new(message: impl Into<String>) -> ClapError {
        ClapError { message: message.into() }
    }
}

impl std::fmt::Display for ClapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ClapError {}

/// The result of talking to a plugin.
pub type Result<T> = std::result::Result<T, ClapError>;

/// A **CLAP plugin**, open.
///
/// The plugin is initialised at most once, and it is deinitialised
/// when this is dropped — before the library is closed, because a
/// plugin's `deinit` runs code that is still in the library.
#[derive(Debug)]
pub struct ClapPlugin {
    /// The handle the operating system gave us.
    handle: usize,
    /// The plugin's entry, inside the library.
    entry: *const ClapPluginEntry,
    /// The path the library was opened with: the plugin's `init` wants
    /// it, because a plugin may look beside itself for its own files.
    path: String,
    /// Whether the plugin has been initialised.
    initialised: Cell<bool>,
}

impl ClapPlugin {
    /// Open a CLAP plugin.
    ///
    /// The path is the library's own path: a file on disk, or a name
    /// the loader searches for. The plugin's version is checked before
    /// anything in it is called — a plugin this host does not
    /// understand is refused, not used.
    pub fn open(path: &str) -> Result<ClapPlugin> {
        let handle = foreign::open(path).map_err(ClapError::new)?;
        let entry = match foreign::entry(handle) {
            Ok(entry) => entry,
            Err(error) => {
                // The library is open and nothing else will be done with
                // it, so it is closed here: a library that is opened and
                // then forgotten is a library that is never unloaded.
                foreign::close(handle);
                return Err(ClapError::new(error));
            }
        };
        let version = foreign::version(entry);
        if !version.is_compatible_with(ClapVersion::CURRENT) {
            foreign::close(handle);
            return Err(ClapError::new(format!(
                "the plugin speaks version {}.{}.{} and this host speaks version {}.{}.{}",
                version.major,
                version.minor,
                version.revision,
                ClapVersion::CURRENT.major,
                ClapVersion::CURRENT.minor,
                ClapVersion::CURRENT.revision
            )));
        }
        Ok(ClapPlugin { handle, entry, path: path.to_string(), initialised: Cell::new(false) })
    }

    /// The version the plugin speaks.
    pub fn version(&self) -> ClapVersion {
        foreign::version(self.entry)
    }

    /// The path the plugin was opened with.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Initialise the plugin.
    ///
    /// The format asks the host to call `init` once, with the plugin's
    /// own path, before anything else is asked of it. Calling it twice
    /// is refused: the format does not say what happens then, and a
    /// host that guesses is a host that breaks somebody's plugin.
    pub fn init(&self) -> Result<()> {
        if self.initialised.get() {
            return Err(ClapError::new("the plugin is already initialised"));
        }
        foreign::init(self.entry, &self.path).map_err(ClapError::new)?;
        self.initialised.set(true);
        Ok(())
    }

    /// The **plugin factory**: the thing that holds the plugins.
    ///
    /// The plugin must be initialised first: the format says the
    /// factory is asked for after `init`, and the host keeps that
    /// order rather than hoping the plugin does not care.
    pub fn factory(&self) -> Result<ClapFactory<'_>> {
        if !self.initialised.get() {
            return Err(ClapError::new("the plugin has not been initialised"));
        }
        let factory = foreign::factory(self.entry).map_err(ClapError::new)?;
        Ok(ClapFactory { factory, _plugin: PhantomData })
    }
}

impl Drop for ClapPlugin {
    fn drop(&mut self) {
        if self.initialised.get() {
            foreign::deinit(self.entry);
        }
        foreign::close(self.handle);
    }
}

/// The **factory** of a plugin: the plugins it holds.
///
/// It cannot outlive the plugin it came from: the factory lives inside
/// the library, and the library is closed when the plugin is dropped.
#[derive(Debug)]
pub struct ClapFactory<'a> {
    /// The factory, inside the library.
    factory: *const ClapPluginFactory,
    /// The plugin it is in, so that it cannot outlive it.
    _plugin: PhantomData<&'a ClapPlugin>,
}

impl ClapFactory<'_> {
    /// How many plugins the factory holds.
    pub fn count(&self) -> u32 {
        foreign::factory_count(self.factory)
    }

    /// The descriptor of the plugin at that index.
    ///
    /// Everything in a descriptor is **copied out** at once: the
    /// strings belong to the plugin, and a string read later may be
    /// gone by then.
    pub fn descriptor(&self, index: u32) -> Result<ClapDescriptor> {
        let descriptor = foreign::factory_descriptor(self.factory, index).map_err(ClapError::new)?;
        foreign::descriptor(descriptor).map_err(ClapError::new)
    }

    /// **Make** the plugin with that id.
    ///
    /// The plugin is given a **host**: the structure it reads to learn
    /// who it is running in, and the one it calls when it wants
    /// something. The host is this program's own, and it is boxed, so
    /// that its address does not move while the plugin holds it.
    pub fn create(&self, id: &str) -> Result<ClapInstance<'_>> {
        let state = Box::new(HostState::default());
        let host = Box::new(ClapHost {
            clap_version: ClapVersion::CURRENT,
            host_data: &*state as *const HostState as *mut std::ffi::c_void,
            name: HOST_NAME.as_ptr() as *const std::ffi::c_char,
            vendor: HOST_VENDOR.as_ptr() as *const std::ffi::c_char,
            url: HOST_URL.as_ptr() as *const std::ffi::c_char,
            version: HOST_VERSION.as_ptr() as *const std::ffi::c_char,
            get_extension: Some(foreign::get_extension),
            request_restart: Some(foreign::request_restart),
            request_process: Some(foreign::request_process),
            request_callback: Some(foreign::request_callback),
        });
        let plugin = foreign::create_plugin(self.factory, &*host, id).map_err(ClapError::new)?;
        Ok(ClapInstance { plugin, _host: host, state, _factory: PhantomData })
    }
}

/// What the plugin asked the host for.
///
/// A plugin cannot speak to the program it is in except by asking:
/// these are the three things it may ask for, and the host answers
/// each of them by **noting** it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HostRequests {
    /// The plugin asked the host to restart it.
    pub restart: bool,
    /// The plugin asked the host to call it back.
    pub process: bool,
    /// The plugin asked the host to call it on its main thread.
    pub callback: bool,
}

/// The host's own state, behind the plugin's back.
///
/// It is in a box because the plugin holds a pointer to it, and a
/// pointer to a thing that moves is a pointer to nothing.
#[derive(Debug, Default)]
struct HostState {
    restart: AtomicBool,
    process: AtomicBool,
    callback: AtomicBool,
}

/// A **block of audio**: what goes into the plugin and what comes out.
///
/// The channels are **planar** — one vector a channel — because that
/// is how the family's own audio is laid out, and a host that copied
/// the format's interleaved shape would have to un-interleave every
/// block.
#[derive(Debug, Clone, PartialEq)]
pub struct ClapBlock {
    /// The channels going in.
    pub inputs: Vec<Vec<f32>>,
    /// The channels coming out.
    pub outputs: Vec<Vec<f32>>,
}

impl ClapBlock {
    /// A block of that many frames, with that many channels each way.
    pub fn new(channels: usize, frames: usize) -> ClapBlock {
        ClapBlock { inputs: vec![vec![0.0; frames]; channels], outputs: vec![vec![0.0; frames]; channels] }
    }
}

/// A **plugin instance**: a plugin that has been made.
///
/// The instance is the plugin's own: it has its own state, and it is
/// destroyed by the host when the host is finished with it. It cannot
/// outlive the factory that made it, because the factory lives in the
/// library and the library is closed when the plugin is dropped.
#[derive(Debug)]
pub struct ClapInstance<'a> {
    /// The instance, inside the library.
    plugin: *const ClapPluginInstance,
    /// The host, boxed: the plugin holds a pointer to it, so it has to
    /// live as long as the instance does.
    _host: Box<ClapHost>,
    /// The host's state, boxed: the plugin holds a pointer to it too.
    state: Box<HostState>,
    /// The factory it came from, so that it cannot outlive it.
    _factory: PhantomData<&'a ClapFactory<'a>>,
}

impl ClapInstance<'_> {
    /// What the plugin says about itself.
    pub fn descriptor(&self) -> Result<ClapDescriptor> {
        let descriptor = foreign::plugin_descriptor(self.plugin).map_err(ClapError::new)?;
        foreign::descriptor(descriptor).map_err(ClapError::new)
    }

    /// The instance is ready to work.
    pub fn init(&self) -> Result<()> {
        foreign::plugin_init(self.plugin).map_err(ClapError::new)
    }

    /// The instance is to work at that rate.
    pub fn activate(&self, sample_rate: f64, min_frames: u32, max_frames: u32) -> Result<()> {
        foreign::plugin_activate(self.plugin, sample_rate, min_frames, max_frames).map_err(ClapError::new)
    }

    /// The instance is to stop working.
    pub fn deactivate(&self) -> Result<()> {
        foreign::plugin_deactivate(self.plugin).map_err(ClapError::new)
    }

    /// The host starts to send audio.
    pub fn start_processing(&self) -> Result<()> {
        foreign::plugin_start_processing(self.plugin).map_err(ClapError::new)
    }

    /// The host stops sending audio.
    pub fn stop_processing(&self) -> Result<()> {
        foreign::plugin_stop_processing(self.plugin).map_err(ClapError::new)
    }

    /// The instance is to forget its audio.
    pub fn reset(&self) -> Result<()> {
        foreign::plugin_reset(self.plugin).map_err(ClapError::new)
    }

    /// The instance is finished with, and is destroyed.
    pub fn destroy(self) {
        foreign::plugin_destroy(self.plugin);
    }

    /// **Process** a block of audio.
    ///
    /// The plugin reads the input channels and writes the output ones,
    /// and the block it is given is **planar**: one vector a channel,
    /// the way the family's own audio is.
    ///
    /// The events are **empty**: this host has no transport and no
    /// event list, and the format says a plugin may be given none.
    pub fn process(&self, frames: u32, block: &mut ClapBlock) -> Result<ClapProcessStatus> {
        foreign::plugin_process(self.plugin, frames, block).map_err(ClapError::new)
    }

    /// What the plugin asked the host for.
    pub fn requests(&self) -> HostRequests {
        HostRequests {
            restart: self.state.restart.load(Ordering::SeqCst),
            process: self.state.process.load(Ordering::SeqCst),
            callback: self.state.callback.load(Ordering::SeqCst),
        }
    }
}

/// What a plugin says about itself.
///
/// Every field is **owned**: the strings were copied out of the plugin
/// when the descriptor was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClapDescriptor {
    /// The plugin's id: the name a host asks for by.
    pub id: String,
    /// The plugin's name, for a person.
    pub name: String,
    /// Who made it.
    pub vendor: String,
    /// Where it lives.
    pub url: String,
    /// Its manual.
    pub manual_url: String,
    /// Where to ask for help.
    pub support_url: String,
    /// Its own version.
    pub version: String,
    /// A line about it.
    pub description: String,
    /// The features it has.
    pub features: Vec<String>,
}

/// The **foreign** part: the only `unsafe` in this crate.
///
/// Everything the operating system and the plugin's own memory go
/// through is here, behind **one** `allow(unsafe_code)`. The rest of
/// the crate is safe Rust that calls into this module.
///
/// The rules this module keeps:
///
/// - a name that goes to the operating system is a **C string**: no
///   zero byte in it, and it is freed by nobody;
/// - a handle that comes back is either null (which is an error) or a
///   handle, and it is closed exactly once, by the drop;
/// - a string that comes back from the plugin is **copied out** at
///   once, and it is read **bounded**: the plugin decides how long it
///   is, and the host decides how much it will read.
#[cfg(unix)]
#[allow(unsafe_code)]
mod foreign {
    use std::ffi::{CString, c_char, c_void};

    use std::sync::atomic::Ordering;

    use craftkit_clap_abi::{
        ClapAudioBuffer, ClapHost, ClapInputEvents, ClapOutputEvents, ClapPlugin as ClapPluginInstance,
        ClapPluginDescriptor, ClapPluginEntry, ClapPluginFactory, ClapProcess, ClapProcessStatus,
        ClapVersion,
    };

    use super::{ClapDescriptor, HostState};

    /// Resolve the library completely before it is used.
    pub const RTLD_NOW: i32 = 2;
    /// Do not offer this library's symbols to anything else.
    pub const RTLD_LOCAL: i32 = 0;

    /// How long a string from the plugin may be, in bytes.
    const MAX_STRING_BYTES: usize = 64 * 1024;
    /// How many features a plugin may list.
    const MAX_FEATURES: usize = 256;

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
        let name = match CString::new(path) {
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

    /// The plugin's **entry**, found by name.
    ///
    /// The symbol is a **variable**: the address the loader gives back
    /// is the address of a pointer, and the pointer is the entry.
    pub fn entry(handle: usize) -> Result<*const ClapPluginEntry, String> {
        let wanted = match CString::new(craftkit_clap_abi::ENTRY_SYMBOL) {
            Ok(wanted) => wanted,
            Err(_) => return Err("the entry symbol's name is not a C string".to_string()),
        };
        // SAFETY: the handle is open, and the name is a valid C string
        // with no zero byte in it. The result is a pointer into the
        // library, which stays open as long as the handle does.
        let symbol = unsafe { dlsym(handle as *mut c_void, wanted.as_ptr()) };
        if symbol.is_null() {
            return Err(format!(
                "the library has no `{}`: {}",
                craftkit_clap_abi::ENTRY_SYMBOL,
                last_error()
            ));
        }
        // SAFETY: the symbol is the plugin's entry point, and the
        // format says it is a variable whose value is the address of
        // the entry struct. The pointer is read once, and the struct it
        // points at is only read after the version has been checked.
        let entry = unsafe { std::ptr::read_unaligned(symbol as *const *const ClapPluginEntry) };
        if entry.is_null() {
            return Err("the plugin's entry is not there".to_string());
        }
        Ok(entry)
    }

    /// The version the plugin says it speaks.
    pub fn version(entry: *const ClapPluginEntry) -> ClapVersion {
        // SAFETY: the entry is the plugin's own struct, it is inside a
        // library that is open, and the version is the first field.
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*entry).clap_version)) }
    }

    /// Initialise the plugin.
    pub fn init(entry: *const ClapPluginEntry, path: &str) -> Result<(), String> {
        // SAFETY: the entry is the plugin's own struct, inside a library
        // that is open.
        let init = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*entry).init)) };
        let Some(init) = init else {
            return Err("the plugin has no `init`".to_string());
        };
        let path = match CString::new(path) {
            Ok(path) => path,
            Err(_) => return Err("the plugin's path has a zero byte in it".to_string()),
        };
        // The plugin's own function, called with the plugin's own
        // path, as the format says. It returns whether it is ready.
        if init(path.as_ptr()) { Ok(()) } else { Err("the plugin refused to initialise".to_string()) }
    }

    /// Deinitialise the plugin.
    pub fn deinit(entry: *const ClapPluginEntry) {
        // SAFETY: the entry is the plugin's own struct, inside a library
        // that is open, and this is called once, from the drop.
        let deinit = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*entry).deinit)) };
        let Some(deinit) = deinit else { return };
        // The plugin's own function, called once, as the format says.
        deinit();
    }

    /// The **plugin factory**, asked for by its id.
    pub fn factory(entry: *const ClapPluginEntry) -> Result<*const ClapPluginFactory, String> {
        // SAFETY: the entry is the plugin's own struct, inside a library
        // that is open.
        let get_factory = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*entry).get_factory)) };
        let Some(get_factory) = get_factory else {
            return Err("the plugin has no `get_factory`".to_string());
        };
        let id = match CString::new(craftkit_clap_abi::PLUGIN_FACTORY_ID) {
            Ok(id) => id,
            Err(_) => return Err("the factory's id is not a C string".to_string()),
        };
        // The plugin's own function, called with the id of the factory
        // the format defines. The answer is the factory's own struct,
        // and it is only read while the library is open.
        let factory = get_factory(id.as_ptr());
        if factory.is_null() {
            return Err(format!("the plugin has no factory `{}`", craftkit_clap_abi::PLUGIN_FACTORY_ID));
        }
        Ok(factory as *const ClapPluginFactory)
    }

    /// How many plugins the factory holds.
    pub fn factory_count(factory: *const ClapPluginFactory) -> u32 {
        // SAFETY: the factory is the plugin's own struct, inside a
        // library that is open.
        let count = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*factory).get_plugin_count)) };
        let Some(count) = count else { return 0 };
        // The plugin's own function, called with the factory back, as
        // the format says.
        count(factory)
    }

    /// The descriptor of the plugin at that index.
    pub fn factory_descriptor(
        factory: *const ClapPluginFactory,
        index: u32,
    ) -> Result<*const ClapPluginDescriptor, String> {
        // SAFETY: the factory is the plugin's own struct, inside a
        // library that is open.
        let get = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*factory).get_plugin_descriptor)) };
        let Some(get) = get else {
            return Err("the factory has no `get_plugin_descriptor`".to_string());
        };
        // The plugin's own function, called with the factory back and
        // an index, as the format says. The answer is the plugin's own
        // descriptor.
        let descriptor = get(factory, index);
        if descriptor.is_null() {
            return Err(format!("the factory has no plugin at {index}"));
        }
        Ok(descriptor)
    }

    /// The descriptor, **copied out**.
    pub fn descriptor(descriptor: *const ClapPluginDescriptor) -> Result<ClapDescriptor, String> {
        // SAFETY: the descriptor is the plugin's own struct, inside a
        // library that is open. Every field is read once, and every
        // string is copied out at once.
        let unsafe_fields = unsafe {
            (
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).id)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).name)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).vendor)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).url)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).manual_url)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).support_url)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).version)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).description)),
                std::ptr::read_unaligned(std::ptr::addr_of!((*descriptor).features)),
            )
        };
        Ok(ClapDescriptor {
            id: read_string(unsafe_fields.0)?,
            name: read_string(unsafe_fields.1)?,
            vendor: read_string(unsafe_fields.2)?,
            url: read_string(unsafe_fields.3)?,
            manual_url: read_string(unsafe_fields.4)?,
            support_url: read_string(unsafe_fields.5)?,
            version: read_string(unsafe_fields.6)?,
            description: read_string(unsafe_fields.7)?,
            features: read_features(unsafe_fields.8)?,
        })
    }

    /// **Make** the plugin with that id, and hand it the host.
    pub fn create_plugin(
        factory: *const ClapPluginFactory,
        host: *const ClapHost,
        id: &str,
    ) -> Result<*const ClapPluginInstance, String> {
        // SAFETY: the factory is the plugin's own struct, inside a
        // library that is open.
        let create = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*factory).create_plugin)) };
        let Some(create) = create else {
            return Err("the factory has no `create_plugin`".to_string());
        };
        let id = match CString::new(id) {
            Ok(id) => id,
            Err(_) => return Err("the plugin's id has a zero byte in it".to_string()),
        };
        // The plugin's own function, called with the factory back, the
        // host and the id, as the format says. The answer is the
        // instance, and it is the plugin's own.
        let plugin = create(factory, host as *const c_void, id.as_ptr());
        if plugin.is_null() {
            return Err(format!("the factory made no plugin `{}`", id.to_string_lossy()));
        }
        Ok(plugin as *const ClapPluginInstance)
    }

    /// The instance's own descriptor.
    pub fn plugin_descriptor(
        plugin: *const ClapPluginInstance,
    ) -> Result<*const ClapPluginDescriptor, String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let desc = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).desc)) };
        if desc.is_null() {
            return Err("the plugin has no descriptor".to_string());
        }
        Ok(desc)
    }

    /// The instance is ready to work.
    pub fn plugin_init(plugin: *const ClapPluginInstance) -> Result<(), String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let init = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).init)) };
        let Some(init) = init else {
            return Err("the plugin has no `init`".to_string());
        };
        // The plugin's own function, called with the instance back, as
        // the format says.
        if init(plugin) { Ok(()) } else { Err("the plugin refused to initialise".to_string()) }
    }

    /// The instance is to work at that rate.
    pub fn plugin_activate(
        plugin: *const ClapPluginInstance,
        sample_rate: f64,
        min_frames: u32,
        max_frames: u32,
    ) -> Result<(), String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let activate = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).activate)) };
        let Some(activate) = activate else {
            return Err("the plugin has no `activate`".to_string());
        };
        // The plugin's own function, called with the instance back and
        // the rate, as the format says.
        if activate(plugin, sample_rate, min_frames, max_frames) {
            Ok(())
        } else {
            Err("the plugin refused to activate".to_string())
        }
    }

    /// The instance is to stop working.
    pub fn plugin_deactivate(plugin: *const ClapPluginInstance) -> Result<(), String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let deactivate = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).deactivate)) };
        let Some(deactivate) = deactivate else {
            return Err("the plugin has no `deactivate`".to_string());
        };
        // The plugin's own function, called with the instance back, as
        // the format says.
        deactivate(plugin);
        Ok(())
    }

    /// The host starts to send audio.
    pub fn plugin_start_processing(plugin: *const ClapPluginInstance) -> Result<(), String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let start = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).start_processing)) };
        let Some(start) = start else {
            return Err("the plugin has no `start_processing`".to_string());
        };
        // The plugin's own function, called with the instance back, as
        // the format says.
        if start(plugin) { Ok(()) } else { Err("the plugin refused to start".to_string()) }
    }

    /// The host stops sending audio.
    pub fn plugin_stop_processing(plugin: *const ClapPluginInstance) -> Result<(), String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let stop = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).stop_processing)) };
        let Some(stop) = stop else {
            return Err("the plugin has no `stop_processing`".to_string());
        };
        // The plugin's own function, called with the instance back, as
        // the format says.
        stop(plugin);
        Ok(())
    }

    /// The instance is to forget its audio.
    pub fn plugin_reset(plugin: *const ClapPluginInstance) -> Result<(), String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let reset = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).reset)) };
        let Some(reset) = reset else {
            return Err("the plugin has no `reset`".to_string());
        };
        // The plugin's own function, called with the instance back, as
        // the format says.
        reset(plugin);
        Ok(())
    }

    /// The instance is finished with.
    pub fn plugin_destroy(plugin: *const ClapPluginInstance) {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open, and this is called once, when the
        // instance is dropped.
        let destroy = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).destroy)) };
        let Some(destroy) = destroy else { return };
        // The plugin's own function, called once, as the format says.
        destroy(plugin);
    }

    /// **Process** a block of audio.
    ///
    /// The buffers are built here, because the plugin's table of
    /// channels is a table of **addresses**, and the addresses have to
    /// point at the caller's own vectors for as long as the call
    /// lasts.
    pub fn plugin_process(
        plugin: *const ClapPluginInstance,
        frames: u32,
        block: &mut super::ClapBlock,
    ) -> Result<ClapProcessStatus, String> {
        // SAFETY: the instance is the plugin's own struct, inside a
        // library that is open.
        let process = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).process)) };
        let Some(process) = process else {
            return Err("the plugin has no `process`".to_string());
        };
        // The channels, as addresses. The vectors they point at are
        // the caller's, and they do not move while the call lasts.
        let mut input_channels: Vec<*mut f32> =
            block.inputs.iter_mut().map(|channel| channel.as_mut_ptr()).collect();
        let mut output_channels: Vec<*mut f32> =
            block.outputs.iter_mut().map(|channel| channel.as_mut_ptr()).collect();
        let input_buffer = ClapAudioBuffer {
            data32: input_channels.as_mut_ptr(),
            data64: std::ptr::null_mut(),
            channel_count: u32::try_from(input_channels.len()).unwrap_or(0),
            latency: 0,
            constant_mask: 0,
        };
        let mut output_buffer = ClapAudioBuffer {
            data32: output_channels.as_mut_ptr(),
            data64: std::ptr::null_mut(),
            channel_count: u32::try_from(output_channels.len()).unwrap_or(0),
            latency: 0,
            constant_mask: 0,
        };
        let input_events = empty_input_events();
        let output_events = empty_output_events();
        let block = ClapProcess {
            steady_time: 0,
            frames_count: frames,
            transport: std::ptr::null(),
            audio_inputs: &input_buffer,
            audio_outputs: &mut output_buffer,
            audio_inputs_count: 1,
            audio_outputs_count: 1,
            in_events: &input_events,
            out_events: &output_events,
        };
        // The plugin's own function, called with the instance back and
        // the block, as the format says.
        Ok(process(plugin, &block))
    }

    /// The **input events**: there are none.
    fn empty_input_events() -> ClapInputEvents {
        ClapInputEvents {
            ctx: std::ptr::null_mut(),
            size: Some(input_events_size),
            get: Some(input_events_get),
        }
    }

    /// How many events there are: none.
    extern "C" fn input_events_size(_list: *const ClapInputEvents) -> u32 {
        0
    }

    /// The event at that index: there is none.
    extern "C" fn input_events_get(_list: *const ClapInputEvents, _index: u32) -> *const c_void {
        std::ptr::null()
    }

    /// The **output events**: there are none.
    fn empty_output_events() -> ClapOutputEvents {
        ClapOutputEvents { ctx: std::ptr::null_mut(), try_push: Some(output_events_push) }
    }

    /// Offer an event to the host: there is no host to take it.
    extern "C" fn output_events_push(_list: *const ClapOutputEvents, _event: *const c_void) -> bool {
        false
    }

    /// The host has **no extensions** to give.
    ///
    /// The format's extensions are the state, the log, the parameters
    /// and the ports; this host has none of them, and says so by
    /// answering with nothing.
    pub extern "C" fn get_extension(_host: *const ClapHost, _extension_id: *const c_char) -> *const c_void {
        std::ptr::null()
    }

    /// The plugin asked the host to restart it.
    pub extern "C" fn request_restart(host: *const ClapHost) {
        if let Some(state) = host_state(host) {
            state.restart.store(true, Ordering::SeqCst);
        }
    }

    /// The plugin asked the host to call it back.
    pub extern "C" fn request_process(host: *const ClapHost) {
        if let Some(state) = host_state(host) {
            state.process.store(true, Ordering::SeqCst);
        }
    }

    /// The plugin asked the host to call it on its main thread.
    pub extern "C" fn request_callback(host: *const ClapHost) {
        if let Some(state) = host_state(host) {
            state.callback.store(true, Ordering::SeqCst);
        }
    }

    /// The host's own state, from the host the plugin was given.
    ///
    /// The state is the loader's own, in a box that lives as long as
    /// the instance does, and the plugin can only reach it while the
    /// instance is alive.
    fn host_state<'a>(host: *const ClapHost) -> Option<&'a HostState> {
        if host.is_null() {
            return None;
        }
        // SAFETY: the host is the loader's own structure, and its
        // `host_data` is the loader's own state.
        let data = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*host).host_data)) };
        if data.is_null() {
            return None;
        }
        // SAFETY: `host_data` was the state the loader put there, and
        // the loader keeps it alive for as long as the instance is.
        Some(unsafe { &*(data as *const HostState) })
    }

    /// A C string the plugin gave us, as a Rust string.
    ///
    /// The read is **bounded**: the plugin says how long the string is,
    /// and the host says how much of it it will read. A string with no
    /// zero byte in the first `MAX_STRING_BYTES` is refused rather
    /// than read on and on.
    fn read_string(pointer: *const c_char) -> Result<String, String> {
        if pointer.is_null() {
            // The format allows a field to be absent, and an absent
            // field is an empty one.
            return Ok(String::new());
        }
        let mut bytes: Vec<u8> = Vec::new();
        let mut at = pointer;
        loop {
            if bytes.len() >= MAX_STRING_BYTES {
                return Err("the string is longer than the host reads".to_string());
            }
            // SAFETY: the pointer is the plugin's own C string, and a
            // byte is read at a time: the read stops at the zero byte
            // that ends the string, or at the bound.
            let byte = unsafe { std::ptr::read_volatile(at as *const u8) };
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            // SAFETY: the byte just read was not the zero byte, so the
            // next one is inside the string the plugin owns.
            at = unsafe { at.add(1) };
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// The **features**: a list of C strings, ending at a null.
    fn read_features(pointer: *const *const c_char) -> Result<Vec<String>, String> {
        let mut features: Vec<String> = Vec::new();
        if pointer.is_null() {
            return Ok(features);
        }
        let mut at = pointer;
        for _ in 0..MAX_FEATURES {
            // SAFETY: the pointer is the plugin's own array, and one
            // entry is read at a time.
            let item = unsafe { std::ptr::read_volatile(at) };
            if item.is_null() {
                return Ok(features);
            }
            features.push(read_string(item)?);
            // SAFETY: the entry just read was not the null that ends
            // the list, so the next one is inside the array the plugin
            // owns.
            at = unsafe { at.add(1) };
        }
        Err("the plugin lists more features than the host reads".to_string())
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
        unsafe { std::ffi::CStr::from_ptr(pointer) }.to_string_lossy().into_owned()
    }
}

/// The **foreign** part, where there is no foreign part.
///
/// There is no shared library to open on this target, so every door
/// here refuses — and it says so, rather than pretending to work.
#[cfg(not(unix))]
mod foreign {
    use craftkit_clap_abi::{ClapPluginDescriptor, ClapPluginEntry, ClapPluginFactory, ClapVersion};

    use super::ClapDescriptor;

    /// Open a shared library: not here.
    pub fn open(path: &str) -> Result<usize, String> {
        Err(format!("a shared library cannot be opened on this target: `{path}`"))
    }

    /// Close a shared library: nothing was opened.
    pub fn close(_handle: usize) {}

    /// The plugin's entry: there is no library to look in.
    pub fn entry(_handle: usize) -> Result<*const ClapPluginEntry, String> {
        Err("there is no shared library on this target".to_string())
    }

    /// The plugin's version: there is no plugin.
    pub fn version(_entry: *const ClapPluginEntry) -> ClapVersion {
        ClapVersion::CURRENT
    }

    /// Initialise the plugin: there is no plugin.
    pub fn init(_entry: *const ClapPluginEntry, _path: &str) -> Result<(), String> {
        Err("there is no plugin on this target".to_string())
    }

    /// Deinitialise the plugin: there is no plugin.
    pub fn deinit(_entry: *const ClapPluginEntry) {}

    /// The factory: there is no plugin.
    pub fn factory(_entry: *const ClapPluginEntry) -> Result<*const ClapPluginFactory, String> {
        Err("there is no plugin on this target".to_string())
    }

    /// How many plugins: none.
    pub fn factory_count(_factory: *const ClapPluginFactory) -> u32 {
        0
    }

    /// The descriptor of a plugin: there is none.
    pub fn factory_descriptor(
        _factory: *const ClapPluginFactory,
        _index: u32,
    ) -> Result<*const ClapPluginDescriptor, String> {
        Err("there is no plugin on this target".to_string())
    }

    /// The descriptor, copied out: there is none.
    pub fn descriptor(_descriptor: *const ClapPluginDescriptor) -> Result<ClapDescriptor, String> {
        Err("there is no plugin on this target".to_string())
    }
}
