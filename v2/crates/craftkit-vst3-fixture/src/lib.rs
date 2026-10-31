//! `craftkit-vst3-fixture` — a **reference VST3 plugin**.
//!
//! # Why it is here
//!
//! A host that opens a plugin is only worth something if the plugin it
//! opens is a **real** one: a shared library, exporting the entry by
//! name, built by the compiler and opened from the disk. So this crate
//! is a plugin, in the strict sense — a `cdylib` — and the host's
//! tests open it the way they would open anybody else's.
//!
//! It is a **fixture**, and it says what it is: it holds one class, one
//! parameter, one bus in and one bus out, and it does the one thing a
//! plugin is for — it makes the sound louder or quieter.
//!
//! # The shape of it
//!
//! A VST3 plugin exports **one function**: `GetPluginFactory`, and
//! calling it gives back the factory. The factory says how many classes
//! it holds and makes one on request; a class is a **component**, and
//! the component holds the parameter, the buses and the state.
//!
//! # The entry is a **function**
//!
//! This is the difference from CLAP, whose entry is a *variable*. Here
//! the host **calls** the symbol and gets the factory back, and it
//! never reads a pointer out of the symbol's address. A door that is
//! called and a door that is read are different doors, and the host has
//! to know which one it is.
//!
//! # The object and its table
//!
//! The factory hands back an **object**, and the object's first word is
//! the address of its table of functions. That is how a virtual table
//! is laid out, and it is what the format means when it says that an
//! object implements an interface.
//!
//! # Two interfaces, two objects
//!
//! In the format, the component and the audio processor are **the same
//! object** seen through two tables: the host asks the factory for the
//! processor and gets the object it already holds.
//!
//! This tree's shape cannot say that. The table is the object's **first
//! word**, and the first word is one thing — so an object made for the
//! processor holds the processor's table, and the two tables cannot
//! live in one object.
//!
//! So the fixture makes the processor a **second instance**, with a
//! state of its own. The divergence is written down here rather than
//! hidden: a program that needs the two to be one object needs a shape
//! that can say so, and this one does not have it.
//!
//! # Why it is `unsafe`
//!
//! Writing through the pointers the host hands over is `unsafe` in the
//! plain sense: those pointers are the host's memory, and the plugin
//! writes into them. So this crate is on v2's **list** of exceptions —
//! the seventh entry — and it is held to the same rules as the others:
//! `deny(unsafe_code)` and not `forbid`, **one** `allow(unsafe_code)`,
//! and the `unsafe` inside **one module**.
//!
//! It has **no** `unsafe impl` and **no** `unsafe fn`: the only unsafe
//! thing it does is read and write through a pointer the host gave it,
//! and give a reference back.

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

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The class's identifier.
///
/// VST3 names a class `Something::Something`, and this one is named
/// after the tree it belongs to.
pub const CID: &str = "Craftkit::Fixture";

/// The class's name, for a person.
pub const NAME: &str = "Craftkit Reference";

/// Who makes it.
pub const VENDOR: &str = "Craftkit";

/// What kind of thing it is.
pub const CATEGORY: &str = "Audio Module Class";

/// The parameter's identifier: the name a host asks by.
pub const GAIN_ID: &str = "gain";

/// The parameter's name, for a person.
pub const GAIN_TITLE: &str = "Gain";

/// The smallest value the gain takes.
pub const GAIN_MIN: f32 = 0.0;

/// The largest value the gain takes.
pub const GAIN_MAX: f32 = 2.0;

/// The value the gain starts at.
pub const GAIN_DEFAULT: f32 = 1.0;

/// How many instances may exist at once.
///
/// The instances are handed out from a fixed pool: a fixture that grew
/// without limit would be a fixture that could not be reasoned about.
/// The pool is **wide**, because the host's tests open the library
/// once — the loader gives the same library to every test in a
/// process — and each of them makes an instance of its own.
pub const MAX_INSTANCES: usize = 64;

