//! `craftkit-vst3` — the **VST3 host**: open a plugin, read its factory,
//! and drive the component inside it.
//!
//! # What this is
//!
//! The other half of [`craftkit_vst3_abi`]. The ABI writes the format
//! down as shapes; this crate **uses** them: it opens a shared library,
//! calls the entry the format names, asks the factory what it can make,
//! makes one, and drives it.
//!
//! It is a **library** and not a plugin: it contributes no command and
//! no kind, and it is at L1, so a battery at L4 may depend on it.
//!
//! # The entry is a **function**
//!
//! This is the difference from CLAP, whose entry is a *variable* whose
//! value is a pointer. A VST3 plugin exports a **function**, and the
//! host **calls** it to get the factory back. A door that is called and
//! a door that is read are different doors, and the host has to know
//! which one it is.
//!
//! # The factory is the **table**
//!
//! The factory the entry gives back is the factory's own table of
//! functions: it has one interface and no state of its own, so the
//! pointer the host holds is the table.
//!
//! An **instance** is different. The factory hands back an **object**,
//! and the object's first word is the address of its table — which is
//! how a virtual table is laid out, and what the format means when it
//! says that an object implements an interface.
//!
//! # The instance is not kept
//!
//! An instance holds addresses into a library that is open, and it is
//! destroyed when the host is finished with it. So nothing here keeps
//! one: a caller opens the plugin, uses the instance, and lets it go.
//! The **state** is what travels — the plugin's own bytes, which the
//! host carries and the plugin alone understands.
//!
//! # Why it is `unsafe`
//!
//! Opening a library, calling into it, and reading the memory it wrote
//! is `unsafe` by definition: the host is calling code it has not read,
//! at an address it did not choose. So this crate is on v2's **list**
//! of exceptions — the seventh entry — and it is held to the same rules
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

use craftkit_vst3_abi::{AUDIO_PROCESSOR_FUID, COMPONENT_FUID, IPluginFactory};

pub use craftkit_vst3_abi::{BUS_DIRECTION_INPUT, BUS_DIRECTION_OUTPUT};

/// What went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vst3Error {
    /// What happened, in words.
    pub message: String,
}

impl Vst3Error {
    /// A failure with a message.
    fn new(message: impl Into<String>) -> Vst3Error {
        Vst3Error { message: message.into() }
    }
}

impl std::fmt::Display for Vst3Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for Vst3Error {}

/// The result of a call into a plugin.
pub type Result<T> = std::result::Result<T, Vst3Error>;

/// One **class** of a factory: what the plugin can make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vst3Class {
    /// The class's identifier: the name a host asks by.
    pub cid: String,
    /// What kind of thing it is.
    pub category: String,
    /// Its name, for a person.
    pub name: String,
    /// Who makes it.
    pub vendor: String,
    /// Its version.
    pub version: i32,
}

/// One **bus** of a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vst3Bus {
    /// What kind of bus it is.
    pub bus_type: i32,
    /// Which way the audio goes.
    pub direction: i32,
    /// How many channels it has.
    pub channels: i32,
    /// Its name.
    pub name: String,
}

/// One **parameter** of a component.
#[derive(Debug, Clone, PartialEq)]
pub struct Vst3Param {
    /// Its identifier: the name a host asks by.
    pub id: String,
    /// Its name, for a person.
    pub title: String,
    /// A shorter name.
    pub short_title: String,
    /// Its units, if it has any.
    pub units: String,
    /// The step between values.
    pub step_size: i32,
    /// The value it starts at, between zero and one.
    pub default_normalized_value: f64,
    /// What the flags say.
    pub flags: i32,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
}

/// Which **interface** an instance was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vst3Interface {
    /// The component: the plugin's own half.
    Component,
    /// The audio processor: the half that does the sound.
    AudioProcessor,
}

/// A **plugin**: an open library and the factory inside it.
///
/// The library is closed when this is dropped, and the factory is only
/// read while it is open.
pub struct Vst3Plugin {
    /// The open library.
    handle: usize,
    /// The factory the entry gave back.
    factory: *const IPluginFactory,
    /// Where the library is, for the messages.
    path: String,
}

