//! The journal: what the session was asked to do, in a form a machine can
//! read again.
//!
//! # What it is
//!
//! The session keeps a **log** of strings for a person to read, and a
//! **history** of documents for a person to undo. Neither of those can be
//! run again: the log is English and the history is a state, not a set of
//! instructions.
//!
//! The **journal** is the third thing: every command that ran, with the
//! values it was given, exactly as they were given. It is a program:
//!
//! ```text
//! sound.track.add   { "name": "drums" }
//! sound.clip.add    { "track": 3, "start": 0, "length": 48000 }
//! ```
//!
//! # Why it matters
//!
//! An agent that drives a program wants to say *what it did* and *do it
//! again*: a macro that records what a person did, a script that is
//! replayed on another document, a test that reproduces a failure. The
//! sound application this port comes from keeps exactly this — a journal
//! of every command and its values — and calls it a journal, and it is
//! what its macro recording is made of.
//!
//! Here it is in the **SDK**, because it belongs to the session and not to
//! an application: every front end gets it, and the third door and the
//! second front end both read it.
//!
//! # The rules
//!
//! - only a command that **ran** is journaled: a refused command changed
//!   nothing, so there is nothing to replay;
//! - the values are the values the caller sent, not the values the
//!   command read out of them: the journal is what was asked for;
//! - the journal is **bounded**. It holds the last thousand entries, and
//!   the oldest go when it is full. A journal that grows for ever is a
//!   leak, and a leak in a session is a leak in a program that runs for
//!   days.

use craftkit_derive::Command;

use crate::{EngineError, Outcome, Session};

/// How many entries the journal holds.
///
/// A thousand is a lot of macro, and it is a number rather than a rule:
/// the journal is a convenience and not a history — the history is the
/// undo stack.
pub const JOURNAL_LIMIT: usize = 1024;

/// One command that ran, with the values it was given.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    /// The command's id.
    pub id: String,
    /// The values the caller sent, as they were sent.
    pub params: crate::serde_json::Value,
}

impl JournalEntry {
    /// The entry, as it is written.
    pub fn to_json(&self) -> crate::serde_json::Value {
        crate::serde_json::json!({ "id": self.id, "params": self.params })
    }
}

/// The journal, as a list of steps.
///
/// The shape [`JournalReplay`] runs: a list of `{"id": …, "params": …}`.
pub fn steps(entries: &[JournalEntry]) -> crate::serde_json::Value {
    crate::serde_json::Value::Array(entries.iter().map(|entry| entry.to_json()).collect())
}

/// The commands that ran.
#[derive(Command)]
#[command(id = "journal.list", area = "journal", icon = "list", no_journal)]
pub struct JournalList {}

impl JournalList {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let entries: Vec<crate::serde_json::Value> =
            session.journal().iter().map(|entry| entry.to_json()).collect();
        let count = entries.len();
        Ok(Outcome::new(format!("{count} command(s) in the journal"))
            .with_data(crate::serde_json::json!({ "entries": entries })))
    }
}

/// Forget the journal.
///
/// It does not change the document, so it is not an undo step: what it
/// forgets is a record, and the record is not the document.
#[derive(Command)]
#[command(id = "journal.clear", area = "journal", icon = "list", no_journal)]
pub struct JournalClear {}

impl JournalClear {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let count = session.journal().len();
        session.clear_journal();
        Ok(Outcome::new(format!("forgot {count} command(s)"))
            .with_data(crate::serde_json::json!({ "forgotten": count })))
    }
}

/// Run a list of commands again.
///
/// The steps are the journal's own shape: a list of `{"id": …, "params":
/// …}`. They run **as one undo step**, so that a replay is undone the way
/// it was done, and a step that fails puts the document back the way it
/// was — the same promise every command makes, over a list of them.
#[derive(Command)]
#[command(id = "journal.replay", area = "journal", icon = "list", mutates, undoable, no_journal)]
pub struct JournalReplay {
    /// The steps: a list of `{"id": …, "params": …}`.
    pub steps: crate::serde_json::Value,
}

impl JournalReplay {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let steps = self
            .steps
            .as_array()
            .ok_or_else(|| EngineError::Failed {
                id: "journal.replay".to_string(),
                message: "the steps are a list".to_string(),
            })?
            .clone();
        let mut ran = 0usize;
        session.batch(|session| {
            for step in &steps {
                let id = step.get("id").and_then(|id| id.as_str()).ok_or_else(|| EngineError::Failed {
                    id: "journal.replay".to_string(),
                    message: "a step has no id".to_string(),
                })?;
                let params = step.get("params").cloned().unwrap_or(crate::serde_json::Value::Null);
                session.execute(id, &params)?;
                ran += 1;
            }
            Ok(())
        })?;
        Ok(Outcome::changed(format!("ran {ran} step(s)"))
            .with_data(crate::serde_json::json!({ "steps": ran })))
    }
}
