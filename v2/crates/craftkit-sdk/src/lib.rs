//! `craftkit-sdk` — the whole surface a plugin needs.
//!
//! # What this crate is
//!
//! The **SDK**, and nothing else. A plugin — a crate that adds commands to
//! an application — depends on this crate and on the derive macro, and on
//! nothing else. It does not depend on the application, and the application
//! does not depend on the plugin's code: the link between them is the
//! registry, and the registry is built at run time from whatever the binary
//! happens to contain.
//!
//! That is the whole idea of v2, and it is worth saying why it is different
//! from a list of commands in one crate:
//!
//! - a **list** is closed. Adding a command means editing the list, which
//!   means every application that wants that command has to take the edit.
//!   The tree this replaces carried that list in one crate, and the price
//!   was paid by every application that re-purposed it;
//! - a **registry** is open. A command is a value, the values are collected
//!   from the crates that made them, and a crate that is linked in
//!   contributes its commands without anybody naming them.
//!
//! # The five things in here
//!
//! | | |
//! |---|---|
//! | [`Params`] | the caller's values, read by name and never indexed |
//! | [`ParamKind`] | a **type** that knows how to read itself and what it is |
//! | [`CommandSpec`] | one command: its id, its parameters, how to run it |
//! | [`Registry`] | the commands, at run time, in any order, from any crate |
//! | [`Session`] | the document, the history and the one way to run a command |

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod document;
pub mod error;
pub mod params;
pub mod registry;
pub mod session;
pub mod spec;

pub use document::{Document, Node};
pub use error::{EngineError, Outcome, ParamDoc, ParamError};
pub use params::{Bool, Doc, Either, NodeRef, Number, ParamKind, Params, Text};
pub use registry::{CommandRegistration, Registry, RegistryError};
pub use session::Session;
pub use spec::CommandSpec;

/// The link-time registry, re-exported so a plugin does not have to depend
/// on `inventory` itself.
///
/// The derive macro writes the registration, and the registration names
/// `inventory` through this re-export. That is deliberate: a plugin's
/// dependency list should be the SDK and the macro, and nothing else.
pub use inventory;

/// A predicate that says a command is always available.
///
/// The default for a command that does not say otherwise. It takes the
/// session so that a command which *does* have a condition can write one:
///
/// ```ignore
/// #[command(id = "node.mute", enabled = has_nodes)]
/// struct NodeMute { … }
/// ```
pub fn always(_session: &Session) -> bool {
    true
}

/// The JSON value type, re-exported.
///
/// A plugin should not have to depend on `serde_json` to hand a value back:
/// the SDK owns the shape of the values, so it re-exports the crate that
/// makes them.
pub use serde_json;
