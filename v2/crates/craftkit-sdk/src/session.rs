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
//! 4. the document is snapshotted, the command runs, and a failure puts the
//!    document back.
//!
//! Nothing else can change the document, because the session's mutable
//! accessor is not public and the commands run inside it.

use serde_json::Value as Json;

use crate::document::Document;
use crate::error::{EngineError, Outcome};
use crate::params::Params;
use crate::registry::Registry;

/// One step of the history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// The command that ran.
    pub id: String,
    /// The document as it was before.
    pub document: Document,
}

/// The document, the commands, and the history of what happened.
#[derive(Debug, Clone)]
pub struct Session {
    registry: Registry,
    document: Document,
    history: Vec<HistoryEntry>,
    log: Vec<String>,
}

impl Session {
    /// A session over a registry.
    pub fn new(registry: Registry) -> Self {
        Session { registry, document: Document::new("untitled"), history: Vec::new(), log: Vec::new() }
    }

    /// The commands this session knows.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The document, to read.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The document, to change.
    ///
    /// Public because a command has to be able to change what it is about —
    /// the command is the thing that is allowed to write, and it writes
    /// here, inside the snapshot the engine took.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
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
        let Some(spec) = self.registry.get(id).copied() else {
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
        let snapshot = spec.mutates.then(|| self.document.clone());
        let result = (spec.run)(self, &params);
        match result {
            Ok(outcome) => {
                if let (true, Some(document)) = (spec.mutates, snapshot) {
                    self.history.push(HistoryEntry { id: id.to_string(), document });
                }
                self.log.push(format!("{id}: {}", outcome.message));
                Ok(outcome)
            }
            Err(error) => {
                if let Some(document) = snapshot {
                    self.document = document;
                }
                self.log.push(format!("{id}: refused ({error})"));
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
        Ok(Outcome::new(format!("undid `{}`", entry.id)))
    }
}
