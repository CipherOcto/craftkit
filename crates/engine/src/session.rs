//! The session: a document, an evaluator, a playhead, a history, and the one
//! `execute` every surface calls.
//!
//! # Clone before you edit
//!
//! Every mutating command runs against a **snapshot** taken before it starts.
//! If the command returns an error, the snapshot goes straight back. If it
//! panics, the snapshot goes back *and* an [`EngineError::Internal`] is
//! returned.
//!
//! That is not a safety net bolted on afterwards — it is the reason this
//! workspace can promise "a crash never loses your work" without asking anyone
//! to trust a `Drop` implementation. It also means the undo stack needs no
//! cleverness: `history.undo` restores a snapshot, which is exactly the same
//! operation.
//!
//! # One `catch_unwind`, in one place
//!
//! There is exactly one here, in [`Session::execute`]. The kernel boundary
//! below it catches panics from code craftkit does not own; this one catches
//! everything else, including a panic in our own future command. Two boundaries
//! with distinct jobs beat seven `catch_unwind`s scattered through call sites.

use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};

use craftkit_core::{NodeId, Params, Tick};
use craftkit_eval::{Evaluator, Plan};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::command;
use crate::error::EngineError;
use crate::history::History;
use crate::log::LogBook;

/// What a command did.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// One line for the status bar and the command log.
    pub message: String,
    /// Whatever the command wants to hand back. Read-only commands put their
    /// answer here; mutating ones usually return the id or path they created.
    pub data: Json,
}

impl Outcome {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), data: Json::Null }
    }

    pub fn with(message: impl Into<String>, data: Json) -> Self {
        Self { message: message.into(), data }
    }

    pub fn to_json(&self) -> Json {
        serde_json::json!({ "ok": true, "message": self.message, "data": self.data })
    }
}

/// One command the session ran, kept so that a crash report can replay it.
///
/// The parameters are stored **as they were given** — as JSON, not as text.
///
/// This used to be a line of text, truncated to a couple of hundred
/// characters, and the truncation was allowed to cut the JSON in half. A
/// truncated parameter object is not valid JSON, so the replay parsed it,
/// failed, and fell back to an *empty* parameter object — and the command
/// then ran with parameters nobody had ever typed. The replay produced a
/// different document and said nothing about it. §47.
///
/// Storing the value itself makes that impossible: there is no text to
/// truncate and nothing to parse back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceStep {
    /// The command id, exactly as it was called.
    pub command: String,
    /// The parameters, exactly as they were given.
    pub params: Json,
}

/// How many bytes a trace step takes in the trace.
///
/// The size of the *serialized* step, which is what the trace holds. A
/// parameter object that cannot be serialized cannot have come from JSON, so
/// the fallback is only there to keep the arithmetic total.
fn step_size(step: &TraceStep) -> usize {
    serde_json::to_string(step).map(|s| s.len()).unwrap_or(0)
}

