//! `craftkit-vst3-abi` — the **VST3 ABI**, written down as shapes.
//!
//! # What this is
//!
//! VST3 is a **published, open plugin format**: an audio plugin ABI
//! that a host and a plugin agree on, described in public documentation
//! and implemented in open source. This crate writes that ABI down as
//! **data** — the structs a host reads out of a plugin and the function
//! tables it calls through.
//!
//! It holds **no `unsafe` at all**, and that is the point of the
//! design: a binary contract is a set of shapes, and a set of shapes is
//! safe to write down. The unsafe is on the two sides of it — the host
//! that opens the library and calls through the shapes
//! (`craftkit-vst3`), and the plugin that exports them
//! (`craftkit-vst3-fixture`).
//!
//! # Why the shapes are here and not in the host
//!
//! The host and the plugin must agree on the **layout** of these
//! structs, byte for byte. If the host had its own copy and the plugin
//! had another, the two would drift apart silently, and a mismatch
//! would be a crash in the middle of an audio callback. So there is
//! **one** copy of the shapes, and both sides use it.
//!
//! # The entry
//!
//! VST3 exports a **function**, not a data symbol: the plugin's
//! library holds `GetPluginFactory`, and calling it gives back the
//! factory. That is a different door from CLAP's, which exports the
//! address of a struct — and it is a difference the host has to know.
//!
//! # The identifiers
//!
//! VST3 names its interfaces by **FUID** — sixteen bytes that stand
//! for an interface — and its classes by a **CID** — a string. The
//! shapes here name the interfaces they belong to, and this crate does
//! **not** carry the published FUID bytes: they live in the SDK's
//! published headers, and this tree does not copy files it was not
//! written for. A program that talks to a real plugin reads them from
//! the SDK it was built against, and a program that talks to another
//! program of this tree uses the ones its own plugin says.
//!
//! # The version
//!
//! There is no version **number** in the ABI: compatibility is by
//! FUID, and a host asks for the interface it wants and the plugin
//! either gives it or does not. What there is, is the **SDK** the
//! shapes describe, and [`SDK_VERSION`] says which one.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::os::raw::{c_char, c_void};

/// The **entry** a VST3 plugin's library exports.
///
/// It is a function and not a data symbol: calling it gives back the
/// plugin factory.
pub const ENTRY_SYMBOL: &str = "GetPluginFactory";

/// The SDK these shapes describe.
///
/// A VST3 plugin does not say "I am version 3" to the host — the
/// interfaces are asked for by name, and the version is the SDK the
/// plugin was built against. This is the one this crate writes down.
pub const SDK_VERSION: &str = "3.7.7";

/// A **FUID**: sixteen bytes that stand for an interface.
///
/// The bytes are laid out the way the SDK lays them out: four bytes,
/// two bytes, two bytes, and eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Fuid {
    /// The first four bytes.
    pub data1: u32,
    /// The next two.
    pub data2: u16,
    /// The next two.
    pub data3: u16,
    /// The last eight.
    pub data4: [u8; 8],
}

impl Fuid {
    /// A FUID from its four parts.
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Fuid {
        Fuid { data1, data2, data3, data4 }
    }

    /// The sixteen bytes, in the order the SDK lays them out.
    pub const fn to_bytes(self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0] = (self.data1 >> 24) as u8;
        bytes[1] = (self.data1 >> 16) as u8;
        bytes[2] = (self.data1 >> 8) as u8;
        bytes[3] = self.data1 as u8;
        bytes[4] = (self.data2 >> 8) as u8;
        bytes[5] = self.data2 as u8;
        bytes[6] = (self.data3 >> 8) as u8;
        bytes[7] = self.data3 as u8;
        let mut index = 8;
        while index < 16 {
            bytes[index] = self.data4[index - 8];
            index += 1;
        }
        bytes
    }
}

