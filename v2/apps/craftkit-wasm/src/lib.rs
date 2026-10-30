//! `craftkit-wasm` — the **browser door**: the kit in a page.
//!
//! # Why it exists
//!
//! The kit had doors for other programs — the MCP server, the control
//! channel, the plugin bridge — and the terminal door for a person
//! (§75). A browser is the other place a person is, and a page is a
//! program that cannot be installed: the whole kit would have to be
//! built for the browser and shipped as a file.
//!
//! # What it does
//!
//! It runs **the terminal door's own logic**: the same `Repl`, the same
//! registry, the same answers. A page asks the kit a question in the
//! kit's own vocabulary, and the answer is the answer the prompt at
//! the terminal gives.
//!
//! # How the page reaches it
//!
//! A browser has no way to call Rust, and the only thing it can hand
//! a module is **bytes in the module's memory**. So the door is four
//! exports:
//!
//! - `craftkit_wasm_request_address(length)` — make room for a
//!   command and say where it goes;
//! - `craftkit_wasm_run()` — run the command that is there;
//! - `craftkit_wasm_response_address()` — where the answer is;
//! - `craftkit_wasm_response_length()` — how long the answer is.
//!
//! That is the whole of the boundary, and it is **four functions and
//! two buffers**. There is no bundler in it: the module is built by
//! cargo and the page loads it with `WebAssembly.instantiate`, so
//! nothing has to be installed to open the kit in a browser.
//!
//! # Why it is `unsafe`
//!
//! Exporting a symbol by name is `unsafe` in the sense that the
//! attribute is: the name is a promise to whoever loads the module.
//! So this crate is on v2's **list** of exceptions — the sixth entry —
//! and it is held to the same rules as the others: `deny(unsafe_code)`
//! and not `forbid`, **one** `allow(unsafe_code)`, and one marked
//! module where it happens.
//!
//! It has no `unsafe` block and no `unsafe impl`: the module's memory
//! is addressed by **numbers**, and a number is not an unsafe thing.
//! The page writes into the memory it was given and reads the memory
//! it was given, and the door itself never dereferences an address it
//! was handed.

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

use std::cell::RefCell;

use craftkit_repl::{Repl, Reply};

/// The most a command may be, in bytes.
///
/// A command comes from a page, and a page can be handed anything at
/// all. The command is **bounded**, and a command over the bound is
/// refused rather than read.
///
/// The bound is **private**: it is this door's own number, and it is
/// named in the tree's README. A public bound is a promise anybody
/// else may rely on, and this one is not.
const MAX_COMMAND_BYTES: usize = 64 * 1024;

thread_local! {
    /// The door the page drives.
    ///
    /// It is built the first time it is asked for, because building it
    /// reads the registry, and the registry is what the linker wrote.
    static DOOR: RefCell<Option<Result<Door, String>>> = const { RefCell::new(None) };

    /// The command the page wrote.
    static REQUEST: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };

    /// The answer the door gave.
    static RESPONSE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// The door: the terminal door's own logic.
pub struct Door {
    /// The prompt, without the prompt.
    repl: Repl,
}

impl Door {
    /// A door onto the kit.
    pub fn new() -> Result<Door, String> {
        let ecosystem = craftkit_studio::ecosystem()
            .map_err(|error| format!("the ecosystem could not be built: {error}"))?;
        Ok(Door { repl: Repl::new(ecosystem) })
    }

    /// Run a command, and answer with the lines.
    ///
    /// The door is the **terminal door**, so the answers are the
    /// terminal door's answers. The one answer of its own is `quit`: a
    /// page has no prompt to leave.
    pub fn run(&mut self, command: &str) -> String {
        match self.repl.run(command) {
            Reply::Lines(lines) => lines.join("\n"),
            Reply::Quit => "a page has no prompt to leave: close the tab".to_string(),
        }
    }
}

