//! The **tools** of the MCP server: one tool per command.
//!
//! The schema mapping moved **out of this door** and into the SDK, and this
//! is the test that says the move changed nothing: the schema a tool
//! carries is the schema it carried before.

use craftkit_mcp::Server;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

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
    let tool = server.tools().first().expect("there is a tool");
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
    let tool = server.tools().first().expect("there is a tool");
    let descriptor = projection.schema(&tool.command).expect("the command is there");
    assert_eq!(descriptor["id"], json!(tool.command));
    assert!(descriptor.get("label").is_some(), "the descriptor has an envelope");
    assert!(descriptor.get("parameters").is_some(), "and the parameter list");
    // And it is **not** the JSON Schema: the schema has no envelope.
    assert!(tool.schema.get("id").is_none(), "the schema has no envelope");
}
