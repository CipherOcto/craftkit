//! The history: a **tree** of states, not a line of them.
//!
//! # The problem this solves
//!
//! The history used to be a **stack**: every change pushed the document as
//! it was, and undo popped one entry off. That is a line, and a line can
//! only be walked backwards.
//!
//! A real editing session is not a line. A person undoes three steps, sees
//! that the third one was wrong, changes something else, and wants the
//! three steps to still be there — one undo away, on the other side of the
//! change they just made.
//!
//! That is a **tree**: every state has a parent, and a state can have
//! several children. Undo walks to the parent, redo walks to a child, and
//! a change made after an undo starts a **new branch** instead of throwing
//! the old one away.
//!
//! The port of a compositing application is what found this: its history
//! is a tree of snapshots, and the tree is what a person expects when
//! they undo something and then change their mind about the undo.
//!
//! # The rules
//!
//! - the **root** is the document before anything happened;
//! - a command that changed the document and is undoable adds a **child**
//!   of the current state, and the current state becomes that child;
//! - **undo** moves to the parent, **redo** moves to a child, and a
//!   change made after an undo adds a child to the state that was undone
//!   to — the old child is still there, on its own branch;
//! - the tree is **bounded**: it holds a fixed number of states, and the
//!   oldest branch that is not the current one is dropped when it is full.
//!   A history that grows for ever is a leak.
//!
//! The tree holds a whole document per state, so it is the same cost the
//! stack was: one document per step. The tree is a different *shape*, not
//! a cheaper one.

use craftkit_derive::Command;

use crate::document::Document;
use crate::select::Selected;
use crate::{EngineError, Outcome, Session};

/// How many states the history holds.
///
/// Two hundred and fifty-six is a number rather than a rule: it is enough
/// for a long session and small enough that the memory is not a surprise.
pub const HISTORY_LIMIT: usize = 256;

/// How many **documents' worth** of memory the history may hold.
///
/// The count above is a count, and a count does not say how much memory
/// what it counted holds: two hundred and fifty-six states of a thirty
/// megabyte document are seven and a half gigabytes, and a count that
/// says nothing about the memory is not a bound.
///
/// So the history has a budget in **bytes**, and the budget is derived
/// from the document's own budget rather than written out: a budget and
/// the thing it bounds must not drift apart.
pub const HISTORY_EXPANSION: usize = 2;

/// How much memory the history may hold.
///
/// The same number the family uses, and for the same reason: a session
/// has to be able to undo more than one step, and the history is the one
/// part of a session that can be given back.
pub const MAX_HISTORY_BYTES: usize = crate::document::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION;

/// One state in the history.
///
/// A state is a whole session state: the document, what was selected, and
/// what was in the hand. Undo puts all three back, because a person who
/// undoes an edit expects the selection to be what it was.
#[derive(Debug, Clone)]
pub struct HistoryNode {
    /// The state's id. The ids are not the positions: a branch is removed
    /// from the middle when the tree is full.
    pub id: u64,
    /// The command that produced this state.
    ///
    /// Empty at the root: nothing produced it.
    pub command: String,
    /// The document, as it was here.
    pub document: Document,
    /// What was selected, here.
    pub selection: Vec<Selected>,
    /// What was in the hand, here.
    pub tool: Option<String>,
    /// The state this one came from.
    pub parent: Option<u64>,
    /// The states that came from this one.
    ///
    /// More than one means a branch: the work was undone and done
    /// differently.
    pub children: Vec<u64>,
    /// What this state costs, in bytes.
    ///
    /// It is measured when the state is made and not again: a pruning
    /// that has to measure a document to know what to drop is a pruning
    /// that walks the whole tree, and the budget is checked on every
    /// push.
    pub bytes: usize,
}

impl HistoryNode {
    /// The state, as it is written.
    ///
    /// The document is left out: a tree of whole documents is not a thing
    /// to print, and the caller that wants the document has it.
    pub fn to_json(&self) -> crate::serde_json::Value {
        crate::serde_json::json!({
            "id": self.id,
            "command": self.command,
            "parent": self.parent,
            "children": self.children,
        })
    }
}

/// The history: a tree of states.
#[derive(Debug, Clone)]
pub struct History {
    nodes: Vec<HistoryNode>,
    current: u64,
    next_id: u64,
    /// What the whole tree costs, in bytes.
    ///
    /// It is the sum of the states' own costs, and it is a number that
    /// is **kept** rather than computed: the budget is checked on every
    /// push, and a budget that has to be measured by walking the tree is
    /// a budget that is checked too late.
    bytes: usize,
    /// How much memory the tree may hold.
    budget: usize,
}

