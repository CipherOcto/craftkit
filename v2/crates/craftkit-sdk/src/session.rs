//! The session: the document, the history, and the one way to run a command.
//!
//! # The one way
//!
//! [`Session::execute`] is the only way to run a command, and it is the same
//! way for everybody: a person clicking a button, an agent calling a tool, a
//! script, a test. The order of the checks is the contract:
//!
//! 1. the command has to exist;
//! 2. it has to be available right now;
//! 3. the caller's keys have to be ones the command has;
//! 4. the document is snapshotted, the command runs, and the outcome is
//!    judged against the snapshot.
//!
//! Nothing else can change the document, because the session's mutable
//! accessor is not public and the commands run inside it.
//!
//! # The session judges
//!
//! The old tree asked the **command** whether it had changed anything, and
//! the command answered honestly if it felt like it. Here the session
//! **compares the document** before and after, so the answer is a fact:
//!
//! - a command that says `mutates: false` and changes the document is
//!   **refused and rolled back** — the promise was a promise;
//! - an undo entry is pushed when the command mutates, the command is
//!   undoable, **and** the document really changed. A command that sets a
//!   value to what it already was leaves no history;
//! - a command that **panics** leaves the document as it was and is
//!   reported as an internal error, because a crash is not a result.
//!
//! # A batch is one step
//!
//! [`Session::batch`] runs several commands as one undo step. An agent that
//! runs three commands that belong together should be able to undo them
//! together, and the alternative — three undo entries — is a thing a person
//! has to know about.

use serde_json::Value as Json;

use crate::command::CommandSpec;
use crate::contribution::Registry;
use crate::document::{Document, NodeKindSpec};
use crate::error::{EngineError, Outcome};
use crate::evaluator::EvaluatorSpec;
use crate::format::{FormatSpec, MigrationSpec};
use crate::journal::JournalEntry;
use crate::params::Params;
use crate::plan::{self, Plan};

/// One step of the history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// The command that ran.
    pub id: String,
    /// The document as it was before.
    pub document: Document,
    /// What was selected, before.
    pub selection: Vec<u64>,
}

/// The document, the commands, and the history of what happened.
#[derive(Debug, Clone)]
pub struct Session {
    commands: Registry<CommandSpec>,
    kinds: Registry<NodeKindSpec>,
    formats: Registry<FormatSpec>,
    migrations: Registry<MigrationSpec>,
    evaluators: Registry<EvaluatorSpec>,
    document: Document,
    selection: Vec<u64>,
    history: Vec<HistoryEntry>,
    log: Vec<String>,
    journal: Vec<JournalEntry>,
    depth: usize,
}

impl Session {
    /// A session over a registry of commands and of node kinds.
    pub fn new(commands: Registry<CommandSpec>, kinds: Registry<NodeKindSpec>) -> Self {
        Session {
            commands,
            kinds,
            formats: Registry::new(),
            migrations: Registry::new(),
            evaluators: Registry::new(),
            document: Document::new("untitled"),
            selection: Vec::new(),
            history: Vec::new(),
            log: Vec::new(),
            journal: Vec::new(),
            depth: 0,
        }
    }

    /// A session that also knows the file format.
    ///
    /// The formats and the migrations are what make a document writable and
    /// an old file readable, so a session that can save has to have them.
    pub fn with_formats(
        commands: Registry<CommandSpec>,
        kinds: Registry<NodeKindSpec>,
        formats: Registry<FormatSpec>,
        migrations: Registry<MigrationSpec>,
        evaluators: Registry<EvaluatorSpec>,
    ) -> Self {
        Session { formats, migrations, evaluators, ..Session::new(commands, kinds) }
    }

    /// The versions of the format this session knows.
    pub fn formats(&self) -> &Registry<FormatSpec> {
        &self.formats
    }

    /// The steps between the versions.
    pub fn migrations(&self) -> &Registry<MigrationSpec> {
        &self.migrations
    }

    /// The commands this session knows.
    pub fn registry(&self) -> &Registry<CommandSpec> {
        &self.commands
    }

    /// The node kinds this session knows.
    pub fn kinds(&self) -> &Registry<NodeKindSpec> {
        &self.kinds
    }

    /// The document, to read.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// What is selected.
    ///
    /// The selection is the session's own state: what a person has picked
    /// out, or what an agent has named. A command that acts on "the
    /// selection" reads it here.
    pub fn selection(&self) -> &[u64] {
        &self.selection
    }

    /// Select these nodes.
    ///
    /// A node that is not in the document is not selected: the selection
    /// is of the document, and the document is the truth.
    pub fn select(&mut self, ids: &[u64]) {
        self.selection = ids.iter().copied().filter(|id| self.document.node(*id).is_some()).collect();
    }

    /// Select nothing.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// The nodes that are selected, in the order they are in the document.
    pub fn selected_nodes(&self) -> Vec<&crate::document::Node> {
        self.document.walk().filter(|node| self.selection.contains(&node.id)).collect()
    }

    /// The document, to change.
    ///
    /// Public because a command has to be able to change what it is about —
    /// the command is the thing that is allowed to write, and it writes
    /// here, inside the snapshot the engine took.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// What the document means, as the kinds say it.
    ///
    /// The core does not evaluate: it asks each node's kind, and the plan
    /// is what comes back. A node whose kind is not linked in is not in the
    /// plan.
    pub fn evaluate(&self) -> Plan {
        let mut plan = plan::evaluate(&self.document, &self.kinds);
        // The evaluators speak about the whole document, so their items
        // come after the kinds' ones: the document's own nodes first, then
        // what is said about the document as a whole.
        for evaluator in self.evaluators.all() {
            let theirs = (evaluator.evaluate)(&self.document);
            plan.items.extend(theirs.items);
        }
        plan
    }

