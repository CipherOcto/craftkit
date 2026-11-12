//! Contributions: the one shape every extension shares.
//!
//! # The problem this solves
//!
//! A tree that has commands, node kinds, tools, panels, layers, formats and
//! importers has **seven registries**. Written by hand, that is seven copies
//! of the same four things: a vector, a lookup, a duplicate check and a
//! malformed-id check. Seven copies is seven places for the checks to drift,
//! and the drift is invisible until two of them disagree.
//!
//! So there is **one** registry here, generic over what it holds, and the
//! checks live in it once.
//!
//! # What a contribution is
//!
//! Anything a crate can add to the application:
//!
//! - a **command** — something a person or an agent can do;
//! - a **node kind** — a kind of thing in the document;
//! - a **tool** — a gesture that runs a command;
//! - a **panel** — a window's worth of commands;
//! - a **layer** — a group of contributions that depend on each other.
//!
//! Every contribution has three facts that the ecosystem checks when the
//! program starts:
//!
//! - an **id**, which is unique;
//! - a **capability**, which says what sort of thing it is — so that one
//!   registry can answer "every effect" or "every exporter" without a list
//!   of its own;
//! - a **layer**, which says where it belongs — so that a plugin that needs
//!   a layer that is not there is refused at start-up rather than failing
//!   later in a `match`.
//!
//! # The link is the wiring
//!
//! A contribution reaches the program because its crate was **linked in**.
//! The registry is filled by the linker, and the application never names a
//! crate's contributions.

use crate::session::Session;

/// Something a crate contributes to the application.
pub trait Contribution: Clone + Send + Sync + 'static {
    /// The contribution's id: dotted, lower case, unique.
    fn id(&self) -> &'static str;

    /// What sort of contribution it is.
    ///
    /// A command says `command`; a command that is really an effect says
    /// `effect`; a node kind says `node kind`. The word is what a
    /// projection groups by and what an agent is told.
    fn capability(&self) -> &'static str;

    /// The layer it belongs to.
    ///
    /// The ecosystem refuses to start when a contribution is in a layer that
    /// is not linked in.
    fn layer(&self) -> &'static str {
        "core"
    }

    /// The **area** it belongs to, when it has one.
    ///
    /// An area is the group a contribution is *listed* under: the `grid` in
    /// `grid.pivot.set`. A command has one; a node kind, a tool and a panel
    /// usually do not, and the default is the empty string, so a kind
    /// without an area is in no area.
    ///
    /// **The area is a fact of the contribution, not of its name.** The id
    /// and the area agree by rule, but the rule has an exception
    /// (`no_prefix`), and a count that read the id would be counting
    /// something else.
    fn area(&self) -> &'static str {
        ""
    }

    /// Why an id of this kind is not well formed, if it is not.
    ///
    /// Most kinds are named `<area>.<thing>`, and a layer is named by one
    /// word — a layer is a group of contributions, not a place on a screen.
    /// So the rule belongs to the kind, and the registry asks.
    fn malformed_id(id: &str) -> Option<String> {
        malformed_dotted_id(id)
    }
}

/// The link-time registration of one contribution.
///
/// A wrapper rather than the contribution itself, because a contribution may
/// own something that needs dropping — a command that runs in a plugin
/// holds an `Arc` — and the linker's registration has to be a constant.
pub struct Registration<T: 'static> {
    /// The contribution.
    pub item: &'static T,
}

impl<T: 'static> Clone for Registration<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Registration<T> {}

impl<T: 'static> std::fmt::Debug for Registration<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Registration").finish()
    }
}