/// What a **factory** says about the classes it makes.
///
/// The strings are **fixed size**: the SDK writes them into the struct
/// and not behind a pointer, so the layout of this struct is a promise
/// about bytes and not about memory.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FactoryInfo {
    /// What kind of thing it is: "Audio Module Class", "Audio Effect Class".
    pub category: [u8; 32],
    /// Its name.
    pub name: [u8; 64],
    /// Who makes it.
    pub vendor: [u8; 64],
    /// Its version.
    pub version: i32,
    /// What the flags say.
    pub flags: u32,
}

impl FactoryInfo {
    /// A factory's information, from the three strings and the version.
    ///
    /// A string longer than its field is **cut**, and not refused: the
    /// field is a fixed size and there is nowhere to put the rest.
    pub fn new(category: &str, name: &str, vendor: &str, version: i32) -> FactoryInfo {
        FactoryInfo {
            category: fixed::<32>(category),
            name: fixed::<64>(name),
            vendor: fixed::<64>(vendor),
            version,
            flags: 0,
        }
    }

    /// The category, as a string.
    pub fn category(&self) -> String {
        text(&self.category)
    }

    /// The name, as a string.
    pub fn name(&self) -> String {
        text(&self.name)
    }

    /// The vendor, as a string.
    pub fn vendor(&self) -> String {
        text(&self.vendor)
    }
}

/// A string in a **fixed** field.
fn fixed<const LENGTH: usize>(text: &str) -> [u8; LENGTH] {
    let mut field = [0u8; LENGTH];
    let bytes = text.as_bytes();
    let count = bytes.len().min(LENGTH.saturating_sub(1));
    field[..count].copy_from_slice(&bytes[..count]);
    field
}

/// A string out of a fixed field.
fn text(field: &[u8]) -> String {
    let end = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).to_string()
}

/// What a factory says about **one** of its classes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ClassInfo {
    /// The class's identifier: a string, by convention `Vst::Something`.
    pub cid: *const c_char,
    /// How many of it the factory makes.
    pub cardinality: *const c_char,
    /// What the class says about itself.
    pub class_info: *const FactoryInfo,
}

/// The **factory**: the plugin's list of what it can make.
///
/// Every method is a **function pointer** in the order the SDK puts
/// them, because the table is the ABI: the host calls through the
/// table and the table is the contract.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IPluginFactory {
    /// How many classes the factory has.
    pub count_classes: Option<extern "system" fn(*const IPluginFactory) -> i32>,
    /// What the class at this index says about itself.
    pub get_class_info: Option<extern "system" fn(*const IPluginFactory, i32, *mut ClassInfo) -> bool>,
    /// Make one, of the class with this identifier, for this interface.
    pub create_instance:
        Option<extern "system" fn(*const IPluginFactory, *const c_char, Fuid) -> *mut c_void>,
}

/// What an **audio bus** is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BusInfo {
    /// What kind of bus it is.
    pub bus_type: i32,
    /// Which way the audio goes.
    pub direction: i32,
    /// How many channels it has.
    pub channels: i32,
    /// Its name.
    pub name: *const c_char,
}

/// What a **parameter** is.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ParamInfo {
    /// Its identifier.
    pub id: *const c_char,
    /// Its name.
    pub title: *const c_char,
    /// A shorter name.
    pub short_title: *const c_char,
    /// Its units, if it has any.
    pub units: *const c_char,
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

/// Where the **channels** of a bus go.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BusArrangement {
    /// Which way the audio goes.
    pub direction: i32,
    /// How many channels the bus has.
    pub num_channels: i32,
    /// Where each channel comes from or goes to.
    pub channel_map: [u64; 64],
}

/// How the host wants the audio **processed**.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProcessSetup {
    /// The mode: live, or the plugin deciding.
    pub process_mode: i32,
    /// The size of a sample: 32 bits or 64.
    pub symbolic_sample_size: i32,
    /// The most samples in one block.
    pub max_samples_per_block: i32,
    /// The sample rate.
    pub sample_rate: f64,
}