/// A live editing session.
///
/// Every surface — the window, the CLI, the control channel, the MCP server,
/// a script — owns one of these and reaches it only through
/// [`Session::execute`].
#[derive(Debug)]
pub struct Session {
    doc: craftkit_model::Document,
    evaluator: Evaluator,
    history: History,
    at: Tick,
    plan: Option<Plan>,
    log: LogBook,
    /// Everything that ran since the trace began, for a crash report.
    ///
    /// Bounded by [`crate::MAX_TRACE_BYTES`], oldest first, and **lossless**:
    /// the parameters are the ones the command was given.
    trace: VecDeque<TraceStep>,
    /// How many bytes the trace holds, so the bound is a running total
    /// rather than a walk over the whole trace on every command.
    trace_bytes: usize,
    /// How many commands have run since the trace began — including the ones
    /// the bound made it drop.
    trace_total: usize,
    /// The document the trace starts from.
    ///
    /// A crash report replays the trace **onto this**, not onto an empty
    /// session: the commands in the trace are relative to the document that
    /// was there when they ran. §47.
    trace_base: craftkit_model::Document,
    dirty: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let doc = craftkit_model::Document::empty(name.clone());
        let mut s = Self {
            trace_base: doc.clone(),
            doc,
            evaluator: Evaluator::new(),
            history: History::new(),
            at: Tick::ZERO,
            plan: None,
            log: LogBook::new(),
            trace: VecDeque::new(),
            trace_bytes: 0,
            trace_total: 0,
            dirty: false,
        };
        s.reset_trace();
        s
    }

    /// A session with a small example document, for `--sample` and for tests.
    ///
    /// Built **through the command registry**, not by writing to the document
    /// directly. That costs three lines and buys two things: the sample is
    /// guaranteed to be buildable by whatever a user or an agent could do, and
    /// the history it would otherwise leave behind is wiped below, so a sample
    /// session opens clean.
    pub fn sample() -> Self {
        let mut s = Self::new("Sample plan");
        for (name, kind, amount) in [("Level", "source", 1.0), ("Taper", "filter", 0.5), ("Out", "sink", 1.0)]
        {
            let outcome =
                s.execute("node.add", &serde_json::json!({ "name": name, "kind": kind, "amount": amount }));
            debug_assert!(outcome.is_ok(), "the sample document must build: {outcome:?}");
        }
        // The sample is the *starting state*, not something the user did: a
        // session that opens with three entries in its log and an undo stack
        // three deep would be a lie that a crash report would then repeat.
        s.history.clear();
        s.log.clear();
        s.dirty = false;
        s.reset_trace();
        s.recompute();
        s
    }

    // --- reading -----------------------------------------------------------

    pub fn document(&self) -> &craftkit_model::Document {
        &self.doc
    }

    pub fn node(&self, id: NodeId) -> Option<&craftkit_model::Node> {
        self.doc.get(id)
    }

    pub fn at(&self) -> Tick {
        self.at
    }

    pub fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// How many undo steps are currently kept.
    pub fn undo_depth(&self) -> usize {
        self.history.len()
    }

    pub fn log(&self) -> &LogBook {
        &self.log
    }

    /// The commands that have run since the trace began, oldest first.
    ///
    /// A crash report replays these onto [`Session::trace_base`], which is
    /// the document they were relative to.
    pub fn trace(&self) -> &VecDeque<TraceStep> {
        &self.trace
    }

    /// The document the trace starts from: the state the session was in when
    /// the first recorded command ran.
    pub fn trace_base(&self) -> &craftkit_model::Document {
        &self.trace_base
    }

    /// How many commands have run since the trace began.
    pub fn trace_total(&self) -> usize {
        self.trace_total
    }

    /// How many commands the trace had to drop to stay inside its bound.
    ///
    /// Zero for a session whose history fits. A report with a non-zero count
    /// replays only the tail of the session, and says so.
    pub fn trace_dropped(&self) -> usize {
        self.trace_total.saturating_sub(self.trace.len())
    }

    /// Forget the trace, and start it again from the document as it is now.
    ///
    /// The base is the document *at this moment*, so the commands recorded
    /// after this point are relative to it.
    fn reset_trace(&mut self) {
        self.trace.clear();
        self.trace_bytes = 0;
        self.trace_total = 0;
        self.trace_base = self.doc.clone();
    }

    /// The whole state, for an API surface that wants it in one object.
    pub fn snapshot_json(&self) -> Json {
        serde_json::json!({
            "name": self.doc.meta.name,
            "at": self.at.raw(),
            "dirty": self.dirty,
            "nodes": self.doc.nodes.iter().map(|n| serde_json::json!({
                "id": n.id.get(),
                "name": n.name,
                "kind": n.kind.label(),
                "muted": n.muted,
            })).collect::<Vec<_>>(),
            "can_undo": self.can_undo(),
            "can_redo": self.can_redo(),
            "plan": self.plan.as_ref().map(|p| serde_json::to_value(p).unwrap_or(Json::Null)),
        })
    }

    // --- the one write path ------------------------------------------------

    /// Run a command by id with a JSON parameter object.
    ///
    /// This is the only way anything changes. The UI's toolbar, the CLI, the
    /// control channel, the MCP server and a script all land here, so a guard
    /// added to a command cannot be bypassed by picking a different front end.
    ///
    /// Note what this does **not** do: it never opens a dialog, prompts, or
    /// touches the clipboard. Those are interactive concerns and belong to
    /// `ui.start`, which the front ends call instead.
    pub fn execute(&mut self, id: &str, params: &Json) -> Result<Outcome, EngineError> {
        let Some(spec) = command::lookup(id) else {
            let err = EngineError::Unknown(id.to_string());
            // Logged like every other refusal: an agent that mistypes an id is
            // exactly the case the log exists to explain.
            self.log.record(id, false, &err.to_string());
            return Err(err);
        };

        if !(spec.enabled)(self) {
            let err = EngineError::Disabled { id: id.to_string(), reason: disabled_reason(self, id) };
            // Logged even though the document was never touched: the log is the
            // record of what an agent asked for, and a refusal is exactly the
            // line someone will be looking for.
            self.log.record(id, false, &err.to_string());
            return Err(err);
        }

        let p = Params::new(params);
        // An agent's typo should be loud, not silently ignored.
        p.deny_unknown(&spec.param_names())
            .map_err(|source| EngineError::Param { id: id.to_string(), source })?;

        // The trace is the reproduction: the command id and the parameters,
        // exactly as they were given. Nothing is compacted away, because a
        // trace that cannot be replayed exactly is not a reproduction.
        let step = TraceStep { command: id.to_string(), params: params.clone() };
        let step_bytes = step_size(&step);
        // A step that does not fit in the budget is refused rather than kept.
        //
        // The loop below drops the oldest step until the trace fits, and it
        // keeps the last one — so a step bigger than the whole budget would be
        // kept whole, and the trace would hold more than it is allowed to. It
        // would also be held for the rest of the session, and the trace holds
        // the *parsed* parameters, which cost up to thirty-two times their
        // text. §70.
        //
        // Refused *here*, before the document is cloned and before the step is
        // copied, so the cost of the refusal is nothing.
        if step_bytes > crate::MAX_TRACE_BYTES {
            let err = EngineError::Param {
                id: id.to_string(),
                source: craftkit_core::ParamError::new(
                    "params",
                    format!(
                        "they are {step_bytes} bytes of JSON, and the crash report's trace keeps at \
                         most {} — a command that cannot be recorded cannot be reproduced",
                        crate::MAX_TRACE_BYTES
                    ),
                ),
            };
            self.log.record(id, false, &err.to_string());
            return Err(err);
        }

        // Clone before edit. Taken for *every* command, mutating or not, so the
        // restore path below is a single unconditional assignment.
        let snapshot = self.doc.clone();
        let before_dirty = self.dirty;

        self.trace_bytes += step_bytes;
        self.trace_total += 1;
        self.trace.push_back(step);
        // Bounded by size, so the bound holds whatever the commands were. The
        // oldest step goes first: a crash is nearly always in what someone
        // was doing a moment ago.
        while self.trace.len() > 1 && self.trace_bytes > crate::MAX_TRACE_BYTES {
            match self.trace.pop_front() {
                Some(old) => self.trace_bytes = self.trace_bytes.saturating_sub(step_size(&old)),
                None => break,
            }
        }

        let result = catch_unwind(AssertUnwindSafe(|| (spec.run)(self, &p)));

        match result {
            Ok(Ok(outcome)) => {
                if spec.mutates {
                    // The snapshot is already in hand, so "did this command
                    // actually change anything?" is a comparison rather than a
                    // flag a command has to remember to set. A command that
                    // declares `mutates: true` and changes nothing is a bug —
                    // almost always one that forgot to write — and would
                    // otherwise put a no-op step on the undo stack.
                    if self.doc == snapshot {
                        // The engine refuses the command, so the session goes
                        // back to exactly where it was — the same restore the
                        // failure and panic paths use, and the same one the
                        // dirty flag is restored by.
                        self.restore(snapshot, before_dirty);
                        let err = EngineError::Internal {
                            id: id.to_string(),
                            detail: "the command declared that it mutates the document but changed nothing"
                                .to_string(),
                        };
                        self.log.record(id, false, &err.to_string());
                        return Err(err);
                    }
                    // The document has to stay a document the file reader
                    // would accept.
                    //
                    // The engine's document and the file's document are the
                    // same thing, and the reader validates it. A command that
                    // broke one of those rules would leave a plan that could
                    // not be saved — and, before the writer learned to
                    // refuse it, a plan that saved to a file the app then
                    // could not open. The check is here rather than in each
                    // command so that the next command cannot forget it.
                    // §50.
                    if let Err(e) = self.doc.validate() {
                        self.restore(snapshot, before_dirty);
                        let err = EngineError::Internal {
                            id: id.to_string(),
                            detail: format!(
                                "the command left the document in a state the file format refuses: {e}"
                            ),
                        };
                        self.log.record(id, false, &err.to_string());
                        return Err(err);
                    }
                    if spec.undoable {
                        self.history.push(snapshot, spec.label);
                    }
                    self.dirty = true;
                    self.recompute();
                }
                self.log.record(id, true, &outcome.message);
                Ok(outcome)
            }
            Ok(Err(e)) => {
                let e = e.with_command(id);
                // The command refused. Put the document back exactly as it was:
                // a half-applied edit is worse than a failed one.
                self.restore(snapshot, before_dirty);
                self.log.record(id, false, &e.to_string());
                Err(e)
            }
            Err(payload) => {
                self.restore(snapshot, before_dirty);
                let detail = crate::log::panic_detail(&payload);
                log::error!("command `{id}` panicked and was recovered: {detail}");
                let err = EngineError::Internal { id: id.to_string(), detail };
                self.log.record(id, false, &err.to_string());
                Err(err)
            }
        }
    }

    /// Re-evaluate at the current playhead.
    pub fn recompute(&mut self) {
        match self.evaluator.evaluate(&self.doc, self.at) {
            Ok(plan) => self.plan = Some(plan),
            Err(e) => {
                // Losing the plan is not fatal: the document is intact and the
                // editor shows the error in the problems panel.
                self.plan = None;
                log::warn!("evaluation failed: {e}");
            }
        }
    }

    /// Put the document back the way it was, and say how dirty it is.
    ///
    /// One function, so the rollback cannot drift apart. The error path and the
    /// panic path used to spell out the same four statements twice each, and
    /// "the same" was a promise the source had to keep by hand — the prose in
    /// `docs/patterns/clone-before-edit.md` said "the same four" for as long as
    /// both blocks happened to agree.
    ///
    /// The cache is cleared because a restore puts a different document in
    /// place. That is belt and braces rather than a correctness requirement:
    /// the evaluator's cache is content-addressed, so an entry is reused only
    /// when the node it was computed from still hashes the same. The property
    /// that makes the clearing redundant is tested, in
    /// `a_restore_cannot_serve_a_state_from_the_document_it_replaced`.
    fn restore(&mut self, doc: craftkit_model::Document, dirty: bool) {
        self.doc = doc;
        self.evaluator.invalidate();
        self.recompute();
        self.dirty = dirty;
    }

    /// Restore a snapshot. Called by undo and redo.
    ///
    /// Public because a document loaded from disk has to reset the evaluator's
    /// cache too — but **loading a document is not this**. This puts back a
    /// state the session was already in; [`Session::load_document`] starts a
    /// session from a document that arrived from outside, and forgets what
    /// came before it.
    pub fn replace_document(&mut self, doc: craftkit_model::Document) {
        self.restore(doc, false);
    }

    /// Start the session from a document that arrived from outside.
    ///
    /// This is what "open a file" means, and it does three things that
    /// [`Session::replace_document`] does not:
    ///
    /// - the undo stack and the log are cleared, because they belong to the
    ///   document that was there before — an undo that reached back into the
    ///   previous file would restore a document the user never asked for;
    /// - the command trace starts again, because the commands in it are
    ///   relative to the document they ran against, and the document they ran
    ///   against is gone;
    /// - the trace's base becomes this document, so a crash report can be
    ///   replayed onto it. §47.
    pub fn load_document(&mut self, doc: craftkit_model::Document) {
        self.restore(doc, false);
        self.history.clear();
        self.log.clear();
        self.reset_trace();
    }

    /// Direct access to the document, for tests that need to build the
    /// *malformed* state a hostile file would produce — a state no command can
    /// create on purpose.
    #[cfg(test)]
    pub(crate) fn doc_mut_for_test(&mut self, id: NodeId) -> &mut craftkit_model::Node {
        self.doc.get_mut(id).unwrap_or_else(|| panic!("node {id} is not in the document"))
    }

    /// Re-evaluate after a test has tampered with the document.
    #[cfg(test)]
    pub(crate) fn recompute_for_test(&mut self) {
        self.recompute();
    }

    /// Mark the project dirty, for a test that needs a command to do it.
    ///
    /// No command in the tree sets the flag — the engine sets it only after
    /// a command succeeds — so the rollbacks that put it back are no-ops
    /// today, and no test could see them work. §44.
    #[cfg(test)]
    pub(crate) fn mark_dirty_for_test(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn doc_mut(&mut self) -> &mut craftkit_model::Document {
        &mut self.doc
    }

    pub(crate) fn set_at(&mut self, at: Tick) {
        self.at = at;
        self.recompute();
    }

    /// Move the playhead to a tick.
    ///
    /// The command `playback.seek` takes seconds or a frame; this takes the
    /// raw tick a crash report recorded, so a replay starts where the crashed
    /// session was standing. §47.
    pub fn seek_to(&mut self, at: i64) {
        self.set_at(Tick::from_raw(i128::from(at)));
    }

    pub(crate) fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }
}

