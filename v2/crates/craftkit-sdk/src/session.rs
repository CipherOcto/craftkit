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
//!
//! # The history is a tree
//!
//! The history is not a stack: it is a **tree** of states, and a state can
//! have several children. Undo walks to the parent, redo walks to a child,
//! and a change made after an undo starts a **branch** instead of throwing
//! the old work away. See [`crate::history`].

use serde_json::Value as Json;

use crate::camera::Camera;
use crate::command::CommandSpec;
use crate::contribution::Registry;
use crate::document::{Document, NodeKindSpec};
use crate::error::{EngineError, Outcome};
use crate::evaluator::EvaluatorSpec;
use crate::format::{FormatSpec, MigrationSpec};
use crate::history::History;
use crate::journal::JournalEntry;
use crate::params::Params;
use crate::plan::{self, Plan};
use crate::plugins::PluginLibraries;
use crate::select::Selected;
use crate::tasks::TaskRegistry;

/// The document, the commands, and the history of what happened.
#[derive(Debug, Clone)]
pub struct Session {
    commands: Registry<CommandSpec>,
    kinds: Registry<NodeKindSpec>,
    formats: Registry<FormatSpec>,
    migrations: Registry<MigrationSpec>,
    evaluators: Registry<EvaluatorSpec>,
    document: Document,
    selection: Vec<Selected>,
    history: History,
    log: Vec<String>,
    journal: Vec<JournalEntry>,
    /// The tool in the hand, if any.
    tool: Option<String>,
    /// The tasks: work that may outlive the process.
    tasks: TaskRegistry,
    /// The camera: what is on screen.
    camera: Camera,
    /// The plugin libraries this session has open.
    plugins: PluginLibraries,
    depth: usize,
    /// How many states the history has been given.
    ///
    /// A command that runs a batch makes its own state, and the command
    /// itself must not make a second one on top of it: a replay is one
    /// undo step, not two.
    states_made: usize,
    /// The command that is running, if any.
    running: Option<String>,
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
            history: History::new(Document::new("untitled")),
            log: Vec::new(),
            journal: Vec::new(),
            tool: None,
            tasks: TaskRegistry::new(),
            camera: Camera::new(),
            plugins: PluginLibraries::new(),
            depth: 0,
            states_made: 0,
            running: None,
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
    ///
    /// A selected thing is a node, and it may be picked out whole or in
    /// part: see [`Selected`].
    pub fn selection(&self) -> &[Selected] {
        &self.selection
    }

    /// Select these things.
    ///
    /// A node that is not in the document is not selected: the selection
    /// is of the document, and the document is the truth.
    pub fn select(&mut self, items: &[Selected]) {
        self.selection =
            items.iter().filter(|item| self.document.node(item.node).is_some()).cloned().collect();
    }

    /// Select these nodes, whole.
    pub fn select_ids(&mut self, ids: &[u64]) {
        let items: Vec<Selected> = ids.iter().map(|id| Selected::node(*id)).collect();
        self.select(&items);
    }

    /// Select nothing.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// The ids of the selected nodes, whole or in part.
    ///
    /// A command that wants whole nodes asks for this, and a command
    /// that can work on a part asks for the selection itself.
    pub fn selected_ids(&self) -> Vec<u64> {
        self.selection.iter().map(|item| item.node).collect()
    }

    /// The ids of the selected nodes that are selected **whole**.
    pub fn selected_whole_ids(&self) -> Vec<u64> {
        self.selection.iter().filter(|item| item.is_whole()).map(|item| item.node).collect()
    }

    /// The nodes that are selected, in the order they are in the document.
    pub fn selected_nodes(&self) -> Vec<&crate::document::Node> {
        self.document.walk().filter(|node| self.selected_ids().contains(&node.id)).collect()
    }

    /// The camera: what is on screen, and how much of it.
    ///
    /// It is the session's state, the way the selection and the tool are:
    /// a camera is something a person is looking through, not something
    /// the document holds.
    ///
    /// It is **not** part of what the undo restores: scrolling is not an
    /// edit, and an undo that scrolls the view is a surprise.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// The camera, to change.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// The tasks: work that may outlive the process.
    ///
    /// They are the session's state, the way the selection and the tool
    /// are: a task is something the program is doing, not something the
    /// document holds. A host that wants them to travel with the document
    /// writes them into the payload; the registry is JSON, and reading it
    /// back is a command.
    pub fn tasks(&self) -> &TaskRegistry {
        &self.tasks
    }