/// Make room for a command of that many bytes, and say where it goes.
///
/// This is the page's **first** step, in Rust: the page asks for the
/// room and writes the command into the module's memory itself.
pub fn reserve_request(length: usize) -> u32 {
    REQUEST.with(|request| {
        let mut request = request.borrow_mut();
        request.clear();
        request.resize(length, 0);
        request.as_ptr() as usize as u32
    })
}

/// Write a command into the room that was made.
///
/// This is the page's **second** step, in Rust: the page writes the
/// bytes into the module's memory at the address it was given.
pub fn write_request(bytes: &[u8]) -> Result<(), String> {
    REQUEST.with(|request| {
        let mut request = request.borrow_mut();
        if request.len() < bytes.len() {
            return Err("the room for the command is too small".to_string());
        }
        request.clear();
        request.extend_from_slice(bytes);
        Ok(())
    })
}

/// Where the answer is, as a number.
///
/// The page reads the answer out of the module's memory at this
/// address; a Rust caller reads it out of the same buffer, safely.
pub fn response_address() -> u32 {
    RESPONSE.with(|response| {
        let response = response.borrow();
        response.as_ptr() as usize as u32
    })
}

/// The answer, as text.
///
/// The page reads the answer out of the module's memory; a Rust
/// caller reads it out of the same buffer, safely.
pub fn response() -> String {
    RESPONSE.with(|response| {
        let response = response.borrow();
        String::from_utf8_lossy(&response).into_owned()
    })
}

/// Put a command in the request buffer, run it, and answer with the
/// lines.
///
/// This is the whole door. The page does the same thing in two steps,
/// because the page has to write into the module's memory itself; a
/// Rust caller says it in one.
pub fn submit(command: &str) -> String {
    if command.len() > MAX_COMMAND_BYTES {
        return format!("refused: the command is longer than {MAX_COMMAND_BYTES} bytes");
    }
    let _ = reserve_request(command.len());
    let _ = write_request(command.as_bytes());
    let answer = DOOR.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(Door::new());
        }
        match slot.as_mut() {
            Some(Ok(door)) => door.run(command),
            Some(Err(error)) => error.clone(),
            None => "the door could not be built".to_string(),
        }
    });
    RESPONSE.with(|response| {
        let mut response = response.borrow_mut();
        response.clear();
        response.extend_from_slice(answer.as_bytes());
    });
    answer
}

/// The **exports**: what the page finds by name.
///
/// The page cannot call Rust, and a browser has no way to hand a
/// string to a module except its memory: so the page writes the
/// command into the module's memory and calls these four functions.
///
/// The unsafe is the attribute and nothing else. A name is a promise
/// to whoever loads the module, and the promise is kept: these four
/// functions are the ones the page calls, and they do what the page's
/// comment says they do.
#[allow(unsafe_code)]
pub mod exports {
    use super::REQUEST;

    /// Make room for a command of that many bytes, and say where it
    /// goes.
    ///
    /// The address is a number: the page writes the command into the
    /// module's memory at that offset. The room is **exactly** that
    /// many bytes, so the command is bounded by what the page asked
    /// for.
    #[unsafe(no_mangle)]
    pub extern "C" fn craftkit_wasm_request_address(length: u32) -> u32 {
        super::reserve_request(length as usize)
    }

    /// Run the command that is in the request buffer.
    ///
    /// The answer is written into the response buffer, and its length
    /// is given back, because the page reads the answer out of the
    /// module's memory.
    #[unsafe(no_mangle)]
    pub extern "C" fn craftkit_wasm_run() -> u32 {
        let command = REQUEST.with(|request| {
            let request = request.borrow();
            String::from_utf8_lossy(&request).into_owned()
        });
        let answer = super::submit(&command);
        answer.len() as u32
    }

    /// Where the answer is.
    #[unsafe(no_mangle)]
    pub extern "C" fn craftkit_wasm_response_address() -> u32 {
        super::response_address()
    }

    /// How long the answer is.
    #[unsafe(no_mangle)]
    pub extern "C" fn craftkit_wasm_response_length() -> u32 {
        super::response().len() as u32
    }
}
