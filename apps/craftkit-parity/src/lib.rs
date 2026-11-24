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
//! five questions:
//!
//! - **which commands do you list?** — the ids, in order;
//! - **what do you say when you run this command?** — the message;
//! - **what plan do you publish?** — the data of `doc.plan`;
//! - **what do you say when you undo it?** — the message;
//! - **what do you say when the command is not there?** — the refusal.
//!
//! The **plan** and the **undo** are the two that the matrix calls "as a
//! command": the plan is the command's work and not the door's, and the
//! undo is a command like any other. Five doors that publish the same
//! plan and say the same thing when they undo are five doors that agree
//! about the document.
//!
//! **The agent door is its transport.** The door is the application, and
//! the application serves the transport's answers over the wire without
//! changing them, so the harness asks the library the application runs.
//! The other four doors are asked directly.
//!
//! And it compares the answers, one door against another — and one door
//! against the **session**.
//!
//! Two doors that are wrong in the same way agree with each other, and
//! the first door was the one every other door was compared with: a
//! terminal door that drifted would have dragged the others with it and
//! the harness would have been quiet. The session is what the doors are
//! doors *of*, so the harness asks a session of its own the same
//! questions and compares the doors with that as well.
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

/// The command every door is asked to make the plan with.
///
/// The plan is the **command's** work and not the door's, so every door
/// that has a plan asks this one.
pub const PLAN: &str = "doc.plan";

/// The command every door is asked to undo with.
///
/// The undo is a command like any other, so every door runs it the same
/// way — and it undoes the [`COMMAND`] that ran before it.
pub const UNDO: &str = "history.undo";

/// What one door said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The door's name, as the matrix names it.
    pub door: String,
    /// The ids the door lists, in the order it lists them.
    pub listed: Vec<String>,
    /// Whether the door **prunes** its list.
    ///
    /// The bridge runs only what it lists, so it lists only what it runs:
    /// pruning is that door's own business, and it is the one door that
    /// says so. A door that does not prune lists **everything** the
    /// session has — and a door that quietly lost a command is a door
    /// that lost a command.
    pub prunes: bool,
    /// What the door said when it ran [`COMMAND`].
    ///
    /// The [`Err`] is the door **refusing** the command: five doors that
    /// all refuse the same command agree about nothing, and a refusal is
    /// not a parity.
    pub ran: Result<String, String>,
    /// The plan the door published, as the data of [`PLAN`].
    ///
    /// The [`Err`] is the door that could not make the plan, and a plan
    /// is the same work in every door: two doors that publish different
    /// plans are two doors that do not agree about the document.
    pub planned: Result<Json, String>,
    /// What the door said when it undid the command.
    pub undone: Result<String, String>,
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

/// What the **session** says — the truth behind the doors.
///
/// The doors are compared with each other, and **two doors that are
/// wrong in the same way agree with each other**: the terminal door is
/// the one every other door is compared with, so a terminal door that
/// drifted would drag the others with it and the harness would be
/// silent.
///
/// The session is what the doors are doors *of*, so the session is the
/// anchor: a door that says something the session does not say is a
/// difference, even when every other door says it too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Truth {
    /// The ids the session has, in the order it has them.
    pub listed: Vec<String>,
    /// What the session said when it ran [`COMMAND`].
    pub ran: Result<String, String>,
    /// The plan the session published, as the data of [`PLAN`].
    pub planned: Result<Json, String>,
    /// What the session said when it undid the command.
    pub undone: Result<String, String>,
    /// What the session said when it was asked for [`UNKNOWN`].
    pub refused: String,
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
    differences.extend(the_differences_in_what_they_said(&answers));
    // The doors are compared with the **session** as well: the session is
    // the truth behind them, and two doors that are wrong together are
    // still wrong.
    match the_session() {
        Ok(truth) => differences.extend(the_differences_from_the_session(&answers, &truth)),
        Err(error) => differences.push(format!("the session could not be asked: {error}")),
    }
    Report { answers, differences }
}

