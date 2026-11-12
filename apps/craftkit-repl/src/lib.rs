//! The **terminal door**: a person drives the kit from a prompt.
//!
//! # Why it exists
//!
//! The kit has three doors for other programs — the MCP server, the
//! control channel and the bridge — and **none of them is for a person**.
//! A person cannot speak MCP, and a prompt is the oldest and the
//! simplest of the kit's doors: it is a line of text, and the kit's own
//! vocabulary is a line of text.
//!
//! So the door is a **prompt**: a command's id and its parameters, and
//! the answer. Everything else — the commands, the bounds, the undo,
//! the journal — is the kit's own and is reached the same way a program
//! reaches it.
//!
//! # What it does
//!
//! - `command.id` runs the command with no parameters;
//! - `command.id {"name": "value"}` runs it with those;
//! - `help` lists the commands, by area;
//! - `commands` lists their ids;
//! - `plan` says what the document means, as the kinds say it;
//! - `memory` says what the session is holding;
//! - `snapshot <path>` writes a picture of the document;
//! - `quit` ends the conversation.
//!
//! # What it does not do
//!
//! It does not edit the line, and it does not complete it: the prompt
//! is a prompt and not a shell.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_sdk::ecosystem::Ecosystem;
use craftkit_sdk::session::Memory;
use craftkit_sdk::{EngineError, Session};

/// The most a line may be, in bytes.
///
/// A line comes from a person, and a person can paste a whole file
/// into a prompt. The line is **bounded**, and a line that is too long
/// is refused rather than read.
///
/// The bound is **private**: it is this door's own number, and it is
/// named in the tree's README. A public bound is a promise anybody
/// else may rely on, and this one is not — the person at the prompt
/// may paste anything, and the door stops.
const MAX_LINE_BYTES: usize = 64 * 1024;

/// What the door said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The lines to print.
    Lines(Vec<String>),
    /// The conversation is over.
    Quit,
}

/// The door: a session and the words of it.
pub struct Repl {
    session: Session,
    ecosystem: Ecosystem,
}

impl Repl {
    /// A door onto that ecosystem.
    pub fn new(ecosystem: Ecosystem) -> Self {
        let session = ecosystem.session();
        Repl { session, ecosystem }
    }

    /// The session the door drives.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The session the door drives, and may be asked through.
    ///
    /// A caller that wants to run a command of the kit's own reaches
    /// the **registry** through here — the same way the door does — and
    /// not the document.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// What the session is holding.
    pub fn memory(&self) -> Memory {
        self.session.memory()
    }

    /// Read a line and run it.
    ///
    /// The door **owns** the line it reads: a person may paste anything
    /// into a prompt, and the door reads what it was asked for and
    /// stops. A line over the bound is refused, and the stream is left
    /// broken — the door cannot know where the next line starts.
    ///
    /// `Ok(None)` is the end of the conversation.
    pub fn read_line(&mut self, reader: &mut impl std::io::BufRead) -> Result<Option<Reply>, String> {
        let Some(line) = craftkit_sdk::lines::read_line_bounded(reader, MAX_LINE_BYTES)? else {
            return Ok(None);
        };
        Ok(Some(self.run(&line)))
    }

    /// Run a line, and say what happened.
    pub fn run(&mut self, line: &str) -> Reply {
        let line = line.trim();
        if line.is_empty() {
            return Reply::Lines(Vec::new());
        }
        let (command, rest) = match line.split_once(char::is_whitespace) {
            Some((command, rest)) => (command, rest.trim()),
            None => (line, ""),
        };
        match command {
            "quit" => Reply::Quit,
            "help" => Reply::Lines(self.help()),
            "commands" => Reply::Lines(self.commands()),
            "plan" => Reply::Lines(self.plan()),
            "memory" => Reply::Lines(self.memory_lines()),
            "snapshot" => Reply::Lines(self.snapshot(rest)),
            _ => Reply::Lines(self.run_command(command, rest)),
        }
    }