impl Vst3Plugin {
    /// **Open** a plugin's library and call its entry.
    ///
    /// The entry is called once, and the factory it gives back is held
    /// until this is dropped.
    pub fn open(path: &str) -> Result<Vst3Plugin> {
        let handle = foreign::open(path).map_err(Vst3Error::new)?;
        let factory = match foreign::entry(handle) {
            Ok(factory) => factory,
            Err(message) => {
                // The library is closed again: it was opened, and it
                // is not going to be used.
                foreign::close(handle);
                return Err(Vst3Error::new(message));
            }
        };
        Ok(Vst3Plugin { handle, factory, path: path.to_string() })
    }

    /// Where the library is.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The **factory**: what the plugin can make.
    pub fn factory(&self) -> Vst3Factory<'_> {
        Vst3Factory { factory: self.factory, plugin: self }
    }
}

impl Drop for Vst3Plugin {
    fn drop(&mut self) {
        foreign::close(self.handle);
    }
}

/// The **factory**: the plugin's list of what it can make.
pub struct Vst3Factory<'a> {
    /// The factory's own table.
    factory: *const IPluginFactory,
    /// The library it is in, so that it stays open.
    plugin: &'a Vst3Plugin,
}

impl Vst3Factory<'_> {
    /// How many classes the factory holds.
    pub fn count(&self) -> i32 {
        foreign::count_classes(self.factory)
    }

    /// What the class at that index says about itself.
    pub fn class(&self, index: i32) -> Result<Vst3Class> {
        foreign::class_info(self.factory, index).map_err(Vst3Error::new)
    }

    /// Every class the factory holds.
    pub fn classes(&self) -> Result<Vec<Vst3Class>> {
        let mut found = Vec::new();
        for index in 0..self.count() {
            found.push(self.class(index)?);
        }
        Ok(found)
    }

    /// Make one instance of the class with this identifier.
    ///
    /// The interface is named by its id: the component is the plugin's
    /// own half, and the audio processor is the half that does the
    /// sound.
    pub fn create(&self, cid: &str, interface: Vst3Interface) -> Result<Vst3Instance> {
        let fuid = match interface {
            Vst3Interface::Component => COMPONENT_FUID,
            Vst3Interface::AudioProcessor => AUDIO_PROCESSOR_FUID,
        };
        let tables = foreign::create_instance(self.factory, cid, fuid).map_err(Vst3Error::new)?;
        // The library is kept open for as long as the instance is:
        // the instance holds addresses into it.
        let _ = &self.plugin;
        Ok(Vst3Instance { tables, interface })
    }
}

/// An **instance**: a plugin that has been made.
pub struct Vst3Instance {
    /// The object's address and the tables behind it.
    tables: foreign::Tables,
    /// Which interface it was made for.
    interface: Vst3Interface,
}

impl Vst3Instance {
    /// Which interface it was made for.
    pub fn interface(&self) -> Vst3Interface {
        self.interface
    }

    /// The **component**'s own methods, and the processor is not the
    /// component.
    ///
    /// The two interfaces are two tables, and a host that called the
    /// component's methods on the processor's table would be calling
    /// the wrong thing. So the host says so.
    fn require_component(&self) -> Result<()> {
        if self.tables.component {
            Ok(())
        } else {
            Err(Vst3Error::new(
                "this instance is the audio processor, and the component's methods are not its own",
            ))
        }
    }

    /// Set the plugin up.
    pub fn initialize(&self) -> Result<()> {
        self.require_component()?;
        foreign::initialize(self.tables.object, self.tables.table).map_err(Vst3Error::new)
    }

    /// Take the plugin down.
    pub fn terminate(&self) -> Result<()> {
        self.require_component()?;
        foreign::terminate(self.tables.object, self.tables.table).map_err(Vst3Error::new)
    }

    /// How many buses of this kind there are.
    pub fn bus_count(&self, direction: i32) -> i32 {
        if !self.tables.component {
            return 0;
        }
        foreign::bus_count(self.tables.object, self.tables.table, direction)
    }

    /// What the bus at that index is.
    pub fn bus_info(&self, direction: i32, index: i32) -> Result<Vst3Bus> {
        self.require_component()?;
        foreign::bus_info(self.tables.object, self.tables.table, direction, index).map_err(Vst3Error::new)
    }

    /// How many parameters there are.
    pub fn param_count(&self) -> i32 {
        if !self.tables.component {
            return 0;
        }
        foreign::param_count(self.tables.object, self.tables.table)
    }