/// The session's own answers.
///
/// The session is a **session of its own**: a door holds one, and the
/// session the harness asks is a sixth one, so that the truth is asked
/// of the same code the doors run.
pub fn the_session() -> Result<Truth, String> {
    let mut session = ecosystem()?.session();
    let listed =
        session.registry().all().iter().map(|command| command.id.to_string()).collect::<Vec<String>>();
    let arguments = from_str(ARGUMENTS).unwrap_or(Json::Null);
    let ran = match session.execute(COMMAND, &arguments) {
        Ok(outcome) => Ok(outcome.message),
        Err(error) => Err(error.to_string()),
    };
    let planned = match session.execute(PLAN, &json!({})) {
        Ok(outcome) => Ok(outcome.data),
        Err(error) => Err(error.to_string()),
    };
    let undone = match session.execute(UNDO, &json!({})) {
        Ok(outcome) => Ok(outcome.message),
        Err(error) => Err(error.to_string()),
    };
    let refused = match session.execute(UNKNOWN, &json!({})) {
        Ok(outcome) => outcome.message,
        Err(error) => error.to_string(),
    };
    Ok(Truth { listed, ran, planned, undone, refused })
}

/// The differences between a door and the **session**.
///
/// Five things are compared:
///
/// - **the list**: every id a door lists is one the session has, and a
///   door that does not prune lists **all** of them;
/// - **the run**: the door said what the session said;
/// - **the plan**: the door published what the session published;
/// - **the undo**: the door said what the session said;
/// - **the refusal**: the door's refusal **names** the command that is
///   not there, as the session's does — the words are the door's own,
///   because every door says "refused", "tool" or "command".
fn the_differences_from_the_session(answers: &[Answer], truth: &Truth) -> Vec<String> {
    let mut differences = Vec::new();
    for answer in answers {
        for id in &answer.listed {
            if !truth.listed.contains(id) {
                differences
                    .push(format!("{} lists `{id}`, and the session has no such command", answer.door));
            }
        }
        // A door that does not prune lists **everything**: a door that
        // lost a command is a door that lost a command, and the bridge
        // is the one door that says it prunes.
        if !answer.prunes {
            let missing: Vec<&str> =
                truth.listed.iter().filter(|id| !answer.listed.contains(id)).map(|id| id.as_str()).collect();
            if !missing.is_empty() {
                differences.push(format!(
                    "{} does not list {}, and it does not prune its list",
                    answer.door,
                    missing.join(", ")
                ));
            }
        }
        match (&answer.ran, &truth.ran) {
            (Ok(door_message), Ok(session_message)) => {
                if door_message != session_message {
                    differences.push(format!(
                        "{} said `{}` when it ran {COMMAND}, and the session said `{}`",
                        answer.door, door_message, session_message
                    ));
                }
            }
            (Ok(door_message), Err(session_message)) => {
                differences.push(format!(
                    "{} ran {COMMAND} and said `{}`, and the session refused it: `{}`",
                    answer.door, door_message, session_message
                ));
            }
            (Err(door_message), Ok(session_message)) => {
                differences.push(format!(
                    "{} refused {COMMAND} and said `{}`, and the session ran it and said `{}`",
                    answer.door, door_message, session_message
                ));
            }
            (Err(door_message), Err(session_message)) => {
                differences.push(format!(
                    "{} and the session both refused {COMMAND}, and said different things: `{}` \
                     and `{}`",
                    answer.door, door_message, session_message
                ));
            }
        }
        if !answer.refused.contains(UNKNOWN) {
            differences.push(format!(
                "{} refused `{}` without naming it: `{}`",
                answer.door, UNKNOWN, answer.refused
            ));
        }
        // And the **session's** own refusal names it too: the truth is
        // the truth, and a session that refused without naming the
        // command would make every door's refusal unreadable.
        if !truth.refused.contains(UNKNOWN) {
            differences
                .push(format!("the session refused `{}` without naming it: `{}`", UNKNOWN, truth.refused));
        }
        // The **plan** is the command's work, and the session's plan is
        // the plan the doors publish.
        match (&answer.planned, &truth.planned) {
            (Ok(door_plan), Ok(session_plan)) => {
                if door_plan != session_plan {
                    differences.push(format!(
                        "{} published a plan the session did not publish: `{}` and `{}`",
                        answer.door, door_plan, session_plan
                    ));
                }
            }
            _ => {
                differences.push(format!(
                    "{} and the session did not both publish a plan: `{}` and `{}`",
                    answer.door,
                    answer.planned.as_ref().err().cloned().unwrap_or_default(),
                    truth.planned.as_ref().err().cloned().unwrap_or_default(),
                ));
            }
        }
        // And the **undo** is a command like any other.
        match (&answer.undone, &truth.undone) {
            (Ok(door_message), Ok(session_message)) => {
                if door_message != session_message {
                    differences.push(format!(
                        "{} said `{}` when it undid, and the session said `{}`",
                        answer.door, door_message, session_message
                    ));
                }
            }
            _ => {
                differences.push(format!(
                    "{} and the session did not both undo: `{}` and `{}`",
                    answer.door,
                    answer.undone.as_ref().err().cloned().unwrap_or_default(),
                    truth.undone.as_ref().err().cloned().unwrap_or_default(),
                ));
            }
        }
    }
    differences
}

