//! `craftkit-au` — the **AU host**: open a unit, make an instance, and
//! drive it.
//!
//! # What this is
//!
//! The other half of [`craftkit_au_abi`]. The ABI writes the format
//! down as shapes; this crate **uses** them: it opens a shared library,
//! finds the symbols the format names, makes an instance, and drives
//! it.
//!
//! It is a **library** and not a plugin: it contributes no command and
//! no kind, and it is at L1, so a battery at L4 may depend on it.
//!
//! # The symbols
//!
//! A unit exports **functions**, and the host finds them **by name**:
//! the two that read and write a property, the two that set the unit
//! up and take it down, the one that puts it back as it was, and the
//! one that renders a block.
//!
//! # The **tree's own** door
//!
//! The format's own way of making an instance is the platform's
//! **component manager**, and this tree has none — it is a macOS thing
//! and this tree is not macOS. So the host also looks for
//! [`craftkit_au_abi::INSTANCE_SYMBOL`], a name of **this tree's own**,
//! and makes the instance through it.
//!
//! The host says so, in its own words, wherever the answer could be
//! mistaken for the platform's: a program that wants the platform's
//! component manager brings its own.
//!
//! # The instance is not kept
//!
//! An instance holds addresses into a library that is open, and it is
//! destroyed when the host is finished with it. So nothing here keeps
//! one: a caller opens the bundle, uses the instance, and lets it go.
//!
//! # Why it is `unsafe`
//!
//! Opening a library, finding a symbol in it and calling it is `unsafe`
//! by definition: the host is calling code it has not read, at an
//! address it did not choose. So this crate is on v2's **list** of
//! exceptions — the **ninth** entry — and it is held to the same rules
//! as the others:
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

use craftkit_au_abi::{
    AudioBufferList, ELEMENT_MASTER, GetProperty, Initialize, MakeInstance,
    PROPERTY_MAXIMUM_FRAMES_PER_SLICE, PROPERTY_SAMPLE_RATE, PROPERTY_STREAM_FORMAT, Render, Reset,
    SCOPE_GLOBAL, SetProperty, Status, StreamFormat, Uninitialize,
};

/// What went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuError {
    /// What happened, in words.
    pub message: String,
}

impl AuError {
    /// A failure with a message.
    fn new(message: impl Into<String>) -> AuError {
        AuError { message: message.into() }
    }
}

impl std::fmt::Display for AuError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for AuError {}

/// The result of a call into a unit.
pub type Result<T> = std::result::Result<T, AuError>;

/// A **unit**: an open bundle and the symbols inside it.
///
/// The bundle is closed when this is dropped, and the symbols are only
/// called while it is open.
pub struct AuPlugin {
    /// The open bundle.
    handle: usize,
    /// Where the bundle is, for the messages.
    path: String,
    /// The symbol that reads a property.
    get_property: GetProperty,
    /// The symbol that writes a property.
    set_property: SetProperty,
    /// The symbol that sets the unit up.
    initialize: Initialize,
    /// The symbol that takes the unit down.
    uninitialize: Uninitialize,
    /// The symbol that puts the unit back as it was.
    reset: Reset,
    /// The symbol that renders a block.
    render: Render,
    /// This tree's own symbol that hands out an instance.
    make_instance: MakeInstance,
}

impl AuPlugin {
    /// **Open** a bundle and find the symbols the format names.
    ///
    /// Every symbol the host needs is found at once: a bundle that is
    /// missing one is refused, and the host does not find out halfway
    /// through a render.
    pub fn open(path: &str) -> Result<AuPlugin> {
        let handle = foreign::open(path).map_err(AuError::new)?;
        let found = (|| -> Result<AuPlugin> {
            Ok(AuPlugin {
                handle,
                path: path.to_string(),
                get_property: foreign::get_property(handle).map_err(AuError::new)?,
                set_property: foreign::set_property(handle).map_err(AuError::new)?,
                initialize: foreign::initialize(handle).map_err(AuError::new)?,
                uninitialize: foreign::uninitialize(handle).map_err(AuError::new)?,
                reset: foreign::reset(handle).map_err(AuError::new)?,
                render: foreign::render(handle).map_err(AuError::new)?,
                make_instance: foreign::make_instance(handle).map_err(AuError::new)?,
            })
        })();
        match found {
            Ok(plugin) => Ok(plugin),
            Err(error) => {
                // The bundle was opened, and it is not going to be
                // used: it is closed again.
                foreign::close(handle);
                Err(error)
            }
        }
    }

    /// Where the bundle is.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Make an **instance** of the unit.
    ///
    /// The instance comes from **this tree's own** door, because the
    /// format's own door is the platform's component manager and this
    /// tree has none.
    pub fn instance(&self) -> Result<AuInstance<'_>> {
        let instance = (self.make_instance)();
        if instance.is_null() {
            return Err(AuError::new("the unit would not make an instance"));
        }
        Ok(AuInstance { instance, plugin: self })
    }
}

