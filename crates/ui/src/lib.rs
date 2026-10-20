//! # craftkit-ui
//!
//! The front end: a thin shell over the engine.
//!
//! ## The rule
//!
//! **Panels read engine state and act through the command registry.** A panel
//! never writes to the document, never calls the evaluator, and never computes
//! a value the evaluator also computes. If a panel needs to change something,
//! it runs a [`craftkit_engine`] command — the same one a script, the CLI, the
//! control channel and the MCP server would run.
//!
//! That is what makes the UI *swappable*. Everything below this crate is
//! testable headlessly; everything above it is a binary. And it is what makes
//! an agent's job tractable: the thing an agent can reach and the thing a person
//! can click are the same surface, so a bug found by one is a bug in both.
//!
//! ## What lives here
//!
//! - [`theme`] — every colour, as a named token, checked for contrast.
//! - [`icons`] — every icon, drawn in code from original coordinates.
//! - [`automation`] — stable names for widgets, so an agent can address one.
//! - [`shell`] — the panels themselves.
//! - [`raster`] — a software rasteriser, so a frame can be produced with no GPU.
//!
//! ## Dependencies
//!
//! `egui` and nothing else. No windowing library, no renderer: this crate draws
//! into an [`egui::Context`] and knows nothing about what is hosting it, which
//! is what lets the same code run in a window, headlessly, and in a browser.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod automation;
pub mod icons;
pub mod raster;
pub mod shell;
pub mod theme;

pub use automation::{Automation, AutomationId, Widget};
pub use raster::{Click, DEFAULT_SIZE, Frame};
pub use shell::{Shell, ShellState, clickable, frame};
pub use theme::Tokens;