/// One **block** of audio, going through the plugin.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProcessData {
    /// The mode.
    pub mode: i32,
    /// How many samples in the block.
    pub num_samples: i32,
    /// The size of a sample.
    pub symbolic_sample_size: i32,
    /// How many input buses.
    pub num_inputs: i32,
    /// How many output buses.
    pub num_outputs: i32,
    /// The input buses.
    pub inputs: *mut c_void,
    /// The output buses.
    pub outputs: *mut c_void,
    /// The parameters that changed.
    pub parameter_changes: *mut c_void,
    /// The events.
    pub events: *mut c_void,
    /// The plugin's own data.
    pub data: *mut c_void,
}

/// The **component**: the plugin's own half.
///
/// It is the half that makes the sound: it takes the parameters and the
/// audio and gives the audio back.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IComponent {
    /// Take a reference.
    ///
    /// The format counts references on the object, so that the host and
    /// the plugin may each hold it for a while. The first two methods
    /// are the count, and they come first because the format puts them
    /// there.
    pub add_ref: Option<extern "system" fn(*mut c_void) -> i32>,
    /// Give a reference back.
    ///
    /// The last one to let go ends the object.
    pub release: Option<extern "system" fn(*mut c_void) -> i32>,
    /// Set the plugin up.
    pub initialize: Option<extern "system" fn(*mut c_void, *const c_void) -> bool>,
    /// Take the plugin down.
    pub terminate: Option<extern "system" fn(*mut c_void) -> bool>,
    /// Say how the buses are arranged.
    pub set_bus_arrangements: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// Say how the buses are arranged.
    pub get_bus_arrangements: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// How many buses of this kind there are.
    pub get_bus_count: Option<extern "system" fn(*mut c_void, i32) -> i32>,
    /// What the bus at this index is.
    ///
    /// The direction and the index, and then the room for the answer.
    pub get_bus_info: Option<extern "system" fn(*mut c_void, i32, i32, *mut BusInfo) -> bool>,
    /// How many parameters there are.
    pub get_param_count: Option<extern "system" fn(*mut c_void) -> i32>,
    /// What the parameter at this index is.
    pub get_param_info: Option<extern "system" fn(*mut c_void, i32, *mut ParamInfo) -> bool>,
    /// The parameter's value, between zero and one.
    pub get_param_normalized: Option<extern "system" fn(*mut c_void, i32, *mut f32) -> bool>,
    /// Set the parameter's value, between zero and one.
    pub set_param_normalized: Option<extern "system" fn(*mut c_void, i32, f32) -> bool>,
    /// The plugin's own state.
    pub get_state: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// The plugin's own state.
    pub set_state: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// Process a block of audio.
    pub process: Option<extern "system" fn(*mut c_void, *mut ProcessData) -> bool>,
}

/// The **audio processor**: the half that does the sound.
///
/// It is a separate interface from the component because a plugin may
/// have both and a host may want one without the other.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IAudioProcessor {
    /// Take a reference.
    ///
    /// The same pair the component has, and in the same place: every
    /// interface of the format counts references the same way, and a
    /// host that let go of one of them wrongly would call the wrong
    /// function.
    pub add_ref: Option<extern "system" fn(*mut c_void) -> i32>,
    /// Give a reference back.
    pub release: Option<extern "system" fn(*mut c_void) -> i32>,
    /// Say how the buses are arranged.
    pub set_bus_arrangements: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// Say how the buses are arranged.
    pub get_bus_arrangements: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// Process a block of audio.
    pub process: Option<extern "system" fn(*mut c_void, *mut ProcessData) -> bool>,
}

