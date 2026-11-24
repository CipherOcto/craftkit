//! What the second front end promises, proved.
//!
//! The claims are in the crate's documentation. These are the tests that
//! hold them.

use craftkit_bridge::{Bridge, Request};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{CommandSpec, Doc, Ecosystem, Outcome, ParamDoc, Text};

/// The ecosystem of this program: the same registry the studio has.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// The bridge over it.
fn bridge() -> Bridge {
    Bridge::new(ecosystem())
}

/// Ask the bridge something.
fn ask(bridge: &mut Bridge, method: &str, params: Json) -> Json {
    let request = Request { id: Some(json!(1)), method: method.to_string(), params };
    bridge.handle(&request)
}

/// The tool list is the **projection**: the same view the studio's menu is
/// made of, so the two cannot disagree.
#[test]
fn the_tool_list_is_the_projection() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "tools/list", Json::Null);
    let tools = answer["result"]["tools"].as_array().expect("there are tools");
    let names: Vec<&str> = tools.iter().filter_map(|tool| tool["name"].as_str()).collect();
    assert!(names.contains(&"hello.greet"), "{names:?}");
    assert!(names.contains(&"geometry.rectangle"), "{names:?}");
    // A tool carries the schema: what it takes and whether it is required.
    let greet = tools.iter().find(|tool| tool["name"] == "hello.greet").expect("the greeting");
    let parameters = greet["parameters"].as_array().expect("the parameters");
    let name = parameters.iter().find(|param| param["name"] == "name").expect("the name");
    assert_eq!(name["kind"], "text");
    assert_eq!(name["required"], true);
}

/// A command runs through the same door the studio's buttons use.
#[test]
fn a_tool_call_runs_a_command() {
    let mut bridge = bridge();
    let answer = ask(
        &mut bridge,
        "tools/call",
        json!({ "name": "hello.greet", "arguments": { "name": "world", "loudly": true } }),
    );
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(answer["result"]["content"][0]["text"], "HELLO, world!");
}

/// A refusal is a refusal, and the agent is told what was refused.
#[test]
fn a_refused_call_says_so() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "tools/call", json!({ "name": "hello.greet", "arguments": { "nope": 1 } }));
    assert!(answer.get("error").is_some(), "{answer}");
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("nope"), "{message}");
}

/// A command that does not exist is refused, not invented.
#[test]
fn a_command_that_does_not_exist_is_refused() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "tools/call", json!({ "name": "no.such.command" }));
    assert!(answer.get("error").is_some(), "{answer}");
}

/// A method that does not exist says which methods there are.
#[test]
fn an_unknown_method_says_which_methods_there_are() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "no/such/method", Json::Null);
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("tools/list"), "{message}");
}

/// A tool says **what it changes**.
///
/// The descriptor carries the command's own promise, so an agent that
/// reads the bridge's tool list knows which tools write to the document
/// and which only read it.
#[test]
fn a_tool_says_what_it_changes() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "tools/list", Json::Null);
    let tools = answer["result"]["tools"].as_array().expect("there are tools");
    let read = tools.iter().find(|tool| tool["id"] == "doc.plan").expect("the plan");
    assert_eq!(read["mutates"], json!(false), "the plan does not change the document");
    let write = tools.iter().find(|tool| tool["id"] == "link.add").expect("the link");
    assert_eq!(write["mutates"], json!(true), "a link changes the document");
}

/// A command that joins the session while the program runs is **listed
/// and can be run**.
///
/// The door's list is read from the session its tools run in, so a door
/// never offers a command the session does not have — and never hides
/// one it has.
#[test]
fn a_command_made_at_run_time_is_listed_and_can_be_run() {
    let mut bridge = bridge();
    let command = CommandSpec::dynamic(
        "runtime.echo",
        "Echo",
        "runtime",
        "Say the text back.",
        vec![ParamDoc::dynamic("text", Doc::Text, true, "What to say.")],
        |_session, params| {
            let text = params.param::<Text>("text", "runtime.echo")?;
            Ok(Outcome::new(text.0))
        },
    );
    bridge.session().register(command).expect("the command joins the session");

    let answer = ask(&mut bridge, "tools/list", Json::Null);
    let tools = answer["result"]["tools"].as_array().expect("there are tools");
    assert!(tools.iter().any(|tool| tool["id"] == "runtime.echo"), "{answer}");

    let answer =
        ask(&mut bridge, "tools/call", json!({ "name": "runtime.echo", "arguments": { "text": "hello" } }));
    assert_eq!(answer["result"]["content"][0]["text"], "hello", "{answer}");
}

