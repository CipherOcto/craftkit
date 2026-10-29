//! One command, as a value.
//!
//! # The shape
//!
//! A [`CommandSpec`] is what the application knows about a command: its id,
//! where it lives, what it takes, and how to run it. It is a **value**, not a
//! constant in a list, and it is `Copy` — a command can be handed around
//! without a reference to the registry it came from.
//!
//! Nothing about it is hand-written in v2. The derive macro writes it from
//! the command's **struct**: the id and the label from the attribute, the
//! documentation from the doc comment, the parameters from the fields and
//! their types, and the `run` from the method.
//!
//! The `area` is a **string**, not an enum. The tree this replaces had seven
//! areas in an enum, which meant a new area was a change to the enum and
//! the compiler said so — and a plugin could not have an area of its own. A
//! string has no such list, and the rule that survives is the one that
//! mattered: the id's first segment is the area.

use crate::error::{EngineError, Outcome, ParamDoc};
use crate::params::Params;
use crate::session::Session;

/// One command.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// The command's id: `<area>.<thing>`, dotted and lower case.
    pub id: &'static str,
    /// What the button says.
    pub label: &'static str,
    /// Which area it belongs to. The id's first segment is the same thing.
    pub area: &'static str,
    /// The icon's name. The drawing is the front end's business.
    pub icon: &'static str,
    /// One line about what it does. The doc comment, by default.
    pub doc: &'static str,
    /// The chord it is advertised with.
    pub shortcut: Option<&'static str>,
    /// What it takes. Written from the command's fields.
    pub params: &'static [ParamDoc],
    /// Whether it changes the document.
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
    pub run: fn(&mut Session, &Params<'_>) -> Result<Outcome, EngineError>,
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