/// The differences in what the doors said **on their own**.
///
/// A door that said nothing has not agreed with anybody: the comparison
/// of two empty answers is a comparison of nothing, and the harness has
/// to say so rather than pass.
///
/// And a door that **refused** the command has not run it: five doors
/// that all refuse the same command agree about nothing.
fn the_differences_in_what_they_said(answers: &[Answer]) -> Vec<String> {
    let mut differences = Vec::new();
    for answer in answers {
        if answer.listed.is_empty() {
            differences.push(format!("{} listed nothing at all", answer.door));
        }
        match &answer.ran {
            Ok(message) if message.is_empty() => {
                differences.push(format!("{} said nothing when it ran {COMMAND}", answer.door));
            }
            Ok(_) => {}
            Err(message) => {
                differences.push(format!("{} did not run {COMMAND}: {message}", answer.door));
            }
        }
        if answer.refused.is_empty() {
            differences.push(format!("{} said nothing when it refused {UNKNOWN}", answer.door));
        }
        if answer.planned.is_err() {
            differences.push(format!("{} published no plan", answer.door));
        }
        if answer.undone.is_err() {
            differences.push(format!("{} did not undo", answer.door));
        }
    }
    differences
}

/// What a door said, or that it refused.
fn said(ran: &Result<String, String>) -> String {
    match ran {
        Ok(message) => message.clone(),
        Err(message) => format!("refused: {message}"),
    }
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
        match (&first.ran, &answer.ran) {
            (Ok(mine), Ok(theirs)) => {
                if mine != theirs {
                    differences.push(format!(
                        "{} and {} answered differently to {COMMAND}: `{}` and `{}`",
                        first.door, answer.door, mine, theirs
                    ));
                }
            }
            _ => {
                differences.push(format!(
                    "{} and {} did not both run {COMMAND}: `{}` and `{}`",
                    first.door,
                    answer.door,
                    said(&first.ran),
                    said(&answer.ran)
                ));
            }
        }
        match (&first.planned, &answer.planned) {
            (Ok(mine), Ok(theirs)) => {
                if mine != theirs {
                    differences.push(format!(
                        "{} and {} published different plans: `{}` and `{}`",
                        first.door, answer.door, mine, theirs
                    ));
                }
            }
            _ => {
                differences.push(format!(
                    "{} and {} did not both publish a plan: `{}` and `{}`",
                    first.door,
                    answer.door,
                    first.planned.as_ref().err().cloned().unwrap_or_default(),
                    answer.planned.as_ref().err().cloned().unwrap_or_default(),
                ));
            }
        }
        match (&first.undone, &answer.undone) {
            (Ok(mine), Ok(theirs)) => {
                if mine != theirs {
                    differences.push(format!(
                        "{} and {} answered differently to {UNDO}: `{}` and `{}`",
                        first.door, answer.door, mine, theirs
                    ));
                }
            }
            _ => {
                differences.push(format!(
                    "{} and {} did not both undo: `{}` and `{}`",
                    first.door,
                    answer.door,
                    first.undone.as_ref().err().cloned().unwrap_or_default(),
                    answer.undone.as_ref().err().cloned().unwrap_or_default(),
                ));
            }
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
    let ran = ran_of(first_line_of(&repl.run(&format!("{COMMAND} {ARGUMENTS}"))));
    let planned = plan_of(&repl.run(PLAN));
    let undone = ran_of(first_line_of(&repl.run(UNDO)));
    let refused = without_the_doors_own_prefix(&first_line_of(&repl.run(UNKNOWN)));
    Ok(Answer { door: "the terminal door".to_string(), listed, prunes: false, ran, planned, undone, refused })
}

/// The **window**.
fn window() -> Result<Answer, String> {
    let mut window =
        craftkit_window::Window::new().map_err(|error| format!("the window could not be built: {error}"))?;
    let listed: Vec<String> = window.commands().iter().map(|command| command.id.to_string()).collect();
    let ran = match window.run_id(COMMAND, from_str(ARGUMENTS).unwrap_or(Json::Null)) {
        Ok(outcome) => Ok(outcome.message),
        Err(error) => Err(error.to_string()),
    };
    let planned = match window.run_id(PLAN, json!({})) {
        Ok(outcome) => Ok(outcome.data),
        Err(error) => Err(error.to_string()),
    };
    let undone = match window.run_id(UNDO, json!({})) {
        Ok(outcome) => Ok(outcome.message),
        Err(error) => Err(error.to_string()),
    };
    let refused = match window.run_id(UNKNOWN, json!({})) {
        Ok(outcome) => outcome.message,
        Err(error) => error.to_string(),
    };
    Ok(Answer { door: "the window".to_string(), listed, prunes: false, ran, planned, undone, refused })
}

/// The **browser door**.
fn browser() -> Result<Answer, String> {
    let mut door = craftkit_wasm::Door::new()
        .map_err(|error| format!("the browser door could not be built: {error}"))?;
    let listed: Vec<String> = door.run("commands").lines().map(|line| line.to_string()).collect();
    let ran = ran_of(first_line(&door.run(&format!("{COMMAND} {ARGUMENTS}"))));
    let planned = plan_of_lines(&door.run(PLAN));
    let undone = ran_of(first_line(&door.run(UNDO)));
    let refused = without_the_doors_own_prefix(&first_line(&door.run(UNKNOWN)));
    Ok(Answer { door: "the browser door".to_string(), listed, prunes: false, ran, planned, undone, refused })
}

/// The **agent door**.
fn agent() -> Result<Answer, String> {
    let mut server = craftkit_mcp::Server::new(ecosystem()?.session());
    let listed: Vec<String> = server.tools().iter().map(|tool| tool.command.clone()).collect();
    let ran = call(&mut server, COMMAND, from_str(ARGUMENTS).unwrap_or(Json::Null));
    let planned = call_plan(&mut server, PLAN, json!({}));
    let undone = call(&mut server, UNDO, json!({}));
    let refused = match call(&mut server, UNKNOWN, json!({})) {
        Ok(text) => text,
        Err(message) => message,
    };
    Ok(Answer { door: "the agent door".to_string(), listed, prunes: false, ran, planned, undone, refused })
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
    let ran =
        answer_of(&bridge.handle(&request("tools/call", json!({ "name": COMMAND, "arguments": arguments }))));
    let planned =
        plan_of_data(&bridge.handle(&request("tools/call", json!({ "name": PLAN, "arguments": {} }))));
    let undone = answer_of(&bridge.handle(&request("tools/call", json!({ "name": UNDO, "arguments": {} }))));
    let refused = match answer_of(
        &bridge.handle(&request("tools/call", json!({ "name": UNKNOWN, "arguments": {} }))),
    ) {
        Ok(text) => text,
        Err(message) => message,
    };
    Ok(Answer { door: "the bridge".to_string(), listed, prunes: true, ran, planned, undone, refused })
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

/// The plan a door published, if it published one.
///
/// The terminal door prints the data under the message, so the plan is
/// the **second** line, and it is the door's own JSON.
fn plan_of(reply: &craftkit_repl::Reply) -> Result<Json, String> {
    let craftkit_repl::Reply::Lines(lines) = reply else {
        return Err("the door did not answer".to_string());
    };
    let Some(line) = lines.get(1) else {
        return Err("the door published no plan".to_string());
    };
    from_str(line.trim()).map_err(|error| error.to_string())
}

/// The plan of a door that speaks in **lines**.
///
/// The browser door answers in the terminal door's lines, so the plan is
/// the second one.
fn plan_of_lines(answer: &str) -> Result<Json, String> {
    let line = answer.lines().nth(1).unwrap_or_default();
    from_str(line.trim()).map_err(|error| error.to_string())
}

/// The plan of a door that publishes **data**.
///
/// The bridge's answer carries the data beside the message, so the plan
/// is there and not in the text.
fn plan_of_data(answer: &Json) -> Result<Json, String> {
    if let Some(message) = answer.pointer("/error/message").and_then(|message| message.as_str()) {
        return Err(message.to_string());
    }
    match answer.pointer("/result/data") {
        Some(data) => Ok(data.clone()),
        None => Err("the door published no plan".to_string()),
    }
}

/// The plan the **agent door** published.
///
/// The protocol puts the data in a second content block, and it is
/// written out in full.
fn call_plan(server: &mut craftkit_mcp::Server, command: &str, arguments: Json) -> Result<Json, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": craftkit_mcp::tool_name(command), "arguments": arguments },
    });
    let Some(answer) = server.handle(&request) else {
        return Err("the door did not answer".to_string());
    };
    if let Some(message) = answer.pointer("/error/message").and_then(|message| message.as_str()) {
        return Err(message.to_string());
    }
    let Some(text) = answer.pointer("/result/content/1/text").and_then(|text| text.as_str()) else {
        return Err("the door published no plan".to_string());
    };
    from_str(text).map_err(|error| error.to_string())
}