    /// The tasks, to change.
    pub fn tasks_mut(&mut self) -> &mut TaskRegistry {
        &mut self.tasks
    }

    /// The **plugin libraries** this session has open.
    ///
    /// They are the session's state, the way the tasks are: a library is
    /// a program the session has open, not something the document
    /// holds.
    pub fn plugins(&self) -> &PluginLibraries {
        &self.plugins
    }

    /// The plugin libraries, to change.
    pub fn plugins_mut(&mut self) -> &mut PluginLibraries {
        &mut self.plugins
    }

    /// The tool in the hand, if any.
    ///
    /// It is the session's state, the way the selection is: a tool is
    /// something a person is holding, not something the document holds.
    pub fn tool(&self) -> Option<&str> {
        self.tool.as_deref()
    }

    /// Put a tool in the hand.
    pub fn hold(&mut self, tool: impl Into<String>) {
        self.tool = Some(tool.into());
    }

    /// Take the tool out of the hand.
    pub fn release(&mut self) {
        self.tool = None;
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

    /// The history: a **tree** of states.
    ///
    /// It is not a stack: a state can have several children, and undoing
    /// and then changing something makes a branch instead of throwing the
    /// old work away. See [`crate::history`].
    pub fn history(&self) -> &History {
        &self.history
    }

    /// The history, to change.
    ///
    /// The commands that move along the tree use it, and nothing else
    /// does: the tree is the session's own business.
    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    /// Put the session back into a state of the history.
    ///
    /// The document, the selection and the tool all go back: a person who
    /// undoes an edit expects the selection to be what it was.
    pub fn restore_state(&mut self, id: u64) {
        let Some(node) = self.history.node(id) else { return };
        let document = node.document.clone();
        let selection = node.selection.clone();
        let tool = node.tool.clone();
        self.document = document;
        self.selection = selection;
        self.tool = tool;
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
        let tool_snapshot = self.tool.clone();
        let states_before = self.states_made;
        self.running = Some(id.to_string());
        let result = self.run_guarded(&spec, &params);
        self.running = None;
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
                if changed {
                    if spec.undoable && self.depth == 0 && self.states_made == states_before {
                        // The state the command came from keeps the
                        // selection and the tool as they were **before**
                        // it, and the new state holds the document as it
                        // is now.
                        self.history.set_current_selection(&selection_snapshot, &tool_snapshot);
                        self.history.push(
                            id,
                            self.document.clone(),
                            self.selection.clone(),
                            self.tool.clone(),
                        );
                    } else {
                        // The document changed and no new state was made:
                        // the current state is now this document.
                        self.history.set_current(
                            self.document.clone(),
                            self.selection.clone(),
                            self.tool.clone(),
                        );
                    }
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
                self.tool = tool_snapshot;
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
        let tool_snapshot = self.tool.clone();
        let depth = self.depth;
        self.depth = depth + 1;
        let result = body(self);
        self.depth = depth;
        match result {
            Ok(value) => {
                if self.document != snapshot {
                    // A batch that changed the document is one state: the
                    // state it came from keeps the document as it was, and
                    // the new state holds the document as it is now.
                    self.history.set_current(snapshot.clone(), selection_snapshot, tool_snapshot);
                    let name = self.running.clone().unwrap_or_else(|| "batch".to_string());
                    self.states_made += 1;
                    self.history.push(
                        &name,
                        self.document.clone(),
                        self.selection.clone(),
                        self.tool.clone(),
                    );
                }
                Ok(value)
            }
            Err(error) => {
                self.history.set_current(snapshot.clone(), selection_snapshot.clone(), tool_snapshot.clone());
                self.document = snapshot;
                self.selection = selection_snapshot;
                self.tool = tool_snapshot;
                Err(error)
            }
        }
    }

    /// Put the document back one step.
    ///
    /// It is a move along the tree of states: to the parent of the state
    /// that is current. The branch that was left is still there.
    pub fn undo(&mut self) -> Result<Outcome, EngineError> {
        crate::history::undo(self)
    }

    /// Go forward into a branch of the history.
    ///
    /// Without a branch, the newest one is taken.
    pub fn redo(&mut self, branch: Option<u64>) -> Result<Outcome, EngineError> {
        crate::history::redo(self, branch)
    }
}