/// The class's identifier, as the host reads it: a C string.
const CID_BYTES: &[u8] = b"Craftkit::Fixture\0";

/// The parameter's identifier, as the host reads it.
const GAIN_ID_BYTES: &[u8] = b"gain\0";

/// The parameter's name, as the host reads it.
const GAIN_TITLE_BYTES: &[u8] = b"Gain\0";

/// The bus's name, as the host reads it.
const BUS_NAME_BYTES: &[u8] = b"Main\0";

/// The parameter's units: it has none.
const GAIN_UNITS_BYTES: &[u8] = b"\0";

/// Whether any instance has been initialised.
///
/// The host's tests check that it did — so the plugin keeps the answer
/// where a test can see it.
static INITIALISED: AtomicBool = AtomicBool::new(false);

/// How many instances have ever been made.
static MADE: AtomicU32 = AtomicU32::new(0);

/// How many slots have been handed out.
static NEXT_SLOT: AtomicU32 = AtomicU32::new(0);

/// One **instance**'s own state.
///
/// The state is behind the slot, and the slot is shared by the
/// component and the processor of the same instance.
struct Slot {
    /// The gain, as the bits of a thirty-two bit float.
    gain: AtomicU32,
}

const fn slot() -> Slot {
    Slot { gain: AtomicU32::new(GAIN_DEFAULT.to_bits()) }
}

/// The instances this plugin can have at once.
static SLOTS: [Slot; MAX_INSTANCES] = [const { slot() }; MAX_INSTANCES];

/// Whether any instance has been initialised.
pub fn initialised() -> bool {
    INITIALISED.load(Ordering::SeqCst)
}

/// How many instances have ever been made.
pub fn made() -> u32 {
    MADE.load(Ordering::SeqCst)
}

