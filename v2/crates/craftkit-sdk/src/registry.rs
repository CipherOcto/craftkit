//! The registry: the commands, at run time.
//!
//! # Why this is a value and not a constant
//!
//! The tree this replaces held the commands in `static ALL: [&CommandSpec; 25]`
//! — one list, in one crate, with a hand-written count. Three things follow
//! from that, and all three are costs:
//!
//! - a crate that wants a command has to edit the list, so every
//!   application that wanted that command had to take the edit;
//! - the count was a number somebody had to remember, and a gate that
//!   checked the prose against it;
//! - the list was fixed at compile time, so nothing could be added while the
//!   program was running.
//!
//! Here the registry is a [`Registry`]: a value, built when the program
//! starts, and open the whole time.
//!
//! # Where the commands come from
//!
//! Three places, and a crate that uses any of them has to do nothing else:
//!
//! 1. **The crates that are linked in.** The derive macro writes a
//!    registration for every command, and the registrations are collected by
//!    the linker. A plugin crate that is a dependency of the application
//!    contributes its commands, and the application never names it.
//! 2. **The application itself**, at any moment: [`Registry::register`] is
//!    public, so a command can be made at run time.
//! 3. **A description**, from a file or from an agent: a command whose
//!    parameters are known and whose body is a script.

use crate::spec::CommandSpec;

/// One command, offered to the registry.
///
/// The derive macro writes one of these for every command, and the linker
/// collects them. The wrapper exists so that the thing collected is a type
/// the SDK owns, rather than a bare `CommandSpec` that every crate would
/// have to agree on.
#[derive(Debug, Clone, Copy)]
pub struct CommandRegistration {
    /// The command.
    pub spec: &'static CommandSpec,
}

// One registry for the type, in this crate, so every crate that submits a
// `CommandRegistration` — the application, a plugin, a test — puts it in the
// same place.
inventory::collect!(CommandRegistration);

/// Why a registration was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Two commands with one id.
    Duplicate(String),
    /// A command's id is not well formed.
    Malformed { id: String, reason: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Duplicate(id) => write!(f, "two commands are called `{id}`"),
            RegistryError::Malformed { id, reason } => write!(f, "`{id}` is not a command id: {reason}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// The commands an application knows about.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    commands: Vec<CommandSpec>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Registry { commands: Vec::new() }
    }

    /// A registry holding every command the binary was linked with.
    ///
    /// This is the whole of the wiring: the linker has already put the
    /// registrations there, so there is nothing to list.
    pub fn from_inventory() -> Result<Self, RegistryError> {
        let mut registry = Registry::new();
        for registration in inventory::iter::<CommandRegistration> {
            registry.register(*registration.spec)?;
        }
        Ok(registry)
    }

    /// Add a command.
    ///
    /// The check is that the id is not already taken: two commands with one
    /// id would make the catalogue a lie, and a refusal here is the only
    /// place that can catch it.
    pub fn register(&mut self, spec: CommandSpec) -> Result<(), RegistryError> {
        if let Some(reason) = malformed_id(spec.id) {
            return Err(RegistryError::Malformed { id: spec.id.to_string(), reason });
        }
        if self.get(spec.id).is_some() {
            return Err(RegistryError::Duplicate(spec.id.to_string()));
        }
        self.commands.push(spec);
        Ok(())
    }

    /// The command with an id, if there is one.
    pub fn get(&self, id: &str) -> Option<&CommandSpec> {
        self.commands.iter().find(|spec| spec.id == id)
    }

    /// Every command, in the order they were registered.
    pub fn all(&self) -> &[CommandSpec] {
        &self.commands
    }

    /// How many commands there are.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The areas that are in use.
    ///
    /// **Derived.** The tree this replaces had seven areas in an enum and a
    /// hand-written list beside it; here the areas are whatever the
    /// registered commands say they are, which means a plugin can have an
    /// area of its own and no list has to be updated.
    pub fn areas(&self) -> Vec<&'static str> {
        let mut areas: Vec<&'static str> = self.commands.iter().map(|spec| spec.area).collect();
        areas.sort_unstable();
        areas.dedup();
        areas
    }

    /// The ids, sorted.
    pub fn ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.commands.iter().map(|spec| spec.id).collect();
        ids.sort_unstable();
        ids
    }
}

/// Whether an id is a command id: dotted, lower case, at least two parts.
///
/// The rule is the tree's own, and the derive macro checks it too — but a
/// command can also be made at run time, and a command made at run time has
/// no compiler behind it. So the registry checks it as well.
fn malformed_id(id: &str) -> Option<String> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return Some("a command id is `<area>.<thing>`".to_string());
    }
    for part in parts {
        if part.is_empty() {
            return Some("a command id has no empty part".to_string());
        }
        if !part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Some("a command id is dotted and lower case".to_string());
        }
    }
    None
}
