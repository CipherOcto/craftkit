//! The command registry: the one list of everything craftkit can do.
//!
//! # Why one registry
//!
//! A toolbar button, a keyboard shortcut, a CLI subcommand, a control-channel
//! request, an MCP tool and a script all end up here. That is not tidiness, it
//! is the reason an agent can drive the application: **the tool an agent calls
//! is the same tool a person clicks**, so it cannot drift, cannot be
//! half-implemented, and cannot be missing a guard the interactive path has.
//!
//! # The shape of a command
//!
//! Each [`CommandSpec`] declares an id, a label, where it lives in the UI, its
//! parameters *as data*, and one `run` function. The parameter table is data
//! rather than a hand-written help string because four consumers read it: the
//! docs generator, the MCP server's tool schema, the CLI's `--help`, and
//! [`Params::deny_unknown`].
//!
//! # Ids
//!
//! Dotted, lower case, ours: `node.add`, `prop.set`, `history.undo`. Never
//! another product's id, and never a bare word — the prefix says which area
//! owns it, which is what makes the list sortable into a stable, conflict-free
//! catalogue.

use crate::commands::*;
use crate::error::EngineError;
use crate::params::Doc;
use crate::session::{Outcome, Session};
use craftkit_foundation::Params;
use serde_json::Value as Json;

/// Where a command appears in the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Area {
    Document,
    Node,
    Property,
    Keyframes,
    Playback,
    History,
    Debug,
}

impl Area {
    /// The id prefix this area's commands use, if they share one.
    ///
    /// Six areas do; `Debug` deliberately does not, because its commands are
    /// the cross-cutting ones — `diagnostics.list`, `plan.evaluate`,
    /// `session.stats` — and giving them a shared prefix would mean naming them
    /// after the *panel* rather than after what they do.
    pub fn prefix(self) -> Option<&'static str> {
        match self {
            Area::Document => Some("doc"),
            Area::Node => Some("node"),
            Area::Property => Some("prop"),
            Area::Keyframes => Some("key"),
            Area::Playback => Some("playback"),
            Area::History => Some("history"),
            Area::Debug => None,
        }
    }

    pub const ALL: [Area; 7] = [
        Area::Document,
        Area::Node,
        Area::Property,
        Area::Keyframes,
        Area::Playback,
        Area::History,
        Area::Debug,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Area::Document => "document",
            Area::Node => "node",
            Area::Property => "property",
            Area::Keyframes => "keyframes",
            Area::Playback => "playback",
            Area::History => "history",
            Area::Debug => "debug",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Area::Document => "Document",
            Area::Node => "Nodes",
            Area::Property => "Properties",
            Area::Keyframes => "Keyframes",
            Area::Playback => "Playback",
            Area::History => "History",
            Area::Debug => "Debug",
        }
    }
}

/// The icon a command shows.
///
/// Just a name. The UI draws it from its own icon set — and a test rejects a
/// command whose icon the set does not contain, so this cannot become an
/// implicit dependency on a particular UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Icon(pub &'static str);

/// One documented parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamDoc {
    pub name: &'static str,
    pub kind: Doc,
    pub required: bool,
    /// What it means, in one line. Rendered into the docs and the MCP schema.
    pub help: &'static str,
    /// The parameter that may be given **instead of this one**.
    ///
    /// `node` and `node_name` are one parameter in two spellings, and so are
    /// `seconds` and `frame`: the command takes either and refuses when it
    /// has neither. So a parameter with an alternative is **not required by
    /// itself** — the pair is, and the schema has to say "one of the two".
    ///
    /// §116: the catalogue marked the first spelling as required, so the
    /// published schema told a client it could not use the second one — and
    /// the tree's own probe called `playback.seek` with `{"frame": 48}`, a
    /// form its own schema forbade.
    pub alternative: Option<&'static str>,
}

impl ParamDoc {
    pub const fn req(name: &'static str, kind: Doc, help: &'static str) -> Self {
        Self { name, kind, required: true, help, alternative: None }
    }

    pub const fn opt(name: &'static str, kind: Doc, help: &'static str) -> Self {
        Self { name, kind, required: false, help, alternative: None }
    }

    /// A parameter the command needs — unless `alternative` is given instead.
    ///
    /// The pair is **"one of the two"**: neither is required on its own, and
    /// the command refuses when it has neither. The schema says so with an
    /// `anyOf`, and the catalogue says it with this field. §116.
    pub const fn either(
        name: &'static str,
        kind: Doc,
        help: &'static str,
        alternative: &'static str,
    ) -> Self {
        Self { name, kind, required: false, help, alternative: Some(alternative) }
    }