    /// The commands, by area.
    fn help(&self) -> Vec<String> {
        let mut out = Vec::new();
        for entry in self.ecosystem.projection().menu(&["command", "effect"]) {
            out.push(format!("{:<10} {:<24} {}", entry.area, entry.id, entry.doc));
        }
        out
    }

    /// The commands' ids.
    fn commands(&self) -> Vec<String> {
        self.ecosystem.commands.ids().into_iter().map(|id| id.to_string()).collect()
    }

    /// What the document means.
    ///
    /// The plan is the **command's** work and not the door's: the door
    /// asks `doc.plan` for it, exactly as the MCP server and the bridge
    /// do, so every door says the same thing about the same document.
    fn plan(&mut self) -> Vec<String> {
        let Ok(outcome) = self.execute("doc.plan", "{}") else {
            return vec!["the plan could not be made".to_string()];
        };
        let mut out = Vec::new();
        let Some(items) = outcome.data.get("items").and_then(|items| items.as_array()) else {
            return vec![outcome.message];
        };
        for item in items {
            let kind = item.get("kind").and_then(|kind| kind.as_str()).unwrap_or("?");
            let node = item.get("node").and_then(|node| node.as_i64()).unwrap_or_default();
            let value = item.get("value").map(|value| value.to_string()).unwrap_or_default();
            out.push(format!("{kind} {node} {value}"));
        }
        out
    }

    /// What the session is holding.
    ///
    /// The memory is the **command's** work and not the door's: the door
    /// asks `session.memory` for it, so the number it shows is the number
    /// the rest of the kit shows.
    fn memory_lines(&mut self) -> Vec<String> {
        let Ok(outcome) = self.execute("session.memory", "{}") else {
            return vec!["the memory could not be read".to_string()];
        };
        let number = |name: &str| outcome.data.get(name).and_then(|value| value.as_u64()).unwrap_or_default();
        // The lines are `name: value`, one space, and no padding: a
        // person reads them and a program parses them, and padding
        // would make the second one harder to read and the first one
        // harder to parse.
        vec![
            format!("document: {} byte(s)", number("document")),
            format!("history: {} byte(s)", number("history")),
            format!("journal: {} byte(s)", number("journal")),
            format!("tasks: {} byte(s)", number("tasks")),
            format!("log: {} byte(s)", number("log")),
            format!("total: {} of {} byte(s)", number("total"), number("budget")),
        ]
    }

    /// A picture of the document.
    ///
    /// The picture is the **command's** work and not the door's: the
    /// door asks `doc.snapshot` to do it, exactly as every other door
    /// would, and there is no second way to make a picture.
    fn snapshot(&mut self, path: &str) -> Vec<String> {
        if path.is_empty() {
            return vec!["a snapshot needs a path".to_string()];
        }
        self.run_command("doc.snapshot", &format!("{{\"path\": {path:?}}}"))
    }

    /// Run a command of the kit's own.
    fn run_command(&mut self, id: &str, rest: &str) -> Vec<String> {
        match self.execute(id, rest) {
            Ok(outcome) => {
                let mut out = vec![outcome.message];
                if !outcome.data.is_null() {
                    out.push(format!("  {}", outcome.data));
                }
                out
            }
            Err(error) => vec![format!("refused: {error}")],
        }
    }

    /// Run a command and keep the whole answer.
    ///
    /// The built-ins of the door are **commands of the kit** with a
    /// friendlier shape: `plan` is `doc.plan`, `memory` is
    /// `session.memory`, and `snapshot` is `doc.snapshot`. The door
    /// changes nothing itself — it asks, and it prints.
    fn execute(&mut self, id: &str, rest: &str) -> Result<craftkit_sdk::Outcome, EngineError> {
        let params = if rest.is_empty() {
            craftkit_sdk::serde_json::json!({})
        } else {
            craftkit_sdk::serde_json::from_str(rest).map_err(|error| EngineError::Failed {
                id: id.to_string(),
                message: format!("the parameters are not JSON: {error}"),
            })?
        };
        self.session.execute(id, &params)
    }
}

/// The error the door reports, when the session refuses.
pub fn refusal(error: &EngineError) -> String {
    format!("refused: {error}")
}
