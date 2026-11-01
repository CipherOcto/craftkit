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
//! # The seams
//!
//! The same registry holds every kind of contribution, so a plugin can add
//! more than commands:
//!
//! | Contribution | What it is | Where it is checked |
//! |---|---|---|
//! | [`CommandSpec`] | something a person or an agent can do | the ecosystem |
//! | [`NodeKindSpec`] | a kind of thing in the document | the ecosystem |
//! | [`ToolSpec`] | a gesture that runs a command | the ecosystem |
//! | [`PanelSpec`] | a window's worth of commands | the ecosystem |
//! | [`LayerSpec`] | a group of contributions that depend on each other | the ecosystem |
//!
//! # The five things in here
//!
//! | | |
//! |---|---|
//! | [`Params`] | the caller's values, read by name and never indexed |
//! | [`ParamKind`] | a **type** that knows how to read itself and what it is |
//! | [`CommandSpec`] | one command: its id, its parameters, how to run it |
//! | [`Registry`] | the contributions, at run time, in any order, from any crate |
//! | [`Session`] | the document, the history and the one way to run a command |

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub use craftkit_derive::{Format, Migration};

pub mod anchor;
pub mod appearance;
pub mod ask;
pub mod assets;
pub mod bundle;
pub mod bytes;
pub mod camera;
pub mod command;
pub mod contribution;
pub mod document;
pub mod ecosystem;
pub mod error;
pub mod evaluator;
pub mod files;
pub mod format;
pub mod group;
pub mod guard;
pub mod hand;
pub mod history;
pub mod jobs;
pub mod journal;
pub mod layer;
pub mod lines;
pub mod link;
pub mod params;
pub mod picture;
pub mod plan;
pub mod plugin;
pub mod plugincmds;
pub mod plugins;
pub mod prompt;
pub mod realtime;
pub mod relation;
pub mod resume;
pub mod seams;
pub mod select;
pub mod session;
pub mod stream;
pub mod taskcmds;
pub mod tasks;
pub mod view;

pub use anchor::{Anchored, PluginAnchor};
pub use appearance::{AddAppearance, AppearanceKind, SetAppearance};
pub use bundle::{Bundle, File};
pub use camera::Camera;
pub use command::{CommandRun, CommandSpec, DynamicRun};
pub use contribution::{Contribution, Registration, Registry, RegistryError};
pub use document::{
    Appearance, Asset, Canvas, Cap, Colour, Document, FillRule, Join, Node, NodeKindSpec, Path, Point,
    RecordingCanvas, Rect, Relation, Segment, Stroke, SubPath,
};
pub use ecosystem::{Ecosystem, EcosystemError, MenuEntry, MenuNode, Projection};
pub use error::{EngineError, Outcome, ParamDoc, ParamError};
pub use evaluator::EvaluatorSpec;
pub use format::{FormatSpec, FormatValue, MigrationSpec};
pub use history::{History, HistoryNode};
pub use jobs::{Cancel, Job, JobId, JobPool, JobResult};
pub use journal::JournalEntry;
pub use params::{Bool, Doc, Either, NodeRef, Number, ParamKind, Params, Text};
pub use plan::{Plan, PlanItem};
pub use plugin::{Catalogue, LoadedPlugin, PluginCommand, PluginError, PluginParam};
pub use prompt::{MapSupplier, Question, Supplier};
pub use realtime::{Block, Lane, LaneJob, LaneRunner, Mailbox, Report};
pub use resume::Pending;
pub use seams::{LayerSpec, PanelSpec, ToolSpec};
pub use select::Selected;
pub use session::Session;
pub use tasks::{Task, TaskRegistry, TaskState};

/// The link-time registry, re-exported so a plugin does not have to depend
/// on `inventory` itself.
///
/// The derive macro writes the registration, and the registration names
/// `inventory` through this re-export. That is deliberate: a plugin's
/// dependency list should be the SDK and the macro, and nothing else.
pub use inventory;

/// The JSON value type, re-exported.
///
/// A plugin should not have to depend on `serde_json` to hand a value back:
/// the SDK owns the shape of the values, so it re-exports the crate that
/// makes them.
pub use serde_json;

/// A predicate that says a command is always available.
///
/// The default for a command that does not say otherwise. It takes the
/// session so that a command which *does* have a condition can write one:
///
/// ```ignore
/// #[derive(Command)]
/// #[command(id = "node.mute", enabled = has_nodes)]
/// struct NodeMute { … }
/// ```
pub fn always(_session: &Session) -> bool {
    true
}
