//! The **resources** of the MCP server: the state a client can read
//! without calling a tool.

use craftkit_mcp::Server;
use craftkit_sdk::serde_json::{Value as Json, json};
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

/// Ask the server a question, and give the answer.
fn ask(server: &mut Server, method: &str, params: Json) -> Json {
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let answer = server.handle_line(&request.to_string()).expect("the server answers");
    serde_json::from_str(&answer).expect("the answer is JSON")
}

/// The **list** of the resources: every resource the server serves.
#[test]
fn the_resources_are_listed() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "resources/list", json!({}));
    let resources = answer["result"]["resources"].as_array().expect("there are resources");
    assert_eq!(resources.len(), 2, "the two resources: {answer:?}");
    let uris: Vec<&str> =
        resources.iter().map(|resource| resource["uri"].as_str().unwrap_or_default()).collect();
    assert_eq!(uris, vec!["craftkit://commands", "craftkit://design"], "and they are named");
    for resource in resources {
        assert!(!resource["name"].as_str().unwrap_or_default().is_empty(), "each has a name");
        assert!(!resource["description"].as_str().unwrap_or_default().is_empty(), "and says what it holds");
        assert_eq!(resource["mimeType"], json!("application/json"), "and a media type");
    }
}

/// The **catalogue** is the commands with their typed parameters: the
/// content the tools are built from.
#[test]
fn the_catalogue_is_the_commands() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "resources/read", json!({ "uri": "craftkit://commands" }));
    let text = answer["result"]["contents"][0]["text"].as_str().expect("there is a text");
    let catalogue: Json = serde_json::from_str(text).expect("the text is JSON");
    let commands = catalogue["commands"].as_array().expect("there are commands");
    assert!(commands.len() > 100, "there are many: {}", commands.len());
    let prose = commands
        .iter()
        .find(|command| command["id"] == json!("prose.document.add"))
        .expect("a battery's command is there");
    assert_eq!(prose["area"], json!("prose"), "and it says what area it is in");
    let first = &commands[0];
    assert!(!first["id"].as_str().unwrap_or_default().is_empty(), "each command has an id");
    assert!(!first["label"].as_str().unwrap_or_default().is_empty(), "and a label");
    let with_params = commands
        .iter()
        .find(|command| !command["params"].as_array().map(|params| params.is_empty()).unwrap_or(true))
        .expect("some command has parameters");
    let param = &with_params["params"][0];
    assert!(!param["name"].as_str().unwrap_or_default().is_empty(), "a parameter has a name");
    assert!(!param["kind"].as_str().unwrap_or_default().is_empty(), "and a kind");
}

/// The **design** is the nodes the session holds, and the relations
/// between them.
#[test]
fn the_design_is_the_session() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "resources/read", json!({ "uri": "craftkit://design" }));
    let text = answer["result"]["contents"][0]["text"].as_str().expect("there is a text");
    let design: Json = serde_json::from_str(text).expect("the text is JSON");
    assert!(design["nodes"].is_array(), "there are nodes: {design:?}");
    assert!(design["relations"].is_array(), "and relations");
}

/// A **template** is a URI with a variable in it, and the variable
/// names a node.
#[test]
fn a_template_names_a_node() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "resources/templates/list", json!({}));
    let templates = answer["result"]["resourceTemplates"].as_array().expect("there are templates");
    assert_eq!(templates.len(), 1, "there is one template: {answer:?}");
    assert_eq!(templates[0]["uriTemplate"], json!("craftkit://node/{id}"), "and it is the node");
    // A node of the session, read by its id.
    let mut session = session();
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let id = outcome.data["id"].as_u64().expect("the document has an id");
    let mut server = Server::new(session);
    let answer = ask(&mut server, "resources/read", json!({ "uri": format!("craftkit://node/{id}") }));
    let text = answer["result"]["contents"][0]["text"].as_str().expect("there is a text");
    let node: Json = serde_json::from_str(text).expect("the text is JSON");
    assert_eq!(node["id"], json!(id), "the node is the one that was asked for");
    assert_eq!(node["name"], json!("the document"), "and it is named");
}

/// A URI that names nothing is refused with the MCP's own code.
#[test]
fn a_uri_that_names_nothing_is_refused() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "resources/read", json!({ "uri": "craftkit://nope" }));
    assert_eq!(answer["error"]["code"], json!(-32002), "the code is the MCP's: {answer:?}");
    assert!(
        answer["error"]["message"].as_str().unwrap_or_default().contains("resources/list"),
        "and the message says where to look: {answer:?}"
    );
    let answer = ask(&mut server, "resources/read", json!({}));
    assert_eq!(answer["error"]["code"], json!(-32602), "a missing uri is a bad request");
}

/// The server says it can serve resources, and it says so in the
/// handshake.
#[test]
fn the_handshake_says_it_serves_resources() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "initialize", json!({ "protocolVersion": "2025-06-18" }));
    let capabilities = &answer["result"]["capabilities"];
    assert!(capabilities.get("tools").is_some(), "the tools are still there: {answer:?}");
    assert!(capabilities.get("resources").is_some(), "and the resources are: {answer:?}");
}

/// A resource **reads**: it changes nothing.
#[test]
fn a_resource_changes_nothing() {
    let session = session();
    let before = session.document().nodes().count();
    let mut server = Server::new(session);
    ask(&mut server, "resources/read", json!({ "uri": "craftkit://design" }));
    ask(&mut server, "resources/read", json!({ "uri": "craftkit://commands" }));
    assert_eq!(server.session().document().nodes().count(), before, "the design is the same");
}