/// The plugin's own functions.
///
/// The `unsafe` is here and nowhere else: every one of the plugin's
/// functions reads or writes through a pointer the host gave it, and
/// every one of them says why in a comment where the pointer is used.
#[allow(unsafe_code)]
mod exports {
    use std::ffi::{c_char, c_void};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU32, Ordering};

    use craftkit_vst3_abi::{
        AudioBusBuffers, BusArrangement, BusInfo, COMPONENT_FUID, ClassInfo, FactoryInfo, Fuid,
        IAudioProcessor, IComponent, IPluginFactory, ParamInfo, ProcessData, Stream,
    };

    use super::{
        BUS_NAME_BYTES, CATEGORY, CID, CID_BYTES, GAIN_DEFAULT, GAIN_ID_BYTES, GAIN_MAX, GAIN_MIN,
        GAIN_TITLE_BYTES, GAIN_UNITS_BYTES, INITIALISED, MADE, MAX_INSTANCES, NAME, NEXT_SLOT, SLOTS, VENDOR,
    };

    /// The **object** the factory hands back.
    ///
    /// Its first word is the address of its table: that is how an
    /// object implements an interface, and it is the shape the host
    /// reads. The rest is this crate's own.
    #[repr(C)]
    pub struct Object {
        /// The table of the interface the host asked for.
        pub table: *const c_void,
        /// Which slot this object is.
        pub slot: usize,
        /// How many references the object has.
        pub refs: AtomicU32,
    }

    /// The **component** table: what an instance of this plugin can do.
    static COMPONENT: IComponent = IComponent {
        add_ref: Some(add_ref),
        release: Some(release),
        initialize: Some(initialize),
        terminate: Some(terminate),
        set_bus_arrangements: Some(set_bus_arrangements),
        get_bus_arrangements: Some(get_bus_arrangements),
        get_bus_count: Some(get_bus_count),
        get_bus_info: Some(get_bus_info),
        get_param_count: Some(get_param_count),
        get_param_info: Some(get_param_info),
        get_param_normalized: Some(get_param_normalized),
        set_param_normalized: Some(set_param_normalized),
        get_state: Some(get_state),
        set_state: Some(set_state),
        process: Some(process),
    };

    /// The **audio processor** table: the half that does the sound.
    static PROCESSOR: IAudioProcessor = IAudioProcessor {
        add_ref: Some(add_ref),
        release: Some(release),
        set_bus_arrangements: Some(set_bus_arrangements),
        get_bus_arrangements: Some(get_bus_arrangements),
        process: Some(process),
    };

    /// The **factory**: the plugin's list of what it can make.
    static FACTORY: IPluginFactory = IPluginFactory {
        count_classes: Some(count_classes),
        get_class_info: Some(get_class_info),
        create_instance: Some(create_instance),
    };

    /// What the class says about itself.
    ///
    /// It is made **once** and kept: the host reads the strings out of
    /// it and the address has to stay where it is.
    static CLASS_INFO: OnceLock<FactoryInfo> = OnceLock::new();

    /// The entry the host looks for.
    ///
    /// The `unsafe` is in the attribute and nowhere else: giving a
    /// symbol a name is a promise to the loader that the thing behind
    /// the name is what the format says it is. It is — the factory of
    /// the format, and nothing else.
    #[unsafe(no_mangle)]
    pub extern "system" fn GetPluginFactory() -> *const IPluginFactory {
        &FACTORY
    }

    /// How many classes the factory holds.
    extern "system" fn count_classes(_factory: *const IPluginFactory) -> i32 {
        1
    }

    /// How many classes the factory holds, said **safely**.
    ///
    #[cfg(test)]
    ///
    /// The door for the crate's own tests: the unsafe is in here,
    /// where the factory is this crate's own.
    pub fn classes() -> i32 {
        // SAFETY: the factory is this crate's own static, and the
        // count is the format's own method called on it.
        let count = unsafe { (*std::ptr::from_ref(&FACTORY)).count_classes };
        match count {
            Some(count) => count(&FACTORY),
            None => 0,
        }
    }

    /// What the class at that index says about itself.
    extern "system" fn get_class_info(
        _factory: *const IPluginFactory,
        index: i32,
        info: *mut ClassInfo,
    ) -> bool {
        if index != 0 || info.is_null() {
            return false;
        }
        let class = CLASS_INFO.get_or_init(|| FactoryInfo::new(CATEGORY, NAME, VENDOR, 1));
        // SAFETY: the host gave room for one class information, and
        // this is that room. The three strings are this crate's own,
        // and they live for as long as the library is open.
        unsafe {
            (*info).cid = CID_BYTES.as_ptr() as *const c_char;
            (*info).cardinality = CID_BYTES.as_ptr() as *const c_char;
            (*info).class_info = std::ptr::from_ref(class);
        }
        true
    }

    /// Make one instance of the class.
    extern "system" fn create_instance(
        _factory: *const IPluginFactory,
        cid: *const c_char,
        fuid: Fuid,
    ) -> *mut c_void {
        if cid.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: the host asks by name, and a name is a C string: a
        // byte is read at a time and the read stops at the zero byte
        // that ends it.
        let wanted = unsafe { std::ffi::CStr::from_ptr(cid) };
        if wanted.to_bytes() != CID.as_bytes() {
            return std::ptr::null_mut();
        }
        // The ids are **compared**, not matched: a name in a pattern
        // is a binding, and a binding would take every arm.
        if fuid == COMPONENT_FUID {
            return make(&COMPONENT as *const IComponent as *const c_void);
        }
        if fuid == craftkit_vst3_abi::AUDIO_PROCESSOR_FUID {
            return make(&PROCESSOR as *const IAudioProcessor as *const c_void);
        }
        std::ptr::null_mut()
    }

    /// An **object**, with a slot behind it.
    ///
    /// Every instance gets a slot of its own: the state belongs to the
    /// instance, and two instances that shared it would be one instance
    /// wearing two hats.
    fn make(table: *const c_void) -> *mut c_void {
        let index = NEXT_SLOT.fetch_add(1, Ordering::SeqCst) as usize;
        if index >= MAX_INSTANCES {
            return std::ptr::null_mut();
        }
        let object = Box::into_raw(Box::new(Object { table, slot: index, refs: AtomicU32::new(1) }));
        MADE.fetch_add(1, Ordering::SeqCst);
        object as *mut c_void
    }

    /// The slot behind an object.
    fn slot_of(this: *mut c_void) -> Option<usize> {
        if this.is_null() {
            return None;
        }
        // SAFETY: the object is this crate's own, and the host hands
        // back the very pointer it was given.
        let object = unsafe { &*(this as *const Object) };
        Some(object.slot)
    }

    /// The gain of a slot.
    fn gain_of(slot: usize) -> f32 {
        f32::from_bits(SLOTS[slot].gain.load(Ordering::SeqCst))
    }

    /// Set the gain of a slot.
    fn set_gain_of(slot: usize, value: f32) {
        SLOTS[slot].gain.store(value.to_bits(), Ordering::SeqCst);
    }

    /// Take a reference.
    extern "system" fn add_ref(this: *mut c_void) -> i32 {
        if this.is_null() {
            return 0;
        }
        // SAFETY: the object is this crate's own, and the host holds
        // the very pointer it was given.
        let object = unsafe { &*(this as *const Object) };
        object.refs.fetch_add(1, Ordering::SeqCst) as i32
    }

    /// Give a reference back.
    ///
    /// The last one to let go ends the object: the format counts
    /// references so that the host and the plugin may each hold it for
    /// a while, and this is where the holding stops.
    extern "system" fn release(this: *mut c_void) -> i32 {
        if this.is_null() {
            return 0;
        }
        // SAFETY: the object is this crate's own, and the host hands
        // back the very pointer it was given. The count is read before
        // the object may be freed.
        let object = unsafe { &*(this as *const Object) };
        let previous = object.refs.fetch_sub(1, Ordering::SeqCst);
        if previous <= 1 {
            // SAFETY: this is the last reference, so nobody else holds
            // the object, and it was made by `Box::into_raw` in `make`.
            drop(unsafe { Box::from_raw(this as *mut Object) });
            return 0;
        }
        (previous - 1) as i32
    }

    /// Set the plugin up.
    extern "system" fn initialize(this: *mut c_void, _controller: *const c_void) -> bool {
        if slot_of(this).is_none() {
            return false;
        }
        INITIALISED.store(true, Ordering::SeqCst);
        true
    }

    /// Take the plugin down.
    extern "system" fn terminate(this: *mut c_void) -> bool {
        slot_of(this).is_some()
    }

    /// Say how the buses are arranged.
    extern "system" fn set_bus_arrangements(_this: *mut c_void, _arrangements: *mut c_void) -> bool {
        true
    }

    /// Say how the buses are arranged.
    extern "system" fn get_bus_arrangements(this: *mut c_void, arrangements: *mut c_void) -> bool {
        if slot_of(this).is_none() || arrangements.is_null() {
            return false;
        }
        // SAFETY: the host asked for the arrangements of a plugin with
        // one bus in and one bus out, and it gave room for both.
        unsafe {
            let list = arrangements as *mut BusArrangement;
            (*list.add(0)) = BusArrangement {
                direction: craftkit_vst3_abi::BUS_DIRECTION_INPUT,
                num_channels: 1,
                channel_map: [0; 64],
            };
            (*list.add(1)) = BusArrangement {
                direction: craftkit_vst3_abi::BUS_DIRECTION_OUTPUT,
                num_channels: 1,
                channel_map: [0; 64],
            };
        }
        true
    }

    /// How many buses of this kind there are.
    extern "system" fn get_bus_count(_this: *mut c_void, direction: i32) -> i32 {
        match direction {
            craftkit_vst3_abi::BUS_DIRECTION_INPUT => 1,
            craftkit_vst3_abi::BUS_DIRECTION_OUTPUT => 1,
            _ => 0,
        }
    }

    /// What the bus at this index is.
    extern "system" fn get_bus_info(
        this: *mut c_void,
        direction: i32,
        index: i32,
        info: *mut BusInfo,
    ) -> bool {
        if slot_of(this).is_none() || info.is_null() || index != 0 {
            return false;
        }
        if direction != craftkit_vst3_abi::BUS_DIRECTION_INPUT
            && direction != craftkit_vst3_abi::BUS_DIRECTION_OUTPUT
        {
            return false;
        }
        // SAFETY: the host gave room for one bus information, and this
        // is that room. The name is this crate's own string.
        unsafe {
            (*info).bus_type = craftkit_vst3_abi::BUS_TYPE_AUDIO;
            (*info).direction = direction;
            (*info).channels = 1;
            (*info).name = BUS_NAME_BYTES.as_ptr() as *const c_char;
        }
        true
    }

    /// How many parameters there are.
    extern "system" fn get_param_count(_this: *mut c_void) -> i32 {
        1
    }

    /// What the parameter at this index is.
    extern "system" fn get_param_info(this: *mut c_void, index: i32, info: *mut ParamInfo) -> bool {
        if slot_of(this).is_none() || info.is_null() || index != 0 {
            return false;
        }
        // SAFETY: the host gave room for one parameter information, and
        // this is that room. The four strings are this crate's own.
        unsafe {
            (*info).id = GAIN_ID_BYTES.as_ptr() as *const c_char;
            (*info).title = GAIN_TITLE_BYTES.as_ptr() as *const c_char;
            (*info).short_title = GAIN_TITLE_BYTES.as_ptr() as *const c_char;
            (*info).units = GAIN_UNITS_BYTES.as_ptr() as *const c_char;
            (*info).step_size = 1;
            (*info).default_normalized_value = normalized(GAIN_DEFAULT) as f64;
            (*info).flags = craftkit_vst3_abi::PARAM_IS_AUTOMATABLE;
            (*info).min = GAIN_MIN as f64;
            (*info).max = GAIN_MAX as f64;
        }
        true
    }

    /// The parameter's value, between zero and one.
    extern "system" fn get_param_normalized(this: *mut c_void, index: i32, out: *mut f32) -> bool {
        let Some(slot) = slot_of(this) else { return false };
        if index != 0 || out.is_null() {
            return false;
        }
        // SAFETY: the host gave room for one number, and this is that
        // room.
        unsafe { *out = normalized(gain_of(slot)) };
        true
    }

    /// Set the parameter's value, between zero and one.
    extern "system" fn set_param_normalized(this: *mut c_void, index: i32, value: f32) -> bool {
        let Some(slot) = slot_of(this) else { return false };
        if index != 0 {
            return false;
        }
        let clamped = value.clamp(0.0, 1.0);
        set_gain_of(slot, GAIN_MIN + clamped * (GAIN_MAX - GAIN_MIN));
        true
    }

    /// The plugin's own state.
    extern "system" fn get_state(this: *mut c_void, stream: *mut c_void) -> bool {
        let Some(slot) = slot_of(this) else { return false };
        write_to(stream, &gain_of(slot).to_le_bytes())
    }

    /// The plugin's own state.
    extern "system" fn set_state(this: *mut c_void, stream: *mut c_void) -> bool {
        let Some(slot) = slot_of(this) else { return false };
        let mut bytes = [0u8; 4];
        if !read_from(stream, &mut bytes) {
            return false;
        }
        set_gain_of(slot, f32::from_le_bytes(bytes));
        true
    }

    /// Process a block of audio.
    ///
    /// The plugin is a gain: what comes in comes out, louder or
    /// quieter, and the parameter says by how much.
    extern "system" fn process(this: *mut c_void, data: *mut ProcessData) -> bool {
        let Some(slot) = slot_of(this) else { return false };
        if data.is_null() {
            return false;
        }
        let gain = gain_of(slot);
        // SAFETY: the host's process data is the host's own, and it is
        // read once, before anything is done with it.
        let (samples, inputs, outputs) = unsafe {
            let data = &*data;
            (data.num_samples, data.num_inputs, data.num_outputs)
        };
        if samples <= 0 || inputs <= 0 || outputs <= 0 {
            return true;
        }
        // SAFETY: the bus buffers are the host's own, and each channel
        // pointer is the host's own array of samples. The channels are
        // counted by the host's own numbers, and the buffer holds room
        // for two.
        unsafe {
            let input = (*(data.add(0))).inputs as *const AudioBusBuffers;
            let output = (*(data.add(0))).outputs as *const AudioBusBuffers;
            if input.is_null() || output.is_null() {
                return false;
            }
            let input = &*input;
            let output = &*output;
            let channels = input.num_channels.min(output.num_channels).min(2);
            for channel in 0..channels {
                let source = input.channel_buffers64[channel as usize] as *const f32;
                let target = output.channel_buffers64[channel as usize] as *mut f32;
                if source.is_null() || target.is_null() {
                    continue;
                }
                for index in 0..samples as usize {
                    *target.add(index) = *source.add(index) * gain;
                }
            }
        }
        true
    }

    /// A gain as the host sees it: between zero and one.
    fn normalized(gain: f32) -> f32 {
        (gain - GAIN_MIN) / (GAIN_MAX - GAIN_MIN)
    }

    /// The table behind a stream the host handed over.
    fn stream_table(stream: *mut c_void) -> Option<*const craftkit_vst3_abi::IBStream> {
        if stream.is_null() {
            return None;
        }
        // SAFETY: the host's stream is the host's own object, and its
        // first word is the address of its table.
        let table = unsafe { (*(stream as *const Stream)).table };
        if table.is_null() { None } else { Some(table) }
    }

    /// Write bytes into the host's stream.
    fn write_to(stream: *mut c_void, bytes: &[u8]) -> bool {
        let Some(table) = stream_table(stream) else { return false };
        // SAFETY: the table is the host's own, and the write function
        // is the one the format names: it takes the stream, the bytes
        // and how many of them there are.
        let Some(write) = (unsafe { (*table).write }) else { return false };
        write(stream, bytes.as_ptr() as *const c_void, bytes.len() as i32) == bytes.len() as i32
    }

    /// Read bytes out of the host's stream.
    fn read_from(stream: *mut c_void, bytes: &mut [u8]) -> bool {
        let Some(table) = stream_table(stream) else { return false };
        // SAFETY: the table is the host's own, and the read function is
        // the one the format names: it takes the stream, the room to
        // read into, and how much there is room for.
        let Some(read) = (unsafe { (*table).read }) else { return false };
        read(stream, bytes.as_mut_ptr() as *mut c_void, bytes.len() as i32) == bytes.len() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class's name is the one the host asks by.
    #[test]
    fn the_class_has_a_name() {
        assert_eq!(CID, "Craftkit::Fixture", "the class is named after the tree");
        assert!(CID_BYTES.ends_with(b"\0"), "and it is a C string");
    }

    /// The gain's range is the one the plugin says it has.
    #[test]
    fn the_gain_has_a_range() {
        assert_eq!(GAIN_MIN, 0.0, "the smallest value");
        assert_eq!(GAIN_MAX, 2.0, "the largest");
        assert_eq!(GAIN_DEFAULT, 1.0, "and the one it starts at");
    }

    /// The pool of instances is **fixed**.
    #[test]
    fn the_pool_of_instances_is_fixed() {
        assert_eq!(MAX_INSTANCES, 64, "sixty-four at a time");
        assert_eq!(SLOTS.len(), MAX_INSTANCES, "and a slot for each of them");
    }

    /// The entry is there, and it is a **factory**.
    #[test]
    fn the_entry_is_a_factory() {
        let factory = exports::GetPluginFactory();
        assert!(!factory.is_null(), "the entry gives back the factory");
        assert_eq!(exports::classes(), 1, "and it holds one class");
    }
}
