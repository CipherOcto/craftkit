//! `craftkit-au-abi` — the **AU ABI**, written down as shapes.
//!
//! # What this is
//!
//! AU — the Audio Unit — is a **published plugin format** of the
//! macOS audio stack: a binary contract that a host and a unit agree
//! on, described in public documentation. This crate writes that
//! contract down as **data** — the numbers a host asks a unit with,
//! the shape of a unit's own function table, and the shape of a block
//! of audio on the way through.
//!
//! It holds **no `unsafe` at all**, and that is the point of the
//! design: a binary contract is a set of shapes, and a set of shapes is
//! safe to write down. The unsafe is on the two sides of it — the host
//! that opens the bundle (`craftkit-au`) and the unit that exports the
//! symbols (`craftkit-au-fixture`).
//!
//! # Why the shapes are here and not in the host
//!
//! The host and the unit must agree on the **layout** of these
//! structs, byte for byte. If the host had its own copy and the unit
//! had another, the two would drift apart silently, and a mismatch
//! would be a crash in the middle of a render. So there is **one** copy
//! of the shapes, and both sides use it.
//!
//! # The four-character codes
//!
//! AU says what a thing is with **four characters** packed into a
//! number: `aufx` for an effect, `appl` for Apple. The codes here are
//! written out as the characters they are, because a number is not
//! readable and a code is a name.
//!
//! # The properties
//!
//! A unit is asked about itself with a **property**: a number that
//! says what is wanted, a **scope** that says where, and an
//! **element** that says which one. The scope is the interesting one:
//! the same property id means different things on the input, on the
//! output and on the unit as a whole.
//!
//! # The audio
//!
//! A block of audio is a **buffer list**: a count and the buffers
//! themselves, each with the address of its samples. The format's list
//! is variable in length — the count says how many there are — and this
//! shape holds **two**, because the kit drives one bus in and one bus
//! out. A bound is not negotiable and a shape is (§9 of the kit's
//! decisions): the tree says what it can carry and refuses the rest.
//!
//! # What is **not** here
//!
//! - **The component manager.** Finding a unit and making an instance
//!   of it is the **platform's** job on macOS, and it is not in this
//!   shape. The tree has no component manager, so the tree's own
//!   fixture hands out an instance through [`INSTANCE_SYMBOL`] — a
//!   name of **this tree's own**, in this tree's own namespace, and
//!   not one of the format's.
//! - **The strings.** Several of the format's properties answer with a
//!   **CoreFoundation string**, and CoreFoundation is the platform's.
//!   The names of a unit and of its parameters are therefore not in
//!   this shape: a program that wants them reads them where the
//!   platform has them.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::os::raw::c_void;

/// An **OSType**: four characters packed into a number.
///
/// The format says what a thing is with four characters, and the number
/// is how they travel.
pub type OSType = u32;

/// The type of a unit that **changes** the audio.
pub const TYPE_EFFECT: OSType = fourcc(b"aufx");

/// The type of a unit that **produces** the audio.
pub const TYPE_OUTPUT: OSType = fourcc(b"auou");

/// The type of a unit that is a **music device**.
pub const TYPE_MUSIC_DEVICE: OSType = fourcc(b"aumu");

/// The type of a unit that **converts** a format.
pub const TYPE_FORMAT_CONVERTER: OSType = fourcc(b"aufc");

/// The subtype of a unit that does **whatever it is told**.
pub const SUBTYPE_UNIVERSAL: OSType = fourcc(b"univ");

/// The subtype of a unit that is a **generic output**.
pub const SUBTYPE_GENERIC_OUTPUT: OSType = fourcc(b"genr");

/// The subtype of a unit that is a **dynamic gain**.
pub const SUBTYPE_DYNAMIC_GAIN: OSType = fourcc(b"dyng");

/// The manufacturer that is **Apple**.
pub const MANUFACTURER_APPLE: OSType = fourcc(b"appl");

/// The manufacturer that is **this tree**.
pub const MANUFACTURER_CRAFTKIT: OSType = fourcc(b"ckit");

/// The format of the samples: **linear PCM**.
pub const FORMAT_LINEAR_PCM: OSType = fourcc(b"lpcm");

/// The name of the property that says what a unit **is**.
pub const PROPERTY_CLASS_INFO: u32 = 0;

/// The name of the property that says the **rate** a unit works at.
pub const PROPERTY_SAMPLE_RATE: u32 = 2;

/// The name of the property that says the **shape** of the audio.
pub const PROPERTY_STREAM_FORMAT: u32 = 8;