impl Drop for AuPlugin {
    fn drop(&mut self) {
        foreign::close(self.handle);
    }
}

/// An **instance**: a unit that has been made.
///
/// It borrows the bundle it is in, so that the bundle stays open for
/// as long as the instance does.
pub struct AuInstance<'a> {
    /// The instance itself.
    instance: *mut std::ffi::c_void,
    /// The bundle it is in, so that it stays open.
    plugin: &'a AuPlugin,
}

impl AuInstance<'_> {
    /// Set the unit up.
    pub fn initialize(&self) -> Result<()> {
        check((self.plugin.initialize)(self.instance), "the unit would not initialise")
    }

    /// Take the unit down.
    pub fn uninitialize(&self) -> Result<()> {
        check((self.plugin.uninitialize)(self.instance), "the unit would not terminate")
    }

    /// Put the unit back as it was.
    pub fn reset(&self) -> Result<()> {
        check(
            (self.plugin.reset)(
                self.instance,
                craftkit_au_abi::RESET_SCOPE_GLOBAL,
                craftkit_au_abi::RESET_ELEMENT_MASTER,
            ),
            "the unit would not reset",
        )
    }

    /// The **rate** the unit works at.
    pub fn sample_rate(&self) -> Result<f64> {
        let bytes = self.property(PROPERTY_SAMPLE_RATE)?;
        let bytes: [u8; 8] =
            bytes.try_into().map_err(|_| AuError::new("the unit said a rate of the wrong size"))?;
        Ok(f64::from_le_bytes(bytes))
    }

    /// Say the **rate** the unit works at.
    pub fn set_sample_rate(&self, rate: f64) -> Result<()> {
        if !rate.is_finite() || rate <= 0.0 {
            return Err(AuError::new("a rate is a positive number"));
        }
        self.set_property(PROPERTY_SAMPLE_RATE, &rate.to_le_bytes())
    }

    /// The **shape** of the audio the unit takes.
    pub fn stream_format(&self) -> Result<StreamFormat> {
        let bytes = self.property(PROPERTY_STREAM_FORMAT)?;
        if bytes.len() < std::mem::size_of::<StreamFormat>() {
            return Err(AuError::new("the unit said a shape of the wrong size"));
        }
        let mut format = StreamFormat::pcm(0.0, 0);
        // SAFETY: the bytes are the unit's own, and they are read into
        // the format's own fields one at a time, so that a byte order
        // that is not this machine's is still read correctly.
        let read = |at: usize| -> u32 {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let rate = f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        format.sample_rate = rate;
        format.format_id = read(8);
        format.format_flags = read(12);
        format.bytes_per_packet = read(16);
        format.frames_per_packet = read(20);
        format.bytes_per_frame = read(24);
        format.channels_per_frame = read(28);
        format.bits_per_channel = read(32);
        format.reserved = read(36);
        Ok(format)
    }

    /// How many frames the unit takes in **one** slice.
    pub fn maximum_frames(&self) -> Result<u32> {
        let bytes = self.property(PROPERTY_MAXIMUM_FRAMES_PER_SLICE)?;
        let bytes: [u8; 4] =
            bytes.try_into().map_err(|_| AuError::new("the unit said a slice of the wrong size"))?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Read a **property**, as bytes.
    ///
    /// The unit's answer is copied out at once: it is the unit's own
    /// memory, and it may change the next moment.
    pub fn property(&self, id: u32) -> Result<Vec<u8>> {
        let mut size = 64u32;
        let mut room = vec![0u8; size as usize];
        let status = (self.plugin.get_property)(
            self.instance,
            id,
            SCOPE_GLOBAL,
            ELEMENT_MASTER,
            room.as_mut_ptr().cast::<std::ffi::c_void>(),
            &mut size,
        );
        check(status, "the unit would not say")?;
        if size as usize > room.len() {
            return Err(AuError::new("the unit said more than the host asked for"));
        }
        room.truncate(size as usize);
        Ok(room)
    }

    /// Write a **property**, as bytes.
    pub fn set_property(&self, id: u32, bytes: &[u8]) -> Result<()> {
        let status = (self.plugin.set_property)(
            self.instance,
            id,
            SCOPE_GLOBAL,
            ELEMENT_MASTER,
            bytes.as_ptr().cast::<std::ffi::c_void>(),
            bytes.len() as u32,
        );
        check(status, "the unit would not take")
    }

    /// **Render** a block of audio.
    ///
    /// The **first** buffer is what goes in and the **second** is what
    /// comes out: the tree's own convention, and the unit's own copy of
    /// it.
    pub fn render(&self, input: &[f32], output: &mut [f32]) -> Result<()> {
        if input.len() != output.len() {
            return Err(AuError::new("a block goes in and comes out the same size"));
        }
        if input.is_empty() {
            return Err(AuError::new("a block holds at least one sample"));
        }
        let mut list = AudioBufferList::empty();
        list.number_buffers = 2;
        list.buffers[0].number_channels = 1;
        list.buffers[0].data_byte_size = (input.len() * 4) as u32;
        list.buffers[0].data = input.as_ptr() as *mut std::ffi::c_void;
        list.buffers[1].number_channels = 1;
        list.buffers[1].data_byte_size = (output.len() * 4) as u32;
        list.buffers[1].data = output.as_mut_ptr() as *mut std::ffi::c_void;
        let mut frames = input.len() as u32;
        let status = (self.plugin.render)(self.instance, &mut frames, &mut list);
        check(status, "the unit would not render")
    }
}

/// A call that the unit **refused**.
fn check(status: Status, what: &str) -> Result<()> {
    if status == craftkit_au_abi::STATUS_OK {
        Ok(())
    } else {
        Err(AuError::new(format!("{what}: the unit said {status}")))
    }
}

/// The **foreign** part: the only `unsafe` in this crate.
///
/// Opening a library, finding a symbol in it and calling it is unsafe
/// by definition: the host is calling code it has not read, at an
/// address it did not choose. Every block below says, in a comment, why
/// it is sound — and the soundness always comes from the same two
/// things: the bundle is open, and the symbol came out of it.
#[cfg(unix)]
#[allow(unsafe_code)]
mod foreign {
    use std::ffi::{CString, c_char, c_void};

    use craftkit_au_abi::{
        GET_PROPERTY_SYMBOL, GetProperty, INITIALIZE_SYMBOL, INSTANCE_SYMBOL, Initialize, MakeInstance,
        RENDER_SYMBOL, RESET_SYMBOL, Render, Reset, SET_PROPERTY_SYMBOL, SetProperty, UNINITIALIZE_SYMBOL,
        Uninitialize,
    };

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

    /// The address of a **symbol**, in an open library.
    fn address(handle: usize, name: &str) -> Result<usize, String> {
        let wanted = match CString::new(name) {
            Ok(wanted) => wanted,
            Err(_) => return Err(format!("the name of `{name}` is not a C string")),
        };
        // SAFETY: the handle is open, and the name is a valid C string
        // with no zero byte in it. The result is a pointer into the
        // library, which stays open as long as the handle does.
        let symbol = unsafe { dlsym(handle as *mut c_void, wanted.as_ptr()) };
        if symbol.is_null() {
            return Err(format!("the library has no `{name}`: {}", last_error()));
        }
        Ok(symbol as usize)
    }

    /// The symbol that reads a property.
    pub fn get_property(handle: usize) -> Result<GetProperty, String> {
        let address = address(handle, GET_PROPERTY_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and the format says what the symbol is: the signature is the
        // one the ABI writes down.
        Ok(unsafe { std::mem::transmute::<usize, GetProperty>(address) })
    }

    /// The symbol that writes a property.
    pub fn set_property(handle: usize) -> Result<SetProperty, String> {
        let address = address(handle, SET_PROPERTY_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and the format says what the symbol is.
        Ok(unsafe { std::mem::transmute::<usize, SetProperty>(address) })
    }

    /// The symbol that sets the unit up.
    pub fn initialize(handle: usize) -> Result<Initialize, String> {
        let address = address(handle, INITIALIZE_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and the format says what the symbol is.
        Ok(unsafe { std::mem::transmute::<usize, Initialize>(address) })
    }

    /// The symbol that takes the unit down.
    pub fn uninitialize(handle: usize) -> Result<Uninitialize, String> {
        let address = address(handle, UNINITIALIZE_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and the format says what the symbol is.
        Ok(unsafe { std::mem::transmute::<usize, Uninitialize>(address) })
    }

    /// The symbol that puts the unit back as it was.
    pub fn reset(handle: usize) -> Result<Reset, String> {
        let address = address(handle, RESET_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and the format says what the symbol is.
        Ok(unsafe { std::mem::transmute::<usize, Reset>(address) })
    }

    /// The symbol that renders a block.
    pub fn render(handle: usize) -> Result<Render, String> {
        let address = address(handle, RENDER_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and the format says what the symbol is.
        Ok(unsafe { std::mem::transmute::<usize, Render>(address) })
    }

    /// **This tree's** symbol that hands out an instance.
    pub fn make_instance(handle: usize) -> Result<MakeInstance, String> {
        let address = address(handle, INSTANCE_SYMBOL)?;
        // SAFETY: the address came out of this library with this name,
        // and this tree's own door has this signature.
        Ok(unsafe { std::mem::transmute::<usize, MakeInstance>(address) })
    }

    /// The last error the loader had.
    fn last_error() -> String {
        // SAFETY: the loader returns either nothing or a C string of
        // its own, and the string is read out at once.
        let pointer = unsafe { dlerror() };
        if pointer.is_null() {
            return "no more".to_string();
        }
        // SAFETY: the loader's own string, ended by a zero byte.
        let message = unsafe { std::ffi::CStr::from_ptr(pointer) };
        message.to_string_lossy().into_owned()
    }
}
