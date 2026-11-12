//! The **resumable command**: a command that asks while it runs.
//!
//! # What was missing
//!
//! The prompt seam (§45) gives the host a turn **before** a command runs: the
//! command's *declared* parameters that the caller did not give are asked
//! for, and the command runs once it has everything.
//!
//! The survey of the family's applications (§40, §43) found the other half.
//! CADCraft's registry has `interactive: Option<Factory>`: a command may
//! want something that only it knows it wants — a point on the drawing, a
//! radius, the name of the layer to put the thing on — and it asks for it
//! **while it runs**. Its MCP server answers that by exposing the prompt
//! itself as a tool, so an agent types at the prompt exactly as a person
//! would.
//!
//! A kit whose commands never ask cannot be driven through the commands
//! that ask, and that was written down as a cost (§44) rather than decided
//! (§45). This module is the decision, taken the **constrained** way.
//!
//! # The rule
//!
//! A command may **ask**, and the host may **answer**, and the command
//! carries on. Three rules make it safe, and all three are enforced by the
//! session rather than by a convention:
//!
//! 1. **A command that asks must ask before it changes the document.**
//!    The session compares the document with the snapshot it took before
//!    the run; if the command changed something and *then* asked, the
//!    change is put back and the command is refused. A command that asks
//!    is therefore **restarted** when the answer arrives, and restarting
//!    is only safe because nothing was changed.
//! 2. **A command may ask a bounded number of times.** Thirty-two
//!    questions, and thirty-two answers: a command that asks for ever is
//!    refused rather than allowed to spin.
//! 3. **A pending command has changed nothing, so dropping it costs
//!    nothing.** Another command may be run while one is waiting; the
//!    waiting one is dropped, and the document is exactly as it was.
//!
//! # What it looks like
//!
//! ```ignore
//! fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
//!     let answer = session.ask(&Question { name: "layer".to_string(), .. })?;
//!     // … the answer is here, and the command goes on to do its work.
//!     Ok(Outcome::changed("done"))
//! }
//! ```
//!
//! The `?` is the whole of it: [`crate::EngineError::NeedsInput`] travels
//! out of the command, and the session turns it into an outcome that says
//! what is wanted. When the host answers, the command runs again, and this
//! time the answer is waiting for it.

use serde_json::Value as Json;

use crate::prompt::Question;

/// The most questions one command may ask.
///
/// A command that asks for ever is a command that never finishes, and the
/// host is the one that has to notice. The limit is here so that it is
/// noticed by the kit.
pub const MAX_QUESTIONS: usize = 32;

/// The most times one command may be answered.
///
/// The same bound from the other side: a command and a host that keep
/// answering each other is a loop, and a loop is not a result.
pub const MAX_RESUMES: usize = 32;

/// A command that is waiting for an answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    /// The command that is waiting.
    pub id: String,
    /// The parameters it was given the first time.
    pub params: Json,
    /// The answers it has been given, by question name.
    pub answers: serde_json::Map<String, Json>,
    /// The questions it has asked, in the order it asked them.
    pub asked: Vec<Question>,
    /// How many times it has been answered.
    pub resumes: usize,
}

impl Pending {
    /// A pending command for this id and these parameters.
    pub fn new(id: impl Into<String>, params: Json) -> Pending {
        Pending { id: id.into(), params, answers: serde_json::Map::new(), asked: Vec::new(), resumes: 0 }
    }

    /// The answer to this question, if it has one.
    pub fn answer(&self, name: &str) -> Option<Json> {
        self.answers.get(name).cloned()
    }

    /// The question it is waiting for: the last one it asked.
    pub fn question(&self) -> Option<&Question> {
        self.asked.last()
    }

    /// Whether another question may be asked.
    pub fn may_ask(&self) -> bool {
        self.asked.len() < MAX_QUESTIONS
    }

    /// Whether another answer may be given.
    pub fn may_resume(&self) -> bool {
        self.resumes < MAX_RESUMES
    }
}
