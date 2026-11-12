//! The **rasteriser**: a canvas that draws a document into a picture.
//!
//! # Why it exists
//!
//! The kit has a **painting seam** — every kind of node knows how it is
//! painted, and the core never matches on a kind — but it had no canvas
//! to paint on. The only backend was a recorder that wrote down what it
//! was asked to draw, which is enough to *test* a kind and not enough to
//! *see* one.
//!
//! Every application in the family draws. So this is the piece that turns
//! the kit from a document model into something a person can look at.
//!
//! # What it is
//!
//! - a **picture**: a buffer of RGBA pixels, with a background;
//! - a **canvas**: the kit's own [`Canvas`] trait, implemented over that
//!   buffer, with the camera's zoom and offset applied;
//! - a **PNG writer**: the picture, written as a PNG file, with no
//!   dependency and no compression (the deflate stream is stored, not
//!   compressed — a picture that is a little bigger is a picture that
//!   anybody can read).
//!
//! # What it does not do
//!
//! - **There is no font.** The kit has the *metrics* of a typeface
//!   (§59) and not its glyphs, so text is drawn as the boxes it would
//!   occupy: a typewriter's worth of rectangles, one per character. It
//!   is honest about what it is — a placeholder that says *there is
//!   text here* — and it is not a font.
//! - **There is no anti-aliasing.** A pixel is either painted or it is
//!   not, and the edges are hard. A picture without soft edges is a
//!   picture a machine can reason about.
//! - **There is no 3D.** The camera is a zoom and an offset, and the
//!   document's own axes are the picture's axes.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod image;
mod png;
mod raster;

pub use image::Image;
pub use raster::{MAX_IMAGE_PIXELS, MAX_IMAGE_SIDE, Raster};

/// The kit's own canvas trait, re-exported so that a program can speak
/// about the seam without depending on the SDK for it.
pub use craftkit_sdk::document::Canvas;