/// Why a contribution was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Two contributions with one id.
    Duplicate(String),
    /// The id is not well formed.
    Malformed { id: String, reason: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Duplicate(id) => write!(f, "two contributions are called `{id}`"),
            RegistryError::Malformed { id, reason } => {
                write!(f, "`{id}` is not a contribution id: {reason}")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// The contributions of one kind.
///
/// The same registry for every kind: a command, a node kind, a tool, a panel
/// and a layer are all held in one of these, and the checks are the same.
#[derive(Debug, Clone)]
pub struct Registry<T: Contribution> {
    items: Vec<T>,
}

impl<T: Contribution> Default for Registry<T> {
    fn default() -> Self {
        Registry { items: Vec::new() }
    }
}

impl<T: Contribution> Registry<T> {
    /// An empty registry.
    pub fn new() -> Self {
        Registry { items: Vec::new() }
    }

    /// Add a contribution.
    ///
    /// The two refusals are the whole of the check: an id has to be well
    /// formed, and it has to be free.
    pub fn register(&mut self, item: T) -> Result<(), RegistryError> {
        let id = item.id();
        if let Some(reason) = T::malformed_id(id) {
            return Err(RegistryError::Malformed { id: id.to_string(), reason });
        }
        if self.get(id).is_some() {
            return Err(RegistryError::Duplicate(id.to_string()));
        }
        self.items.push(item);
        Ok(())
    }

    /// The contribution with an id, if there is one.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.items.iter().find(|item| item.id() == id)
    }

    /// Every contribution, in the order they were registered.
    pub fn all(&self) -> &[T] {
        &self.items
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ids, sorted.
    pub fn ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.items.iter().map(|item| item.id()).collect();
        ids.sort_unstable();
        ids
    }

    /// The capabilities in use, sorted.
    ///
    /// **Derived.** A tree with commands, effects, exporters and panels does
    /// not have a list of them: it has a registry per kind, and each
    /// contribution says what it is.
    pub fn capabilities(&self) -> Vec<&'static str> {
        let mut capabilities: Vec<&'static str> = self.items.iter().map(|item| item.capability()).collect();
        capabilities.sort_unstable();
        capabilities.dedup();
        capabilities
    }

    /// The contributions of one capability.
    pub fn of_capability(&self, capability: &str) -> Vec<&T> {
        self.items.iter().filter(|item| item.capability() == capability).collect()
    }

    /// The contributions of one area.
    ///
    /// **The area, not the id.** The two agree by rule and the rule has an
    /// exception, and a count that read the id would be counting the
    /// exception's spelling rather than the group.
    pub fn of_area(&self, area: &str) -> Vec<&T> {
        self.items.iter().filter(|item| item.area() == area).collect()
    }
}

impl<T: Contribution> Registry<T> {
    /// A registry holding every contribution of this kind the binary was
    /// linked with.
    ///
    /// This is the whole of the wiring: the linker has already put the
    /// contributions there, so there is nothing to list.
    ///
    /// The bound is on the *registration* rather than on the contribution,
    /// because the link collects registrations and the registry holds
    /// contributions.
    pub fn from_inventory() -> Result<Self, RegistryError>
    where
        Registration<T>: inventory::Collect,
    {
        let mut registry = Registry::new();
        for registration in inventory::iter::<Registration<T>> {
            registry.register((*registration.item).clone())?;
        }
        Ok(registry)
    }
}

/// Whether an id is well formed: dotted, lower case, at least two parts.
///
/// The rule for most kinds of contribution, which is why it is one function
/// and not seven.
pub fn malformed_dotted_id(id: &str) -> Option<String> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return Some("a contribution id is `<area>.<thing>`".to_string());
    }
    for part in parts {
        if part.is_empty() {
            return Some("a contribution id has no empty part".to_string());
        }
        if !part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Some("a contribution id is dotted and lower case".to_string());
        }
    }
    None
}

/// Whether a **layer's** id is well formed: one word, lower case.
///
/// A layer is not an area: it names a group of contributions, and the group
/// is not somewhere on a screen.
pub fn malformed_word_id(id: &str) -> Option<String> {
    if id.is_empty() {
        return Some("a layer's id is a word, and it is empty".to_string());
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Some("a layer's id is one word, in lower case".to_string());
    }
    None
}

/// A predicate that says a contribution can be used now.
pub type Enabled = fn(&Session) -> bool;
