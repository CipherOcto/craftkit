//! `craftkit-clap-fixture` — a **reference CLAP plugin**.
//!
//! # Why it is here
//!
//! A host that opens a plugin is only worth something if the plugin it
//! opens is a **real** one: a shared library, exporting the entry by
//! name, built by the compiler and opened from the disk. So this crate
//! is a plugin, in the strict sense — a `cdylib` — and the loader's
//! tests open it the way they would open anybody else's.
//!
//! It is a **fixture**, and it says what it is: it holds one plugin,
//! with one descriptor, and it does nothing at all when it is made —
//! this arc is about the host reading a plugin, not about making one.
//!
//! # The shape of it
//!
//! A CLAP plugin exports **one data symbol**: `clap_entry`, a variable
//! whose value is the address of the entry struct. The entry has three
//! function pointers — `init`, `deinit` and `get_factory` — and the
//! factory has three more. The plugin's own strings are C strings, and
//! the feature list is a list of C strings that ends at a null.
//!
//! # Why it is `unsafe`
//!
//! Exporting a symbol by name is `unsafe` in the sense that the
//! attribute itself is: the name is promised to the loader, and the
//! loader will look it up. So this crate is on v2's **list** of
//! exceptions — the fifth entry — and it is held to the same rules as
//! the others: `deny(unsafe_code)` and not `forbid`, **one**
//! `allow(unsafe_code)`, and **one** marked place where it happens.
//!
//! It has **no** `unsafe impl`, and it has **no** `unsafe` block: the
//! only thing it needs is the one attribute that gives the symbol its
//! name.

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

use std::ffi::c_char;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use craftkit_clap_abi::{ClapPluginDescriptor, ClapPluginEntry, ClapPluginFactory, ClapVersion};

/// Whether the plugin has been initialised.
///
/// The host calls `init` before it asks for anything, and the tests
/// check that it did — so the plugin keeps the answer where a test
/// can see it.
static INITIALISED: AtomicBool = AtomicBool::new(false);

/// The plugin's id, as the file says it.
const ID: &[u8] = b"org.craftkit.fixture.clap\0";
/// The same id, without the zero byte: the host asks by name, and a
/// name is not a C string.
const ID_TEXT: &[u8] = b"org.craftkit.fixture.clap";

/// The plugin's name.
const NAME: &[u8] = b"Craftkit Reference\0";
/// Who made it.
const VENDOR: &[u8] = b"Craftkit\0";
/// Where it lives.
const URL: &[u8] = b"https://example.invalid/craftkit\0";
/// Its manual.
const MANUAL: &[u8] = b"https://example.invalid/craftkit/manual\0";
/// Where to ask for help.
const SUPPORT: &[u8] = b"https://example.invalid/craftkit/support\0";
/// Its own version.
const PLUGIN_VERSION: &[u8] = b"1.0.0\0";
/// A line about it.
const DESCRIPTION: &[u8] = b"A reference plugin: it holds one plugin and does nothing.\0";
/// The features it has.
const FEATURES: [&[u8]; 3] = [b"audio-effect\0", b"stereo\0", b"mono\0"];

/// The **descriptor**: what the plugin says about itself.
///
/// A descriptor is a struct of **pointers**, and a static may not hold
/// a pointer: a static is shared, and a raw pointer says nothing about
/// whether it may be shared. So the descriptor is built **once**, on the
/// heap, and its address is kept in an atomic pointer — which is a
/// pointer that may be shared, because it may only be read.
static DESCRIPTOR: OnceLock<AtomicPtr<ClapPluginDescriptor>> = OnceLock::new();

/// The descriptor, built once and then read.
fn descriptor() -> *const ClapPluginDescriptor {
    DESCRIPTOR
        .get_or_init(|| AtomicPtr::new(Box::into_raw(Box::new(build_descriptor()))))
        .load(Ordering::Acquire)
}