/// The name of the property that says how many frames a unit takes in
/// **one** slice.
pub const PROPERTY_MAXIMUM_FRAMES_PER_SLICE: u32 = 16;

/// The name of the property that holds the **last error**.
pub const PROPERTY_LAST_RENDER_ERROR: u32 = 22;

/// The name of the property that says how many **parameters** a unit
/// holds.
pub const PROPERTY_PARAMETER_LIST: u32 = 4102;

/// The scope of a property that is about the unit **as a whole**.
pub const SCOPE_GLOBAL: u32 = 0;

/// The scope of a property that is about the **input**.
pub const SCOPE_INPUT: u32 = 1;

/// The scope of a property that is about the **output**.
pub const SCOPE_OUTPUT: u32 = 2;

/// The element that is the unit's **master**.
pub const ELEMENT_MASTER: u32 = 0;

/// The name of the exported symbol that reads a property.
pub const GET_PROPERTY_SYMBOL: &str = "AudioUnitGetProperty";

/// The name of the exported symbol that writes a property.
pub const SET_PROPERTY_SYMBOL: &str = "AudioUnitSetProperty";

/// The name of the exported symbol that sets a unit up.
pub const INITIALIZE_SYMBOL: &str = "AudioUnitInitialize";

/// The name of the exported symbol that takes a unit down.
pub const UNINITIALIZE_SYMBOL: &str = "AudioUnitUninitialize";

/// The name of the exported symbol that puts a unit back as it was.
pub const RESET_SYMBOL: &str = "AudioUnitReset";

/// The name of the exported symbol that renders a block.
pub const RENDER_SYMBOL: &str = "AudioUnitRender";

/// The name of **this tree's** symbol that hands out an instance.
///
/// It is not one of the format's: the format's own way of making an
/// instance is the platform's **component manager**, and this tree has
/// none. The name is in this tree's namespace so that it cannot be
/// mistaken for the format's own.
pub const INSTANCE_SYMBOL: &str = "craftkit_au_instance";

/// Four characters packed into a number.
pub const fn fourcc(code: &[u8; 4]) -> OSType {
    ((code[0] as u32) << 24) | ((code[1] as u32) << 16) | ((code[2] as u32) << 8) | (code[3] as u32)
}

/// Four characters out of a number.
pub fn characters(code: OSType) -> String {
    let bytes =
        [(code >> 24) as u8, ((code >> 16) & 0xff) as u8, ((code >> 8) & 0xff) as u8, (code & 0xff) as u8];
    String::from_utf8_lossy(&bytes).into_owned()
}

/// What a unit says the audio **is**: its rate, its shape and its
/// channels.
///
/// The numbers are in the format's order, and the sizes are the
/// format's: this struct is a promise about bytes and not about
/// memory.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct StreamFormat {
    /// The rate, in cycles a second.
    pub sample_rate: f64,
    /// What the samples are: [`FORMAT_LINEAR_PCM`] and the rest.
    pub format_id: u32,
    /// What the samples are made of, in flags.
    pub format_flags: u32,
    /// How many bytes a packet takes.
    pub bytes_per_packet: u32,
    /// How many frames a packet holds.
    pub frames_per_packet: u32,
    /// How many bytes a frame takes.
    pub bytes_per_frame: u32,
    /// How many channels a frame has.
    pub channels_per_frame: u32,
    /// How many bits a channel takes.
    pub bits_per_channel: u32,
    /// The format's own reserved field.
    pub reserved: u32,
}

impl StreamFormat {
    /// A **linear PCM** format at a rate, with so many channels.
    pub fn pcm(sample_rate: f64, channels: u32) -> StreamFormat {
        StreamFormat {
            sample_rate,
            format_id: FORMAT_LINEAR_PCM,
            format_flags: 0,
            bytes_per_packet: 4 * channels,
            frames_per_packet: 1,
            bytes_per_frame: 4 * channels,
            channels_per_frame: channels,
            bits_per_channel: 32,
            reserved: 0,
        }
    }
}

/// **One buffer** of audio: the address of a channel's samples.
///
/// The count of bytes is the unit's own: it says how much is there, so
/// that a unit that writes less than it was given is visible.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AudioBuffer {
    /// How many channels the buffer holds.
    pub number_channels: u32,
    /// How many bytes the samples take.
    pub data_byte_size: u32,
    /// The samples themselves.
    pub data: *mut c_void,
}

/// The **buffers** of one render call.
///
/// The format's list is variable in length — the count says how many
/// there are — and this shape holds **two**, because the kit drives
/// one bus in and one bus out. The count is still the format's, and a
/// unit that is asked for more is refused by the host before the call.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AudioBufferList {
    /// How many buffers there are.
    pub number_buffers: u32,
    /// The buffers themselves.
    pub buffers: [AudioBuffer; 2],
}