impl History {
    /// A history over a document that has not changed yet.
    ///
    /// The root is the document as it is, and the root has no parent.
    pub fn new(document: Document) -> Self {
        let bytes = document.estimated_bytes();
        let root = HistoryNode {
            id: 0,
            command: String::new(),
            document,
            selection: Vec::new(),
            tool: None,
            parent: None,
            children: Vec::new(),
            bytes,
        };
        History { nodes: vec![root], current: 0, next_id: 1, bytes, budget: MAX_HISTORY_BYTES }
    }

    /// A history with a budget of its own.
    ///
    /// The budget is **the caller's**: the tree's default is the
    /// family's number, and a program that knows better says so here.
    pub fn with_budget(document: Document, budget: usize) -> Self {
        let mut history = History::new(document);
        history.budget = budget;
        history
    }

    /// What the tree costs, in bytes.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// How much memory the tree may hold.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Whether nothing has happened yet.
    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// How many steps the history holds.
    pub fn len(&self) -> usize {
        self.nodes.len().saturating_sub(1)
    }

    /// The states, in the order they were made.
    pub fn nodes(&self) -> &[HistoryNode] {
        &self.nodes
    }

    /// The state that is current.
    pub fn current(&self) -> u64 {
        self.current
    }

    /// The state that is current.
    pub fn current_node(&self) -> &HistoryNode {
        self.node(self.current).unwrap_or(&self.nodes[0])
    }

    /// The state with an id.
    pub fn node(&self, id: u64) -> Option<&HistoryNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// The states that come from the current one.
    ///
    /// More than one means there is a branch.
    pub fn branches(&self) -> Vec<u64> {
        self.current_node().children.clone()
    }

    /// The states from the root to the current one.
    pub fn path(&self) -> Vec<u64> {
        let mut path = Vec::new();
        let mut here = Some(self.current);
        while let Some(id) = here {
            path.push(id);
            here = self.node(id).and_then(|node| node.parent);
        }
        path.reverse();
        path
    }

    /// The parent of the current state, if there is one.
    pub fn parent(&self) -> Option<u64> {
        self.current_node().parent
    }

    /// The children of the current state.
    pub fn children(&self) -> Vec<u64> {
        self.current_node().children.clone()
    }

    /// Add a state under the current one, and make it current.
    ///
    /// The new state holds the document as it is now. The state it comes
    /// from keeps the document as it was — which is the whole point of an
    /// undo step.
    pub fn push(
        &mut self,
        command: &str,
        document: Document,
        selection: Vec<Selected>,
        tool: Option<String>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let parent = self.current;
        let bytes = document.estimated_bytes();
        let node = HistoryNode {
            id,
            command: command.to_string(),
            document,
            selection,
            tool,
            parent: Some(parent),
            children: Vec::new(),
            bytes,
        };
        self.nodes.push(node);
        if let Some(parent) = self.node_mut(parent) {
            parent.children.push(id);
        }
        self.current = id;
        self.bytes += bytes;
        self.prune();
        id
    }

    /// Put the current state's document, selection and tool back.
    ///
    /// A command that changed the document but is not undoable leaves the
    /// state it was in: the state is what the document is, and the
    /// document changed.
    pub fn set_current(&mut self, document: Document, selection: Vec<Selected>, tool: Option<String>) {
        let bytes = document.estimated_bytes();
        let before = self.node(self.current).map(|node| node.bytes).unwrap_or(0);
        // The state costs what it costs now, not what it cost when it
        // was made: the budget is the tree's memory, and a stale number
        // is not the tree's memory.
        self.bytes = self.bytes.saturating_sub(before) + bytes;
        if let Some(node) = self.node_mut(self.current) {
            node.bytes = bytes;
            node.document = document;
            node.selection = selection;
            node.tool = tool;
        }
    }

    /// Put the current state's selection and tool back.
    ///
    /// Undo puts back the selection and the tool as they were **before**
    /// the command, and the state that was undone to is the one that holds
    /// them.
    pub fn set_current_selection(&mut self, selection: &[Selected], tool: &Option<String>) {
        if let Some(node) = self.node_mut(self.current) {
            node.selection = selection.to_vec();
            node.tool = tool.clone();
        }
    }