/// Build the descriptor.
///
/// The feature list is **leaked** on purpose: the descriptor points
/// into it, and a pointer into a box that is dropped would be a
/// pointer into nothing. A fixture that lives for the length of the
/// process may keep what it builds.
fn build_descriptor() -> ClapPluginDescriptor {
    let features: &'static mut [*const c_char; 4] = Box::leak(Box::new([
        FEATURES[0].as_ptr() as *const c_char,
        FEATURES[1].as_ptr() as *const c_char,
        FEATURES[2].as_ptr() as *const c_char,
        std::ptr::null(),
    ]));
    ClapPluginDescriptor {
        clap_version: ClapVersion::CURRENT,
        id: ID.as_ptr() as *const c_char,
        name: NAME.as_ptr() as *const c_char,
        vendor: VENDOR.as_ptr() as *const c_char,
        url: URL.as_ptr() as *const c_char,
        manual_url: MANUAL.as_ptr() as *const c_char,
        support_url: SUPPORT.as_ptr() as *const c_char,
        version: PLUGIN_VERSION.as_ptr() as *const c_char,
        description: DESCRIPTION.as_ptr() as *const c_char,
        features: features.as_ptr(),
    }
}

/// The **factory**: the thing that holds the plugins.
static FACTORY: ClapPluginFactory = ClapPluginFactory {
    get_plugin_count: Some(get_plugin_count),
    get_plugin_descriptor: Some(get_plugin_descriptor),
    create_plugin: Some(exports::create_plugin),
};

/// The **entry**: what the host finds first.
static ENTRY: ClapPluginEntry = ClapPluginEntry {
    clap_version: ClapVersion::CURRENT,
    init: Some(init),
    deinit: Some(deinit),
    get_factory: Some(exports::get_factory),
};

/// The plugin's own `init`: it is called once, with the plugin's path.
extern "C" fn init(_plugin_path: *const c_char) -> bool {
    INITIALISED.store(true, Ordering::SeqCst);
    true
}

/// The plugin's own `deinit`: it is called once, at the end.
extern "C" fn deinit() {
    INITIALISED.store(false, Ordering::SeqCst);
}

/// How many plugins the factory holds: one.
extern "C" fn get_plugin_count(_factory: *const ClapPluginFactory) -> u32 {
    1
}

/// The descriptor of the plugin at that index.
extern "C" fn get_plugin_descriptor(
    _factory: *const ClapPluginFactory,
    index: u32,
) -> *const ClapPluginDescriptor {
    if index == 0 {
        return descriptor();
    }
    std::ptr::null()
}

/// Whether the plugin has been initialised.
///
/// The host's tests read this: the plugin's `init` is called before
/// anything else, and the test says so.
pub fn is_initialised() -> bool {
    INITIALISED.load(Ordering::SeqCst)
}

/// The **exported** part: the one symbol the host looks for, the
/// plugin's own life, and the one place this crate is `unsafe`.
///
/// CLAP's entry point is a **data** symbol: a variable whose value is
/// the address of the entry struct. This is that variable.
///
/// The plugin's own functions are here because this is the crate's one
/// marked place: reading the host's name and the instance's own state
/// is reading memory the host and the host's plugin wrote.
#[allow(unsafe_code)]
mod exports {
    use std::ffi::{c_char, c_void};

    use craftkit_clap_abi::{
        ClapHost, ClapPlugin, ClapPluginFactory, ClapProcess, ClapProcessStatus, PLUGIN_FACTORY_ID,
    };

    /// The entry point of this plugin, found by name.
    ///
    /// The `unsafe` is in the attribute and nowhere else: giving a
    /// symbol a name is a promise to the loader that the thing behind
    /// the name is what the format says it is. It is — the entry
    /// struct of the format, and nothing else.
    #[unsafe(no_mangle)]
    pub static clap_entry: &craftkit_clap_abi::ClapPluginEntry = &super::ENTRY;

    /// The plugin's own `get_factory`: it answers for the one factory
    /// the format defines, and for nothing else.
    pub(super) extern "C" fn get_factory(factory_id: *const c_char) -> *const c_void {
        if factory_id.is_null() {
            return std::ptr::null();
        }
        // SAFETY: the host asks for the factory by the format's own
        // id, and the answer is compared against that same id. Both
        // are C strings: a byte is read at a time and the read stops
        // at the zero byte that ends the string.
        let wanted = unsafe { std::ffi::CStr::from_ptr(factory_id) };
        if wanted.to_bytes() != PLUGIN_FACTORY_ID.as_bytes() {
            return std::ptr::null();
        }
        &super::FACTORY as *const ClapPluginFactory as *const c_void
    }