/// The menu and the tool list are **one list**.
///
/// The bridge draws its menu from the session the commands run in, so a
/// command that joins the session while the program runs is in the menu
/// and in the tools — and the two never disagree.
#[test]
fn the_menu_and_the_tool_list_are_one_list() {
    let mut bridge = bridge();
    let command = CommandSpec::dynamic(
        "runtime.echo",
        "Echo",
        "runtime",
        "Say the text back.",
        vec![ParamDoc::dynamic("text", Doc::Text, true, "What to say.")],
        |_session, params| {
            let text = params.param::<Text>("text", "runtime.echo")?;
            Ok(Outcome::new(text.0))
        },
    );
    bridge.session().register(command).expect("the command joins the session");

    let menu = ask(&mut bridge, "menu", Json::Null);
    let mut listed = Vec::new();
    flatten(&menu["result"]["menu"], &mut listed);
    assert!(listed.iter().any(|id| id == "runtime.echo"), "the menu names it: {listed:?}");

    let tools = ask(&mut bridge, "tools/list", Json::Null);
    let tools = tools["result"]["tools"].as_array().expect("there are tools");
    assert!(tools.iter().any(|tool| tool["id"] == "runtime.echo"), "{tools:?}");
}

/// Every command of a menu, by its id.
fn flatten(part: &Json, out: &mut Vec<String>) {
    let Some(part) = part.as_array() else { return };
    for part in part {
        for command in part["commands"].as_array().into_iter().flatten() {
            if let Some(id) = command["id"].as_str() {
                out.push(id.to_string());
            }
        }
        flatten(&part["children"], out);
    }
}

/// The resources are the kinds, the formats and the **manifest**: the
/// rest of the registry, read through the projection's own door.
///
/// The manifest is where the evaluators are named, so a door that
/// published them would have to reach into the evaluator's registry to
/// list them — and a front end that does that has a second path into
/// the design.
#[test]
fn the_resources_are_the_rest_of_the_registry() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "resources/list", Json::Null);
    let kinds = answer["result"]["kinds"].as_array().expect("there are kinds");
    assert!(kinds.iter().any(|kind| kind["kind"] == "geometry.rect"), "{answer}");
    assert_eq!(
        answer["result"]["formats"],
        // The tags are sorted as **strings**, so the tenth version
        // comes after the first and before the second: a version
        // number that reaches two digits does not sort where a person
        // expects it.
        json!([
            "craftkit/1",
            "craftkit/10",
            "craftkit/2",
            "craftkit/3",
            "craftkit/4",
            "craftkit/5",
            "craftkit/6",
            "craftkit/7",
            "craftkit/8",
            "craftkit/9"
        ])
    );
    let manifest = answer["result"]["manifest"].as_array().expect("there is a manifest");
    assert!(
        manifest.iter().any(|line| line.as_str().unwrap_or_default().starts_with("evaluator ")),
        "the manifest names the evaluators: {manifest:?}"
    );
}

/// The plan is the **command's** work.
///
/// The bridge asks `doc.plan` for it, the way every other door does, so
/// every door says the same thing about the same document.
#[test]
fn the_plan_is_the_projects_plan() {
    let mut bridge = bridge();
    ask(
        &mut bridge,
        "tools/call",
        json!({ "name": "geometry.rectangle", "arguments": { "x": 0.0, "y": 0.0, "width": 3.0, "height": 4.0 } }),
    );
    let answer = ask(&mut bridge, "plan", Json::Null);
    let items = answer["result"]["items"].as_array().expect("there are items");
    let area = items.iter().find(|item| item["kind"] == "geometry.rect").expect("the rectangle");
    assert_eq!(area["value"]["area"], json!(12.0), "{answer}");
}

/// The bridge says what it is.
#[test]
fn the_bridge_says_what_it_is() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "initialize", Json::Null);
    assert_eq!(answer["result"]["serverInfo"]["name"], "craftkit-bridge");
    assert!(answer["result"]["protocolVersion"].is_string(), "{answer}");
}

