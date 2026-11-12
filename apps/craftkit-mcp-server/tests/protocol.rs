//! The MCP door: the registry, spoken to an agent.
//!
//! One tool per command, the schema built from the command's typed
//! parameters, and calling the tool runs the command.

use craftkit_mcp::{Server, command_id, tool_name};
use craftkit_sdk::serde_json::{Value as Json, json};

/// The ecosystem the tests drive.
fn ecosystem() -> craftkit_sdk::Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A server over a fresh session.
fn server() -> Server {
    Server::new(ecosystem().session())
}

/// The tool's name is the command's id.
#[test]
fn the_tools_name_is_the_commands_id() {
    assert_eq!(tool_name("doc.open"), "craftkit_doc_open");
    assert_eq!(command_id("craftkit_doc_open"), Some("doc.open".to_string()));
    assert_eq!(command_id("not_craftkit"), None);
}

/// Every command is a tool, and there is **one tool per command**.
#[test]
fn every_command_is_a_tool() {
    let ecosystem = ecosystem();
    let server = Server::new(ecosystem.session());
    assert_eq!(
        server.tools().len(),
        ecosystem.commands.all().len(),
        "one tool per command: {} commands, {} tools",
        ecosystem.commands.all().len(),
        server.tools().len()
    );
    // The tool list is the command list: there is no second vocabulary.
    for spec in ecosystem.commands.all() {
        let tool = server
            .tools()
            .iter()
            .find(|tool| tool.command == spec.id)
            .unwrap_or_else(|| panic!("`{}` is a command and there is no tool for it", spec.id));
        assert_eq!(tool.name, tool_name(spec.id));
    }
}

/// The tool's schema is the command's **typed** parameters.
#[test]
fn the_schema_is_the_typed_parameters() {
    let server = server();
    let tool = server.tools().iter().find(|tool| tool.command == "node.add").expect("the command is there");
    let schema = &tool.schema;
    assert_eq!(schema["type"], json!("object"));
    assert_eq!(schema["additionalProperties"], json!(false), "no other keys are allowed");
    assert_eq!(schema["properties"]["name"]["type"], json!("string"));
    assert_eq!(schema["properties"]["kind"]["type"], json!("string"));
    assert_eq!(schema["required"], json!(["name", "kind"]));
}

/// The initialize handshake answers with the protocol and the server.
#[test]
fn the_handshake_answers() {
    let mut server = server();
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } }))
        .expect("the handshake is answered");
    assert_eq!(answer["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(answer["result"]["serverInfo"]["name"], json!("craftkit"));
    let instructions = answer["result"]["instructions"].as_str().unwrap_or_default().to_string();
    assert!(instructions.contains("a tool of its own"), "{answer}");
    // The handshake says how many commands there are, and it is the
    // number of tools: the registry and the tool list are the same size.
    let commands = ecosystem().commands.all().len();
    assert!(instructions.contains(&format!("{commands} commands")), "{answer}");
}

/// A protocol version the server does not know falls back to the one it
/// speaks.
#[test]
fn an_unknown_protocol_version_falls_back() {
    let mut server = server();
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "1999-01-01" } }))
        .expect("the handshake is answered");
    assert_eq!(answer["result"]["protocolVersion"], json!("2025-06-18"));
}

/// The tool list is the command list, in the protocol's shape.
#[test]
fn the_tool_list_is_the_command_list() {
    let mut server = server();
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
        .expect("the list is answered");
    let tools = answer["result"]["tools"].as_array().expect("there are tools");
    assert!(!tools.is_empty());
    let names: Vec<&str> = tools.iter().filter_map(|tool| tool["name"].as_str()).collect();
    assert!(names.contains(&"craftkit_doc_open"), "{names:?}");
    assert!(names.contains(&"craftkit_node_add"), "{names:?}");
}

/// Calling a tool runs the command.
#[test]
fn calling_a_tool_runs_the_command() {
    let mut server = server();
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "craftkit_node_add", "arguments": { "name": "a box", "kind": "geometry.rect" } },
        }))
        .expect("the call is answered");
    assert_eq!(answer["result"]["isError"], json!(false), "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    assert!(text.contains("a box"), "{answer}");
}

/// A command that fails is an **error result**, and the client is told.
#[test]
fn a_command_that_fails_is_an_error_result() {
    let mut server = server();
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": { "name": "craftkit_node_add", "arguments": { "name": "a box" } },
        }))
        .expect("the call is answered");
    assert_eq!(answer["result"]["isError"], json!(true), "{answer}");
}