    /// Move to the parent of the current state.
    ///
    /// Gives back the new current state, or `None` when the current state
    /// is the root and there is nowhere to go.
    pub fn step_back(&mut self) -> Option<u64> {
        let parent = self.parent()?;
        self.current = parent;
        Some(parent)
    }

    /// Move to a child of the current state.
    ///
    /// Without a branch, the newest child is the one that is taken: it is
    /// the work that was done last. With a branch, that one is taken.
    pub fn step_forward(&mut self, branch: Option<u64>) -> Option<u64> {
        let children = self.children();
        let target = match branch {
            Some(branch) => {
                if children.contains(&branch) {
                    branch
                } else {
                    return None;
                }
            }
            None => *children.last()?,
        };
        self.current = target;
        Some(target)
    }

    /// Move to any state in the tree.
    ///
    /// This is the door into a branch: a person who undoes three steps and
    /// then wants the state at the other end of the branch can go there.
    pub fn go_to(&mut self, id: u64) -> Option<u64> {
        self.node(id)?;
        self.current = id;
        Some(id)
    }

    /// The node with an id, to change it.
    fn node_mut(&mut self, id: u64) -> Option<&mut HistoryNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// Drop the oldest states until the tree is inside a budget.
    ///
    /// The budget is **the caller's** — the session has one of its own
    /// that covers everything the session holds, and the tree is the
    /// part of it that gives way last.
    ///
    /// The state that is current is never dropped: it holds the document
    /// the session is working on, and a bound that drops the working
    /// document is not a bound, it is a data loss.
    pub fn prune_to(&mut self, budget: usize) {
        while self.bytes > budget {
            let Some(victim) = self.nodes.iter().find(|node| node.id != self.current).map(|node| node.id)
            else {
                break;
            };
            self.drop(victim);
        }
    }

    /// Drop one state, and take its bytes off the count.
    fn drop(&mut self, victim: u64) {
        let parent = self.node(victim).and_then(|node| node.parent);
        let children = self.node(victim).map(|node| node.children.clone()).unwrap_or_default();
        let bytes = self.node(victim).map(|node| node.bytes).unwrap_or(0);
        if let Some(parent) = parent.and_then(|parent| self.node_mut(parent)) {
            parent.children.retain(|child| *child != victim);
            parent.children.extend(children.iter().copied());
        }
        for child in &children {
            if let Some(child) = self.node_mut(*child) {
                child.parent = parent;
            }
        }
        self.nodes.retain(|node| node.id != victim);
        self.bytes = self.bytes.saturating_sub(bytes);
    }

    /// Drop the oldest states when the tree is full.
    ///
    /// The oldest state that is not the current one goes, and what came
    /// from it takes its place: the tree stays a tree, and the state that
    /// is current stays current. The cost is that the history forgets how
    /// far back it goes, which is what a bound is.
    ///
    /// Two bounds bite here: the **count** of states and the **bytes**
    /// they cost. A tree of small documents is bounded by the count, and
    /// a tree of large ones by the bytes — and a tree that is over the
    /// bytes is pruned even when it is under the count, because the bytes
    /// are what the memory is.
    fn prune(&mut self) {
        while self.nodes.len() > HISTORY_LIMIT || self.bytes > self.budget {
            let Some(victim) = self.nodes.iter().find(|node| node.id != self.current).map(|node| node.id)
            else {
                // Only the current state is left: it cannot be dropped
                // without losing the document, and the document is not
                // the history's to lose.
                break;
            };
            self.drop(victim);
        }
    }
}

/// Put the document back one step.
///
/// The step is a move along the tree: to the parent of the state that is
/// current. The branch that was left is still there, and redo goes back
/// into it.
pub fn undo(session: &mut Session) -> Result<Outcome, EngineError> {
    let undone = session.history().current_node().command.clone();
    let Some(parent) = session.history_mut().step_back() else {
        return Err(EngineError::Failed {
            id: "history.undo".to_string(),
            message: "there is nothing to undo".to_string(),
        });
    };
    session.restore_state(parent);
    Ok(Outcome::changed(format!("undid `{undone}`")).with_data(crate::serde_json::json!({ "state": parent })))
}