/// The **controller**: the half that talks to the person.
///
/// It holds the parameters as the person sees them, and it is the half
/// that a program asks about the plugin without making it sound.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IEditController {
    /// The controller's own state.
    pub set_component_state: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// The controller's own state.
    pub get_component_state: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// How many parameters there are.
    pub get_param_count: Option<extern "system" fn(*mut c_void) -> i32>,
    /// What the parameter at this index is.
    pub get_param_info: Option<extern "system" fn(*mut c_void, i32, *mut ParamInfo) -> bool>,
    /// The parameter's value, between zero and one.
    pub get_param_normalized: Option<extern "system" fn(*mut c_void, i32, *mut f32) -> bool>,
    /// Set the parameter's value, between zero and one.
    pub set_param_normalized: Option<extern "system" fn(*mut c_void, i32, f32) -> bool>,
    /// The parameter's value, in the units the person sees.
    pub get_param_plain: Option<extern "system" fn(*mut c_void, i32, *mut f32) -> bool>,
    /// Set the parameter's value, in the units the person sees.
    pub set_param_plain: Option<extern "system" fn(*mut c_void, i32, f32) -> bool>,
    /// How many programs there are.
    pub get_program_count: Option<extern "system" fn(*mut c_void) -> i32>,
    /// What the program at this index is.
    pub get_program_info: Option<extern "system" fn(*mut c_void, i32, *mut c_void) -> bool>,
    /// The name of the program at this index.
    pub get_program_name: Option<extern "system" fn(*mut c_void, i32, *mut c_char) -> bool>,
}

/// The **view**: the plugin's own window.
///
/// A host shows it when the plugin has one, and does not when it does
/// not: the host asks the platform whether the view can be shown at
/// all.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IPlugView {
    /// Whether the view can be shown on this platform.
    pub is_platform_type_supported: Option<extern "system" fn(*mut c_void, i32) -> bool>,
    /// The view is on the screen.
    pub attached: Option<extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> bool>,
    /// The view is off the screen.
    pub removed: Option<extern "system" fn(*mut c_void) -> bool>,
    /// The wheel.
    pub on_wheel: Option<extern "system" fn(*mut c_void, i16, i16) -> bool>,
    /// A key went down.
    pub on_key_down: Option<extern "system" fn(*mut c_void, i16, i16, i16, i16) -> bool>,
    /// A key came up.
    pub on_key_up: Option<extern "system" fn(*mut c_void, i16, i16, i16, i16) -> bool>,
    /// How big the view wants to be.
    pub get_size: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// The view has the focus.
    pub on_focus: Option<extern "system" fn(*mut c_void, i32) -> bool>,
    /// The window the view is in.
    pub set_frame: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// The window the view is in.
    pub get_frame: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
    /// The view can be made bigger.
    pub can_resize: Option<extern "system" fn(*mut c_void) -> bool>,
    /// The view has been made bigger.
    pub on_resize: Option<extern "system" fn(*mut c_void, *mut c_void) -> bool>,
}

/// What a bus **carries**.
///
/// The format names a bus by two numbers — what it carries and which way
/// it goes — and this is the first of them: audio.
pub const MEDIA_TYPE_AUDIO: i32 = 1;

/// What a bus is **for**.
///
/// The main bus is the one the sound goes through; the side chain is
/// the one the plugin listens to while it works.
pub const BUS_TYPE_AUDIO: i32 = 0;

/// Which way the audio **goes**: in.
pub const BUS_DIRECTION_INPUT: i32 = 0;

/// Which way the audio **goes**: out.
pub const BUS_DIRECTION_OUTPUT: i32 = 1;

/// The bus is the **main** one.
pub const AUDIO_BUS_TYPE_MAIN: i32 = 0;

/// The bus is the **side chain**.
pub const AUDIO_BUS_TYPE_SIDE_CHAIN: i32 = 1;

/// The parameter may be **automated**.
pub const PARAM_IS_AUTOMATABLE: i32 = 1 << 0;

/// The plugin works in **real time**.
pub const PROCESS_MODE_REALTIME: i32 = 0;

/// The samples are **thirty-two** bits.
pub const SAMPLE_SIZE_32: i32 = 4;