/// A tool that is not there is refused by name.
#[test]
fn a_tool_that_is_not_there_is_refused() {
    let mut server = server();
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": { "name": "craftkit_nothing_at_all", "arguments": {} },
        }))
        .expect("the call is answered");
    assert_eq!(answer["result"]["isError"], json!(true), "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    assert!(text.contains("no command"), "{answer}");
}

/// A method that is not there is an error, with the code that says so.
#[test]
fn a_method_that_is_not_there_is_an_error() {
    let mut server = server();
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 6, "method": "nothing/at/all" }))
        .expect("the call is answered");
    assert_eq!(answer["error"]["code"], json!(-32601), "{answer}");
}

/// A **notification** gets no answer.
#[test]
fn a_notification_gets_no_answer() {
    let mut server = server();
    let answer = server.handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    assert!(answer.is_none(), "{answer:?}");
}

/// A line that is not JSON is answered with a parse error, and the
/// server carries on.
#[test]
fn a_line_that_is_not_json_is_answered() {
    let mut server = server();
    let answer = server.handle_line("this is not json").expect("the answer is a line");
    let parsed: Json = serde_json::from_str(&answer).expect("the answer is JSON");
    assert_eq!(parsed["error"]["code"], json!(-32700), "{parsed}");
    // And the server is still a server.
    let answer =
        server.handle_line("{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}").expect("the answer");
    let parsed: Json = serde_json::from_str(&answer).expect("the answer is JSON");
    assert_eq!(parsed["id"], json!(7), "{parsed}");
}

/// The session is **the session**: a command that changes the document
/// changes it for the next call.
#[test]
fn the_session_is_the_session() {
    let mut server = server();
    server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": { "name": "craftkit_node_add", "arguments": { "name": "a box", "kind": "geometry.rect" } },
        }))
        .expect("the call is answered");
    assert_eq!(server.session().document().nodes.len(), 1);
}

/// An agent is told **what** is missing, of what kind, and that it can
/// call again: the missing parameter is the one failure an agent can fix
/// by calling again.
#[test]
fn an_agent_is_told_what_is_missing() {
    let mut server = server();
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": { "name": "craftkit_node_add", "arguments": { "kind": "geometry.rect" } },
        }))
        .expect("the call is answered");
    assert_eq!(answer["result"]["isError"], json!(true), "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    assert!(text.contains("needs `name`"), "{text}");
    assert!(text.contains("Call it again with it"), "{text}");
}

/// A command that **asks** while it runs is told what it wants, in the
/// same words as a command that is missing a parameter: the agent learns
/// one thing and not two.
#[test]
fn a_command_that_asks_is_told_what_it_wants() {
    let mut server = server();
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": { "name": "craftkit_doc_ask", "arguments": { "question": "why?" } },
        }))
        .expect("the call is answered");
    assert_eq!(answer["result"]["isError"], json!(true), "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    assert!(text.contains("needs `answer`"), "{answer}");
    assert!(text.contains("Call it again with it."), "{answer}");
}

/// And the agent can answer it: the command runs again and goes on.
#[test]
fn the_agent_can_answer_the_question() {
    let mut server = server();
    server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": { "name": "craftkit_doc_ask", "arguments": { "question": "why?" } },
        }))
        .expect("the call is answered");
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "craftkit_doc_ask",
                "arguments": { "question": "why?", "answer": "because" },
            },
        }))
        .expect("the call is answered");
    assert_eq!(answer["result"]["isError"], json!(false), "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    assert!(text.contains("why?"), "{answer}");
}

/// A **typed** parameter says what it is: the tool's schema carries the
/// kind's own name and its unit, so that an agent is told what the
/// number means and not only what shape it has.
#[test]
fn a_typed_parameter_says_its_name_and_its_unit() {
    let mut server = server();
    let answer = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 20, "method": "tools/list" }))
        .expect("the list is answered");
    let tools = answer["result"]["tools"].as_array().expect("there are tools");
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == json!("craftkit_numeric_length_add"))
        .expect("the tool is there");
    let schema = &tool["inputSchema"];
    let value = &schema["properties"]["value"];
    assert_eq!(value["type"], json!("number"), "the shape is a number: {value}");
    assert_eq!(value["title"], json!("length"), "and the name says what it is: {value}");
    assert_eq!(value["unit"], json!("mm"), "and the unit says what the number means: {value}");
}