/// Why a command says it is unavailable. The registry knows only *whether*;
/// this names the reason, which is what makes the message useful.
fn disabled_reason(s: &Session, id: &str) -> String {
    match id {
        "history.undo" => "there is nothing to undo".to_string(),
        "history.redo" => "there is nothing to redo".to_string(),
        "playback.seek" if s.document().nodes.is_empty() => "the document is empty".to_string(),
        _ => format!("`{id}` needs a selection or a document state that does not exist yet"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogEntry;
    use serde_json::json;

    fn sample() -> Session {
        Session::sample()
    }

    /// Every read-only command, and the parameters it needs.
    ///
    /// The table is checked against the registry below, so a new read-only
    /// command has to be added here — which is the point: the list of read-only
    /// commands is the list of commands an agent may use as a probe.
    const READ_ONLY_CASES: &[(&str, &str)] = &[
        ("playback.seek", r#"{"frame": 48}"#),
        ("diagnostics.list", "{}"),
        ("plan.evaluate", "{}"),
        ("session.stats", "{}"),
    ];

    /// Every read-only command leaves the document alone.
    ///
    /// The `mutates` flag is a promise: *"this is safe to use as a probe"*. The
    /// test that used to guard it checked **one** command — `plan.evaluate` —
    /// while the registry has four read-only commands, so three of them were
    /// unguarded. A read-only command that wrote to the document would have
    /// been caught by nobody, and the flag is exactly what an agent reads
    /// before using a command as a probe.
    #[test]
    fn every_read_only_command_leaves_the_document_alone() {
        for (id, params) in READ_ONLY_CASES {
            let mut s = sample();
            let before = s.document().clone();
            let json: serde_json::Value = serde_json::from_str(params).expect("the table is JSON");
            s.execute(id, &json).unwrap_or_else(|e| panic!("{id} refused its own parameters: {e}"));
            assert_eq!(s.document(), &before, "{id} changed the document");
            assert!(!s.is_dirty(), "{id} marked the document dirty");
            assert!(!s.can_undo(), "{id} pushed an undo entry");
        }
    }

    /// The table and the registry are the same list.
    ///
    /// Without this half, the test above would keep passing while a new
    /// read-only command went unchecked — which is how the three unguarded
    /// commands were unguarded in the first place.
    #[test]
    fn the_table_of_read_only_commands_is_the_registry_s_own_list() {
        let catalogue = crate::catalogue();
        let read_only: Vec<String> = catalogue
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|spec| spec["mutates"] == serde_json::Value::Bool(false))
            .filter_map(|spec| spec["id"].as_str().map(|s| s.to_string()))
            .collect();
        assert!(!read_only.is_empty(), "the registry has no read-only command at all");

        for id in &read_only {
            assert!(
                READ_ONLY_CASES.iter().any(|(case, _)| case == id),
                "`{id}` is read-only and is not in READ_ONLY_CASES"
            );
        }
        for (id, _) in READ_ONLY_CASES {
            assert!(read_only.iter().any(|r| r == id), "`{id}` is in the table and is not read-only");
        }
    }

    #[test]
    fn a_mutating_command_pushes_undo_and_marks_the_document_dirty() {
        let mut s = sample();
        assert!(!s.is_dirty());
        s.execute("node.add", &json!({ "name": "Extra", "kind": "filter" })).expect("ok");
        assert!(s.is_dirty());
        assert!(s.can_undo());
        assert_eq!(s.document().nodes.len(), 4);
    }

    /// A restore cannot serve a state from the document it replaced.
    ///
    /// The evaluator's cache is content-addressed: an entry is reused only
    /// when the node it was computed from still hashes the same. That is what
    /// makes `Evaluator::invalidate` belt and braces rather than a correctness
    /// requirement, and it is the reason the pattern doc may say that clearing
    /// the cache is redundant.
    ///
    /// The rule was already tested three times over at the evaluator's own
    /// level — `editing_the_first_node_reuses_nothing`,
    /// `editing_the_last_node_reuses_the_whole_prefix` and
    /// `reordering_nodes_is_not_served_from_the_cache` all fail if the
    /// fingerprint stops distinguishing nodes. What was missing was the
    /// statement at the *engine's* level, where the restore actually happens:
    /// the prose said the cache "belongs to the document that produced it",
    /// which is not why the cache is safe.
    ///
    /// So: change the document twice, put the *first* change back without
    /// clearing the cache, and compare the result with a plan computed by a
    /// session that has never seen any of the three documents. If the
    /// fingerprint were not doing its job, the restored session would be
    /// served states belonging to a node it no longer has — the renamed node
    /// would still be called `Level` — and the two plans would differ.
    #[test]
    fn a_restore_cannot_serve_a_state_from_the_document_it_replaced() {
        let mut warm = sample();
        // The first change.
        warm.execute("node.rename", &json!({ "node": 1, "name": "First" })).expect("ok");
        let middle = warm.document().clone();
        // The second change, so the cache holds something the restore does not.
        warm.execute("node.rename", &json!({ "node": 2, "name": "Second" })).expect("ok");

        // Put the middle document back, deliberately leaving the cache alone.
        warm.doc = middle.clone();
        warm.recompute();

        // A session that has never seen any of the three documents.
        let mut cold = Session::new("Cold");
        cold.doc = middle;
        cold.recompute();

        let warm_plan = warm.plan().expect("a plan");
        let cold_plan = cold.plan().expect("a plan");
        assert_eq!(warm_plan.nodes, cold_plan.nodes, "the restored plan was served from a stale cache");
        assert_eq!(warm_plan.diagnostics, cold_plan.diagnostics);
    }

    #[test]
    fn undo_restores_the_previous_document_exactly() {
        let mut s = sample();
        let before = s.document().clone();
        s.execute("node.add", &json!({ "name": "Extra", "kind": "filter" })).expect("ok");
        s.execute("history.undo", &json!({})).expect("ok");
        assert_eq!(s.document(), &before);
    }

    #[test]
    fn undo_then_redo_returns_to_the_same_state() {
        let mut s = sample();
        s.execute("node.add", &json!({ "name": "Extra", "kind": "filter" })).expect("ok");
        let after_add = s.document().clone();
        s.execute("history.undo", &json!({})).expect("ok");
        assert!(s.can_redo());
        s.execute("history.redo", &json!({})).expect("ok");
        assert_eq!(s.document(), &after_add);
    }

    /// An undo re-evaluates the plan, and not only the document.
    ///
    /// The rollback is four statements, and two of them were pinned by no
    /// test at all: deleting `self.recompute()` from `Session::restore` left
    /// every test in the workspace green. The document came back; the *plan*
    /// — the thing the window draws, the status bar reads and `plan.evaluate`
    /// returns — did not.
    ///
    /// That is the §21 bug wearing a different hat: the engine says one thing
    /// and the screen says another, with `ok: true` on the reply and nothing
    /// reporting a problem. The document was checked; the plan was not.
    #[test]
    fn an_undo_re_evaluates_the_plan_and_not_only_the_document() {
        let mut s = sample();
        let before = s.plan().expect("a plan").output();
        s.execute("prop.set", &json!({ "node": 1, "path": "amount", "value": 5.0 })).expect("ok");
        let changed = s.plan().expect("a plan").output();
        assert_ne!(changed, before, "the command did not change the plan");
        s.execute("history.undo", &json!({})).expect("ok");
        assert_eq!(
            s.plan().expect("a plan").output(),
            before,
            "the plan was not re-evaluated after the undo"
        );
    }

    /// A failed command leaves the plan as it was, and not only the document.
    ///
    /// The rollback restores the document and re-evaluates the plan. The
    /// second half was invisible until §43: `self.recompute()` was deleted
    /// from the restore and every test in the workspace stayed green.
    ///
    /// It is invisible because the plan is recomputed on the *success* path
    /// too, so a command that is refused before it changes anything never
    /// shows the difference. `test.half_apply` is a command that changes the
    /// document and *then* fails, which is the only state where the two
    /// recomputes differ.
    #[test]
    fn a_failed_command_leaves_the_plan_as_it_was() {
        let mut s = sample();
        let before = s.plan().expect("a plan").output();
        let err = s.execute("test.half_apply", &json!({})).expect_err("refused");
        assert_eq!(err.kind(), "bad-parameter");
        assert!(!s.document().nodes[1].muted, "the document was not restored");
        assert_eq!(
            s.plan().expect("a plan").output(),
            before,
            "the plan was not re-evaluated after the failure"
        );
    }

    /// Loading a document re-evaluates the plan.
    ///
    /// `Session::load_document` is the public entry point for a document that
    /// arrives from outside: the desktop app, the web app and the CLI all
    /// call it when they open a file. It is not a command, so `execute`'s
    /// success path never runs and **nothing else recomputes the plan**.
    ///
    /// §43: with `self.recompute()` deleted from the restore, every test in
    /// the workspace was green. A file that had just been opened would have
    /// shown the *previous* document's plan — the §21 bug, with a load in
    /// place of a command.
    #[test]
    fn loading_a_document_re_evaluates_the_plan() {
        let mut s = sample();
        let before = s.plan().expect("a plan").output();

        // A different document, with a different plan.
        let mut other = Session::sample();
        other.execute("node.mute", &json!({ "node": 2 })).expect("ok");
        let other_doc = other.document().clone();
        let other_output = other.plan().expect("a plan").output();
        assert_ne!(other_output, before, "the two documents have the same plan");

        s.replace_document(other_doc);
        assert_eq!(
            s.plan().expect("a plan").output(),
            other_output,
            "the plan was not re-evaluated after the load"
        );
    }

    /// Opening a document throws away the undo stack of the one before it.
    ///
    /// Measured before the fix: a session that had added a node, then opened a
    /// file with three nodes in it, still had the old snapshot on its undo
    /// stack. The first Ctrl+Z said "undid Add node" and replaced the file's
    /// contents with an **empty** document — a command the user never ran in
    /// that file. The undo stack belongs to the document it was made from.
    ///
    /// §47.
    #[test]
    fn opening_a_document_clears_the_undo_stack() {
        let mut s = sample();
        s.execute("node.add", &json!({ "name": "Before the open", "kind": "cue" })).expect("ok");
        assert!(s.can_undo(), "the edit before the open should be undoable");

        let opened = Session::sample().document().clone();
        s.load_document(opened);
        assert!(!s.can_undo(), "the undo stack of the previous document survived the open");
        assert_eq!(s.document().nodes.len(), 3, "the loaded document is not the one on screen");

        // And the undo that used to throw the file away is now a refusal.
        let refused = s.execute("history.undo", &json!({})).expect_err("there is nothing to undo");
        assert!(refused.to_string().contains("nothing to undo"), "{refused}");
        assert_eq!(s.document().nodes.len(), 3, "the undo threw the loaded document away");
    }

    /// Loading a document starts the command trace again.
    ///
    /// The commands in the trace are relative to the document they ran
    /// against, so a trace that outlived a document load would replay the
    /// wrong thing — onto the wrong base. §47.
    #[test]
    fn opening_a_document_starts_the_trace_again() {
        let mut s = sample();
        s.execute("node.add", &json!({ "name": "Before the open", "kind": "cue" })).expect("ok");
        assert_eq!(s.trace().len(), 1);

        let opened = Session::sample().document().clone();
        s.load_document(opened);
        assert!(s.trace().is_empty(), "the trace of the previous document survived the open");
        assert_eq!(s.trace_total(), 0);
        assert_eq!(s.trace_dropped(), 0);

        s.execute("node.add", &json!({ "name": "After the open", "kind": "cue" })).expect("ok");
        assert_eq!(s.trace().len(), 1, "only the command after the open is in the trace");
        assert_eq!(s.trace()[0].command, "node.add");
        // The base is the document the command was relative to.
        assert_eq!(s.trace_base().nodes.len(), 3, "the base is not the document that was loaded");
    }

    /// The restore puts the dirty flag back.
    ///
    /// `before_dirty` is captured alongside the snapshot so that a rollback
    /// leaves the project dirty exactly as it was — a save prompt for changes
    /// that were rolled back would be a lie. No command sets the flag today
    /// (the engine sets it only after a command succeeds), so this is a
    /// precaution against the next one, and a precaution nobody checks is a
    /// comment.
    #[test]
    fn a_restore_puts_the_dirty_flag_back() {
        let mut s = sample();
        let doc = s.document().clone();
        s.dirty = true;
        s.restore(doc, false);
        assert!(!s.is_dirty(), "the restore did not put the dirty flag back");
        s.restore(s.document().clone(), true);
        assert!(s.is_dirty(), "the restore did not keep the project dirty");
    }

    #[test]
    fn undo_with_nothing_to_undo_is_a_readable_refusal_not_a_crash() {
        let mut s = sample();
        let err = s.execute("history.undo", &json!({})).expect_err("refused");
        assert_eq!(err.kind(), "disabled");
        assert!(err.to_string().contains("nothing to undo"), "{err}");
    }

    #[test]
    fn an_unknown_command_is_refused_and_names_the_way_to_find_the_list() {
        let mut s = sample();
        let err = s.execute("node.frobnicate", &json!({})).expect_err("refused");
        assert_eq!(err.kind(), "unknown-command");
        assert!(err.to_string().contains("craftkit-cli commands"), "{err}");
    }

    #[test]
    fn an_unknown_parameter_is_refused_rather_than_ignored() {
        let mut s = sample();
        let err = s
            .execute("node.add", &json!({ "name": "X", "kind": "filter", "colour": "red" }))
            .expect_err("refused");
        assert_eq!(err.kind(), "bad-parameter");
        assert!(err.to_string().contains("colour"), "{err}");
    }

    #[test]
    fn a_failing_command_leaves_the_document_exactly_as_it_was() {
        let mut s = sample();
        let before = s.document().clone();
        // `amount` of the wrong type is refused partway through the command.
        let err = s
            .execute("prop.set", &json!({ "node": 1, "path": "amount", "value": { "a": 1 } }))
            .expect_err("refused");
        assert_eq!(err.kind(), "bad-parameter");
        assert_eq!(s.document(), &before, "a failed command must not half-apply");
        assert!(!s.can_undo(), "a failed command must not push history");
    }

    #[test]
    fn executing_never_opens_a_dialog_or_blocks() {
        // The property under test: `execute` is synchronous, total and silent.
        // If a future command ever pops a dialog, this test is the thing that
        // should fail to compile rather than hang.
        let mut s = sample();
        let out = s.execute("session.stats", &json!({})).expect("ok");
        assert!(out.message.contains("command"), "{}", out.message);
    }

    #[test]
    fn the_sample_is_the_example_and_nothing_but_it() {
        // The three `node.add` calls in `sample()` go through the command
        // registry, so they leave entries in the log, on the undo stack and in
        // the trace. Clearing them is the whole difference between "you opened an
        // example" and "you made three edits and undid nothing" — and a crash
        // report replays that log, so a sample that kept it would make every
        // report from a freshly opened window lie about what the user did.
        //
        // Nothing asserted any of this. The `debug_assert!` inside `sample()`
        // would have caught a broken node kind, except that `debug-assertions`
        // is off in the `ci` profile, so it was compiled out exactly where CI
        // runs it. Found by asking for a node kind that does not exist.
        let s = Session::sample();

        let names: Vec<&str> = s.document().nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["Level", "Taper", "Out"], "the sample is the example every screenshot shows");

        assert_eq!(s.undo_depth(), 0, "the sample must not arrive with an undo stack");
        assert!(s.log().entries().is_empty(), "the sample must not arrive with a command log");
        assert!(s.trace().is_empty(), "the sample must not arrive with a trace");
        assert!(!s.is_dirty(), "opening an example is not an edit");
    }

    /// Fire every registered command at every documented parameter with
    /// hostile values.
    ///
    /// Returns the `(command, parameter)` pairs it fired, so the coverage
    /// test can assert that it really did fire all of them.
    ///
    /// The values are **derived from the registry**, not written out here.
    /// The test used to fire five fixed parameter objects, and the doc said it
    /// fired every command at every documented parameter name: measured, 58 of
    /// the 105 firings were stopped by `deny_unknown` before the command's
    /// own code ran, and 27 documented parameters — `node.add.name`,
    /// `node.add.kind`, `doc.range.fps`, `key.ease.easing` — were never fired
    /// at all. Deriving them is what makes the promise true. §92.
    fn fire_hostile_params() -> Vec<String> {
        let hostile = [
            json!({}),
            json!({"node": -1}),
            json!({"node": "<not a node>"}),
            json!({"node": u64::MAX, "path": "../../etc", "value": []}),
            json!({"amount": [1, 2, 3], "index": -5, "time": "yesterday"}),
        ];
        // One hostile value per documented parameter, on its own.
        let per_parameter: Vec<(String, Vec<serde_json::Value>)> = command::ids()
            .into_iter()
            .flat_map(|id| {
                let spec = command::lookup(id).expect("a spec for every id");
                spec.param_names()
                    .into_iter()
                    .map(move |name| {
                        let values = vec![
                            serde_json::Value::Null,
                            serde_json::Value::Bool(true),
                            serde_json::Value::String(String::new()),
                            serde_json::Value::String("  ".to_string()),
                            serde_json::Value::String("<not a node>".to_string()),
                            serde_json::json!(-1),
                            serde_json::json!(1e300),
                            serde_json::json!(u64::MAX),
                            serde_json::json!([]),
                            serde_json::json!({}),
                            serde_json::json!([1, 2, 3]),
                            serde_json::json!("../../etc"),
                        ];
                        (format!("{id}.{name}"), values)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut fired: Vec<String> = Vec::new();
        for id in command::ids() {
            for params in &hostile {
                let mut s = sample();
                // Either it refuses, or it succeeds — both fine. Panicking is not.
                if let Err(e) = s.execute(id, params) {
                    assert!(!e.to_string().is_empty(), "`{id}` produced an empty message");
                }
                if let Some(obj) = params.as_object() {
                    for key in obj.keys() {
                        fired.push(format!("{id}.{key}"));
                    }
                }
            }
        }

        for (label, values) in &per_parameter {
            let (id, name) = label.split_once('.').expect("a command and a parameter");
            for value in values {
                let mut obj = serde_json::Map::new();
                obj.insert(name.to_string(), value.clone());
                let params = serde_json::Value::Object(obj);
                let mut s = sample();
                if let Err(e) = s.execute(id, &params) {
                    assert!(!e.to_string().is_empty(), "`{label}` produced an empty message");
                }
            }
            fired.push(label.clone());
        }
        fired
    }

    #[test]
    fn hostile_params_never_panic() {
        // Every registered command, fired at every documented parameter name
        // with hostile values. Either it refuses or it succeeds; panicking is
        // not on the menu. This is the test `crates/core/src/params.rs` refers
        // to, and it is why "never index a parameter" is a rule with teeth.
        let fired = fire_hostile_params();
        assert!(!fired.is_empty(), "nothing was fired at all");
    }

    /// Every documented parameter is fired with a hostile value.
    ///
    /// The test above derives its values from the registry, so this is the
    /// assertion that the derivation still happens: a command that documents a
    /// parameter nobody fires is a parameter nobody has checked. §92.
    #[test]
    fn every_documented_parameter_is_fired_with_a_hostile_value() {
        let fired = fire_hostile_params();
        let mut missing: Vec<String> = Vec::new();
        for id in command::ids() {
            let spec = command::lookup(id).expect("a spec for every id");
            for name in spec.param_names() {
                if !fired.contains(&format!("{id}.{name}")) {
                    missing.push(format!("{id}.{name}"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "{} documented parameters are never fired with a hostile value: {missing:?}",
            missing.len()
        );
    }

    /// A command that leaves an invalid document is refused and rolled back.
    ///
    /// The engine's document and the file's document are the same thing, and
    /// the reader validates it. A plan that cannot be saved is a plan the app
    /// cannot open again — so the engine refuses the command, puts the
    /// document back, and says why. §50.
    #[test]
    fn a_command_that_leaves_an_invalid_document_is_refused_and_rolled_back() {
        let mut s = sample();
        let before = s.document().clone();
        let err = s.execute("test.invalidate", &json!({})).expect_err("the engine refuses it");
        assert!(err.is_internal(), "{err}");
        assert!(err.to_string().contains("file format refuses"), "the error must say why: {err}");
        assert_eq!(s.document(), &before, "the invalid document was not rolled back");
        assert!(s.document().validate().is_ok(), "the document is still invalid");
        assert!(!s.is_dirty(), "a refused command must not mark the project dirty");
    }

    /// No command may leave a document the file reader would refuse.
    ///
    /// The engine's document and the file's document are the same thing, and
    /// the reader validates it: *"the document is empty: start 0t is not
    /// before end 0t"*, *"two nodes share an id"*. A command that broke one of
    /// those rules would leave a plan that could not be saved — and, before
    /// the writer learned to refuse it, a plan that saved to a file the app
    /// then could not open. §50.
    #[test]
    fn no_command_leaves_a_document_the_file_would_refuse() {
        let cases = [
            json!({}),
            json!({ "node": 1, "path": "a/trigger/lead", "value": 1 }),
            json!({ "node": 1, "path": "trigger/lead", "value": 2 }),
            json!({ "start": 0, "end": 0 }),
            json!({ "name": "", "kind": "source" }),
        ];
        for id in command::ids() {
            for params in &cases {
                let mut s = sample();
                // Whatever the command does — succeed, refuse, or change
                // nothing — the document it leaves behind must be one the file
                // reader would accept.
                let _ = s.execute(id, params);
                if let Err(e) = s.document().validate() {
                    panic!("`{id}` left a document the file reader would refuse: {e}");
                }
            }
        }
    }

    #[test]
    fn a_panicking_command_leaves_the_document_unchanged_and_reports_internal() {
        for how in [0, 1] {
            let mut s = sample();
            let before = s.document().clone();
            let err =
                s.execute("test.panic", &json!({ "how": how })).expect_err("the test command always panics");
            assert!(err.is_internal(), "a panic is Internal, not a refusal");
            assert_eq!(err.kind(), "internal");
            assert_eq!(err.command(), Some("test.panic"));
            let msg = err.to_string();
            assert!(msg.contains("unchanged"), "{msg}");
            // The half-applied edit the `how == 1` arm makes before it panics
            // must be rolled back with everything else.
            assert_eq!(s.document(), &before, "`how = {how}` left a partial edit behind");
            assert!(!s.can_undo(), "a panic must not push history");
            assert!(!s.is_dirty(), "a panic must not mark the document dirty");
        }
    }

    #[test]
    fn a_recovered_panic_is_logged_as_a_failure() {
        let mut s = sample();
        let _ = s.execute("test.panic", &json!({ "how": 0 }));
        let last = s.log().entries().last().expect("one entry");
        assert!(!last.ok);
        assert_eq!(last.command, "test.panic");
    }

    #[test]
    fn the_session_survives_a_panic_and_keeps_working() {
        let mut s = sample();
        let _ = s.execute("test.panic", &json!({ "how": 1 }));
        // The whole point of the boundary: not just "no crash", but "still fine".
        s.execute("node.add", &json!({ "name": "After", "kind": "cue" })).expect("ok");
        assert_eq!(s.document().nodes.len(), 4);
        assert!(s.plan().is_some());
    }

    #[test]
    fn the_trace_records_every_command_for_a_crash_report() {
        let mut s = sample();
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        s.execute("prop.set", &json!({ "node": 4, "path": "amount", "value": 3 })).expect("ok");
        assert_eq!(s.trace().len(), 2);
        assert!(s.trace()[0].command.starts_with("node.add"), "{:?}", s.trace());
    }

    /// A command that panics is logged as a failure.
    ///
    /// The third of the three ways a command can end badly, and the third
    /// one whose log entry nothing checked: the log test used an unknown id
    /// and the refusal test used a command that refused, so the panic path's
    /// entry was never written by a test.
    #[test]
    fn a_panicking_command_is_logged_as_a_failure() {
        let mut s = sample();
        s.execute("test.panic", &json!({})).expect_err("recovered");
        let entries: &[LogEntry] = s.log().entries();
        assert_eq!(entries.len(), 1, "the panic was not logged");
        assert!(!entries[0].ok, "the panic was not logged as a failure");
        assert_eq!(entries[0].seq, 1);
    }

    /// A command that says it changes the document and does not is an error.
    ///
    /// The engine compares the document with the snapshot it took before the
    /// command, and a command that declares `mutates: true` and writes
    /// nothing is a bug: it would put a no-op step on the undo stack and
    /// mark the project dirty for no reason. The engine says so — with
    /// `EngineError::Internal` — and the pattern document promises it.
    ///
    /// **Nothing tested it.** No command in the tree declares `mutates: true`
    /// and changes nothing, so the check never fired; deleting it left every
    /// test in the workspace green. `test.noop` is the command that does it,
    /// registered only in test builds.
    #[test]
    fn a_command_that_says_it_changes_the_document_and_does_not_is_an_error() {
        let mut s = sample();
        let before = s.document().clone();
        let err = s.execute("test.noop", &json!({})).expect_err("refused");
        assert_eq!(err.kind(), "internal");
        assert!(err.to_string().contains("changed nothing"), "{err}");
        assert_eq!(s.document(), &before, "a no-op must not change the document");
        assert!(!s.can_undo(), "a no-op must not push an undo entry");
        assert!(!s.is_dirty(), "a no-op must not mark the project dirty");
    }

    /// A command that marks the project dirty and changes nothing leaves the
    /// flag where it was.
    ///
    /// The engine sets the dirty flag only after a command succeeds, so the
    /// rollbacks that put it back were no-ops: nothing in the tree could
    /// change the flag between the capture and the restore. This command can
    /// — it sets the flag, leaves the document alone, and succeeds — so the
    /// engine refuses it as a no-op and the rollback has to undo the flag.
    ///
    /// Without this test, the whole dirty-flag story was untestable, because
    /// the thing it protects against does not exist yet.
    #[test]
    fn a_command_that_marks_the_project_dirty_and_changes_nothing_leaves_the_flag_alone() {
        let mut s = sample();
        assert!(!s.is_dirty());
        let err = s.execute("test.dirty_noop", &json!({})).expect_err("refused");
        assert_eq!(err.kind(), "internal");
        assert!(!s.is_dirty(), "the rollback did not put the dirty flag back");
    }

    /// A command that refuses is logged as a failure.
    ///
    /// The log test used an unknown id, which fails in the lookup — before
    /// the snapshot, before the rollback, before the log entry. So the
    /// *refusal* path's log entry was never exercised, and deleting it left
    /// every test in the workspace green.
    #[test]
    fn a_refused_command_is_logged_as_a_failure() {
        let mut s = sample();
        // A command that refuses, rather than one that was never found: this
        // one reaches the rollback.
        s.execute("prop.set", &json!({ "node": 1, "path": "amount", "value": { "a": 1 } }))
            .expect_err("refused");
        let entries: &[LogEntry] = s.log().entries();
        assert_eq!(entries.len(), 1, "the refusal was not logged");
        assert!(!entries[0].ok, "the refusal was not logged as a failure");
        assert_eq!(entries[0].seq, 1);
    }

    #[test]
    fn the_log_records_successes_and_failures_in_order() {
        let mut s = sample();
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        let _ = s.execute("history.undo", &json!({})).expect("ok");
        let _ = s.execute("nope.nope", &json!({}));
        let entries: &[LogEntry] = s.log().entries();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].ok);
        assert!(entries[1].ok);
        assert!(!entries[2].ok);
        assert_eq!(entries[0].seq, 1);
        assert_eq!(entries[2].seq, 3);
    }

    /// A command whose parameters cannot be kept in the trace is refused.
    ///
    /// §70: the trace holds the **parsed** parameters, and the loop that keeps
    /// the trace inside its budget keeps the *last* step — so a step bigger
    /// than the whole budget was kept whole, at up to thirty-two times its
    /// text in memory, for the rest of the session.
    #[test]
    fn a_command_whose_parameters_do_not_fit_in_the_trace_is_refused() {
        let mut s = sample();
        // The control channel's own line limit is a megabyte of text, so a
        // payload this size is one an agent can send.
        let big = json!({
            "node": 1,
            "path": "amount",
            "value": "x".repeat(crate::MAX_TRACE_BYTES + 1_000),
        });
        let err = s.execute("prop.set", &big).expect_err("refused");
        let message = err.to_string();
        assert!(message.contains("trace"), "the refusal does not say why: {message}");
        assert!(
            s.trace().is_empty(),
            "the trace kept a step of {} bytes, and its budget is {}",
            s.trace().iter().map(step_size).sum::<usize>(),
            crate::MAX_TRACE_BYTES
        );
    }

    /// The trace never holds more than its budget, however many commands ran.
    #[test]
    fn the_trace_never_holds_more_than_its_budget() {
        let mut s = sample();
        for _ in 0..200 {
            let _ = s.execute("node.mute", &json!({ "node": 1 }));
        }
        let held: usize = s.trace().iter().map(step_size).sum();
        assert!(held <= crate::MAX_TRACE_BYTES, "the trace holds {held} bytes");
    }

    /// The trace's budget is derived from the memory it may hold.
    ///
    /// The trace holds parsed values, and a parsed JSON value costs up to
    /// [`craftkit_core::JSON_EXPANSION`] times its text, so a limit written in
    /// bytes of text has to be read as a limit in bytes of memory that many
    /// times larger. §70.
    #[test]
    fn the_trace_budget_is_derived_from_the_memory_budget() {
        assert_eq!(crate::MAX_TRACE_BYTES, crate::MAX_TRACE_MEMORY_BYTES / craftkit_core::JSON_EXPANSION);
        // The **source** is checked, and not only the value: 8 MiB divided by
        // 32 is 262 144, which is also 256 × 1024, and a number that happens
        // to be the same is not a derived number.
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("the engine's own source is readable");
        let line =
            source.lines().find(|l| l.contains("MAX_TRACE_BYTES: usize =")).expect("the constant is defined");
        assert!(
            line.contains("MAX_TRACE_MEMORY_BYTES") && line.contains("JSON_EXPANSION"),
            "the trace budget is not derived from the memory budget: {line}"
        );
    }
}