/// The tree's own id for the **component** interface.
///
/// VST3 names its interfaces by FUID, and the **published** bytes belong
/// to the SDK's headers — which this tree does not copy. So the tree
/// names its **own** interfaces, and the host and the fixture agree on
/// them because both of them are written here. A program that talks to
/// a plugin built against the SDK asks for the SDK's bytes instead, and
/// the shape of the call is the same either way.
pub const COMPONENT_FUID: Fuid = Fuid::new(0x4352_4b54, 0x0001, 0x0000, [0; 8]);

/// The tree's own id for the **audio processor** interface.
///
/// The same reasoning as [`COMPONENT_FUID`]: a name of this tree's own,
/// so that the two sides of this tree can agree without the SDK's
/// headers being copied into it.
pub const AUDIO_PROCESSOR_FUID: Fuid = Fuid::new(0x4352_4b54, 0x0002, 0x0000, [0; 8]);

/// The tree's own id for the **edit controller** interface.
///
/// The same reasoning as [`COMPONENT_FUID`].
pub const EDIT_CONTROLLER_FUID: Fuid = Fuid::new(0x4352_4b54, 0x0003, 0x0000, [0; 8]);

/// An **instance**: what the factory hands back.
///
/// The pointer is the **object**, and the object's first word is the
/// address of its table of functions. That is how a virtual table is
/// laid out, and it is what the format means when it says that an
/// object implements an interface: the object *is* the interface, and
/// the table is how the calls find their way in.
///
/// The factory is the one thing that is **not** like this: it has one
/// interface and no state of its own, so the pointer it hands back is
/// the table itself.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Instance {
    /// The address of the object's table.
    pub table: *const c_void,
}

/// The **stream** a state travels through.
///
/// A state is a set of bytes, and the format does not say "give me the
/// bytes": it says "read from this stream" and "write to this one".
/// The stream is the **host's** own — the host makes one, and the
/// plugin reads and writes through it.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IBStream {
    /// Read that many bytes into the buffer, and say how many there were.
    pub read: Option<extern "system" fn(stream: *mut c_void, buffer: *mut c_void, num_bytes: i32) -> i32>,
    /// Write that many bytes from the buffer, and say how many there were.
    pub write: Option<extern "system" fn(stream: *mut c_void, buffer: *const c_void, num_bytes: i32) -> i32>,
    /// Move to a place in the stream.
    pub seek: Option<extern "system" fn(stream: *mut c_void, pos: i64) -> i32>,
    /// Where the stream is now.
    pub tell: Option<extern "system" fn(stream: *mut c_void) -> i64>,
}

/// A **stream**: the object the host hands the plugin.
///
/// Its first word is the address of its table — the same convention the
/// instances follow — so that the plugin can find the functions and the
/// host can keep its own state behind them.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Stream {
    /// The address of the table.
    pub table: *const IBStream,
}