/// A message that is not a request is not answered.
#[test]
fn a_message_that_is_not_a_request_is_refused() {
    assert!(Request::parse("not json").is_err());
    assert!(Request::parse(r#"{"jsonrpc":"2.0","id":1}"#).is_err());
}

/// The program itself: a real front end, driven over its standard input.
#[test]
fn the_bridge_program_answers_over_its_stdin() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let program = env!("CARGO_BIN_EXE_craftkit-bridge");
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the bridge starts");
    {
        let stdin = child.stdin.as_mut().expect("the bridge reads");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"hello.greet","arguments":{{"name":"world"}}}}}}"#
        )
        .expect("the request is written");
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"shutdown"}}"#).expect("the stop is written");
    }
    let output = child.wait_with_output().expect("the bridge finishes");
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "{text}");
    let first: Json = craftkit_sdk::serde_json::from_str(lines[0]).expect("the answer is JSON");
    assert_eq!(first["result"]["content"][0]["text"], "Hello, world.");
}

/// The menu is over the wire, and it is a tree.
#[test]
fn the_menu_is_over_the_wire() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "menu", Json::Null);
    let menu = answer["result"]["menu"].as_array().expect("there is a menu");
    assert!(!menu.is_empty(), "{answer}");
    let names: Vec<&str> = menu.iter().filter_map(|part| part["name"].as_str()).collect();
    assert!(names.contains(&"geometry"), "{names:?}");
}
/// The journal is over the wire.
///
/// It is what the session was asked to do, and an agent that drove the
/// program can read it back.
#[test]
fn the_journal_is_over_the_wire() {
    let mut bridge = bridge();
    let _ = ask(
        &mut bridge,
        "tools/call",
        json!({ "name": "geometry.rectangle", "arguments": { "x": 1, "y": 2, "width": 3, "height": 4 } }),
    );
    let answer = ask(&mut bridge, "journal", Json::Null);
    let entries = answer["result"]["entries"].as_array().expect("there are entries");
    assert!(!entries.is_empty(), "{answer}");
    assert_eq!(entries[0]["id"], json!("geometry.rectangle"), "{answer}");
}

/// The manifest names the **session's** commands.
///
/// The manifest is what an agent is told, and the tool list is what it
/// may call: a command that joined the session while the program runs
/// is in both, and the two views are one list.
#[test]
fn the_manifest_names_the_sessions_commands() {
    let mut bridge = bridge();
    let command = CommandSpec::dynamic(
        "runtime.echo",
        "Echo",
        "runtime",
        "Say the text back.",
        vec![ParamDoc::dynamic("text", Doc::Text, true, "What to say.")],
        |_session, params| {
            let text = params.param::<Text>("text", "runtime.echo")?;
            Ok(Outcome::new(text.0))
        },
    );
    bridge.session().register(command).expect("the command joins the session");

    let answer = ask(&mut bridge, "resources/list", Json::Null);
    let manifest = answer["result"]["manifest"].as_array().expect("there is a manifest");
    assert!(
        manifest.iter().any(|line| line.as_str().unwrap_or_default().contains("runtime.echo")),
        "{manifest:?}"
    );
}

/// The door runs only what it lists.
///
/// A command the door did not offer is refused, so the list an agent
/// read is the list it can call.
#[test]
fn the_bridge_runs_only_what_it_lists() {
    let mut bridge = bridge();
    let mut command = CommandSpec::dynamic(
        "runtime.echo",
        "Echo",
        "runtime",
        "Say the text back.",
        vec![ParamDoc::dynamic("text", Doc::Text, true, "What to say.")],
        |_session, params| {
            let text = params.param::<Text>("text", "runtime.echo")?;
            Ok(Outcome::new(text.0))
        },
    );
    // A capability the door does not show: it is not on the list, so it
    // is not runnable.
    command.capability = "exporter";
    bridge.session().register(command).expect("the command joins the session");

    let tools = ask(&mut bridge, "tools/list", Json::Null);
    let tools = tools["result"]["tools"].as_array().expect("there are tools");
    assert!(!tools.iter().any(|tool| tool["id"] == "runtime.echo"), "{tools:?}");

    let answer =
        ask(&mut bridge, "tools/call", json!({ "name": "runtime.echo", "arguments": { "text": "hi" } }));
    let message = answer["error"]["message"].as_str().unwrap_or_default().to_string();
    assert!(message.contains("runtime.echo"), "{answer}");
}
