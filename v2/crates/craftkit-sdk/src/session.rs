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
use crate::prompt::Question;
use crate::resume::Pending;
use crate::select::Selected;
use crate::tasks::TaskRegistry;

/// How many lines the session's log holds.
///
/// The log is what a person reads to see what the session did, and it is
/// the one part of the session that is written on every command. A
/// number rather than a rule, but a bound all the same: a session that
/// runs for days would otherwise hold every line it ever wrote.
pub const MAX_LOG_ENTRIES: usize = 256;

/// How many documents a session holds at once.
///
/// The document being edited, the snapshot a command takes before it
/// runs, and the history's own budget — which is two documents' worth,
/// because the family says it is (§ the history's budget).
///
/// The number is the **family's**, and the name is the family's, so that
/// a reader of either tree can look it up.
pub const SESSION_OWN_DOCUMENTS: usize = 2;

/// The snapshot a command takes before it edits anything.
///
/// It is taken for every command, and it exists **alongside** the
/// document it was taken from: a session holds one more document while
/// a command runs than it does between them, and a budget that counts
/// only the resting state is a budget that every command exceeds.
pub const SESSION_SNAPSHOT_DOCUMENTS: usize = 1;

/// How many documents a session holds at once.
pub const SESSION_DOCUMENTS: usize =
    SESSION_OWN_DOCUMENTS + SESSION_SNAPSHOT_DOCUMENTS + crate::history::HISTORY_EXPANSION;

/// How much memory the documents a session holds may take.
///
/// This is the number that says what the **process** uses — which is what
/// the document's own budget used to claim for itself, and was wrong by
/// a factor of five.
///
/// It is the document's budget times the number of documents a session
/// holds, because every document it holds is bounded by that budget and
/// the history is bounded by its own.
pub const MAX_SESSION_BYTES: usize = crate::document::MAX_DOCUMENT_BYTES * SESSION_DOCUMENTS;

/// What a session is holding, in bytes.
///
/// It is the **parts**, and not one number: a session that is over its
/// budget has to give something up, and the part that gives way is the
/// part that is cheapest to lose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    /// The document being edited.
    pub document: usize,
    /// The history: every state it holds.
    pub history: usize,
    /// The journal: every entry it holds.
    pub journal: usize,
    /// The tasks: every task it holds.
    pub tasks: usize,
    /// The log: every line it holds.
    pub log: usize,
}

impl Memory {
    /// What all of it comes to.
    pub fn total(&self) -> usize {
        self.document + self.history + self.journal + self.tasks + self.log
    }
}

/// How wide the **resize handle** is, in pixels of the screen.
///
/// It is a thing on the screen and not a thing in the world: it is the
/// same size whatever the zoom.
pub const HANDLE_PIXELS: f64 = 8.0;

