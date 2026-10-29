//! What the second front end promises, proved.
//!
//! The claims are in the crate's documentation. These are the tests that
//! hold them.

use craftkit_bridge::{Bridge, Request};
use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::{Value as Json, json};

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

/// The resources are the kinds, the formats and the evaluators: the rest
/// of the registry.
#[test]
fn the_resources_are_the_rest_of_the_registry() {
    let mut bridge = bridge();
    let answer = ask(&mut bridge, "resources/list", Json::Null);
    let kinds = answer["result"]["kinds"].as_array().expect("there are kinds");
    assert!(kinds.iter().any(|kind| kind["kind"] == "geometry.rect"), "{answer}");
    assert_eq!(
        answer["result"]["formats"],
        json!([
            "craftkit/1",
            "craftkit/2",
            "craftkit/3",
            "craftkit/4",
            "craftkit/5",
            "craftkit/6",
            "craftkit/7",
            "craftkit/8"
        ])
    );
    assert!(answer["result"]["evaluators"].as_array().expect("there are evaluators").len() >= 2);
}

/// The plan is what the document means, and it is the same plan the studio
/// would show.
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