impl AudioBufferList {
    /// A list with room for two buffers.
    pub fn empty() -> AudioBufferList {
        AudioBufferList {
            number_buffers: 0,
            buffers: [
                AudioBuffer { number_channels: 0, data_byte_size: 0, data: std::ptr::null_mut() },
                AudioBuffer { number_channels: 0, data_byte_size: 0, data: std::ptr::null_mut() },
            ],
        }
    }
}

/// The signature of the symbol that reads a property.
pub type GetProperty = extern "C" fn(*mut c_void, u32, u32, u32, *mut c_void, *mut u32) -> Status;

/// The signature of the symbol that writes a property.
pub type SetProperty = extern "C" fn(*mut c_void, u32, u32, u32, *const c_void, u32) -> Status;

/// The signature of the symbol that sets a unit up.
pub type Initialize = extern "C" fn(*mut c_void) -> Status;

/// The signature of the symbol that takes a unit down.
pub type Uninitialize = extern "C" fn(*mut c_void) -> Status;

/// The signature of the symbol that puts a unit back as it was.
pub type Reset = extern "C" fn(*mut c_void, u32, u32) -> Status;

/// The signature of the symbol that renders a block.
pub type Render = extern "C" fn(*mut c_void, *mut u32, *mut AudioBufferList) -> Status;

/// The signature of **this tree's** symbol that hands out an instance.
pub type MakeInstance = extern "C" fn() -> *mut c_void;

/// What went wrong, in the format's own number.
pub type Status = i32;

/// The status of a call that **worked**.
pub const STATUS_OK: Status = 0;

/// The status of a call that was **refused**: the unit did not do it.
pub const STATUS_ERROR: Status = -10868;

/// The scope of a **reset**: the unit as a whole.
pub const RESET_SCOPE_GLOBAL: u32 = 0;

/// The element of a **reset**: the unit's master.
pub const RESET_ELEMENT_MASTER: u32 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    /// The four characters are **packed in order**, and come back the
    /// same way.
    #[test]
    fn the_four_characters_are_packed_in_order() {
        assert_eq!(TYPE_EFFECT, fourcc(b"aufx"), "the effect type");
        assert_eq!(characters(TYPE_EFFECT), "aufx", "and it comes back");
        assert_eq!(characters(MANUFACTURER_CRAFTKIT), "ckit", "and so does the tree's own");
    }

    /// A four-character code is **four bytes**.
    #[test]
    fn a_four_character_code_is_four_bytes() {
        assert_eq!(std::mem::size_of::<OSType>(), 4, "the code is a number");
    }

    /// The stream format is the format's **layout**: four numbers, a
    /// double, and the reserved field.
    #[test]
    fn the_stream_format_is_the_formats_layout() {
        let format = StreamFormat::pcm(48_000.0, 2);
        assert_eq!(std::mem::size_of::<StreamFormat>(), 40, "the fields, in order");
        assert_eq!(format.sample_rate, 48_000.0, "the rate is a double");
        assert_eq!(format.format_id, FORMAT_LINEAR_PCM, "the samples are linear PCM");
        assert_eq!(format.channels_per_frame, 2, "and there are two channels");
        assert_eq!(format.bytes_per_frame, 8, "so a frame is eight bytes");
    }

    /// The buffer list holds **two** buffers, and the count is the
    /// format's.
    #[test]
    fn the_buffer_list_holds_two_buffers() {
        let list = AudioBufferList::empty();
        assert_eq!(std::mem::size_of::<AudioBufferList>(), 4 + 4 + 2 * 16, "a count, a pad and two buffers");
        assert_eq!(list.number_buffers, 0, "and none there yet");
    }

    /// The property ids are the format's, and the scopes are the
    /// format's.
    #[test]
    fn the_properties_are_the_formats() {
        assert_eq!(PROPERTY_SAMPLE_RATE, 2, "the rate");
        assert_eq!(PROPERTY_STREAM_FORMAT, 8, "the shape");
        assert_eq!(PROPERTY_MAXIMUM_FRAMES_PER_SLICE, 16, "the slice");
        assert_eq!(SCOPE_GLOBAL, 0, "the whole unit");
        assert_eq!(SCOPE_INPUT, 1, "the input");
        assert_eq!(SCOPE_OUTPUT, 2, "the output");
        assert_ne!(SCOPE_INPUT, SCOPE_OUTPUT, "and the two are not the same");
    }
}