    /// Whether a person has to fill this in before the command can run.
    ///
    /// A required parameter has to be filled in, and so does one that has an
    /// alternative: the pair is required, even though neither half is.
    /// §116, §118.
    pub const fn needed(&self) -> bool {
        self.required || self.alternative.is_some()
    }
}

/// One command.
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub area: Area,
    pub icon: Icon,
    /// The key chord this command is advertised with.
    ///
    /// The tree binds **no keys**: there is no shortcut parser and no key
    /// handling in the UI, so a chord here is a *specification* for a binding
    /// that does not exist yet. The MCP tool description repeats it to an
    /// agent and says that it is intended rather than bound. §83.
    ///
    /// The chords are checked for uniqueness and shape by
    /// `the_advertised_shortcuts_are_unique_and_well_formed`, so the
    /// specification is one somebody could implement.
    pub shortcut: Option<&'static str>,
    /// The parameters a person is asked for when the command is started from
    /// the interface.
    ///
    /// The parameter table says what the command **takes**; this says what
    /// the **interactive path** asks for. The two differ on purpose:
    /// `node.add` takes a name, a kind and an amount and is perfectly happy
    /// with none of them, while `doc.range` takes three optional parameters
    /// and refuses to run with none of them. §118.
    ///
    /// A name in this list is a parameter of this command. A gate keeps it
    /// that way, and keeps every required parameter in it — a command whose
    /// form cannot ask for what it needs would ask for the wrong thing.
    pub asks: &'static [&'static str],
    pub params: &'static [ParamDoc],
    /// One line for `commands --json` and the MCP tool description.
    pub doc: &'static str,
    /// Whether the command changes the document. A read-only command must not
    /// mark the project dirty.
    ///
    /// It is a promise the engine keeps, on both sides. A command that
    /// declares `mutates: true` and changes nothing is refused, and a command
    /// that declares `mutates: false` and changes it is refused too: the write
    /// would not be validated, would not be undoable, and would not mark the
    /// project dirty, so the design would change and the app would not know
    /// that it had. The flag is published to agents in the command schema, and
    /// an agent that reads it as *"safe to use as a probe"* has to be right.
    /// §44, §93, §96.
    pub mutates: bool,
    /// Whether a successful run pushes an undo entry.
    ///
    /// Separate from `mutates` because two commands must be false here:
    /// `history.undo` and `history.redo` change the document *by* restoring a
    /// snapshot, and pushing that as a new step would push a history entry that
    /// points at the state we just came from — so undoing again would appear to
    /// do nothing, and the redo branch would be cleared by its own undo.
    ///
    /// **Only meaningful together with `mutates`.** A command that does not
    /// change the document has nothing to undo: the push lives inside the
    /// `mutates` branch, so a read-only command cannot push an entry however
    /// it is declared. Four commands — `playback.seek`, `plan.evaluate`,
    /// `diagnostics.list` and `session.stats` — declared `undoable: true` and
    /// never could, and the flag is published to agents in the command
    /// schema. `no_command_that_does_not_mutate_is_undoable` keeps the data
    /// honest. §94.
    pub undoable: bool,
    /// Whether the command is available right now.
    pub enabled: fn(&Session) -> bool,
    pub run: fn(&mut Session, &Params<'_>) -> Result<Outcome, EngineError>,
}

