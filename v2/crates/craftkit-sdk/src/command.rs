//! One command, as a value.
//!
//! # The shape
//!
//! A [`CommandSpec`] is what the application knows about a command: its id,
//! where it lives, what it takes, what it is *for*, and how to run it. It is
//! a **value**, not a constant in a list.
//!
//! Nothing about it is hand-written in v2. The derive macro writes it from
//! the command's **struct**: the id and the label from the attribute, the
//! documentation from the doc comment, the parameters from the fields and
//! their types, and the `run` from the method.
//!
//! # Two things it says that the old tree's did not
//!
//! - the **capability**: what the command is *for*. A command that exports
//!   a file and a command that mutes a node are both commands, and the
//!   difference is one word. A projection turns capabilities into menus,
//!   palettes and schemas, so a capability is a list nobody has to write;
//! - the **layer**: which group of contributions it belongs to, so that the
//!   ecosystem can refuse to start when a layer is missing.
//!
//! # How it runs
//!
//! [`CommandRun`] is two things: a function the binary was compiled with, or
//! a closure supplied after it was built. A plugin loaded at run time — a
//! subprocess, a dylib, a script — is the second, and it has the same
//! metadata, the same session and the same door as the first.

use std::fmt;
use std::sync::Arc;

use crate::contribution::Contribution;
use crate::error::{EngineError, Outcome, ParamDoc};
use crate::params::Params;
use crate::session::Session;

/// A command's body, supplied after the binary was built.
pub type DynamicRun = Arc<dyn Fn(&mut Session, &Params<'_>) -> Result<Outcome, EngineError> + Send + Sync>;

/// How a command runs.
#[derive(Clone)]
pub enum CommandRun {
    /// A function the binary was compiled with.
    Static(fn(&mut Session, &Params<'_>) -> Result<Outcome, EngineError>),
    /// A closure, from a plugin that was loaded at run time.
    Dynamic(DynamicRun),
}

impl fmt::Debug for CommandRun {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandRun::Static(_) => formatter.write_str("Static"),
            CommandRun::Dynamic(_) => formatter.write_str("Dynamic"),
        }
    }
}

impl CommandRun {
    /// Run it.
    pub fn call(&self, session: &mut Session, params: &Params<'_>) -> Result<Outcome, EngineError> {
        match self {
            CommandRun::Static(run) => run(session, params),
            CommandRun::Dynamic(run) => run(session, params),
        }
    }
}

/// One command.
#[derive(Clone, Debug)]
pub struct CommandSpec {
    /// The command's id: `<area>.<thing>`, dotted and lower case.
    pub id: &'static str,
    /// What the button says.
    pub label: &'static str,
    /// Which area it belongs to. The id's first segment is the same thing.
    pub area: &'static str,
    /// What the command is **for**: `command`, `effect`, `exporter`, …
    ///
    /// The word is what a projection groups by, and what an agent is told.
    pub capability: &'static str,
    /// The layer it belongs to.
    pub layer: &'static str,
    /// The icon's name. The drawing is the front end's business.
    pub icon: &'static str,
    /// One line about what it does. The doc comment, by default.
    pub doc: &'static str,
    /// The chord it is advertised with.
    pub shortcut: Option<&'static str>,
    /// Where the command goes in the **menu bar**: a path.
    ///
    /// The names are the path from the bar down to the command, and the
    /// first one is the area unless the command says otherwise. A menu is
    /// a **tree**, and one level of it is not enough: the sound
    /// application this port comes from has five hundred and twelve leaf
    /// paths, and a menu bar that can only say `area` cannot draw them.
    pub menu: &'static [&'static str],
    /// What it takes. Written from the command's fields.
    pub params: &'static [ParamDoc],
    /// Whether it is supposed to change the document.
    ///
    /// A promise the session keeps: a command that says `false` and changes
    /// the document is refused and the document is put back.
    pub mutates: bool,
    /// Whether a successful run pushes an undo entry.
    pub undoable: bool,
    /// Whether it is available right now.
    pub enabled: fn(&Session) -> bool,
    /// How to run it.
    ///
    /// This is the *generated* entry point: it reads the parameters into the
    /// command's struct and then calls the command's own method. The body of
    /// a command never sees a [`Params`], which is why it cannot read a
    /// parameter it did not declare.
    pub run: CommandRun,
}

// The link collects commands here.
inventory::collect!(crate::contribution::Registration<CommandSpec>);

impl Contribution for CommandSpec {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capability(&self) -> &'static str {
        self.capability
    }

    fn layer(&self) -> &'static str {
        self.layer
    }
}

impl CommandSpec {
    /// The parameter names, for the engine's refusal of an unknown key.
    ///
    /// The **alternative spelling** is one of them: a caller may give
    /// `node` or `node_name`, and neither is an unknown key.
    pub fn param_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        for param in self.params {
            names.push(param.name);
            if let Some(alternative) = param.alternative {
                names.push(alternative);
            }
        }
        names
    }

    /// The parameters a person is asked for.
    pub fn ask_params(&self) -> Vec<&'static ParamDoc> {
        self.params.iter().filter(|param| param.needed()).collect()
    }
}
