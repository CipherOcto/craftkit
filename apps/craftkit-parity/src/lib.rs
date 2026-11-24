//! `craftkit-parity` — the doors, side by side.
//!
//! # Why it exists
//!
//! The tree has **six doors**, and each one has its own tests: a test
//! that says "this door lists what the session has", a test that says
//! "this door runs a command", a test that says "this door refuses a
//! command that is not there".
//!
//! Six sets of tests prove six doors. They do **not** prove that the six
//! doors **agree** — and a door that drifted from the others would pass
//! its own tests and fail the tree's claim that the doors are one kit
//! with six faces. A review of the tree found exactly that: the bridge
//! published a manifest that was not the session's, and every door's own
//! test was green.
//!
//! So this crate asks the same questions of every door and compares the
//! answers. It is the **parity matrix** made executable: the matrix in
//! [`docs/door-parity.md`](../../docs/door-parity.md) says what each door
//! does, cell by cell, and this program says whether the doors still do
//! it.
//!
//! # What it does
//!
//! It drives **five** doors: the terminal door, the window, the browser
//! door, the agent door and the bridge. It asks each of them the same
//! three questions:
//!
//! - **which commands do you list?** — the ids, in order;
//! - **what do you say when you run this command?** — the message;
//! - **what do you say when the command is not there?** — the refusal.
//!
//! And it compares the answers, one door against another.
//!
//! **The a2ui door is not in it**, and that is not an oversight: the a2ui
//! door speaks a2ui's own vocabulary — a catalogue of a2ui's components
//! and not a list of the kit's commands — so there is nothing to compare.
//! The matrix says the same thing, and the a2ui door's own tests hold it
//! to that.
//!
//! # What it does not do
//!
//! It does not **decide** anything. A difference is a difference, and it
//! is reported as one: the program never crashes, never guesses, and
//! never rewrites a door to make the doors agree. What a difference
//! means is a question for a person, and the report is where the
//! question is written down.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_sdk::serde_json::{Value as Json, from_str, json};

/// The command every door is asked to run.
///
/// It is the core's own, it changes the document, and every door knows
/// it: a door that could not run it would be a door that is not a door.
pub const COMMAND: &str = "node.add";

/// The arguments the command is run with.
pub const ARGUMENTS: &str = r#"{"name": "box", "kind": "geometry.rect"}"#;

/// The id that is **not** there.
///
/// The refusal is the door's own answer for a command that does not
/// exist, and every door has one.
pub const UNKNOWN: &str = "no.such.command";

/// What one door said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The door's name, as the matrix names it.
    pub door: String,
    /// The ids the door lists, in the order it lists them.
    pub listed: Vec<String>,
    /// What the door said when it ran [`COMMAND`].
    pub ran: String,
    /// What the door said when it was asked for [`UNKNOWN`].
    pub refused: String,
}

/// The doors' answers, and where they differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// What each door said.
    pub answers: Vec<Answer>,
    /// The differences, one line each.
    ///
    /// An empty list means the doors agree.
    pub differences: Vec<String>,
}

impl Report {
    /// Whether the doors agree.
    pub fn agreed(&self) -> bool {
        self.differences.is_empty()
    }
}

/// Drive every door and compare their answers.
///
/// The program **never crashes**: a door that could not be built is a
/// difference, and the report says so.
pub fn compare() -> Report {
    let mut answers = Vec::new();
    let mut differences = Vec::new();
    for door in [terminal, window, browser, agent, bridge] {
        match door() {
            Ok(answer) => answers.push(answer),
            Err(error) => differences.push(error),
        }
    }
    differences.extend(the_differences_among_them(&answers));
    Report { answers, differences }
}