/// The answer of a door that speaks in **lines**.
///
/// The terminal door says "refused: there is no command `x`" when the
/// command did not run, and the browser door is the terminal door in a
/// page — so the word is how the two say that nothing ran.
fn ran_of(message: String) -> Result<String, String> {
    match message.strip_prefix("refused: ") {
        Some(refused) => Err(refused.to_string()),
        None => Ok(message),
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
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let second = Answer {
            door: "the second door".to_string(),
            listed: vec!["two".to_string()],
            prunes: false,
            ran: Ok("something else".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: "refused".to_string(),
        };
        let differences = the_differences_among_them(&[first, second]);
        assert_eq!(differences.len(), 3, "{differences:?}");
    }

    /// A door that said **nothing** is a difference.
    ///
    /// Two doors that both list nothing agree about nothing, and a
    /// harness that says they agree is a harness that passes.
    #[test]
    fn a_door_that_said_nothing_is_a_difference() {
        let answer = Answer {
            door: "a door".to_string(),
            listed: Vec::new(),
            prunes: false,
            ran: Ok(String::new()),
            planned: Ok(json!({})),
            undone: Ok("undid".to_string()),
            refused: String::new(),
        };
        let differences = the_differences_in_what_they_said(&[answer]);
        assert_eq!(differences.len(), 3, "{differences:?}");
    }

    /// A door that **refused** the command is a difference.
    #[test]
    fn a_door_that_refused_the_command_is_a_difference() {
        let answer = Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string()],
            prunes: false,
            ran: Err("there is no command `node.add`".to_string()),
            planned: Ok(json!({})),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_in_what_they_said(&[answer]);
        assert_eq!(differences.len(), 1, "{differences:?}");
    }

    /// Two doors that publish **different plans** are a difference.
    ///
    /// The plan is the command's work, and the document is the same
    /// document: two doors that disagree about it do not agree about
    /// anything.
    #[test]
    fn two_plans_that_differ_are_a_difference() {
        let first = Answer {
            door: "the first door".to_string(),
            listed: vec!["one".to_string()],
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [{ "node": 1 }] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let second = Answer {
            door: "the second door".to_string(),
            listed: vec!["one".to_string()],
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [{ "node": 2 }] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_among_them(&[first, second]);
        assert_eq!(differences.len(), 1, "{differences:?}");
    }

    /// Two doors that answer the same are **not** a difference.
    #[test]
    fn two_doors_that_agree_are_not_a_difference() {
        let answer = || Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string(), "two".to_string()],
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_among_them(&[answer(), answer()]);
        assert!(differences.is_empty(), "{differences:?}");
    }

    /// The **session** is the truth behind the doors, and a door that
    /// lists a command the session does not have is a difference — even
    /// when every other door lists it too.
    #[test]
    fn a_door_that_lists_what_the_session_does_not_have_is_a_difference() {
        let truth = Truth {
            listed: vec!["one".to_string()],
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let answer = Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string(), "two".to_string()],
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_from_the_session(&[answer], &truth);
        assert_eq!(differences.len(), 1, "{differences:?}");
        assert!(differences[0].contains("the session has no such command"), "{differences:?}");
    }

    /// A door that says something the session does not say is a
    /// difference: two doors that are wrong in the same way agree with
    /// each other, and the session is what says otherwise.
    #[test]
    fn a_door_that_says_something_else_is_a_difference() {
        let truth = Truth {
            listed: vec!["one".to_string()],
            ran: Ok("the session's answer".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let answer = Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string()],
            prunes: false,
            ran: Ok("the door's own answer".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_from_the_session(&[answer], &truth);
        assert_eq!(differences.len(), 1, "{differences:?}");
        assert!(differences[0].contains("the session said"), "{differences:?}");
    }

    /// A door that **lists fewer** than the session is not a difference:
    /// the bridge prunes its list, and pruning is the door's own
    /// business.
    #[test]
    fn a_door_that_prunes_may_list_less_than_the_session() {
        let truth = Truth {
            listed: vec!["one".to_string(), "two".to_string()],
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let answer = Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string()],
            prunes: true,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_from_the_session(&[answer], &truth);
        assert!(differences.is_empty(), "{differences:?}");
    }

    /// And a door that lists less and does **not** prune is a
    /// difference: a door that quietly lost a command is a door that
    /// lost a command.
    #[test]
    fn a_door_that_lists_less_and_does_not_prune_is_a_difference() {
        let truth = Truth {
            listed: vec!["one".to_string(), "two".to_string()],
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let answer = Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string()],
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_from_the_session(&[answer], &truth);
        assert_eq!(differences.len(), 1, "{differences:?}");
        assert!(differences[0].contains("does not prune"), "{differences:?}");
    }

    /// A door that agrees with the session is **not** a difference.
    #[test]
    fn a_door_that_agrees_with_the_session_is_not_a_difference() {
        let truth = Truth {
            listed: vec!["one".to_string()],
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let answer = Answer {
            door: "a door".to_string(),
            listed: vec!["one".to_string()],
            prunes: false,
            ran: Ok("the same".to_string()),
            planned: Ok(json!({ "items": [] })),
            undone: Ok("undid".to_string()),
            refused: format!("there is no command `{UNKNOWN}`"),
        };
        let differences = the_differences_from_the_session(&[answer], &truth);
        assert!(differences.is_empty(), "{differences:?}");
    }
}
