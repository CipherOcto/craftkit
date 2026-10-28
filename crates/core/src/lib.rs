//! # craftkit-core
//!
//! craftkit's own core: the **exact time base** and the **identifiers**.
//!
//! ## What is here, and what is one crate down
//!
//! §276: the readers that turn hostile input into typed values, the typed
//! values themselves, the atomic writer, the one temporary folder and the
//! JSON limits used to live here too. **None of them know what a document
//! is**, and a new application — a PDF editor, a vector illustrator, a
//! video editor — wants all of them and none of what is left. So they
//! moved down into [`craftkit_foundation`], which has **no workspace
//! dependency at all** and could be lifted out on its own.
//!
//! What is left here is the half that *is* craftkit:
//!
//! 1. [`Tick`] — exact integer time. Never store seconds as `f64`.
//! 2. [`NodeId`], [`DocumentId`] and [`IdCounter`] — the identifiers
//!    a document hands out.
//!
//! And the generic half is re-exported here, so `craftkit_core::Params` and
//! `craftkit_core::Value` still name what they named — the tree's own
//! code says `craftkit_foundation::` where it can, and the re-export is for
//! the reader who already knows the old name.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod ids;
pub mod tick;

// **The generic half of the tree is one crate away.** §276: the readers, the
// parameter readers, the typed values, the atomic writer, the temporary
// folder and the JSON limits do not know what a node is, and they did not
// need to be in the same crate as the things that do. A new application
// keeps this crate and the layer above it, and replaces what is above that.
pub use craftkit_foundation::{
    Color, CoreError, JSON_EXPANSION, MAX_JSON_MEMORY_BYTES, MAX_JSON_TEXT_BYTES, MAX_MAGNITUDE, MAX_TEXT,
    MAX_VALUE_DEPTH, ParamError, Params, ReadError, Value, atomic, finite, in_range, measure, params,
    read_file_bounded, read_line_bounded, read_to_string_bounded, readers, temp, text, value, writer,
};

pub use ids::{DocumentId, IdCounter, NodeId};
pub use tick::{FrameRate, MAX_INSTANT_TICKS, MIN_INSTANT_TICKS, TICKS_PER_FRAME_24, TICKS_PER_SECOND, Tick};
