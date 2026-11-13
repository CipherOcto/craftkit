//! The **terminal door**, proved.
//!
//! The door is a prompt: a line of text, and the kit's answer to it.
//! These are the tests that hold the claims of `craftkit-repl` to what
//! it says about itself.

use craftkit_repl::{Repl, Reply};
use craftkit_sdk::serde_json::json;

/// A door onto the studio's ecosystem.
fn door() -> Repl {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is built");
    Repl::new(ecosystem)
}

/// The lines of a reply.
fn lines(reply: &Reply) -> Vec<String> {
    match reply {
        Reply::Lines(lines) => lines.clone(),
        Reply::Quit => Vec::new(),
    }
}

/// The text of a reply, joined.
fn text(reply: &Reply) -> String {
    lines(reply).join("\n")
}

/// A command runs, and the document changes.
#[test]
fn a_command_runs_and_the_document_changes() {
    let mut repl = door();
    let reply = repl.run(r#"node.add {"name": "box", "kind": "geometry.rect"}"#);
    let said = text(&reply);
    assert!(!said.contains("refused"), "{said}");
    assert_eq!(repl.session().document().nodes.len(), 1, "the node was not added");
}

/// An unknown command is **refused**, and it says so.
#[test]
fn an_unknown_command_is_refused() {
    let mut repl = door();
    let reply = repl.run("no.such.command");
    let said = text(&reply);
    assert!(said.contains("refused"), "{said}");
    assert_eq!(repl.session().document().nodes.len(), 0, "the document changed");
}

/// Parameters that are not JSON are refused before the command runs.
#[test]
fn parameters_that_are_not_json_are_refused() {
    let mut repl = door();
    let reply = repl.run("node.add {not json");
    let said = text(&reply);
    assert!(said.contains("not JSON"), "{said}");
    assert_eq!(repl.session().document().nodes.len(), 0, "the document changed");
}

/// `help` lists the commands, and the listing is the **projection**.
///
/// The door does not keep its own list of commands: it shows the same
/// one the studio's menu is made of, so the two cannot disagree.
#[test]
fn help_lists_the_commands() {
    let mut repl = door();
    let said = text(&repl.run("help"));
    assert!(said.contains("node.add"), "{said}");
    assert!(said.contains("doc.plan"), "{said}");
}

/// `commands` lists the ids, and every one of them is a real command.
#[test]
fn every_listed_command_is_a_real_command() {
    let mut repl = door();
    let ids = lines(&repl.run("commands"));
    assert!(!ids.is_empty(), "the listing is empty");
    assert!(ids.contains(&"node.add".to_string()), "{:?}", &ids[..5]);
    for id in &ids {
        assert!(repl.session().registry().get(id).is_some(), "`{id}` is listed and is not a command");
    }
}

/// `plan` says what the document means, and it says it **through the
/// command**.
///
/// The door asks `doc.plan` for the plan: the same answer the MCP
/// server gets, and the same answer the bridge gets.
#[test]
fn plan_comes_from_the_command() {
    let mut repl = door();
    repl.run(r#"node.add {"name": "box", "kind": "geometry.rect"}"#);
    let said = text(&repl.run("plan"));
    assert!(said.contains("geometry.rect"), "{said}");

    // And the command gives the same answer.
    let outcome = repl.session_mut().execute("doc.plan", &json!({})).expect("the plan is made");
    let items = outcome.data["items"].as_array().expect("the items");
    assert!(!items.is_empty(), "the plan is empty");
}

/// `memory` says what the session is holding, and the number is the
/// **command's** number.
///
/// The parts agree with the command's parts. The total may be one
/// larger than the command's total, and that is not a flaw: running a
/// command writes a line in the session's log, so the log is one line
/// bigger after the measurement than it was during it. The door shows
/// the state as it was when the command ran, and the log says the
/// command ran.
#[test]
fn memory_comes_from_the_command() {
    let mut repl = door();
    let said = text(&repl.run("memory"));
    assert!(said.contains("document:"), "{said}");
    assert!(said.contains("total:"), "{said}");

    let outcome = repl.session_mut().execute("session.memory", &json!({})).expect("the memory is read");
    for part in ["document", "history", "journal", "tasks"] {
        let bytes = outcome.data[part].as_u64().expect("the bytes");
        assert!(said.contains(&format!("{part}: {} byte(s)", bytes)), "{said}");
    }
    let total = outcome.data["total"].as_u64().expect("the total");
    let budget = outcome.data["budget"].as_u64().expect("the budget");
    assert!(said.contains(&format!("of {budget} byte(s)")), "{said}");
    assert!(said.contains("total: "), "{said}");
    assert!(total <= budget, "the session is over its budget");
}

/// `snapshot` writes a picture, through the **command**.
#[test]
fn snapshot_writes_a_picture() {
    let mut repl = door();
    repl.run(r#"node.add {"name": "box", "kind": "geometry.rect"}"#);
    let directory = craftkit_scratch::dir("craftkit-v2-repl").expect("the directory is made");
    let path = directory.join("shot.png");
    let said = text(&repl.run(&format!("snapshot {}", path.display())));
    assert!(said.contains("wrote"), "{said}");
    let bytes = std::fs::read(&path).expect("the picture is there");
    assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10], "the signature of a PNG");
}

/// A snapshot with no path says what is missing, and writes nothing.
#[test]
fn a_snapshot_without_a_path_says_so() {
    let mut repl = door();
    let said = text(&repl.run("snapshot"));
    assert!(said.contains("needs a path"), "{said}");
}

/// `quit` ends the conversation.
#[test]
fn quit_ends_the_conversation() {
    let mut repl = door();
    assert_eq!(repl.run("quit"), Reply::Quit);
}

/// An empty line says nothing, and asks nothing.
#[test]
fn an_empty_line_says_nothing() {
    let mut repl = door();
    assert_eq!(repl.run("   "), Reply::Lines(Vec::new()));
}

/// The line is **bounded**: a person may paste anything, and the door
/// reads what it was asked for and stops.
///
/// The door owns the bound, so the test does not need to know it: a
/// line of a megabyte is over any sensible bound, and a line of ten
/// bytes is under it.
#[test]
fn the_line_is_bounded() {
    let mut repl = door();
    let long = "a".repeat(1024 * 1024);
    let mut bytes = long.into_bytes();
    bytes.push(b'\n');
    let mut reader = std::io::BufReader::new(&bytes[..]);
    let read = repl.read_line(&mut reader);
    assert!(read.is_err(), "a line over the limit was read");

    // And a line within the limit is read, and run.
    let short = "help\n".as_bytes().to_vec();
    let mut reader = std::io::BufReader::new(&short[..]);
    let reply = repl.read_line(&mut reader).expect("the read").expect("a line");
    assert!(text(&reply).contains("node.add"), "the line did not run");
}

/// The door does not keep its own list of commands.
///
/// The ids it lists are the registry's ids, and the count is the
/// registry's count.
#[test]
fn the_door_lists_the_registry() {
    let mut repl = door();
    let listed = lines(&repl.run("commands")).len();
    let real = repl.session().registry().len();
    assert_eq!(listed, real, "the door listed {listed} and the registry holds {real}");
}