    /// What the parameter at that index is.
    pub fn param_info(&self, index: i32) -> Result<Vst3Param> {
        self.require_component()?;
        foreign::param_info(self.tables.object, self.tables.table, index).map_err(Vst3Error::new)
    }

    /// The parameter's value, between zero and one.
    pub fn param_value(&self, index: i32) -> Result<f32> {
        self.require_component()?;
        foreign::param_value(self.tables.object, self.tables.table, index).map_err(Vst3Error::new)
    }

    /// Set the parameter's value, between zero and one.
    pub fn set_param_value(&self, index: i32, value: f32) -> Result<()> {
        self.require_component()?;
        foreign::set_param_value(self.tables.object, self.tables.table, index, value).map_err(Vst3Error::new)
    }

    /// The plugin's own state, as bytes.
    ///
    /// The host cannot read a state: the bytes are the plugin's own.
    /// So the plugin is asked for them and they are carried whole.
    pub fn state(&self) -> Result<Vec<u8>> {
        self.require_component()?;
        foreign::state(self.tables.object, self.tables.table).map_err(Vst3Error::new)
    }

    /// Give the plugin the state it had.
    pub fn set_state(&self, bytes: &[u8]) -> Result<()> {
        self.require_component()?;
        foreign::set_state(self.tables.object, self.tables.table, bytes).map_err(Vst3Error::new)
    }

    /// Process a block of audio.
    ///
    /// The **channels** are the host's own, and the plugin is told how
    /// many there are: a host that guessed the shape of a plugin's
    /// audio would break the plugins that check.
    pub fn process(&self, inputs: &[&[f32]], outputs: &[&mut [f32]], frames: usize) -> Result<()> {
        foreign::process(
            self.tables.object,
            self.tables.table,
            self.tables.component,
            inputs,
            outputs,
            frames,
        )
        .map_err(Vst3Error::new)
    }
}

impl Drop for Vst3Instance {
    fn drop(&mut self) {
        // The reference is given back: the object ends when the last
        // one lets go.
        foreign::release(self.tables.object, self.tables.table);
    }
}

/// The **foreign** part: the only `unsafe` in this crate.
///
/// Opening a library, calling into it and reading the memory it wrote
/// is unsafe by definition: the host is calling code it has not read,
/// at an address it did not choose. Every block below says, in a
/// comment, why it is sound — and the soundness always comes from the
/// same three things: the library is open, the table came out of the
/// library, and the pointer came from the host itself.
#[cfg(unix)]
#[allow(unsafe_code)]
mod foreign {
    use std::ffi::{CString, c_char, c_void};
    use std::sync::OnceLock;

    use craftkit_vst3_abi::{
        AUDIO_PROCESSOR_FUID, AudioBusBuffers, BusArrangement, BusInfo, COMPONENT_FUID, ClassInfo,
        FactoryInfo, Fuid, IAudioProcessor, IBStream, IComponent, IPluginFactory, Instance, ParamInfo,
        ProcessData, SAMPLE_SIZE_32,
    };

    use super::{Vst3Bus, Vst3Class, Vst3Param};

    /// Resolve the library completely before it is used.
    pub const RTLD_NOW: i32 = 2;
    /// Do not offer this library's symbols to anything else.
    pub const RTLD_LOCAL: i32 = 0;

    /// How long a string from the plugin may be, in bytes.
    const MAX_STRING_BYTES: usize = 64 * 1024;

    /// The tables of an **instance**.
    pub struct Tables {
        /// The object itself.
        pub object: *mut c_void,
        /// The address of the table the object was made for.
        pub table: *const c_void,
        /// Whether the table is the **component's**.
        ///
        /// The component and the audio processor are different tables,
        /// and the instance knows which one it holds: a host that asked
        /// for the processor must not go on to call the component's
        /// methods.
        pub component: bool,
    }

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