/// The colour of a **picked-out part**: a wash.
///
/// The part is marked with a wash and not with a frame, because a part
/// is not a thing that can be outlined and dragged: it is a piece of
/// something else. The wash is **thin**, so the thing under it still
/// shows through.
const SELECTION_WASH: crate::document::Colour = crate::document::Colour([0.0, 0.0, 0.0, 0.18]);

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
    /// What the journal costs, in bytes.
    ///
    /// It is kept rather than computed, so that the oldest entry can be
    /// dropped by subtracting what it cost.
    journal_bytes: usize,
    /// How much memory the journal may hold.
    journal_budget: usize,
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
    /// The command that is **waiting** for an answer, if any.
    ///
    /// It is the session's own state, the way the selection and the tool
    /// are: a command that is waiting is not in the document, and the
    /// document is as it was. See [`crate::resume`].
    pending: Option<Pending>,
    /// How much memory the session may hold.
    memory_budget: usize,
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
            journal_bytes: 0,
            journal_budget: crate::journal::MAX_JOURNAL_BYTES,
            tool: None,
            tasks: TaskRegistry::new(),
            camera: Camera::new(),
            plugins: PluginLibraries::new(),
            depth: 0,
            states_made: 0,
            running: None,
            pending: None,
            memory_budget: MAX_SESSION_BYTES,
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

    /// Where a node **is**.
    ///
    /// The kind says where its node is, if it says anything at all: a
    /// clip in a sound document is at a **time**, and the rectangle the
    /// document would hold for it would go stale the moment the track
    /// above it moved. A kind that says nothing has the place the
    /// document gives it.
    pub fn place_of(&self, node: &crate::document::Node) -> Option<crate::document::Rect> {
        let Some(kind) = self.kinds.get(&node.kind) else { return node.rect };
        match kind.place {
            Some(place) => place(node, &self.document),
            None => node.rect,
        }
    }

    /// Where the **part** of a node is.
    ///
    /// A selection may hold a part of a node, and a part is in the
    /// node's **own units**: samples for a clip, characters for a
    /// paragraph. Only the kind knows what those units are, so the kind
    /// is asked — the same bargain the place is asked under.
    ///
    /// A part may be in more than one piece: a range of characters that
    /// crosses a line is two pieces. A kind that says nothing has no
    /// part, and the list is empty.
    pub fn parts_of(&self, node: &crate::document::Node, from: f64, to: f64) -> Vec<crate::document::Rect> {
        let Some(kind) = self.kinds.get(&node.kind) else { return Vec::new() };
        let Some(part) = kind.part else { return Vec::new() };
        part(node, &self.document, from, to).unwrap_or_default()
    }

    /// The **bounds** of the document: the place that frames all of it.
    ///
    /// It asks the kinds where their nodes are, so that a document whose
    /// geometry is derived — a timeline, a diagram — is framed by what
    /// is really there and not by what the document happens to hold.
    pub fn bounds(&self) -> Option<crate::document::Rect> {
        let mut bounds: Option<crate::document::Rect> = None;
        for node in self.document.walk_visible() {
            let Some(rect) = self.place_of(node) else { continue };
            bounds = Some(match bounds {
                None => rect,
                Some(current) => crate::document::Rect::new(
                    current.x.min(rect.x),
                    current.y.min(rect.y),
                    (current.x + current.w).max(rect.x + rect.w) - current.x.min(rect.x),
                    (current.y + current.h).max(rect.y + rect.h) - current.y.min(rect.y),
                ),
            });
        }
        bounds
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

    /// **Draw** the document.
    ///
    /// The kit draws it and not the front end: every node that is not
    /// hidden is offered to the kind that says what it is, and the
    /// **resize handle** of the selected node is drawn on top.
    ///
    /// A front end that drew the document itself would have a second
    /// rule for what is seen, and the second rule would drift from the
    /// first — the way the CAD's colour rule drifted from the
    /// appearance rule until §103 joined them.
    ///
    /// The handle is a **screen** size: it is the same size whatever
    /// the zoom, and how big it is in the world's units comes from the
    /// camera.
    pub fn paint(&self, canvas: &mut dyn crate::document::Canvas) {
        let document = self.document();
        for node in document.walk_visible() {
            let Some(kind) = self.kinds.get(&node.kind) else { continue };
            (kind.paint)(node, document, canvas);
        }
    }

    /// **Draw** the document **and the chrome**.
    ///
    /// The chrome is the **handle** of the selected node: a thing that
    /// belongs to the view and not to the document. So a picture of the
    /// document — a snapshot for a file — is drawn with
    /// [`Session::paint`], and the picture on the screen is drawn with
    /// this one.
    pub fn paint_view(&self, canvas: &mut dyn crate::document::Canvas) {
        self.paint(canvas);
        self.paint_handles(canvas);
    }

    /// The **chrome** of the selection: its frame and its handles.
    ///
    /// The frame says what is selected, and the handle says that it
    /// can be resized. Both are **chrome**: they belong to the view
    /// and not to the document, so a picture of the document — a
    /// snapshot for a file — has neither.
    ///
    /// A selection may hold a **part** of a node, and a part is marked
    /// differently: the kind says where the part is, and the part is
    /// washed over. A part cannot be resized, so it has no handle.
    ///
    /// The line is a **screen** size, like the handle: one pixel
    /// across, whatever the zoom.
    fn paint_handles(&self, canvas: &mut dyn crate::document::Canvas) {
        let zoom = self.camera().zoom();
        let size = HANDLE_PIXELS / zoom;
        let width = 1.0 / zoom;
        for item in self.selection() {
            let Some(node) = self.document.node(item.node) else { continue };
            // A **part** that is picked out: the kind says where it is.
            if let (Some(from), Some(to)) = (item.from, item.to) {
                let parts = self.parts_of(node, from, to);
                if !parts.is_empty() {
                    for rect in parts {
                        canvas.rect(rect, SELECTION_WASH);
                    }
                    continue;
                }
                // The kind cannot place the part, so the node's frame is
                // what says that something is picked out.
            }
            let Some(rect) = self.place_of(node) else { continue };
            let frame = crate::document::Path::polygon(&[
                crate::document::Point::new(rect.x, rect.y),
                crate::document::Point::new(rect.x + rect.w, rect.y),
                crate::document::Point::new(rect.x + rect.w, rect.y + rect.h),
                crate::document::Point::new(rect.x, rect.y + rect.h),
            ]);
            canvas.path(
                &frame,
                crate::document::Colour::NONE,
                Some(crate::document::Stroke { colour: crate::document::Colour::BLACK, width }),
            );
            let handle = rect.resize_handle();
            canvas.rect(
                crate::document::Rect::new(handle.x - size / 2.0, handle.y - size / 2.0, size, size),
                crate::document::Colour::BLACK,
            );
        }
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
    ///
    /// **And only a command may have it.** The session knows whether a
    /// command is running, and outside one this is a refusal: a program
    /// that could change the document without the registry would have a
    /// **second path** to it — a path with no snapshot, no undo step and
    /// nothing in the log. The family tree refuses that in the same way,
    /// and this is the same rule: **the registry is the only way**.
    ///
    /// So the caller is a command, and the error says so.
    pub fn document_mut(&mut self) -> Result<&mut Document, EngineError> {
        if self.running.is_none() {
            return Err(EngineError::Failed {
                id: "session".to_string(),
                message: "the document is changed by a command, and no command is running".to_string(),
            });
        }
        Ok(&mut self.document)
    }

    /// Put a document into the session, whole.
    ///
    /// This is the session's own business: it is how a program hands a
    /// document to a session that has none — a plugin's bridge, a test
    /// that opens a file — and it is **not** a command, because it is
    /// not an edit: nothing is undone and nothing is logged, because
    /// there was no edit.
    pub(crate) fn set_document(&mut self, document: Document) {
        self.document = document;
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
            // An evaluator is a contribution too: code this tree does
            // not own. A contribution that panics is skipped, and the
            // plan says what it could not say.
            let guarded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (evaluator.evaluate)(&self.document)
            }));
            match guarded {
                Ok(theirs) => plan.items.extend(theirs.items),
                Err(panic) => plan.failures.push(crate::plan::PlanFailure {
                    who: evaluator.id.to_string(),
                    message: crate::guard::message_of(&panic),
                }),
            }
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

    /// How much memory the journal may hold.
    pub fn journal_budget(&self) -> usize {
        self.journal_budget
    }

    /// Say how much memory the journal may hold.
    ///
    /// The budget is **the caller's**: the kit's own number is the
    /// default, and a program that wants a bigger macro says so here.
    pub fn set_journal_budget(&mut self, bytes: usize) {
        self.journal_budget = bytes;
    }

    /// Forget the journal.
    pub fn clear_journal(&mut self) {
        self.journal.clear();
        self.journal_bytes = 0;
    }

    /// Write an entry in the journal.
    ///
    /// The limit is here and not in the caller: a session that grows a
    /// journal for ever is a leak, and the limit is what makes it bounded.
    ///
    /// There are two limits: the **count** of entries and the **bytes**
    /// they cost. A count alone does not bound the memory, because an
    /// entry holds the parameters the caller sent, and those are as big
    /// as the caller made them. The bytes of an entry are measured when
    /// it is made, so the oldest entry can be dropped by subtracting what
    /// it cost.
    fn journal_entry(&mut self, id: &str, params: &Json) {
        let entry = JournalEntry { id: id.to_string(), params: params.clone() };
        let bytes = entry.bytes();
        self.journal.push(entry);
        self.journal_bytes += bytes;
        if self.journal.len() > crate::journal::JOURNAL_LIMIT {
            let excess = self.journal.len() - crate::journal::JOURNAL_LIMIT;
            self.drop_journal(0, excess);
        }
        while self.journal_bytes > self.journal_budget && self.journal.len() > 1 {
            // The oldest entry goes: the journal is a convenience and
            // not a history, and the newest is the one that is useful.
            self.drop_journal(0, 1);
        }
    }

    /// Drop entries from the journal, and take their bytes off the count.
    fn drop_journal(&mut self, from: usize, count: usize) {
        let dropped: Vec<JournalEntry> = self.journal.drain(from..from + count).collect();
        for entry in dropped {
            self.journal_bytes = self.journal_bytes.saturating_sub(entry.bytes());
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

    /// What the session is holding, in bytes.
    ///
    /// It is the **parts**, and not one number: a session that is over
    /// its budget has to give something up, and the part that gives way
    /// is the part that is cheapest to lose.
    pub fn memory(&self) -> Memory {
        let log = self.log.capacity() * std::mem::size_of::<String>()
            + self.log.iter().map(|line| line.capacity()).sum::<usize>();
        Memory {
            document: self.document.estimated_bytes(),
            history: self.history.bytes(),
            journal: self.journal_bytes,
            tasks: self.tasks.bytes(),
            log,
        }
    }

    /// How much memory the session may hold.
    pub fn memory_budget(&self) -> usize {
        self.memory_budget
    }

    /// Say how much memory the session may hold.
    ///
    /// The budget is **the caller's**: the kit's own number is the
    /// default, and a program that knows better says so here.
    pub fn set_memory_budget(&mut self, bytes: usize) {
        self.memory_budget = bytes;
    }

    /// Give things up until the session is inside its budget.
    ///
    /// The order is the **order of what is cheapest to lose**, and it is
    /// the same in every session:
    ///
    /// 1. the **journal** — a record of what was asked for, and the
    ///    newest entries are the useful ones;
    /// 2. the **finished tasks** — their results have been collected, or
    ///    nobody is waiting for them;
    /// 3. the **history** — the oldest states, which are the ones a
    ///    person has already undone past.
    ///
    /// The document is never given up: it is the work.
    fn give_way(&mut self) {
        let budget = self.memory_budget;
        if self.memory().total() <= budget {
            return;
        }
        while self.memory().total() > budget && self.journal.len() > 1 {
            self.drop_journal(0, 1);
        }
        while self.memory().total() > budget && self.tasks.drop_oldest_finished() {}
        self.history.prune_to(budget.saturating_sub(self.memory().document));
    }

    /// What the session has been asked to do.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// Write a line in the session's log.
    ///
    /// The log is for a person to read, and it is **bounded**: a session
    /// that runs for days would otherwise hold every line it ever wrote.
    /// The oldest lines go.
    fn note(&mut self, message: String) {
        self.log.push(message);
        if self.log.len() > MAX_LOG_ENTRIES {
            let excess = self.log.len() - MAX_LOG_ENTRIES;
            self.log.drain(0..excess);
        }
    }

    /// Run a command, and ask the host for what it is missing.
    ///
    /// The command does not change and the document does not change: the
    /// host is simply given a turn **before** the command runs. The
    /// questions are the command's own required parameters that the
    /// caller did not give, and they are asked in the order they are
    /// declared.
    ///
    /// A command that is already given everything is run exactly as
    /// [`Session::execute`] would run it, and the supplier is never
    /// asked. A supplier that cannot answer leaves the parameter missing,
    /// and the command fails the way it would have failed anyway.
    pub fn execute_with(
        &mut self,
        id: &str,
        raw: &Json,
        supplier: &mut dyn crate::prompt::Supplier,
    ) -> Result<Outcome, EngineError> {
        let Some(spec) = self.commands.get(id).cloned() else {
            return self.execute(id, raw);
        };
        let mut params = raw.clone();
        if !params.is_object() {
            return self.execute(id, raw);
        }
        for doc in spec.params {
            if !doc.needed() {
                continue;
            }
            if params.get(doc.name).is_some() {
                continue;
            }
            // The other spelling of the parameter counts as given.
            if let Some(alternative) = doc.alternative
                && params.get(alternative).is_some()
            {
                continue;
            }
            let question = crate::prompt::Question {
                name: doc.name.to_string(),
                kind: doc.kind,
                help: doc.help.to_string(),
            };
            if let Some(answer) = supplier.supply(&question) {
                params[doc.name] = answer;
            }
        }
        self.execute(id, &params)
    }

    /// **Ask the host** something, while the command runs.
    ///
    /// This is the other half of the prompt seam (§45): that one asks for
    /// the command's *declared* parameters before it runs, and this one is
    /// for what the command itself decides it needs, at the moment it
    /// needs it.
    ///
    /// The answer comes back as the value. If the host has not answered
    /// yet, the command is **stopped**: the error that comes out of this
    /// is [`crate::EngineError::NeedsInput`], and the session turns it
    /// into an outcome that says what is wanted. When the host answers
    /// with [`Session::answer`], the command runs again and this time the
    /// answer is waiting for it.
    ///
    /// The rule is that a command **asks before it changes the document**:
    /// the session compares the document with the snapshot it took before
    /// the run, and a command that changed something and then asked is
    /// refused and put back. That is what makes running the command again
    /// safe. See [`crate::resume`].
    pub fn ask(&mut self, question: &Question) -> Result<Json, EngineError> {
        let id = self.running.clone().unwrap_or_default();
        if let Some(pending) = &self.pending
            && let Some(answer) = pending.answer(&question.name)
        {
            return Ok(answer);
        }
        if let Some(pending) = &self.pending
            && !pending.may_ask()
        {
            return Err(EngineError::Failed {
                id,
                message: format!("the command asked more than {} questions", crate::resume::MAX_QUESTIONS),
            });
        }
        Err(EngineError::NeedsInput { id, question: question.clone() })
    }

    /// The command that is **waiting** for an answer, if there is one.
    pub fn pending(&self) -> Option<&Pending> {
        self.pending.as_ref()
    }

    /// **Answer** the question the waiting command asked.
    ///
    /// The command runs again, and this time the answer is waiting for it.
    /// The command's own parameters are the ones it was given the first
    /// time, and its undo is one step for the whole of it: asking and
    /// answering are one command, not two.
    pub fn answer(&mut self, value: Json) -> Result<Outcome, EngineError> {
        let Some(pending) = &self.pending else {
            return Err(EngineError::Failed {
                id: "answer".to_string(),
                message: "there is no command waiting for an answer".to_string(),
            });
        };
        if !pending.may_resume() {
            return Err(EngineError::Failed {
                id: pending.id.clone(),
                message: format!("the command was answered more than {} times", crate::resume::MAX_RESUMES),
            });
        }
        let Some(question) = pending.question().cloned() else {
            return Err(EngineError::Failed {
                id: pending.id.clone(),
                message: "the command has not asked anything".to_string(),
            });
        };
        let id = pending.id.clone();
        let params = pending.params.clone();
        let name = question.name.clone();
        if let Some(pending) = self.pending.as_mut() {
            pending.answers.insert(name, value);
            pending.resumes += 1;
        }
        self.execute(&id, &params)
    }

    /// **Give up** on the command that is waiting.
    ///
    /// It changed nothing — that is the rule — so giving up costs nothing
    /// but the answer that will not come. It says whether there was one.
    pub fn cancel(&mut self) -> bool {
        self.pending.take().is_some()
    }

    /// Run a command.
    pub fn execute(&mut self, id: &str, raw: &Json) -> Result<Outcome, EngineError> {
        // A command that is waiting for an answer is dropped when another
        // command runs. It changed nothing, so dropping it costs nothing.
        if let Some(pending) = &self.pending
            && pending.id != id
        {
            self.pending = None;
        }
        let Some(spec) = self.commands.get(id).cloned() else {
            let error = EngineError::Unknown(id.to_string());
            self.note(format!("{id}: refused ({error})"));
            return Err(error);
        };
        if !(spec.enabled)(self) {
            let error = EngineError::Disabled {
                id: id.to_string(),
                reason: "the session says it is not available".to_string(),
            };
            self.note(format!("{id}: refused ({error})"));
            return Err(error);
        }
        let params = Params::new(raw);
        if let Err(source) = params.deny_unknown(&spec.param_names()) {
            let error = EngineError::Param { id: id.to_string(), source };
            self.note(format!("{id}: refused ({error})"));
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
                    self.note(format!("{id}: refused ({error})"));
                    return Err(error);
                }
                let mut outcome = outcome;
                outcome.changed = changed;
                if changed {
                    // The document is judged **after** the command, and
                    // not only when it is written: a command that made
                    // the document too big is refused and the document is
                    // put back, because a document nobody could save is
                    // a document nobody should have.
                    let bytes = self.document.estimated_bytes();
                    if bytes > crate::document::MAX_DOCUMENT_BYTES {
                        self.document = snapshot;
                        self.selection = selection_snapshot;
                        self.tool = tool_snapshot;
                        let error = EngineError::Failed {
                            id: id.to_string(),
                            message: format!(
                                "the document is {bytes} bytes, and the limit is {}",
                                crate::document::MAX_DOCUMENT_BYTES
                            ),
                        };
                        self.note(format!("{id}: refused ({error})"));
                        return Err(error);
                    }
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
                self.note(format!("{id}: {}", outcome.message));
                if spec.journal {
                    self.journal_entry(id, params.value());
                }
                // The session gives way if it is over its budget: the
                // journal, the finished tasks and the history, in that
                // order — what is cheapest to lose goes first.
                self.give_way();
                // The command finished, so it is not waiting any more.
                self.pending = None;
                Ok(outcome)
            }
            Err(error) => {
                if let EngineError::NeedsInput { question, .. } = &error {
                    return self.waiting(
                        id,
                        question.clone(),
                        params.value(),
                        snapshot,
                        selection_snapshot,
                        tool_snapshot,
                    );
                }
                self.document = snapshot;
                self.selection = selection_snapshot;
                self.tool = tool_snapshot;
                self.pending = None;
                self.note(format!("{id}: refused ({error})"));
                Err(error)
            }
        }
    }

    /// The command **asked**: the document is as it was, and the host has
    /// to answer.
    ///
    /// The rule is enforced here, and not by a convention: a command that
    /// changed the document and *then* asked cannot be run again safely,
    /// so the change is put back and the command is refused.
    fn waiting(
        &mut self,
        id: &str,
        question: Question,
        params: &Json,
        snapshot: Document,
        selection_snapshot: Vec<Selected>,
        tool_snapshot: Option<String>,
    ) -> Result<Outcome, EngineError> {
        if self.document != snapshot {
            self.document = snapshot;
            let error = EngineError::Failed {
                id: id.to_string(),
                message: "a command that asks must ask before it changes the document".to_string(),
            };
            self.note(format!("{id}: refused ({error})"));
            return Err(error);
        }
        // The session's own state goes back too: a command that is
        // waiting has not finished, and it did not leave the selection
        // moved.
        self.selection = selection_snapshot;
        self.tool = tool_snapshot;
        let pending = self.pending.get_or_insert_with(|| Pending::new(id, params.clone()));
        pending.asked.push(question.clone());
        let outcome = Outcome::needs_input(question);
        self.note(format!("{id}: {}", outcome.message));
        Ok(outcome)
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

/// What the session is holding, and what it may hold.
///
/// It changes nothing, so it is not an undo step, and it is not in the
/// journal: a journal of a measurement is not a program.
///
/// It is the one place a person — or an agent — can see what the session
/// is spending its memory on, which is the question every **budget** in
/// this kit exists to answer.
#[derive(craftkit_derive::Command)]
#[command(id = "session.memory", area = "session", icon = "list", no_journal)]
pub struct SessionMemory {}

impl SessionMemory {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let memory = session.memory();
        let total = memory.total();
        let budget = session.memory_budget();
        Ok(Outcome::new(format!("{total} byte(s) of {budget}")).with_data(crate::serde_json::json!({
            "document": memory.document,
            "history": memory.history,
            "journal": memory.journal,
            "tasks": memory.tasks,
            "log": memory.log,
            "total": total,
            "budget": budget,
            "inside": total <= budget,
        })))
    }
}