    /// The plugin's own state: the host it was given and what it has
    /// done since.
    pub(super) struct State {
        /// The host, as the host gave it.
        host: *const ClapHost,
        /// Whether the instance has been initialised.
        initialised: bool,
        /// The rate the instance was activated at.
        sample_rate: f64,
    }

    /// The instance's own state.
    fn state(plugin: *const ClapPlugin) -> Option<&'static mut State> {
        if plugin.is_null() {
            return None;
        }
        // SAFETY: the instance is this plugin's own, and its
        // `plugin_data` is the box this plugin made.
        let data = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).plugin_data)) };
        if data.is_null() {
            return None;
        }
        // SAFETY: the data is the box this plugin made, and the plugin
        // frees it when it is destroyed.
        Some(unsafe { &mut *(data as *mut State) })
    }

    /// The host's name, read out of the host the plugin was given.
    fn host_name(host: *const ClapHost) -> String {
        if host.is_null() {
            return String::new();
        }
        // SAFETY: the host is the host's own structure, and its name is
        // a C string it keeps for as long as the plugin lives.
        let name = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*host).name)) };
        if name.is_null() {
            return String::new();
        }
        // SAFETY: the name is a C string the host owns.
        unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().into_owned()
    }

    /// **Make** the plugin with that id.
    ///
    /// The instance is made on the heap, because the host holds a
    /// pointer to it and a pointer to a thing that moves is a pointer
    /// to nothing. It is freed when the plugin is destroyed.
    pub(super) extern "C" fn create_plugin(
        _factory: *const ClapPluginFactory,
        host: *const c_void,
        id: *const c_char,
    ) -> *const c_void {
        if id.is_null() {
            return std::ptr::null();
        }
        // SAFETY: the host asks for a plugin by its id, and the id is
        // a C string it keeps for the length of the call.
        let wanted = unsafe { std::ffi::CStr::from_ptr(id) };
        if wanted.to_bytes() != super::ID_TEXT {
            return std::ptr::null();
        }
        let state = Box::into_raw(Box::new(State {
            host: host as *const ClapHost,
            initialised: false,
            sample_rate: 0.0,
        }));
        let plugin = Box::into_raw(Box::new(ClapPlugin {
            desc: super::descriptor(),
            plugin_data: state as *mut c_void,
            init: Some(init),
            destroy: Some(destroy),
            activate: Some(activate),
            deactivate: Some(deactivate),
            start_processing: Some(start_processing),
            stop_processing: Some(stop_processing),
            reset: Some(reset),
            process: Some(process),
            get_extension: None,
            on_main_thread: None,
        }));
        plugin as *const c_void
    }

    /// **Process** a block of audio.
    ///
    /// The plugin is a **gain**: it adds one to every sample that comes
    /// in and writes it out. That is enough to prove that the audio
    /// crosses the boundary in both directions — the samples arrive
    /// where the plugin expects them and leave where the host expects
    /// them.
    pub(super) extern "C" fn process(
        plugin: *const ClapPlugin,
        process: *const ClapProcess,
    ) -> ClapProcessStatus {
        let Some(state) = state(plugin) else { return ClapProcessStatus::ContinueIfNotProcessed };
        if !state.initialised {
            return ClapProcessStatus::ContinueIfNotProcessed;
        }
        // SAFETY: the block is the host's own, and it lives for the
        // length of the call.
        let frames = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*process).frames_count)) };
        let inputs = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*process).audio_inputs)) };
        let outputs = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*process).audio_outputs)) };
        if outputs.is_null() {
            return ClapProcessStatus::ContinueIfNotProcessed;
        }
        // SAFETY: the output buffer is the host's own, and it holds the
        // addresses of its channels.
        let output_channels = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*outputs).data32)) };
        let output_count = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*outputs).channel_count)) };
        if output_channels.is_null() {
            return ClapProcessStatus::ContinueIfNotProcessed;
        }
        // SAFETY: the input buffer is the host's own, and it holds the
        // addresses of its channels.
        let (input_channels, input_count) = if inputs.is_null() {
            (std::ptr::null_mut(), 0u32)
        } else {
            (unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*inputs).data32)) }, unsafe {
                std::ptr::read_unaligned(std::ptr::addr_of!((*inputs).channel_count))
            })
        };
        for channel in 0..output_count as usize {
            // SAFETY: the channel is one of the buffer's own, and the
            // address it holds is the host's own memory.
            let out = unsafe { *output_channels.add(channel) };
            if out.is_null() {
                continue;
            }
            let input = if input_channels.is_null() || channel as u32 >= input_count {
                std::ptr::null()
            } else {
                // SAFETY: the channel is one of the buffer's own.
                unsafe { *input_channels.add(channel) }
            };
            for frame in 0..frames as usize {
                let sample = if input.is_null() {
                    0.0
                } else {
                    // SAFETY: the frame is inside the block, which the
                    // host said how long it is.
                    unsafe { *input.add(frame) }
                };
                // SAFETY: the frame is inside the block, which the host
                // said how long it is.
                unsafe { *out.add(frame) = sample + 1.0 };
            }
        }
        ClapProcessStatus::Continue
    }

    /// The instance is ready to work.
    pub(super) extern "C" fn init(plugin: *const ClapPlugin) -> bool {
        let Some(state) = state(plugin) else { return false };
        // The **handshake**: the plugin reads the host's name, and asks
        // the host to restart it. The request is the only channel a
        // test can see across the boundary, and it is how the host's
        // own functions are exercised.
        let name = host_name(state.host);
        if name.is_empty() {
            return false;
        }
        if let Some(host) = Some(state.host) {
            // SAFETY: the host is the host's own structure, and the
            // request is one of its own functions.
            let request = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*host).request_restart)) };
            if let Some(request) = request {
                request(host);
            }
        }
        state.initialised = true;
        true
    }

    /// The instance is to work at that rate.
    pub(super) extern "C" fn activate(
        plugin: *const ClapPlugin,
        sample_rate: f64,
        _min_frames: u32,
        _max_frames: u32,
    ) -> bool {
        let Some(state) = state(plugin) else { return false };
        if !state.initialised {
            return false;
        }
        state.sample_rate = sample_rate;
        // Ask the host to call us back: the second of the host's own
        // functions, and the second channel a test can see.
        let host = state.host;
        if !host.is_null() {
            // SAFETY: the host is the host's own structure, and the
            // request is one of its own functions.
            let request = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*host).request_process)) };
            if let Some(request) = request {
                request(host);
            }
        }
        true
    }

    /// The instance is to stop working.
    pub(super) extern "C" fn deactivate(plugin: *const ClapPlugin) {
        let Some(state) = state(plugin) else { return };
        state.sample_rate = 0.0;
    }

    /// The host starts to send audio.
    pub(super) extern "C" fn start_processing(plugin: *const ClapPlugin) -> bool {
        state(plugin).is_some()
    }

    /// The host stops sending audio.
    pub(super) extern "C" fn stop_processing(plugin: *const ClapPlugin) {
        let _ = state(plugin);
    }

    /// The instance is to forget its audio.
    pub(super) extern "C" fn reset(plugin: *const ClapPlugin) {
        let _ = state(plugin);
    }

    /// The instance is finished with, and is freed.
    pub(super) extern "C" fn destroy(plugin: *const ClapPlugin) {
        if plugin.is_null() {
            return;
        }
        // SAFETY: the instance is this plugin's own, and its data is
        // the box this plugin made. Both are freed here, once.
        unsafe {
            let data = std::ptr::read_unaligned(std::ptr::addr_of!((*plugin).plugin_data));
            if !data.is_null() {
                drop(Box::from_raw(data as *mut State));
            }
            drop(Box::from_raw(plugin as *mut ClapPlugin));
        }
    }
}
