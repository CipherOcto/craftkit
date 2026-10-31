//! The **question**: a command that asks the host while it runs.
//!
//! # What this is
//!
//! The worked example of the resumable command ([`crate::resume`]): a
//! command that **asks** the host something while it runs, waits for the
//! answer, and goes on.
//!
//! The answer lands in the document, as a **note**: a node of the kind
//! `core.note`, holding the question and the answer in its payload. So a
//! document can hold a question and what was said about it, and a reader
//! of the document later can see both.
//!
//! # Why the answer is a parameter too
//!
//! The command takes the answer as an **optional parameter** as well as
//! asking for it. That is not two ways to do the same thing: it is the
//! difference between a caller that **knows** the answer and a host that
//! has to be asked. A caller that knows it passes it and the command never
//! asks. A caller that does not know it leaves it out, the command asks,
//! and the answer comes back through the seam.
//!
//! Over a protocol the second is the useful one, and it is why the
//! question's answer is also a declared parameter: the host calls the
//! command again with the answer in it, and the seam knows what to do with
//! it.

use craftkit_derive::{Command, NodeKind};

use crate::document::{Canvas, Node, Point};
use crate::params::Doc;
use crate::prompt::Question;
use crate::{EngineError, Outcome, Session, Text};

/// A **note**: a question and the answer to it.
///
/// It is the core's kind, and it is the plain one: it draws nothing and it
/// is hit by nothing, and what it *means* is in its payload.
#[derive(NodeKind)]
#[node_kind(kind = "core.note", label = "Note", layer = "core")]
pub struct NoteKind;

impl NoteKind {
    /// How a note is drawn.
    ///
    /// Nothing.
    fn paint(_node: &Node, _document: &crate::Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a note.
    ///
    /// Nothing.
    fn hit(_node: &Node, _point: Point) -> bool {
        false
    }
}

/// **Ask** the host a question, and keep the answer.
///
/// The command asks while it runs: if the caller gave the answer, it is
/// used; if not, the host is asked, and the command waits for the answer
/// and goes on. See [`crate::resume`].
#[derive(Command)]
#[command(id = "doc.ask", area = "doc", icon = "question", menu("File"), mutates, undoable)]
pub struct Ask {
    /// The question to ask.
    pub question: Text,
    /// The answer, if the caller already has it.
    pub answer: Option<Text>,
    /// What to call the note.
    pub name: Option<Text>,
}

impl Ask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let question = self.question.0.trim().to_string();
        if question.is_empty() {
            return Err(EngineError::Failed {
                id: "doc.ask".to_string(),
                message: "a question has to say something".to_string(),
            });
        }
        // The answer: the caller's if there is one, and the host's if
        // there is not.
        let answer = match &self.answer {
            Some(answer) => answer.0.clone(),
            None => {
                let wanted = Question {
                    name: "answer".to_string(),
                    kind: Doc::Text,
                    help: "The answer to the question.".to_string(),
                };
                let answer = session.ask(&wanted)?;
                let Some(answer) = answer.as_str() else {
                    return Err(EngineError::Failed {
                        id: "doc.ask".to_string(),
                        message: "the answer is a text".to_string(),
                    });
                };
                answer.to_string()
            }
        };
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => question.clone(),
        };
        let payload = crate::serde_json::json!({
            "question": question,
            "answer": answer,
        });
        let id = session.document_mut()?.add_node_with_under(None, name.clone(), "core.note", None, payload);
        Ok(Outcome::changed(format!("asked `{question}`")).with_data(crate::serde_json::json!({
            "id": id,
            "name": name,
        })))
    }
}