    /// The plugin's **factory**, called through the entry the format
    /// names.
    ///
    /// The entry is a **function**: the host calls it, and it gives
    /// back the factory. This is the difference from the formats whose
    /// entry is a variable.
    pub fn entry(handle: usize) -> Result<*const IPluginFactory, String> {
        let wanted = match CString::new(craftkit_vst3_abi::ENTRY_SYMBOL) {
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
                craftkit_vst3_abi::ENTRY_SYMBOL,
                last_error()
            ));
        }
        // SAFETY: the symbol is the plugin's entry point, and the format
        // says it is a function that gives back the factory. The
        // function is called once, and the factory it gives back lives
        // as long as the library is open.
        let entry: extern "system" fn() -> *const IPluginFactory = unsafe { std::mem::transmute(symbol) };
        let factory = entry();
        if factory.is_null() {
            return Err("the plugin's entry gave back no factory".to_string());
        }
        Ok(factory)
    }

    /// How many classes the factory holds.
    pub fn count_classes(factory: *const IPluginFactory) -> i32 {
        // SAFETY: the factory came out of the entry of a library that
        // is open, and the count is the format's own method called on
        // it.
        let count = unsafe { (*factory).count_classes };
        match count {
            Some(count) => count(factory),
            None => 0,
        }
    }

    /// What the class at that index says about itself.
    pub fn class_info(factory: *const IPluginFactory, index: i32) -> Result<Vst3Class, String> {
        // SAFETY: the factory came out of the entry of a library that
        // is open, and the method is the format's own.
        let get_class_info = unsafe { (*factory).get_class_info };
        let Some(get_class_info) = get_class_info else {
            return Err("the factory has no `get_class_info`".to_string());
        };
        let mut info: ClassInfo =
            ClassInfo { cid: std::ptr::null(), cardinality: std::ptr::null(), class_info: std::ptr::null() };
        // SAFETY: the room is this stack's own, and the format says
        // the method fills it in when it says yes.
        if !get_class_info(factory, index, &mut info) {
            return Err(format!("the factory has no class at {index}"));
        }
        // SAFETY: the plugin filled the room in, and the two pointers
        // are the plugin's own — the class's identifier and its
        // information. Both are read out at once, because the plugin
        // may change them later.
        let (cid, class) = unsafe {
            let cid = text(info.cid);
            let class =
                if info.class_info.is_null() { FactoryInfo::new("", "", "", 0) } else { *info.class_info };
            (cid, class)
        };
        Ok(Vst3Class {
            cid,
            category: class.category(),
            name: class.name(),
            vendor: class.vendor(),
            version: class.version,
        })
    }

    /// Make one instance of the class with this identifier.
    pub fn create_instance(factory: *const IPluginFactory, cid: &str, fuid: Fuid) -> Result<Tables, String> {
        // SAFETY: the factory came out of the entry of a library that
        // is open, and the method is the format's own.
        let create_instance = unsafe { (*factory).create_instance };
        let Some(create_instance) = create_instance else {
            return Err("the factory has no `create_instance`".to_string());
        };
        let wanted = match CString::new(cid) {
            Ok(wanted) => wanted,
            Err(_) => return Err("the class's name has a zero byte in it".to_string()),
        };
        // SAFETY: the factory is the plugin's own, the name is a valid
        // C string, and the id is the format's own sixteen bytes.
        let object = create_instance(factory, wanted.as_ptr(), fuid);
        if object.is_null() {
            return Err(format!("the plugin would not make `{cid}`"));
        }
        // SAFETY: the object is the plugin's own, and its first word is
        // the address of its table — which is what the format means by
        // an object implementing an interface.
        let table = unsafe { (*(object as *const Instance)).table };
        if table.is_null() {
            return Err("the instance has no table".to_string());
        }
        let tables = if fuid == COMPONENT_FUID {
            Tables { object, table, component: true }
        } else if fuid == AUDIO_PROCESSOR_FUID {
            Tables { object, table, component: false }
        } else {
            return Err("the plugin was asked for an interface it does not have".to_string());
        };
        Ok(tables)
    }

    /// The **component** table of an instance.
    fn component(table: *const c_void) -> Result<*const IComponent, String> {
        if table.is_null() {
            return Err("the instance has no table".to_string());
        }
        Ok(table as *const IComponent)
    }

    /// Give a reference back.
    pub fn release(object: *mut c_void, table: *const c_void) {
        if object.is_null() {
            return;
        }
        if table.is_null() {
            return;
        }
        // SAFETY: the table came out of the object itself, and the
        // release is the format's own method called on it. It is
        // called once, from the drop of the instance.
        let release = unsafe { (*(table as *const IComponent)).release };
        if let Some(release) = release {
            release(object);
        }
    }

    /// Set the plugin up.
    pub fn initialize(object: *mut c_void, table: *const c_void) -> Result<(), String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own. The controller is not one: this
        // host asks the plugin about itself, and nobody is listening.
        let initialize = unsafe { (*table).initialize };
        let Some(initialize) = initialize else {
            return Err("the component has no `initialize`".to_string());
        };
        if initialize(object, std::ptr::null()) {
            Ok(())
        } else {
            Err("the plugin refused to initialise".to_string())
        }
    }

    /// Take the plugin down.
    pub fn terminate(object: *mut c_void, table: *const c_void) -> Result<(), String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let terminate = unsafe { (*table).terminate };
        let Some(terminate) = terminate else {
            return Err("the component has no `terminate`".to_string());
        };
        if terminate(object) { Ok(()) } else { Err("the plugin refused to terminate".to_string()) }
    }

    /// How many buses of this kind there are.
    pub fn bus_count(object: *mut c_void, table: *const c_void, direction: i32) -> i32 {
        let Ok(table) = component(table) else { return 0 };
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let count = unsafe { (*table).get_bus_count };
        match count {
            Some(count) => count(object, direction),
            None => 0,
        }
    }

    /// What the bus at that index is.
    pub fn bus_info(
        object: *mut c_void,
        table: *const c_void,
        direction: i32,
        index: i32,
    ) -> Result<Vst3Bus, String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let get_bus_info = unsafe { (*table).get_bus_info };
        let Some(get_bus_info) = get_bus_info else {
            return Err("the component has no `get_bus_info`".to_string());
        };
        let mut info = BusInfo { bus_type: 0, direction: 0, channels: 0, name: std::ptr::null() };
        // SAFETY: the room is this stack's own, and the format says the
        // method fills it in when it says yes.
        if !get_bus_info(object, direction, index, &mut info) {
            return Err(format!("the plugin has no bus at {index} going {direction}"));
        }
        // SAFETY: the plugin filled the room in, and the name is the
        // plugin's own string. It is read out at once.
        let name = text(info.name);
        Ok(Vst3Bus { bus_type: info.bus_type, direction: info.direction, channels: info.channels, name })
    }

    /// How many parameters there are.
    pub fn param_count(object: *mut c_void, table: *const c_void) -> i32 {
        let Ok(table) = component(table) else { return 0 };
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let count = unsafe { (*table).get_param_count };
        match count {
            Some(count) => count(object),
            None => 0,
        }
    }

    /// What the parameter at that index is.
    pub fn param_info(object: *mut c_void, table: *const c_void, index: i32) -> Result<Vst3Param, String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let get_param_info = unsafe { (*table).get_param_info };
        let Some(get_param_info) = get_param_info else {
            return Err("the component has no `get_param_info`".to_string());
        };
        let mut info = ParamInfo {
            id: std::ptr::null(),
            title: std::ptr::null(),
            short_title: std::ptr::null(),
            units: std::ptr::null(),
            step_size: 0,
            default_normalized_value: 0.0,
            flags: 0,
            min: 0.0,
            max: 0.0,
        };
        // SAFETY: the room is this stack's own, and the format says the
        // method fills it in when it says yes.
        if !get_param_info(object, index, &mut info) {
            return Err(format!("the plugin has no parameter at {index}"));
        }
        // SAFETY: the plugin filled the room in, and the four strings
        // are the plugin's own. They are read out at once.
        let (id, title, short_title, units) =
            (text(info.id), text(info.title), text(info.short_title), text(info.units));
        Ok(Vst3Param {
            id,
            title,
            short_title,
            units,
            step_size: info.step_size,
            default_normalized_value: info.default_normalized_value,
            flags: info.flags,
            min: info.min,
            max: info.max,
        })
    }

    /// The parameter's value, between zero and one.
    pub fn param_value(object: *mut c_void, table: *const c_void, index: i32) -> Result<f32, String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let get_param_normalized = unsafe { (*table).get_param_normalized };
        let Some(get_param_normalized) = get_param_normalized else {
            return Err("the component has no `get_param_normalized`".to_string());
        };
        let mut value = 0.0f32;
        // SAFETY: the room is this stack's own, and the format says the
        // method fills it in when it says yes.
        if !get_param_normalized(object, index, &mut value) {
            return Err(format!("the plugin would not say the value of {index}"));
        }
        Ok(value)
    }

    /// Set the parameter's value, between zero and one.
    pub fn set_param_value(
        object: *mut c_void,
        table: *const c_void,
        index: i32,
        value: f32,
    ) -> Result<(), String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let set_param_normalized = unsafe { (*table).set_param_normalized };
        let Some(set_param_normalized) = set_param_normalized else {
            return Err("the component has no `set_param_normalized`".to_string());
        };
        if set_param_normalized(object, index, value) {
            Ok(())
        } else {
            Err(format!("the plugin would not take the value of {index}"))
        }
    }

    /// The plugin's own state, as bytes.
    pub fn state(object: *mut c_void, table: *const c_void) -> Result<Vec<u8>, String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let get_state = unsafe { (*table).get_state };
        let Some(get_state) = get_state else {
            return Err("the component has no `get_state`".to_string());
        };
        let mut stream = MemStream::new();
        // SAFETY: the stream is this host's own, and the plugin writes
        // into it through the table it finds in its first word.
        if !get_state(object, &mut stream as *mut MemStream as *mut c_void) {
            return Err("the plugin did not save its state".to_string());
        }
        Ok(stream.data)
    }

    /// Give the plugin the state it had.
    pub fn set_state(object: *mut c_void, table: *const c_void, bytes: &[u8]) -> Result<(), String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let set_state = unsafe { (*table).set_state };
        let Some(set_state) = set_state else {
            return Err("the component has no `set_state`".to_string());
        };
        let mut stream = MemStream::with(bytes);
        // SAFETY: the stream is this host's own, and the plugin reads
        // from it through the table it finds in its first word.
        if set_state(object, &mut stream as *mut MemStream as *mut c_void) {
            Ok(())
        } else {
            Err("the plugin would not take the state".to_string())
        }
    }

    /// Process a block of audio.
    pub fn process(
        object: *mut c_void,
        table: *const c_void,
        component: bool,
        inputs: &[&[f32]],
        outputs: &[&mut [f32]],
        frames: usize,
    ) -> Result<(), String> {
        if table.is_null() {
            return Err("the instance has no table".to_string());
        }
        // The **component** and the **audio processor** both have a
        // process, and the two shapes are the same — but the tables
        // are not, so the table is read as the one it is.
        // SAFETY: the table came out of the object itself, and the
        // host knows which of the two it is holding.
        let process = unsafe {
            if component {
                (*(table as *const IComponent)).process
            } else {
                (*(table as *const IAudioProcessor)).process
            }
        };
        let Some(process) = process else {
            return Err("the instance cannot process audio".to_string());
        };
        let input_channels: Vec<*const f32> = inputs.iter().map(|channel| channel.as_ptr()).collect();
        let output_channels: Vec<*const f32> = outputs.iter().map(|channel| channel.as_ptr()).collect();
        let mut input_buffers = vec![bus_buffers(&input_channels)];
        let mut output_buffers = vec![bus_buffers(&output_channels)];
        let mut data = ProcessData {
            mode: 0,
            num_samples: frames as i32,
            symbolic_sample_size: SAMPLE_SIZE_32,
            num_inputs: input_buffers.len() as i32,
            num_outputs: output_buffers.len() as i32,
            inputs: input_buffers.as_mut_ptr() as *mut c_void,
            outputs: output_buffers.as_mut_ptr() as *mut c_void,
            parameter_changes: std::ptr::null_mut(),
            events: std::ptr::null_mut(),
            data: std::ptr::null_mut(),
        };
        // SAFETY: the process data is this host's own, and the buffers
        // it points at are this host's own. The plugin writes into the
        // output channels, which are this host's own memory too.
        if process(object, &mut data) {
            Ok(())
        } else {
            Err("the plugin would not process the block".to_string())
        }
    }

    /// The buffers of one bus.
    fn bus_buffers(channels: &[*const f32]) -> AudioBusBuffers {
        let mut pointers = [std::ptr::null_mut(); 2];
        for (index, channel) in channels.iter().take(2).enumerate() {
            pointers[index] = *channel as *mut c_void;
        }
        AudioBusBuffers {
            num_channels: channels.len() as i32,
            channel_buffers64: pointers,
            silent_flags: [0; 2],
        }
    }

    /// The **stream** the state travels through.
    ///
    /// Its first word is the address of its table — the same convention
    /// the instances follow — so that the plugin can find the
    /// functions and the host can keep the bytes behind them.
    #[repr(C)]
    pub struct MemStream {
        /// The address of the table.
        table: *const IBStream,
        /// The bytes.
        data: Vec<u8>,
        /// Where in them the stream is.
        position: usize,
    }

    impl MemStream {
        /// A stream with nothing in it.
        fn new() -> MemStream {
            MemStream { table: table(), data: Vec::new(), position: 0 }
        }

        /// A stream with bytes in it, ready to be read.
        fn with(bytes: &[u8]) -> MemStream {
            MemStream { table: table(), data: bytes.to_vec(), position: 0 }
        }
    }

    /// The table of the stream, made once.
    fn table() -> *const IBStream {
        static TABLE: OnceLock<IBStream> = OnceLock::new();
        TABLE.get_or_init(|| IBStream {
            read: Some(read),
            write: Some(write),
            seek: Some(seek),
            tell: Some(tell),
        })
    }

    /// Read bytes out of the stream.
    extern "system" fn read(stream: *mut c_void, buffer: *mut c_void, num_bytes: i32) -> i32 {
        let Some(stream) = (unsafe { stream.cast::<MemStream>().as_mut() }) else { return 0 };
        let wanted = num_bytes.max(0) as usize;
        let available = stream.data.len().saturating_sub(stream.position);
        let count = wanted.min(available);
        if count > 0 {
            // SAFETY: the buffer is the caller's own, and the count is
            // no more than the room the caller asked for.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    stream.data.as_ptr().add(stream.position),
                    buffer.cast::<u8>(),
                    count,
                );
            }
            stream.position += count;
        }
        count as i32
    }

    /// Write bytes into the stream.
    extern "system" fn write(stream: *mut c_void, buffer: *const c_void, num_bytes: i32) -> i32 {
        let Some(stream) = (unsafe { stream.cast::<MemStream>().as_mut() }) else { return 0 };
        let count = num_bytes.max(0) as usize;
        if count == 0 {
            return 0;
        }
        // SAFETY: the buffer is the caller's own, and the count is the
        // number of bytes the caller says are there.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), count) };
        stream.data.truncate(stream.position);
        stream.data.extend_from_slice(bytes);
        stream.position = stream.data.len();
        count as i32
    }

    /// Move to a place in the stream.
    extern "system" fn seek(stream: *mut c_void, pos: i64) -> i32 {
        let Some(stream) = (unsafe { stream.cast::<MemStream>().as_mut() }) else { return -1 };
        let wanted = pos.max(0) as usize;
        stream.position = wanted.min(stream.data.len());
        0
    }

    /// Where the stream is now.
    extern "system" fn tell(stream: *mut c_void) -> i64 {
        let Some(stream) = (unsafe { stream.cast::<MemStream>().as_mut() }) else { return -1 };
        stream.position as i64
    }

    /// A string out of the plugin's own memory, read **bounded**.
    ///
    /// The plugin decides how long its string is and the host decides
    /// how much it will read: a string that is not ended is the
    /// plugin's mistake and not the host's crash.
    fn text(pointer: *const c_char) -> String {
        if pointer.is_null() {
            return String::new();
        }
        // SAFETY: the pointer came out of the plugin's own structure,
        // and the read stops at the zero byte that ends the string or
        // at the bound, whichever comes first.
        let bytes = unsafe {
            let mut length = 0usize;
            while length < MAX_STRING_BYTES {
                let byte = *pointer.add(length);
                if byte == 0 {
                    break;
                }
                length += 1;
            }
            std::slice::from_raw_parts(pointer.cast::<u8>(), length)
        };
        String::from_utf8_lossy(bytes).into_owned()
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

    /// The **arrangement** of the buses, read out of the component.
    ///
    /// It is not used by this crate yet, and it is here so that the
    /// shape is not lost: a host that asks for the arrangement is a
    /// host that knows how many channels a plugin has.
    #[allow(dead_code)]
    pub fn arrangements(
        object: *mut c_void,
        table: *const c_void,
        arrangements: &mut [BusArrangement],
    ) -> Result<(), String> {
        let table = component(table)?;
        // SAFETY: the table came out of the object itself, and the
        // method is the format's own.
        let get = unsafe { (*table).get_bus_arrangements };
        let Some(get) = get else {
            return Err("the component has no `get_bus_arrangements`".to_string());
        };
        // SAFETY: the room is the caller's own, and the plugin fills in
        // as much of it as it has buses.
        if get(object, arrangements.as_mut_ptr() as *mut c_void) {
            Ok(())
        } else {
            Err("the plugin would not say how its buses are arranged".to_string())
        }
    }
}
