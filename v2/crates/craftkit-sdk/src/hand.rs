//! The tool in the hand: what the user is holding.
//!
//! # What it is
//!
//! A **tool** is a gesture that runs a command — it is a contribution, and
//! the ecosystem checks that the command it names is there. But a tree of
//! tools is not a person: a person holds **one** of them at a time, and
//! which one is the session's state and not the document's.
//!
//! So the session holds it, and the commands that change it are here,
//! because they belong to the session and every front end should have
//! the same ones.
//!
//! # Why it is not in the document
//!
//! Because a document does not say which tool is in the hand. Two people
//! editing the same document may hold different tools, and neither of
//! them has changed the document. The selection is the same kind of
//! thing, and it is in the same place for the same reason.

use craftkit_derive::Command;

use crate::{EngineError, Outcome, Session, Text};

/// Put a tool in the hand.
///
/// The tool is named by its id, and the id is a **string**: the tools are
/// contributions and the core does not know what they are.
#[derive(Command)]
#[command(id = "tool.hold", area = "tool", icon = "tool", menu("Tools"))]
pub struct HoldTool {
    /// The tool's id.
    pub tool: Text,
}

impl HoldTool {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let tool = self.tool.0.clone();
        if tool.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "tool.hold".to_string(),
                message: "a tool has a name".to_string(),
            });
        }
        session.hold(tool.clone());
        Ok(Outcome::new(format!("the `{tool}` tool is in the hand"))
            .with_data(crate::serde_json::json!({ "tool": tool })))
    }
}

/// Take the tool out of the hand.
#[derive(Command)]
#[command(id = "tool.release", area = "tool", icon = "tool", menu("Tools"))]
pub struct ReleaseTool {}

impl ReleaseTool {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let held = session.tool().map(str::to_string);
        session.release();
        Ok(Outcome::new(match &held {
            Some(tool) => format!("the `{tool}` tool is out of the hand"),
            None => "there was no tool in the hand".to_string(),
        })
        .with_data(crate::serde_json::json!({ "tool": held })))
    }
}

/// The tool in the hand.
///
/// A query: it does not change anything.
#[derive(Command)]
#[command(id = "tool.held", area = "tool", icon = "tool", menu("Tools"))]
pub struct HeldTool {}

impl HeldTool {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let held = session.tool().map(str::to_string);
        Ok(Outcome::new(match &held {
            Some(tool) => format!("the `{tool}` tool is in the hand"),
            None => "there is no tool in the hand".to_string(),
        })
        .with_data(crate::serde_json::json!({ "tool": held })))
    }
}