/// Go forward into a branch.
///
/// Without a branch, the newest one is taken.
pub fn redo(session: &mut Session, branch: Option<u64>) -> Result<Outcome, EngineError> {
    let children = session.history().children();
    if children.is_empty() {
        return Err(EngineError::Failed {
            id: "history.redo".to_string(),
            message: "there is nothing to redo".to_string(),
        });
    }
    let Some(target) = session.history_mut().step_forward(branch) else {
        return Err(EngineError::Failed {
            id: "history.redo".to_string(),
            message: "that is not a branch of the current state".to_string(),
        });
    };
    let command = session.history().current_node().command.clone();
    session.restore_state(target);
    Ok(Outcome::changed(format!("redid `{command}`"))
        .with_data(crate::serde_json::json!({ "state": target })))
}

/// Put the document back one step.
///
/// It is a command like any other: a person clicks it, an agent calls it,
/// and the toolbar and the script reach the same thing.
#[derive(Command)]
#[command(id = "history.undo", area = "history", icon = "undo", menu("Edit"), mutates, no_journal)]
pub struct HistoryUndo {}

impl HistoryUndo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        undo(session)
    }
}

/// Go forward into a branch.
///
/// It is **not** an undo step: going forward does not make a new state, it
/// moves to a state that is already there.
#[derive(Command)]
#[command(id = "history.redo", area = "history", icon = "redo", menu("Edit"), mutates, no_journal)]
pub struct HistoryRedo {
    /// The branch to take, if not the newest one.
    pub branch: Option<crate::serde_json::Value>,
}

impl HistoryRedo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let branch = match self.branch.as_ref() {
            None | Some(crate::serde_json::Value::Null) => None,
            Some(value) => {
                let id = value.as_u64().ok_or_else(|| EngineError::Param {
                    id: "history.redo".to_string(),
                    source: crate::ParamError::new("branch", "a branch is a state id"),
                })?;
                Some(id)
            }
        };
        redo(session, branch)
    }
}

/// Go to a state in the tree.
///
/// The door into a branch: it takes the state that is current and puts it
/// where it was, and the work that was done after it is kept.
#[derive(Command)]
#[command(id = "history.go", area = "history", icon = "redo", menu("Edit"), mutates, no_journal)]
pub struct HistoryGo {
    /// The state to go to.
    pub state: crate::serde_json::Value,
}

impl HistoryGo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.state.as_u64().ok_or_else(|| EngineError::Param {
            id: "history.go".to_string(),
            source: crate::ParamError::new("state", "a state is a number"),
        })?;
        let Some(target) = session.history_mut().go_to(id) else {
            return Err(EngineError::Failed {
                id: "history.go".to_string(),
                message: format!("there is no state {id} in the history"),
            });
        };
        session.restore_state(target);
        Ok(Outcome::changed(format!("went to state {target}"))
            .with_data(crate::serde_json::json!({ "state": target })))
    }
}

/// The tree of states.
///
/// It changes nothing, so it is not an undo step, and it is not in the
/// journal: a journal of a tree is not a program.
#[derive(Command)]
#[command(id = "history.tree", area = "history", icon = "list", no_journal)]
pub struct HistoryTree {}

impl HistoryTree {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let history = session.history();
        let states: Vec<crate::serde_json::Value> =
            history.nodes().iter().map(|node| node.to_json()).collect();
        let branches = history.branches();
        Ok(Outcome::new(format!("{} state(s) in the history", history.len())).with_data(
            crate::serde_json::json!({
                "states": states,
                "current": history.current(),
                "branches": branches,
                "path": history.path(),
            }),
        ))
    }
}

/// **Undo** the last change.
///
/// The undo is a **command**, and not an exception to the rule: a
/// person and an agent both reach the history this way, and a front end
/// that opened the session's own door would be a front end with a
/// second way into the design.
///
/// It is not **undoable** itself: it moves along the tree of states and
/// it adds nothing to it.
#[derive(Command)]
#[command(id = "doc.undo", area = "doc", icon = "undo", menu("Edit"), mutates)]
pub struct Undo {}

impl Undo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        session.undo()
    }
}

/// **Redo**: go forward into a branch of the history.
///
/// The **newest** branch is taken. A change made after an undo starts a
/// **new** branch, and the old one is still there — the session can go
/// into it, and this command does not yet say which one: a branch is a
/// number, and a number is not a parameter the kit's own types can
/// carry.
#[derive(Command)]
#[command(id = "doc.redo", area = "doc", icon = "redo", menu("Edit"), mutates)]
pub struct Redo {}

impl Redo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        session.redo(None)
    }
}