impl CommandSpec {
    /// The parameter names, for [`craftkit_foundation::Params::deny_unknown`].
    pub fn param_names(&self) -> Vec<&'static str> {
        self.params.iter().map(|p| p.name).collect()
    }

    /// The parameters the interactive path asks for, in the order it asks.
    ///
    /// A name in [`CommandSpec::asks`] that is not a parameter of this
    /// command is **dropped**, not invented: the form shows what the
    /// command declares, and a gate refuses a name that is not one of them.
    pub fn ask_params(&self) -> Vec<&'static ParamDoc> {
        self.asks.iter().filter_map(|name| self.params.iter().find(|p| p.name == *name)).collect()
    }

    /// The pairs of parameters of which **one** must be given.
    ///
    /// A parameter with an alternative is one of a pair: neither half is
    /// required alone, and the command refuses when it has neither. Two
    /// entries that both say `required: false` do not say that — they say
    /// the opposite, and a reader of the catalogue is entitled to believe
    /// them. So the catalogue says the pairs too, and the plain listing
    /// prints one line for the pair. §120.
    pub fn one_of_groups(&self) -> Vec<Vec<&'static str>> {
        self.params.iter().filter_map(|p| p.alternative.map(|other| vec![p.name, other])).collect()
    }

    /// A one-line description for a listing.
    pub fn signature(&self) -> String {
        if self.params.is_empty() {
            return format!("{}()", self.id);
        }
        let parts: Vec<String> =
            self.params.iter().map(|p| format!("{}:{}", p.name, p.kind.json_name())).collect();
        format!("{}({})", self.id, parts.join(", "))
    }

    /// The JSON shape the MCP server and the CLI share.
    pub fn schema(&self) -> Json {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "area": self.area.key(),
            "icon": self.icon.0,
            "shortcut": self.shortcut,
            "mutates": self.mutates,
            "undoable": self.undoable,
            "doc": self.doc,
            "asks": self.asks,
            // The pairs of which one must be given. Two entries that both
            // say `required: false` do not say that the command needs one
            // of them — they say the opposite. §120.
            "one_of": self.one_of_groups(),
            "params": self.params.iter().map(|p| {
                let mut param = serde_json::json!({
                    "name": p.name,
                    "type": p.kind.json_name(),
                    "required": p.required,
                    "help": p.help,
                });
                // The alternative is part of the parameter's meaning: it says
                // that this one and that one are one parameter in two
                // spellings. §116.
                if let Some(alternative) = p.alternative {
                    param["alternative"] = serde_json::json!(alternative);
                }
                param
            }).collect::<Vec<_>>(),
        })
    }
}

impl std::fmt::Debug for CommandSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandSpec")
            .field("id", &self.id)
            .field("area", &self.area)
            .field("mutates", &self.mutates)
            .finish()
    }
}

/// The whole catalogue.
#[derive(Debug, Clone, Copy, Default)]
pub struct Registry;

/// Every registered command, in a stable order.
///
/// A hand-written list on purpose. Deriving it from a macro would save three
/// lines and lose the ability to read the catalogue top to bottom, which is how
/// a reviewer checks that a new command has a sensible neighbour and no
/// duplicate id. `all_specs_is_sorted_and_unique` enforces the invariants.
pub fn all() -> &'static [&'static CommandSpec] {
    &ALL
}

/// The catalogue as a `static`, so `all()` returns a reference rather than a
/// reference to a temporary — which is also why the array has a declared
/// length: adding a command without adding it to the count is a compile error,
/// not a silently truncated list. The order is checked by
/// `the_catalogue_is_sorted_and_free_of_duplicates`.
static ALL: [&CommandSpec; 23] = [
    &DOC_NOTE,
    &DOC_RANGE,
    &DOC_RENAME,
    &NODE_ADD,
    &NODE_DUPLICATE,
    &NODE_KIND,
    &NODE_MOVE,
    &NODE_MUTE,
    &NODE_REMOVE,
    &NODE_RENAME,
    &PROP_RESET,
    &PROP_SET,
    &KEY_CLEAR,
    &KEY_EASE,
    &KEY_LIST,
    &KEY_REMOVE,
    &KEY_SET,
    &PLAYBACK_SEEK,
    &HISTORY_REDO,
    &HISTORY_UNDO,
    &DIAGNOSTICS_LIST,
    &PLAN_EVALUATE,
    &SESSION_STATS,
];

/// Look a command up by id.
pub fn get(id: &str) -> Option<&'static CommandSpec> {
    all().iter().copied().find(|c| c.id == id)
}

/// A command that panics, present only in test builds.
///
/// The restore-on-panic path in `Session::execute` cannot be reached any other
/// way, and it is the single path a user cares about most: a real command that
/// panics must leave the document exactly as it was. Registering it here rather
/// than in a test module keeps it out of the shipped registry while still
/// exercising the real `catch_unwind`.
#[cfg(test)]
pub static TEST_PANIC: CommandSpec = CommandSpec {
    id: "test.panic",
    label: "Panic (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Panics on purpose. Registered only in test builds.",
    asks: &[],
    params: &[ParamDoc::opt("how", Doc::Number, "Which kind of panic to raise.")],
    mutates: true,
    undoable: false,
    enabled: always_in_tests,
    run: |s, p| {
        let how = p.num("how").ok().flatten().unwrap_or(0.0);
        if how == 1.0 {
            s.doc_mut().meta.name = "half-applied".to_string();
            panic!("deliberate, after a partial edit");
        }
        std::panic::panic_any(42u32)
    },
};

