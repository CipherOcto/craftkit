//! `craftkit-au-fixture` — a **reference AU**.
//!
//! # Why it is here
//!
//! A host that opens a unit is only worth something if the unit it
//! opens is a **real** one: a shared library, exporting the symbols by
//! name, built by the compiler and opened from the disk. So this crate
//! is a unit, in the strict sense — a `cdylib` — and the host's tests
//! open it the way they would open anybody else's.
//!
//! It is a **fixture**, and it says what it is: it is an **output
//! unit** that passes the audio through, and it knows its rate, its
//! shape and how many frames it takes in one slice.
//!
//! # The shape of it
//!
//! AU's unit exports six **functions** by name: the two that read and
//! write a property, the two that set the unit up and take it down, the
//! one that puts it back as it was, and the one that renders a block.
//! A host finds them with `dlsym` and calls them.
//!
//! # The **tree's own** door
//!
//! The format's own way of making an instance is the platform's
//! **component manager**, and this tree has none. So the fixture also
//! exports one symbol of **this tree's own** — [`INSTANCE_SYMBOL`] —
//! which hands back an instance. The name is in this tree's namespace
//! so that it cannot be mistaken for the format's, and the host says
//! in its own words that it is using the tree's door and not the
//! platform's.
//!
//! # The audio
//!
//! The format's render call is given the **output** buffers, and the
//! input arrives through a callback the host sets. This tree has no
//! such callback: the host gives the unit a list with the input in the
//! **first** buffer and the output in the **second**, and the unit
//! reads one and writes the other. That is the tree's own convention,
//! and it is written down here because it is not the format's.
//!
//! # Why it is `unsafe`
//!
//! Writing through the pointers the host hands over is `unsafe` in the
//! plain sense: those pointers are the host's memory, and the unit
//! writes into them. So this crate is on v2's **list** of exceptions —
//! the **tenth** entry — and it is held to the same rules as the
//! others: `deny(unsafe_code)` and not `forbid`, **one**
//! `allow(unsafe_code)`, and the `unsafe` inside **one module**.
//!
//! It has **no** `unsafe impl` and **no** `unsafe fn`: the only unsafe
//! thing it does is read and write through a pointer the host gave it.

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

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use craftkit_au_abi::StreamFormat;

/// How many instances may exist at once.
///
/// The instances are handed out from a fixed pool: a fixture that grew
/// without limit would be a fixture that could not be reasoned about.
pub const MAX_INSTANCES: usize = 16;

/// The rate the unit works at until it is told otherwise.
pub const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;

/// The most frames the unit takes in one slice.
pub const DEFAULT_MAXIMUM_FRAMES: u32 = 1024;

/// Whether any instance has been initialised.
///
/// The host's tests check that it did — so the unit keeps the answer
/// where a test can see it.
static INITIALISED: AtomicBool = AtomicBool::new(false);

/// How many instances have ever been made.
static MADE: AtomicU32 = AtomicU32::new(0);

/// The unit's own state.
struct State {
    /// Whether the unit has been initialised.
    initialised: bool,
    /// The rate it works at.
    sample_rate: f64,
    /// The shape of the audio.
    format: StreamFormat,
    /// The most frames it takes in one slice.
    max_frames: u32,
}

impl State {
    /// The state a unit starts with.
    fn new() -> State {
        State {
            initialised: false,
            sample_rate: DEFAULT_SAMPLE_RATE,
            format: StreamFormat::pcm(DEFAULT_SAMPLE_RATE, 1),
            max_frames: DEFAULT_MAXIMUM_FRAMES,
        }
    }
}

/// One **slot**: a place a unit's state can live.
///
/// A slot that is **empty** is free, and a slot with a state in it is
/// taken.
struct Slot {
    /// The state, when the slot is taken.
    state: Mutex<Option<State>>,
}

const fn slot() -> Slot {
    Slot { state: Mutex::new(None) }
}

/// The instances this unit can have at once.
static SLOTS: [Slot; MAX_INSTANCES] = [const { slot() }; MAX_INSTANCES];

/// Whether any instance has been initialised.
pub fn initialised() -> bool {
    INITIALISED.load(Ordering::SeqCst)
}

/// How many instances have ever been made.
pub fn made() -> u32 {
    MADE.load(Ordering::SeqCst)
}

