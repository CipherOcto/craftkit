//! `craftkit-deflate` — the **inflate**: DEFLATE, from RFC 1951.
//!
//! # What this is
//!
//! A compressed stream is a stream of **bits**, and DEFLATE is the way
//! that a stream of bytes is written as bits that say what to repeat.
//! This crate reads it: the three kinds of block DEFLATE has — stored,
//! fixed Huffman and dynamic Huffman — and the **zlib** wrapper around
//! them.
//!
//! # Why it is a library and not a plugin
//!
//! Because a compressed stream is a compressed stream: the reader reads
//! bytes and gives back bytes, and it knows nothing about a document, a
//! canvas or the kit. A picture reader (§67, §68), a package reader
//! (§901, §902) and a PDF filter (§437) all want the same thing, and
//! they want it from one place.
//!
//! # The file is hostile
//!
//! A compressed stream came from somewhere else, so it is a stream that
//! could be anything. The reader **refuses** what it cannot make sense
//! of, and it refuses **within limits**: a stream that says it holds a
//! gigabyte is not read, because the reading would be the problem, and
//! not the stream.
//!
//! # The Huffman decoding
//!
//! It is the **canonical** one: the code lengths are counted, the codes
//! are numbered in order, and the bits are read one at a time until a
//! code is found. It is the slowest of the ways to do it and the one
//! that is the easiest to be sure of, and a decoder that is easy to be
//! sure of is a decoder that can be checked.
//!
//! # What this does not do
//!
//! - It does not **compress**. The tree's own compressor is not here,
//!   and a writer is a different program: this one only reads.
//! - It does not check the checksum of a zlib stream that does not have
//!   one, and it does not decode a stream that is not DEFLATE: those
//!   are refusals with names, and they are made here.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod inflate;

pub use inflate::{Error, Limits, adler32};

/// Inflate a **DEFLATE** stream.
///
/// This is the raw thing: the bits of the stream and nothing else. A
/// **zlib** stream is this with two bytes at the front and four at the
/// end, and [`zlib`] reads that.
pub fn deflate(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    inflate::deflate_bytes(bytes, limits)
}

/// Inflate a **zlib** stream.
///
/// The two header bytes are checked, the DEFLATE stream is inflated, and
/// the Adler-32 at the end is checked against what was inflated.
pub fn zlib(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    inflate::zlib(bytes, limits)
}