#[cfg(test)]
fn always_in_tests(_s: &crate::session::Session) -> bool {
    true
}

/// A command that changes the document and then refuses.
///
/// Registered only in test builds. The engine's rollback has to put the
/// document **and the plan** back, and a half-applied failure is the only way
/// to see the second half: the plan is recomputed on the *success* path, so
/// every other failure is refused before it changes anything, and every other
/// change is followed by a recompute that has nothing to do with the
/// rollback.
///
/// §43 is the finding: with `self.recompute()` deleted from the restore, every
/// test in the workspace was green — the document came back and the plan did
/// not, and nothing said so.
#[cfg(test)]
pub static TEST_HALF_APPLY: CommandSpec = CommandSpec {
    id: "test.half_apply",
    label: "Half-apply then fail (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Changes the document and then refuses. Registered only in test builds.",
    asks: &[],
    params: &[],
    mutates: true,
    undoable: false,
    enabled: always_in_tests,
    run: |s, _p| {
        // Muting the filter changes the plan's output, so a plan that was not
        // re-evaluated is visible in the number the caller reads.
        let node = s.doc_mut().get_mut(craftkit_core::NodeId(2)).ok_or_else(|| EngineError::Param {
            id: TEST_HALF_APPLY.id.to_string(),
            source: craftkit_foundation::ParamError::new("node", "the sample has no node 2"),
        })?;
        node.muted = true;
        Err(EngineError::Param {
            id: TEST_HALF_APPLY.id.to_string(),
            source: craftkit_foundation::ParamError::new(
                "command",
                "this command failed on purpose, after changing the document",
            ),
        })
    },
};

/// A command that declares that it mutates the document and does not.
///
/// Registered only in test builds. The engine compares the document with the
/// snapshot it took before the command, and a command that declares
/// `mutates: true` and writes nothing is a bug — it would put a no-op step on
/// the undo stack and mark the project dirty for no reason.
///
/// No command in the tree does that on purpose, so nothing ever reached the
/// check: §44 found that deleting the check left every test in the workspace
/// green.
///
/// But the check was reached **every day** by another route. A command that
/// writes the value a field already holds leaves the document identical, and
/// the comparison cannot tell that from a command that forgot to write.
/// Measured: eight of the nine commands that can be asked to do what they
/// have already done were refused as *"failed unexpectedly (the command
/// declared that it mutates the document but changed nothing)"*. §93 fixed
/// that by having the command say whether it changed anything — and this
/// command, which claims it did, is what keeps the check alive.
#[cfg(test)]
pub static TEST_NOOP: CommandSpec = CommandSpec {
    id: "test.noop",
    label: "Change nothing and claim otherwise (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Changes nothing and declares that it mutates. Registered only in test builds.",
    asks: &[],
    params: &[],
    mutates: true,
    undoable: true,
    enabled: always_in_tests,
    run: |_s, _p| Ok(Outcome::new("changed nothing")),
};

/// A command that marks the project dirty and then changes nothing.
///
/// Registered only in test builds. The engine sets the dirty flag only after
/// a command *succeeds*, so the rollbacks that put it back are no-ops today
/// and no test could see them work. This command is the way to see one: it
/// sets the flag, leaves the document alone, and succeeds — so the engine
/// refuses it as a no-op and has to put the flag back.
#[cfg(test)]
pub static TEST_DIRTY_NOOP: CommandSpec = CommandSpec {
    id: "test.dirty_noop",
    label: "Mark dirty and change nothing (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Marks the project dirty and changes nothing. Registered only in test builds.",
    asks: &[],
    params: &[],
    mutates: true,
    undoable: true,
    enabled: always_in_tests,
    run: |s, _p| {
        s.mark_dirty_for_test();
        Ok(Outcome::new("marked the project dirty and changed nothing"))
    },
};