/// The **buffers** of one audio bus.
///
/// The bus's channels are pointed at: each entry is the address of one
/// channel's samples. There are two entries because a bus has at most
/// two channels in this shape, and a bus with one channel leaves the
/// second one alone.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AudioBusBuffers {
    /// How many channels the bus has.
    pub num_channels: i32,
    /// The addresses of the channels.
    ///
    /// The SDK has a thirty-two bit field for the same thing and a
    /// sixty-four bit one beside it, and this shape is the **wide** one:
    /// a program that runs where addresses are sixty-four bits wide
    /// cannot use the narrow field, and this tree holds no narrow one.
    pub channel_buffers64: [*mut c_void; 2],
    /// Which channels hold silence and need not be read.
    pub silent_flags: [u64; 2],
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    /// The FUID is sixteen bytes, and the SDK lays them out in that
    /// order.
    #[test]
    fn a_fuid_is_sixteen_bytes() {
        assert_eq!(size_of::<Fuid>(), 16, "a FUID is sixteen bytes");
        let fuid = Fuid::new(0x1122_3344, 0x5566, 0x7788, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(fuid.to_bytes()[0], 0x11, "the first byte");
        assert_eq!(fuid.to_bytes()[3], 0x44, "the fourth byte");
        assert_eq!(fuid.to_bytes()[8], 1, "and the last eight start at eight");
        assert_eq!(fuid.to_bytes()[15], 8, "and end at sixteen");
    }

    /// The factory's information is fixed size, so its layout is a
    /// promise about bytes.
    #[test]
    fn the_factorys_information_is_fixed_size() {
        assert_eq!(size_of::<FactoryInfo>(), 32 + 64 + 64 + 4 + 4, "the fields, in order");
        assert_eq!(align_of::<FactoryInfo>(), 4, "and the alignment");
    }

    /// The strings in it are **cut** and not refused.
    ///
    /// The field is a fixed size and there is nowhere to put the rest.
    #[test]
    fn a_string_longer_than_its_field_is_cut() {
        let info = FactoryInfo::new("a category that is far too long for its field", "short", "vendor", 1);
        assert_eq!(info.category().len(), 31, "the category is cut");
        assert_eq!(info.name(), "short", "and the name is not");
        assert_eq!(info.vendor(), "vendor");
        assert_eq!(info.version, 1, "and the version is there");
    }

    /// The interface tables are **pointers**, so their layout is the
    /// SDK's and not this crate's to choose.
    #[test]
    fn the_interface_tables_are_pointers() {
        assert_eq!(size_of::<IPluginFactory>(), 3 * size_of::<usize>(), "three pointers");
        assert_eq!(size_of::<IComponent>(), 15 * size_of::<usize>(), "fifteen");
        assert_eq!(size_of::<IAudioProcessor>(), 5 * size_of::<usize>(), "five");
        assert_eq!(size_of::<IEditController>(), 11 * size_of::<usize>(), "eleven");
        assert_eq!(size_of::<IPlugView>(), 12 * size_of::<usize>(), "twelve");
    }

    /// The audio structures carry their numbers in the SDK's order.
    #[test]
    fn the_audio_structures_carry_their_numbers() {
        let setup = ProcessSetup {
            process_mode: 1,
            symbolic_sample_size: 4,
            max_samples_per_block: 512,
            sample_rate: 48_000.0,
        };
        assert_eq!(size_of::<ProcessSetup>(), 24, "four numbers and a double");
        assert_eq!(setup.sample_rate, 48_000.0, "the sample rate is a double");

        let arrangement = BusArrangement { direction: 0, num_channels: 2, channel_map: [0; 64] };
        assert_eq!(size_of::<BusArrangement>(), 8 + 64 * 8, "two numbers and the map");
        assert_eq!(arrangement.num_channels, 2, "and the channels are there");
    }

    /// An instance is **one word**: the object, whose first word is the
    /// table.
    #[test]
    fn an_instance_is_one_word() {
        assert_eq!(size_of::<Instance>(), size_of::<usize>(), "one word");
        assert_eq!(size_of::<IBStream>(), 4 * size_of::<usize>(), "the stream is four pointers");
    }

    /// The audio buffers are the **wide** shape, and say so.
    #[test]
    fn the_audio_buffers_are_the_wide_shape() {
        assert_eq!(
            size_of::<AudioBusBuffers>(),
            4 + 4 + 2 * size_of::<usize>() + 2 * 8,
            "a number, a pad, two addresses and two flags"
        );
    }

    /// The tree's own interface ids are **three different** names.
    ///
    /// Two sides of this tree have to agree on them, and the only way
    /// they can is if the names are written down once and used twice.
    #[test]
    fn the_trees_own_interface_ids_are_different_names() {
        assert_ne!(COMPONENT_FUID, AUDIO_PROCESSOR_FUID, "the component is its own interface");
        assert_ne!(COMPONENT_FUID, EDIT_CONTROLLER_FUID, "and so is the controller");
        assert_ne!(AUDIO_PROCESSOR_FUID, EDIT_CONTROLLER_FUID, "and so is the processor");
        assert_eq!(COMPONENT_FUID.to_bytes().len(), 16, "and every one of them is sixteen bytes");
    }
}
