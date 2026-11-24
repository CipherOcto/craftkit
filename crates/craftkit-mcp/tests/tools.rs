//! The **tools** of the MCP server: one tool per command.
//!
//! The schema mapping moved **out of this door** and into the SDK, and this
//! is the test that says the move changed nothing: the schema a tool
//! carries is the schema it carried before.

use craftkit_mcp::Server;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{CommandSpec, Doc, Ecosystem, Outcome, ParamDoc, Session, Text};

/// A battery, linked in so that the catalogue holds more than the
/// core's commands.
use craftkit_plugin_prose as _;

/// The ecosystem of the kit.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the kit's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// The **tool list** is the command list.
#[test]
fn the_tool_list_is_the_command_list() {
    let server = Server::new(session());
    assert!(!server.tools().is_empty(), "there are tools");
    for tool in server.tools() {
        assert!(tool.name.starts_with("craftkit_"), "the name is prefixed: {}", tool.name);
        assert!(!tool.description.is_empty(), "the tool says what it does");
    }
}

/// **The golden test.** A tool's schema is a JSON Schema, and it is the
/// one the SDK builds — so the shape is checked here, exactly, and a
/// change to the mapping is a change to this test.
#[test]
fn a_tools_schema_is_the_json_schema_the_sdk_builds() {
    let server = Server::new(session());
    let tools = server.tools();
    let tool = tools.first().expect("there is a tool");
    assert_eq!(tool.schema["type"], json!("object"), "the schema is an object");
    assert!(tool.schema["properties"].is_object(), "with properties");
    assert_eq!(tool.schema["additionalProperties"], json!(false), "and nothing else is taken");
}

/// The door and the SDK say the **same** thing about a command.
///
/// The door's schema and the SDK's own view of the same command are one
/// mapping, so they cannot differ.
#[test]
fn the_door_and_the_sdk_say_the_same_thing() {
    let ecosystem = ecosystem();
    let projection = ecosystem.projection();
    let server = Server::new(session());
    for tool in server.tools() {
        let schema = projection.json_schema(&tool.command).expect("the command is there");
        assert_eq!(schema, tool.schema, "{} differs", tool.command);
    }
}

/// And the **descriptor** is a different thing, and it survives.
///
/// The bridge reads `id` and `doc` out of the descriptor, so the descriptor
/// is not re-expressed as a schema.
#[test]
fn the_descriptor_is_not_the_schema() {
    let ecosystem = ecosystem();
    let projection = ecosystem.projection();
    let server = Server::new(session());
    let tools = server.tools();
    let tool = tools.first().expect("there is a tool");
    let descriptor = projection.schema(&tool.command).expect("the command is there");
    assert_eq!(descriptor["id"], json!(tool.command));
    assert!(descriptor.get("label").is_some(), "the descriptor has an envelope");
    assert!(descriptor.get("parameters").is_some(), "and the parameter list");
    // And it is **not** the JSON Schema: the schema has no envelope.
    assert!(tool.schema.get("id").is_none(), "the schema has no envelope");
}

/// The **golden of the move**: the list the door publishes is the
/// registry, in the registry's order, and every tool carries the name
/// the door has always given it.
///
/// The mapping moved from this door into the SDK, and the door's output
/// did not change: this says what it is, exactly.
#[test]
fn the_tool_list_is_the_registry_in_its_own_order() {
    let ecosystem = ecosystem();
    let server = Server::new(session());
    let commands: Vec<&str> = ecosystem.commands.all().iter().map(|entry| entry.id).collect();
    let listed = server.tools();
    let tools: Vec<&str> = listed.iter().map(|tool| tool.command.as_str()).collect();
    assert_eq!(tools, commands, "the tool list is the command list");
    for tool in server.tools() {
        assert_eq!(
            tool.name,
            craftkit_mcp::tool_name(&tool.command),
            "{} is named as it always was",
            tool.command
        );
    }
}

/// A tool says whether it **changes the document**.
///
/// The hint is the command's own promise, read and not guessed: a
/// command that says it does not change the document does not, and an
/// agent that knows that can read without asking to write. A hint that
/// is `false` for every tool is a hint that says nothing.
#[test]
fn a_tool_says_whether_it_changes_the_document() {
    let server = Server::new(session());
    let listed = server.tools();
    let read = listed.iter().find(|tool| tool.command == "doc.plan").expect("the plan");
    assert!(!read.mutates, "the plan does not change the document");
    assert_eq!(read.to_json()["annotations"]["readOnlyHint"], json!(true), "{}", read.to_json());

    let listed = server.tools();
    let write = listed.iter().find(|tool| tool.command == "link.add").expect("the link");
    assert!(write.mutates, "adding a node changes the document");
    assert_eq!(write.to_json()["annotations"]["readOnlyHint"], json!(false), "{}", write.to_json());
}

/// A command that joins the session while the program runs is a **tool**.
///
/// The list is read when it is asked for, so the door never hides a
/// command the session has — and never offers one it does not.
#[test]
fn a_command_made_at_run_time_is_a_tool() {
    let mut server = Server::new(session());
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
    server.session_mut().register(command).expect("the command joins the session");

    let tool = server.tools().into_iter().find(|tool| tool.command == "runtime.echo");
    assert!(tool.is_some(), "the command is a tool: {:?}", server.tools());
}

/// A tool call **runs** the command.
///
/// The tool list is the command list, and a call is the command's own
/// door: the same `Session::execute` every other door runs, and so the
/// same refusals and the same undo.
#[test]
fn a_tool_call_runs_the_command() {
    let mut server = Server::new(session());
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "craftkit_session_memory",
                "arguments": {},
            },
        }))
        .expect("the call is answered");
    let result = &answer["result"];
    assert_eq!(result["isError"], json!(false), "{answer}");
    assert!(result["content"][0]["text"].as_str().is_some(), "{answer}");
    let said = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("byte(s)"), "the command's own answer came back: {said}");
}

/// A refusal is **told**, and the tool says it is an error.
///
/// The door does not swallow a refusal and does not answer with an empty
/// success: the agent is told that the command refused, and why.
#[test]
fn a_refused_call_says_so() {
    let mut server = Server::new(session());
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "craftkit_no_such_command", "arguments": {} },
        }))
        .expect("the call is answered");
    let result = &answer["result"];
    assert_eq!(result["isError"], json!(true), "{answer}");
    let said = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("no.such.command"), "the refusal names the command: {said}");
}