/// A command that leaves the document in a state the file reader refuses.
///
/// Registered only in test builds. The engine checks the document after every
/// mutating command, because the engine's document and the file's document
/// are the same thing: a plan that cannot be saved is a plan the app cannot
/// open again. A command that breaks one of the file's rules has to be
/// refused and rolled back, and this is the only way to see that happen —
/// every real command keeps the rules, so without this one the check would
/// be a precaution nobody could see work. §50.
#[cfg(test)]
pub static TEST_INVALIDATE: CommandSpec = CommandSpec {
    id: "test.invalidate",
    label: "Invalidate the document (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Leaves the document in a state the file reader refuses. Registered only in test builds.",
    asks: &[],
    params: &[],
    mutates: true,
    undoable: false,
    enabled: always_in_tests,
    run: |s, _p| {
        // An empty range: `Document::validate` refuses a range that is not
        // strictly increasing, so this document cannot be written to a file.
        let meta = &mut s.doc_mut().meta;
        meta.end = meta.start;
        Ok(Outcome::new("the document is now empty"))
    },
};

/// A command that declares that it does not change the document and changes
/// it anyway.
///
/// Registered only in test builds. The engine compares the document with the
/// snapshot it took before the command, and a command that says it does not
/// mutate the document but mutates it is a bug: the change would not be
/// validated, would not be undoable, and would not mark the project dirty —
/// so the design would change and the app would not know that it had. §96.
///
/// No command in the tree does that on purpose, so nothing ever reached the
/// check. This is the same shape as §44's test-only command, on the other
/// side of the flag: there, a command said it changed something and did not;
/// here, a command says it changes nothing and does.
#[cfg(test)]
pub static TEST_WRITE_WHILE_READ_ONLY: CommandSpec = CommandSpec {
    id: "test.write_while_read_only",
    label: "Write while claiming to be read-only (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Changes the document and declares that it does not mutate it. Registered only in test builds.",
    asks: &[],
    params: &[],
    mutates: false,
    undoable: false,
    enabled: always_in_tests,
    run: |s, _p| {
        let node = s.doc_mut().get_mut(craftkit_core::NodeId(2)).ok_or_else(|| EngineError::Param {
            id: TEST_WRITE_WHILE_READ_ONLY.id.to_string(),
            source: craftkit_foundation::ParamError::new("node", "the sample has no node 2"),
        })?;
        node.name = "renamed".to_string();
        Ok(Outcome::new("renamed the node and claimed to be read-only"))
    },
};

/// A command that declares that it does not change the document and leaves it
/// in a state the file reader refuses.
///
/// Registered only in test builds. The check that the document stays a
/// document the reader would accept runs inside the `mutates` branch, so a
/// command that claims not to mutate escapes it — and a design that cannot be
/// written to a file is a design the app cannot open again. §96.
#[cfg(test)]
pub static TEST_INVALIDATE_WHILE_READ_ONLY: CommandSpec = CommandSpec {
    id: "test.invalidate_while_read_only",
    label: "Invalidate while claiming to be read-only (test only)",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "Leaves the document in a state the file reader refuses and declares that it does not mutate it. Registered only in test builds.",
    asks: &[],
    params: &[],
    mutates: false,
    undoable: false,
    enabled: always_in_tests,
    run: |s, _p| {
        // An empty range: `Document::validate` refuses a range that is not
        // strictly increasing, so this document cannot be written to a file.
        let meta = &mut s.doc_mut().meta;
        meta.end = meta.start;
        Ok(Outcome::new("the document is now invalid"))
    },
};

/// The commands that exist only in a test build.
#[cfg(test)]
fn test_only() -> [&'static CommandSpec; 7] {
    [
        &TEST_PANIC,
        &TEST_HALF_APPLY,
        &TEST_NOOP,
        &TEST_DIRTY_NOOP,
        &TEST_INVALIDATE,
        &TEST_WRITE_WHILE_READ_ONLY,
        &TEST_INVALIDATE_WHILE_READ_ONLY,
    ]
}

/// The command a given id resolves to, including the test-only ones.
#[cfg(test)]
pub fn lookup(id: &str) -> Option<&'static CommandSpec> {
    get(id).or_else(|| test_only().into_iter().find(|spec| spec.id == id))
}

/// The command a given id resolves to.
#[cfg(not(test))]
pub fn lookup(id: &str) -> Option<&'static CommandSpec> {
    get(id)
}

/// Every id, sorted.
pub fn ids() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = all().iter().map(|c| c.id).collect();
    v.sort_unstable();
    v
}