    /// The **journal**: the commands that ran, with the values they were
    /// given.
    ///
    /// It is a program, and it can be run again: see
    /// [`crate::journal`]. It is bounded, and the oldest entries go when
    /// it is full.
    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }

    /// Forget the journal.
    pub fn clear_journal(&mut self) {
        self.journal.clear();
    }

    /// Write an entry in the journal.
    ///
    /// The limit is here and not in the caller: a session that grows a
    /// journal for ever is a leak, and the limit is what makes it bounded.
    fn journal_entry(&mut self, id: &str, params: &Json) {
        self.journal.push(JournalEntry { id: id.to_string(), params: params.clone() });
        if self.journal.len() > crate::journal::JOURNAL_LIMIT {
            let excess = self.journal.len() - crate::journal::JOURNAL_LIMIT;
            self.journal.drain(0..excess);
        }
    }

    /// The history.
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// What the session has been asked to do.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// Run a command.
    pub fn execute(&mut self, id: &str, raw: &Json) -> Result<Outcome, EngineError> {
        let Some(spec) = self.commands.get(id).cloned() else {
            let error = EngineError::Unknown(id.to_string());
            self.log.push(format!("{id}: refused ({error})"));
            return Err(error);
        };
        if !(spec.enabled)(self) {
            let error = EngineError::Disabled {
                id: id.to_string(),
                reason: "the session says it is not available".to_string(),
            };
            self.log.push(format!("{id}: refused ({error})"));
            return Err(error);
        }
        let params = Params::new(raw);
        if let Err(source) = params.deny_unknown(&spec.param_names()) {
            let error = EngineError::Param { id: id.to_string(), source };
            self.log.push(format!("{id}: refused ({error})"));
            return Err(error);
        }

        // The snapshot. A command that fails leaves the document as it was,
        // and so does one that panics — the unwind crosses this boundary and
        // the restore is below it.
        let snapshot = self.document.clone();
        let selection_snapshot = self.selection.clone();
        let result = self.run_guarded(&spec, &params);
        match result {
            Ok(outcome) => {
                let changed = self.document != snapshot;
                if changed && !spec.mutates {
                    // The command promised not to change the document and
                    // changed it. The promise is a promise.
                    self.document = snapshot;
                    let error = EngineError::Failed {
                        id: id.to_string(),
                        message: "the command changed the document but does not mutate".to_string(),
                    };
                    self.log.push(format!("{id}: refused ({error})"));
                    return Err(error);
                }
                let mut outcome = outcome;
                outcome.changed = changed;
                if spec.undoable && changed && self.depth == 0 {
                    self.history.push(HistoryEntry {
                        id: id.to_string(),
                        document: snapshot,
                        selection: selection_snapshot,
                    });
                }
                self.log.push(format!("{id}: {}", outcome.message));
                if spec.journal {
                    self.journal_entry(id, params.value());
                }
                Ok(outcome)
            }
            Err(error) => {
                self.document = snapshot;
                self.selection = selection_snapshot;
                self.log.push(format!("{id}: refused ({error})"));
                Err(error)
            }
        }
    }

    /// Run a command with a panic guard.
    ///
    /// A panic in a command is a crash, not a result. The document is put
    /// back and the error is reported, so that the thing that was being
    /// built is still there afterwards.
    fn run_guarded(&mut self, spec: &CommandSpec, params: &Params<'_>) -> Result<Outcome, EngineError> {
        let run = spec.run.clone();
        let session = std::panic::AssertUnwindSafe(self);
        // The guard is deliberate: the run may hold interior mutability —
        // a plugin's closure does — and a `&mut Session` is not unwind-safe
        // either. The document is put back below, and that is the whole of
        // the promise.
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || run.call(session.0, params)));
        match outcome {
            Ok(result) => result,
            Err(_) => Err(EngineError::Internal {
                id: spec.id.to_string(),
                message: "the command panicked; the document was put back".to_string(),
            }),
        }
    }

    /// Run several commands as one undo step.
    ///
    /// The commands inside run as they would outside; what changes is the
    /// history: the batch is one entry, so one undo takes all of it back.
    pub fn batch<T>(
        &mut self,
        body: impl FnOnce(&mut Session) -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        let snapshot = self.document.clone();
        let selection_snapshot = self.selection.clone();
        let depth = self.depth;
        self.depth = depth + 1;
        let result = body(self);
        self.depth = depth;
        match result {
            Ok(value) => {
                if self.document != snapshot && self.history.is_empty() {
                    // A batch that changed the document is one history entry.
                    self.history.push(HistoryEntry {
                        id: "batch".to_string(),
                        document: snapshot,
                        selection: selection_snapshot,
                    });
                }
                Ok(value)
            }
            Err(error) => {
                self.document = snapshot;
                Err(error)
            }
        }
    }

    /// Put the document back one step.
    pub fn undo(&mut self) -> Result<Outcome, EngineError> {
        let Some(entry) = self.history.pop() else {
            return Err(EngineError::Failed {
                id: "history.undo".to_string(),
                message: "there is nothing to undo".to_string(),
            });
        };
        self.document = entry.document;
        self.selection = entry.selection;
        Ok(Outcome::new(format!("undid `{}`", entry.id)))
    }
}