/// The differences between the doors, one line each.
///
/// The **first** door is the one every other door is compared with: the
/// terminal door is the kit's own face, and the browser door is the
/// terminal door in a page.
fn the_differences_among_them(answers: &[Answer]) -> Vec<String> {
    let mut differences = Vec::new();
    let Some(first) = answers.first() else { return differences };
    for answer in answers.iter().skip(1) {
        let missing: Vec<&str> =
            first.listed.iter().filter(|id| !answer.listed.contains(id)).map(|id| id.as_str()).collect();
        let extra: Vec<&str> =
            answer.listed.iter().filter(|id| !first.listed.contains(id)).map(|id| id.as_str()).collect();
        if !missing.is_empty() || !extra.is_empty() {
            differences.push(format!(
                "{} and {} do not list the same commands: {} has {} and {} has {}",
                first.door,
                answer.door,
                first.door,
                if missing.is_empty() { "nothing else".to_string() } else { missing.join(", ") },
                answer.door,
                if extra.is_empty() { "nothing else".to_string() } else { extra.join(", ") },
            ));
        }
        if first.ran != answer.ran {
            differences.push(format!(
                "{} and {} answered differently to {COMMAND}: `{}` and `{}`",
                first.door, answer.door, first.ran, answer.ran
            ));
        }
        // The **words** of a refusal are the door's own: the terminal
        // door says "refused", the bridge says "tool" and the agent
        // door says "command". What they have to agree on is that the
        // refusal **names** the command that is not there.
        for said in [&first.refused, &answer.refused] {
            if !said.contains(UNKNOWN) {
                differences.push(format!(
                    "{} refused `{}` without naming it: `{}`",
                    if said == &first.refused { first.door.as_str() } else { answer.door.as_str() },
                    UNKNOWN,
                    said
                ));
            }
        }
    }
    differences
}

/// The **terminal door**.
fn terminal() -> Result<Answer, String> {
    let mut repl = craftkit_repl::Repl::new(ecosystem()?);
    let listed = lines(&repl.run("commands"));
    let ran = first_line_of(&repl.run(&format!("{COMMAND} {ARGUMENTS}")));
    let refused = without_the_doors_own_prefix(&first_line_of(&repl.run(UNKNOWN)));
    Ok(Answer { door: "the terminal door".to_string(), listed, ran, refused })
}

/// The **window**.
fn window() -> Result<Answer, String> {
    let mut window =
        craftkit_window::Window::new().map_err(|error| format!("the window could not be built: {error}"))?;
    let listed: Vec<String> = window.commands().iter().map(|command| command.id.to_string()).collect();
    let ran = match window.run_id(COMMAND, from_str(ARGUMENTS).unwrap_or(Json::Null)) {
        Ok(outcome) => outcome.message,
        Err(error) => error.to_string(),
    };
    let refused = match window.run_id(UNKNOWN, json!({})) {
        Ok(outcome) => outcome.message,
        Err(error) => error.to_string(),
    };
    Ok(Answer { door: "the window".to_string(), listed, ran, refused })
}

/// The **browser door**.
fn browser() -> Result<Answer, String> {
    let mut door = craftkit_wasm::Door::new()
        .map_err(|error| format!("the browser door could not be built: {error}"))?;
    let listed: Vec<String> = door.run("commands").lines().map(|line| line.to_string()).collect();
    let ran = first_line(&door.run(&format!("{COMMAND} {ARGUMENTS}")));
    let refused = without_the_doors_own_prefix(&first_line(&door.run(UNKNOWN)));
    Ok(Answer { door: "the browser door".to_string(), listed, ran, refused })
}

/// The **agent door**.
fn agent() -> Result<Answer, String> {
    let mut server = craftkit_mcp::Server::new(ecosystem()?.session());
    let listed: Vec<String> = server.tools().iter().map(|tool| tool.command.clone()).collect();
    let ran = match call(&mut server, COMMAND, from_str(ARGUMENTS).unwrap_or(Json::Null)) {
        Ok(text) => text,
        Err(message) => message,
    };
    let refused = match call(&mut server, UNKNOWN, json!({})) {
        Ok(text) => text,
        Err(message) => message,
    };
    Ok(Answer { door: "the agent door".to_string(), listed, ran, refused })
}