/// The full catalogue as JSON, for the docs, the CLI and the MCP tool list.
///
/// This is the shape `docs/agent-playbook.md` and the MCP tools are generated
/// from, so the contract belongs in an assertion rather than in prose. If a
/// command is added without a label, a doc string or a parameter description,
/// this fails — which is the point: the schema is what an agent reads.
///
/// # Examples
///
/// ```
/// use craftkit_engine::catalogue;
///
/// let catalogue = catalogue();
/// let commands = catalogue.as_array().expect("an array");
/// assert!(commands.len() >= 21, "the registry has shrunk");
///
/// for command in commands {
///     // An id is our own dotted, lower-case form: `node.add`, never another
///     // product's spelling.
///     let id = command["id"].as_str().expect("every command has an id");
///     assert!(id.contains('.'), "`{id}` has no area prefix");
///     assert_eq!(*id, id.to_lowercase(), "`{id}` is not lower case");
///     assert!(command["label"].as_str().is_some_and(|l| !l.is_empty()), "`{id}` has no label");
///     assert!(command["doc"].as_str().is_some_and(|d| !d.is_empty()), "`{id}` has no doc string");
///     assert!(command["area"].as_str().is_some_and(|a| !a.is_empty()), "`{id}` has no area");
///
///     // A parameter that does not say what it wants is a parameter an agent
///     // has to guess at, which is the failure this whole registry exists to
///     // avoid.
///     for param in command["params"].as_array().into_iter().flatten() {
///         let name = param["name"].as_str().expect("a parameter has a name");
///         assert!(param["type"].as_str().is_some_and(|t| !t.is_empty()), "{id}.{name} has no type");
///         assert!(param["required"].is_boolean(), "{id}.{name} does not say if it is required");
///         assert!(param["help"].as_str().is_some_and(|h| !h.is_empty()), "{id}.{name} has no help");
///     }
/// }
/// ```
pub fn catalogue() -> Json {
    serde_json::Value::Array(all().iter().map(|c| c.schema()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_catalogue_is_sorted_and_free_of_duplicates() {
        let list = all();
        assert!(!list.is_empty(), "the registry must not be empty");
        for pair in list.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let key = (a.area, a.id);
            let next = (b.area, b.id);
            assert!(
                key < next,
                "commands must be sorted by (area, id): `{}` in {} comes before `{}` in {}",
                a.id,
                a.area.key(),
                b.id,
                b.area.key()
            );
        }
    }

    /// A command that does not change the document cannot be undoable.
    ///
    /// The undo entry is pushed inside the `mutates` branch, so a command that
    /// does not mutate never pushes one — whatever its `undoable` flag says.
    /// The flag is published to agents in the command schema, so a command that
    /// claims it is undoable while it can never be is a claim the catalogue
    /// makes to every reader. Four commands made it. §94.
    #[test]
    fn no_command_that_does_not_mutate_is_undoable() {
        for c in all() {
            assert!(
                c.mutates || !c.undoable,
                "`{}` does not change the document, so it cannot be undoable",
                c.id
            );
        }
    }

    #[test]
    fn every_id_is_dotted_lower_case_and_starts_with_its_area() {
        for c in all() {
            if let Some(prefix) = c.area.prefix() {
                assert!(
                    c.id.starts_with(prefix),
                    "`{}` is in area `{}` and should start with `{prefix}`",
                    c.id,
                    c.area.key()
                );
            }
            assert!(
                c.id.chars().all(|ch| ch.is_ascii_lowercase() || ch == '.'),
                "`{}` must be dotted lower case",
                c.id
            );
            assert!(!c.id.ends_with('.'), "`{}` has a trailing dot", c.id);
            assert!(c.label.chars().next().is_some_and(|ch| ch.is_uppercase()), "`{}` label", c.id);
            assert!(!c.doc.is_empty(), "`{}` has no documentation", c.id);
            assert!(!c.icon.0.is_empty(), "`{}` has no icon", c.id);
        }
    }

    #[test]
    fn every_parameter_is_documented_and_uniquely_named() {
        for c in all() {
            let mut seen: Vec<&str> = Vec::new();
            for p in c.params {
                assert!(!p.help.is_empty(), "`{}.{}` has no help text", c.id, p.name);
                assert!(!seen.contains(&p.name), "`{}.{}` is declared twice", c.id, p.name);
                seen.push(p.name);
            }
            // The first parameter is the *subject* of the command, so if a
            // command needs anything it needs its subject. Later ones may also
            // be needed — `key.set` genuinely needs a path and a value as well
            // as a node.
            //
            // A subject with an alternative needs **one of its two
            // spellings**, not this one: `node` and `node_name` are one
            // parameter. §116.
            let needed = |p: &ParamDoc| p.required || p.alternative.is_some();
            if c.params.iter().any(needed) {
                assert!(
                    c.params.first().is_some_and(needed),
                    "`{}`'s first parameter should be needed",
                    c.id
                );
            }
        }
    }

    /// An alternative is a parameter of the same command, and it is not
    /// needed on its own.
    ///
    /// §116: the pair is "one of the two". A parameter that is *both* an
    /// alternative and required would say two things at once, and an
    /// alternative that names a parameter the command does not have would be
    /// a name nothing resolves.
    #[test]
    fn an_alternative_is_a_parameter_of_the_same_command() {
        for c in all() {
            for p in c.params {
                let Some(alternative) = p.alternative else { continue };
                let other = c.params.iter().find(|q| q.name == alternative).unwrap_or_else(|| {
                    panic!(
                        "`{}`'s parameter `{}` names an alternative `{}` the command does not have",
                        c.id, p.name, alternative
                    )
                });
                assert!(!other.required, "`{}.{}` is required as well as an alternative", c.id, other.name);
                assert!(!p.required, "`{}.{}` is required and has an alternative", c.id, p.name);
            }
        }
    }

    #[test]
    fn a_signature_names_every_parameter() {
        let s = get("node.add").expect("registered").signature();
        assert!(s.contains("node.add("), "{s}");
        assert!(s.contains("kind"), "{s}");
        assert!(s.contains("name"), "{s}");
    }

    #[test]
    fn the_schema_is_valid_json_shaped_for_an_mcp_tool() {
        let json = catalogue();
        let arr = json.as_array().expect("an array");
        assert_eq!(arr.len(), all().len());
        let first = &arr[0];
        assert!(first.get("id").and_then(Json::as_str).is_some());
        assert!(first.get("params").and_then(Json::as_array).is_some());
    }

    /// The catalogue says which parameter may be given instead of which.
    ///
    /// §116: the catalogue marked the first spelling as required, so the
    /// schema built from it told a client it could not use the second one —
    /// which the command accepts. The pair has to be in the catalogue, or
    /// the schema cannot say "one of the two".
    #[test]
    fn the_catalogue_says_which_parameter_may_be_given_instead() {
        let json = catalogue();
        let arr = json.as_array().expect("an array");
        let mut pairs = 0;
        for spec in arr {
            let params = spec["params"].as_array().expect("params");
            for p in params {
                let Some(alternative) = p["alternative"].as_str() else { continue };
                pairs += 1;
                assert!(
                    !p["required"].as_bool().unwrap_or(true),
                    "`{}`'s parameter `{}` is required and has an alternative",
                    spec["id"],
                    p["name"]
                );
                assert!(
                    params.iter().any(|q| q["name"] == json!(alternative)),
                    "`{}`'s parameter `{}` names an alternative the command does not have",
                    spec["id"],
                    p["name"]
                );
            }
        }
        assert!(pairs > 0, "no command in the tree has a parameter with an alternative");
        // The two pairs the tree has, spelled out.
        assert_eq!(pairs, 14, "the tree's pairs: node/node_name thirteen times, seconds/frame once");
    }

    #[test]
    fn looking_up_an_unknown_id_is_none_not_a_panic() {
        assert!(get("no.such.command").is_none());
        assert!(get("").is_none());
    }

    /// The advertised chords are unique and well-formed.
    ///
    /// The tree binds no keys yet — there is no shortcut parser and no key
    /// handling in the UI — so a chord is a *specification*. A specification
    /// with two commands on one chord, or a chord nobody could parse, is a
    /// specification nobody could implement, and nothing else in the tree
    /// would notice: the chords are only read to be repeated in an MCP tool
    /// description.
    ///
    /// §83 found the shape of the claim ("in the form the UI's shortcut
    /// parser understands") pointing at a parser that does not exist.
    #[test]
    fn the_advertised_shortcuts_are_unique_and_well_formed() {
        let mut seen: Vec<(&'static str, &'static str)> = Vec::new();
        for spec in all() {
            let Some(chord) = spec.shortcut else { continue };
            assert!(!chord.is_empty(), "`{}` advertises an empty chord", spec.id);
            assert!(
                chord.chars().all(|c| c.is_alphanumeric() || c == '+'),
                "`{chord}` ({}) has a character that is not part of a key name",
                spec.id
            );
            let parts: Vec<&str> = chord.split('+').collect();
            let (mods, key) = parts.split_at(parts.len().saturating_sub(1));
            assert!(
                key[0].chars().all(|c| c.is_alphanumeric()),
                "`{chord}` ({}) does not end in a key name",
                spec.id
            );
            assert!(
                mods.iter().all(|m| *m == "Ctrl" || *m == "Shift"),
                "`{chord}` ({}) has a modifier that is not Ctrl or Shift",
                spec.id
            );
            assert!(
                mods.windows(2).all(|w| w[0] == "Ctrl"),
                "`{chord}` ({}) puts Shift before Ctrl",
                spec.id
            );
            if let Some((other, _)) = seen.iter().find(|(_, c)| *c == chord) {
                panic!("`{chord}` is advertised by both {other} and {}", spec.id);
            }
            seen.push((spec.id, chord));
        }
    }

    /// Every name a command asks for is a parameter of that command.
    ///
    /// The interactive path asks for the parameters by name, and a name that
    /// is not a parameter would be a field that fills nothing. §118.
    #[test]
    fn every_name_a_command_asks_for_is_a_parameter_of_that_command() {
        for spec in all() {
            for name in spec.asks {
                assert!(
                    spec.params.iter().any(|p| p.name == *name),
                    "`{}` asks for `{name}`, which is not one of its parameters",
                    spec.id
                );
            }
        }
    }

    /// A command is not asked for the same parameter twice.
    ///
    /// Two fields for one parameter would be two answers to one question,
    /// and only one of them would be read. §118.
    #[test]
    fn a_command_never_asks_for_the_same_parameter_twice() {
        for spec in all() {
            let mut seen: Vec<&str> = Vec::new();
            for name in spec.asks {
                assert!(!seen.contains(name), "`{}` asks for `{name}` twice", spec.id);
                seen.push(name);
            }
        }
    }

    /// Every parameter a command cannot do without is one it asks for.
    ///
    /// A required parameter that the form does not ask for is a parameter
    /// the person cannot supply, and the command would be refused for a
    /// reason the form never mentioned. §118.
    #[test]
    fn every_parameter_a_command_needs_is_one_it_asks_for() {
        for spec in all() {
            for param in spec.params {
                if !param.needed() {
                    continue;
                }
                assert!(
                    spec.asks.contains(&param.name),
                    "`{}` needs `{}` but does not ask for it",
                    spec.id,
                    param.name
                );
            }
        }
    }

    /// A command that asks for nothing is a command that runs when it is
    /// clicked.
    ///
    /// The other half of the rule: a command that asks for parameters has a
    /// form, and the form is the only way it is started. §118.
    #[test]
    fn a_command_that_asks_for_nothing_has_no_form() {
        for spec in all() {
            if spec.asks.is_empty() {
                assert!(spec.ask_params().is_empty(), "`{}` asks for nothing but has parameters", spec.id);
            }
        }
    }

    /// The catalogue says which pairs one of must be given.
    ///
    /// §120: two entries that both say `required: false` do not say that the
    /// command needs one of them — they say the opposite, and a reader of
    /// the catalogue is entitled to believe them. The MCP's JSON Schema said
    /// so; the catalogue's own JSON and the plain listing did not.
    #[test]
    fn the_catalogue_says_which_pairs_one_of_must_be_given() {
        let mut pairs = 0;
        for spec in all() {
            for group in spec.one_of_groups() {
                assert_eq!(group.len(), 2, "a group of one is not a pair: {group:?}");
                for name in &group {
                    let param = spec
                        .params
                        .iter()
                        .find(|p| p.name == *name)
                        .unwrap_or_else(|| panic!("`{name}` is not a parameter of {}", spec.id));
                    assert!(
                        !param.required,
                        "`{name}` ({}) is required on its own, so it is not a pair",
                        spec.id
                    );
                }
                pairs += 1;
            }
            // The schema says the same thing the command does.
            let schema = spec.schema();
            assert_eq!(
                schema["one_of"].as_array().map(Vec::len),
                Some(spec.one_of_groups().len()),
                "{}",
                spec.id
            );
        }
        assert!(pairs >= 13, "only {pairs} pair(s) in the catalogue");
    }

    /// A command that needs one of a pair refuses with neither.
    ///
    /// The catalogue's claim has to match what the command does: the pair is
    /// required, so a call with neither is refused. §120.
    #[test]
    fn a_command_that_needs_one_of_a_pair_refuses_with_neither() {
        let mut s = Session::sample();
        let err = s.execute("node.mute", &serde_json::json!({})).expect_err("neither");
        let message = err.to_string();
        assert!(message.contains("node"), "{message}");
        assert!(message.contains("node_name"), "{message}");
    }
}