/// The unit's own functions.
///
/// The `unsafe` is here and nowhere else: every one of the unit's
/// functions reads or writes through a pointer the host gave it, and
/// every one of them says why in a comment where the pointer is used.
#[allow(unsafe_code)]
mod exports {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU32, Ordering};

    use craftkit_au_abi::{
        AudioBufferList, ELEMENT_MASTER, PROPERTY_MAXIMUM_FRAMES_PER_SLICE, PROPERTY_SAMPLE_RATE,
        PROPERTY_STREAM_FORMAT, RESET_ELEMENT_MASTER, RESET_SCOPE_GLOBAL, SCOPE_GLOBAL, STATUS_ERROR,
        STATUS_OK, Status, StreamFormat,
    };

    use super::{INITIALISED, MADE, SLOTS, State};

    /// The **instance** the tree's own door hands back.
    ///
    /// It says which slot its state is in and how many references the
    /// instance has.
    #[repr(C)]
    pub struct Instance {
        /// Which slot the state is in.
        pub slot: usize,
        /// How many references the instance has.
        pub refs: AtomicU32,
    }

    /// The state behind an instance, and the lock that guards it.
    fn state_of(this: *mut c_void) -> Option<std::sync::MutexGuard<'static, Option<State>>> {
        if this.is_null() {
            return None;
        }
        // SAFETY: the instance is this crate's own, and the host hands
        // back the very pointer it was given.
        let instance = unsafe { &*(this as *const Instance) };
        let slot = SLOTS.get(instance.slot)?;
        // SAFETY: the lock is a static, and the guard lives no longer
        // than the borrow of it.
        let guard = slot.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none() {
            return None;
        }
        Some(guard)
    }

    /// The tree's own door: it hands back an **instance**.
    ///
    /// The `unsafe` is in the attribute and nowhere else: giving a
    /// symbol a name is a promise to the loader that the thing behind
    /// the name is what this tree says it is.
    #[unsafe(no_mangle)]
    pub extern "C" fn craftkit_au_instance() -> *mut c_void {
        // A slot that is free is taken; a unit that has no free slot
        // answers with nothing, and the host says so.
        for (index, slot) in SLOTS.iter().enumerate() {
            let Ok(mut guard) = slot.state.lock() else { continue };
            if guard.is_some() {
                continue;
            }
            *guard = Some(State::new());
            let instance = Box::into_raw(Box::new(Instance { slot: index, refs: AtomicU32::new(1) }));
            MADE.fetch_add(1, Ordering::SeqCst);
            return instance as *mut c_void;
        }
        std::ptr::null_mut()
    }

    /// Read a property.
    #[unsafe(no_mangle)]
    pub extern "C" fn AudioUnitGetProperty(
        this: *mut c_void,
        id: u32,
        scope: u32,
        element: u32,
        out_data: *mut c_void,
        io_data_size: *mut u32,
    ) -> Status {
        let Some(state) = state_of(this) else { return STATUS_ERROR };
        if scope != SCOPE_GLOBAL || element != ELEMENT_MASTER {
            return STATUS_ERROR;
        }
        if out_data.is_null() || io_data_size.is_null() {
            return STATUS_ERROR;
        }
        let Some(current) = state.as_ref() else { return STATUS_ERROR };
        // SAFETY: the size is the host's own, and it says how much room
        // there is.
        let room = unsafe { *io_data_size };
        match id {
            PROPERTY_SAMPLE_RATE => {
                if room < 8 {
                    return STATUS_ERROR;
                }
                // SAFETY: the room is the host's, and the host said it
                // is big enough for the number.
                unsafe {
                    out_data.cast::<f64>().write(current.sample_rate);
                    *io_data_size = 8;
                }
                STATUS_OK
            }
            PROPERTY_MAXIMUM_FRAMES_PER_SLICE => {
                if room < 4 {
                    return STATUS_ERROR;
                }
                // SAFETY: the room is the host's, and the host said it
                // is big enough for the number.
                unsafe {
                    out_data.cast::<u32>().write(current.max_frames);
                    *io_data_size = 4;
                }
                STATUS_OK
            }
            PROPERTY_STREAM_FORMAT => {
                if room < std::mem::size_of::<StreamFormat>() as u32 {
                    return STATUS_ERROR;
                }
                // SAFETY: the room is the host's, and the host said it
                // is big enough for the format.
                unsafe {
                    out_data.cast::<StreamFormat>().write(current.format);
                    *io_data_size = std::mem::size_of::<StreamFormat>() as u32;
                }
                STATUS_OK
            }
            _ => STATUS_ERROR,
        }
    }

    /// Write a property.
    #[unsafe(no_mangle)]
    pub extern "C" fn AudioUnitSetProperty(
        this: *mut c_void,
        id: u32,
        scope: u32,
        element: u32,
        in_data: *const c_void,
        in_data_size: u32,
    ) -> Status {
        let Some(mut state) = state_of(this) else { return STATUS_ERROR };
        if scope != SCOPE_GLOBAL || element != ELEMENT_MASTER || in_data.is_null() {
            return STATUS_ERROR;
        }
        // SAFETY: the bytes are the host's own, and the size says how
        // many there are.
        let value = unsafe {
            match id {
                PROPERTY_SAMPLE_RATE if in_data_size == 8 => Some(in_data.cast::<f64>().read()),
                PROPERTY_MAXIMUM_FRAMES_PER_SLICE if in_data_size == 4 => {
                    Some(in_data.cast::<u32>().read() as f64)
                }
                _ => None,
            }
        };
        let Some(value) = value else { return STATUS_ERROR };
        let Some(current) = state.as_mut() else { return STATUS_ERROR };
        match id {
            PROPERTY_SAMPLE_RATE => current.sample_rate = value,
            PROPERTY_MAXIMUM_FRAMES_PER_SLICE => current.max_frames = value as u32,
            _ => return STATUS_ERROR,
        }
        STATUS_OK
    }

    /// Set the unit up.
    #[unsafe(no_mangle)]
    pub extern "C" fn AudioUnitInitialize(this: *mut c_void) -> Status {
        let Some(mut state) = state_of(this) else { return STATUS_ERROR };
        let Some(current) = state.as_mut() else { return STATUS_ERROR };
        current.initialised = true;
        INITIALISED.store(true, Ordering::SeqCst);
        STATUS_OK
    }

    /// Take the unit down.
    #[unsafe(no_mangle)]
    pub extern "C" fn AudioUnitUninitialize(this: *mut c_void) -> Status {
        let Some(mut state) = state_of(this) else { return STATUS_ERROR };
        let Some(current) = state.as_mut() else { return STATUS_ERROR };
        current.initialised = false;
        STATUS_OK
    }

    /// Put the unit back as it was.
    #[unsafe(no_mangle)]
    pub extern "C" fn AudioUnitReset(_this: *mut c_void, scope: u32, element: u32) -> Status {
        if scope != RESET_SCOPE_GLOBAL || element != RESET_ELEMENT_MASTER {
            return STATUS_ERROR;
        }
        STATUS_OK
    }

    /// Render a block of audio.
    ///
    /// The **first** buffer is what came in and the **second** is what
    /// goes out: the tree's own convention, written down in the crate's
    /// own prose.
    #[unsafe(no_mangle)]
    pub extern "C" fn AudioUnitRender(
        this: *mut c_void,
        io_number_frames: *mut u32,
        io_data: *mut AudioBufferList,
    ) -> Status {
        let Some(state) = state_of(this) else { return STATUS_ERROR };
        let Some(current) = state.as_ref() else { return STATUS_ERROR };
        if !current.initialised {
            return STATUS_ERROR;
        }
        if io_number_frames.is_null() || io_data.is_null() {
            return STATUS_ERROR;
        }
        // SAFETY: the buffers are the host's own, and the count is the
        // host's own. A list with fewer than two buffers cannot carry
        // an input and an output, so it is refused.
        let (frames, buffers) = unsafe { (*io_number_frames, &*io_data) };
        if buffers.number_buffers < 2 {
            return STATUS_ERROR;
        }
        let input = buffers.buffers[0];
        let output = buffers.buffers[1];
        if input.data.is_null() || output.data.is_null() {
            return STATUS_ERROR;
        }
        let channels = input.number_channels.min(output.number_channels) as usize;
        if channels == 0 {
            return STATUS_ERROR;
        }
        let count = frames as usize;
        // SAFETY: the samples are the host's own, and the count is the
        // host's own: the unit is given the frames and writes no more
        // than it was given.
        unsafe {
            let source = input.data.cast::<f32>();
            let target = output.data.cast::<f32>();
            for frame in 0..count {
                for channel in 0..channels {
                    let index = frame * channels + channel;
                    *target.add(index) = *source.add(index);
                }
            }
        }
        STATUS_OK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pool of instances is **fixed**.
    #[test]
    fn the_pool_of_instances_is_fixed() {
        assert_eq!(MAX_INSTANCES, 16, "sixteen at a time");
        assert_eq!(SLOTS.len(), MAX_INSTANCES, "and a slot for each of them");
    }

    /// The unit starts at the rate it is told to work at.
    #[test]
    fn the_unit_starts_at_its_rate() {
        let state = State::new();
        assert_eq!(state.sample_rate, DEFAULT_SAMPLE_RATE, "the rate");
        assert_eq!(state.format.channels_per_frame, 1, "and one channel");
        assert!(!state.initialised, "and it is not set up yet");
    }
}