/// The **bridge**.
fn bridge() -> Result<Answer, String> {
    let mut bridge = craftkit_bridge::Bridge::new(ecosystem()?);
    let listed: Vec<String> = bridge
        .handle(&request("tools/list", Json::Null))
        .pointer("/result/tools")
        .and_then(|tools| tools.as_array())
        .map(|tools| {
            tools.iter().filter_map(|tool| tool["name"].as_str()).map(|name| name.to_string()).collect()
        })
        .unwrap_or_default();
    let arguments: Json = from_str(ARGUMENTS).unwrap_or(Json::Null);
    let ran = match answer_of(
        &bridge.handle(&request("tools/call", json!({ "name": COMMAND, "arguments": arguments }))),
    ) {
        Ok(text) => text,
        Err(message) => message,
    };
    let refused = match answer_of(
        &bridge.handle(&request("tools/call", json!({ "name": UNKNOWN, "arguments": {} }))),
    ) {
        Ok(text) => text,
        Err(message) => message,
    };
    Ok(Answer { door: "the bridge".to_string(), listed, ran, refused })
}

/// A **request** for the bridge.
fn request(method: &str, params: Json) -> craftkit_bridge::Request {
    craftkit_bridge::Request { id: Some(json!(1)), method: method.to_string(), params }
}

/// Call a command through the **agent door**, and say what it said.
fn call(server: &mut craftkit_mcp::Server, command: &str, arguments: Json) -> Result<String, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": craftkit_mcp::tool_name(command), "arguments": arguments },
    });
    let Some(answer) = server.handle(&request) else {
        return Err("the door did not answer".to_string());
    };
    answer_of(&answer)
}

/// What a JSON-RPC answer said.
///
/// The **text** of the answer, or the message of the error: the two
/// protocols answer in the same shape, and this is where they are read.
fn answer_of(answer: &Json) -> Result<String, String> {
    if let Some(message) = answer.pointer("/error/message").and_then(|message| message.as_str()) {
        return Err(message.to_string());
    }
    Ok(answer
        .pointer("/result/content/0/text")
        .and_then(|text| text.as_str())
        .unwrap_or_default()
        .to_string())
}

/// The ecosystem the doors drive.
///
/// Every door gets its **own**: a door holds one session, and two doors
/// sharing one session would be one door with two faces.
fn ecosystem() -> Result<craftkit_sdk::Ecosystem, String> {
    craftkit_studio::ecosystem().map_err(|error| format!("the ecosystem could not be built: {error}"))
}

/// The lines of a reply.
fn lines(reply: &craftkit_repl::Reply) -> Vec<String> {
    match reply {
        craftkit_repl::Reply::Lines(lines) => lines.clone(),
        craftkit_repl::Reply::Quit => Vec::new(),
    }
}

/// The first line of the terminal door's answer.
fn first_line_of(reply: &craftkit_repl::Reply) -> String {
    match reply {
        craftkit_repl::Reply::Lines(lines) => lines.first().cloned().unwrap_or_default(),
        craftkit_repl::Reply::Quit => String::new(),
    }
}

/// The first line of an answer, and the rest of it dropped.
///
/// A door's answer may have more than one line — the terminal door adds
/// the data under the message — and the **message** is what every door
/// has to agree on.
fn first_line(answer: &str) -> String {
    answer.lines().next().unwrap_or_default().to_string()
}

/// The door's own word in front of the refusal.
///
/// The terminal door says "refused: there is no command `x`", and the
/// word is the door's: what the doors have to agree on is the refusal
/// itself.
fn without_the_doors_own_prefix(message: &str) -> String {
    match message.strip_prefix("refused: ") {
        Some(rest) => rest.to_string(),
        None => message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two doors that answer differently **are** a difference.
    ///
    /// The harness has to bite: a comparison that cannot fail is not a
    /// comparison.
    #[test]
    fn two_doors_that_differ_are_a_difference() {
        let first = Answer {
            door: "the first door".to_string(),
            listed: vec!["one".to_string()],
            ran: "the same".to_string(),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let second = Answer {
            door: "the second door".to_string(),
            listed: vec!["two".to_string()],
            ran: "something else".to_string(),
            refused: "refused".to_string(),
        };
        let differences = the_differences_among_them(&[first, second]);
        assert_eq!(differences.len(), 3, "{differences:?}");
    }

    /// Two doors that answer the same are **not** a difference.
    #[test]
    fn two_doors_that_agree_are_not_a_difference() {
        let answer = || Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string(), "two".to_string()],
            ran: "the same".to_string(),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_among_them(&[answer(), answer()]);
        assert!(differences.is_empty(), "{differences:?}");
    }
}
