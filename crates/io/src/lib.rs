//! # craftkit-io
//!
//! The `.craftkit` file format: what is written, how a version is chosen, and
//! how a file from the future is refused.
//!
//! ## Where this crate sits
//!
//! It knows about [`craftkit_model::Document`] and about JSON, and about
//! nothing else. It has no reference to the engine, the evaluator or the UI,
//! which is what lets the desktop app, the CLI and a script share one
//! implementation of "open this file" — and what lets that implementation be
//! tested without a window.
//!
//! ## Three rules
//!
//! 1. **A file says what it is.** `"format": "craftkit/N"`, checked before
//!    anything else is interpreted.
//! 2. **A version is never guessed.** A file from a newer craftkit is refused
//!    with a message that says so; reading it "best effort" is how a design
//!    gets quietly rewritten.
//! 3. **A change is one new file.** See [`format::pending`].

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod format;
pub mod store;

pub use format::{CURRENT, File, Format, FormatError, Meta, tag};
pub use store::{SaveError, load, load_text, save, save_text};
