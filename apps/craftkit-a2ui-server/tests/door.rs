//! The **door** — it speaks a2ui, and it refuses what it cannot serve.
//!
//! The test says what the plan says: the method names are **a2ui's**, the
//! handshake is **two-way**, the inline catalogue is published **only** if
//! the agent accepts one, and a catalogue the door does not serve is
//! **refused**.

use craftkit_a2ui_server::render::Headless;
use craftkit_a2ui_server::{METHODS, Request, Server};
use craftkit_sdk::Ecosystem;
use craftkit_sdk::Point;
use craftkit_sdk::serde_json::{Value as Json, json};

// The plugin's anchor, held by the test: without it the linker drops the
// plugin's items and the registry is empty.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_a2ui::CRAFTKIT_PLUGIN_ANCHOR;

/// A door over the ecosystem.
fn server() -> Server {
    Server::new(Ecosystem::from_inventory().expect("the ecosystem is built"))
}

/// A request.
fn request(method: &str, params: Json) -> Request {
    Request { id: Some(json!(1)), method: method.to_string(), params }
}

/// The **methods** are a2ui's, and the transport's one.
#[test]
fn the_methods_are_a2uis() {
    for method in [
        "createSurface",
        "updateComponents",
        "updateDataModel",
        "deleteSurface",
        "callRendererFunction",
        "agentFunctionResponse",
    ] {
        assert!(METHODS.contains(&method), "{method} is a method");
    }
    assert!(METHODS.contains(&"initialize"), "and the handshake is the transport's");
}

/// The **handshake** is two-way: the agent's capabilities up, the
/// renderer's down.
#[test]
fn the_handshake_is_two_way() {
    let mut server = server();
    let answer =
        server.handle(&request("initialize", json!({ "capabilities": { "acceptsInlineCatalogs": true } })));
    let capabilities = &answer["result"]["a2uiRendererCapabilities"];
    assert!(capabilities.is_object(), "{answer}");
    assert!(capabilities["inlineCatalogs"].is_array(), "the inline catalogue is there: {answer}");
    assert!(capabilities["supportedCatalogIds"].is_array(), "and the catalogue is named");
}

/// The inline catalogue is **only** published when the agent accepts it.
#[test]
fn the_inline_catalogue_is_only_published_when_it_is_accepted() {
    let mut server = server();
    let answer =
        server.handle(&request("initialize", json!({ "capabilities": { "acceptsCatalogFiles": true } })));
    let capabilities = &answer["result"]["a2uiRendererCapabilities"];
    assert!(capabilities.get("inlineCatalogs").is_none(), "{answer}");
}

/// An agent that accepts **neither** is refused.
#[test]
fn an_agent_that_accepts_neither_is_refused() {
    let mut server = server();
    let answer = server.handle(&request("initialize", json!({ "capabilities": {} })));
    assert!(answer["result"]["error"].is_string(), "{answer}");
}

/// The catalogue the door serves is the **layer's own**.
#[test]
fn the_catalogue_the_door_serves_is_the_layers_own() {
    let mut server = server();
    let answer =
        server.handle(&request("initialize", json!({ "capabilities": { "acceptsInlineCatalogs": true } })));
    let catalogue = &answer["result"]["a2uiRendererCapabilities"]["inlineCatalogs"][0];
    assert_eq!(
        catalogue["catalogId"],
        json!(craftkit_a2ui::catalog::CATALOG_ID),
        "and it is the one the door serves"
    );
    assert!(catalogue["components"].is_object(), "and it holds the components");
}

/// A surface that names a catalogue the door does not serve is
/// **refused**.
#[test]
fn a_surface_that_names_another_catalogue_is_refused() {
    let mut server = server();
    let answer = server.handle(&request(
        "createSurface",
        json!({ "surfaceId": "s1", "catalogId": "https://example.com/other.json" }),
    ));
    assert!(answer.get("error").is_some(), "{answer}");
}

/// A surface that omits the catalogue gets the door's own.
#[test]
fn a_surface_that_omits_the_catalogue_gets_the_doors_own() {
    let mut server = server();
    let answer = server.handle(&request("createSurface", json!({ "surfaceId": "s2" })));
    assert!(answer.get("error").is_none(), "{answer}");
    assert_eq!(answer["result"]["surfaceId"], json!("s2"));
}

/// A surface that carries **extensions** is refused.
#[test]
fn a_surface_that_carries_extensions_is_refused() {
    let mut server = server();
    let answer = server.handle(&request(
        "createSurface",
        json!({ "surfaceId": "s3", "metadata": { "extensions": { "anything": 1 } } }),
    ));
    assert!(answer.get("error").is_some(), "{answer}");
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("extensions"), "{message}");
}

/// A **reserved** function is refused.
#[test]
fn a_reserved_function_is_refused() {
    let mut server = server();
    let answer = server.handle(&request("callRendererFunction", json!({ "name": "@index" })));
    assert!(answer.get("error").is_some(), "{answer}");
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("reserved"), "{message}");
}

/// An **unknown** function is refused, not forwarded.
#[test]
fn an_unknown_function_is_refused() {
    let mut server = server();
    let answer = server.handle(&request("callRendererFunction", json!({ "name": "somethingElse" })));
    assert!(answer.get("error").is_some(), "{answer}");
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("neither"), "{message}");
}

/// A method that is not there is an error, and it says which are.
#[test]
fn an_unknown_method_is_refused() {
    let mut server = server();
    let answer = server.handle(&request("nope", Json::Null));
    assert!(answer.get("error").is_some(), "{answer}");
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("createSurface"), "{message}");
}

/// The **catalogue file** is written, and it is the catalogue.
#[test]
fn the_catalogue_file_is_written() {
    let server = server();
    let path = craftkit_scratch::file("a2ui-catalog.json").expect("the scratch is asked for");
    server.write_catalogue(&path).expect("the catalogue is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    let catalogue: Json = craftkit_sdk::serde_json::from_str(&text).expect("and it is JSON");
    assert_eq!(catalogue["catalogId"], json!(craftkit_a2ui::catalog::CATALOG_ID));
    assert!(catalogue["components"].is_object(), "and it holds the components");
}

/// The door's **own** first request is the handshake, and it is the only
/// method name that is not a2ui's.
#[test]
fn the_handshake_is_the_transports_own() {
    let mut server = server();
    let answer =
        server.handle(&request("initialize", json!({ "capabilities": { "acceptsInlineCatalogs": true } })));
    assert_eq!(answer["result"]["protocolVersion"], json!(craftkit_a2ui_server::PROTOCOL_VERSION));
    assert_eq!(answer["result"]["serverInfo"]["name"], json!(craftkit_a2ui_server::SERVER_NAME));
}

/// The door **answers** `createSurface` with the surface it made.
#[test]
fn the_door_answers_create_surface_with_a_surface() {
    let mut server = server();
    let answer = server.handle(&request("createSurface", json!({ "surfaceId": "s1" })));
    assert_eq!(answer["result"]["surfaceId"], json!("s1"), "{answer}");
    let in_the_tree = server.with_session(|session| {
        session.document().nodes().any(|node| node.kind == "a2ui.surface" && node.name == "s1")
    });
    assert!(in_the_tree, "and the surface is in the tree");
}

/// And a **button** runs its command: the tree holds the button, the
/// click finds it, and the action it carries is the command that runs.
#[test]
fn a_button_runs_its_command() {
    let mut server = server();
    server.handle(&request("createSurface", json!({ "surfaceId": "s2" })));
    let answer = server.handle(&request(
        "updateComponents",
        json!({
            "surfaceId": "s2",
            "components": [{
                "component": "Button",
                "child": "press",
                "action": { "name": "dataSet", "args": { "surface_id": "s2", "path": "/pressed", "value": true } },
            }],
        }),
    ));
    assert_eq!(answer["result"]["surfaceId"], json!("s2"), "{answer}");
    // The **action** the button carries is a command of the tree's own.
    let action = server.with_session(|session| {
        session
            .document()
            .walk()
            .find(|node| node.kind == "a2ui.button")
            .map(|node| node.data["action"]["name"].clone())
    });
    assert_eq!(action, Some(json!("dataSet")), "and it names the command");
    // And the command **runs**: the door runs the local function the
    // button names.
    let answer = server.handle(&request(
        "callRendererFunction",
        json!({ "name": "dataSet", "args": { "surface_id": "s2", "path": "/pressed", "value": true } }),
    ));
    assert_eq!(answer["result"]["name"], json!("dataSet"), "{answer}");
    assert!(answer.get("error").is_none(), "{answer}");
    let model = craftkit_plugin_a2ui::model::model();
    assert_eq!(model["pressed"], json!(true), "and the model has the value");
}

/// The **control channel** serves the door's **own** session, and the
/// one message that is not a reply goes on the wire.
///
/// The door's document is the channel's document: a command that is run
/// over the channel changes what the door is drawing. And a click that
/// raises an **agent-side** action produces a2ui's `action` message — the
/// door's one outbound message that is not a reply to a call.
#[test]
fn the_channel_serves_the_doors_own_session() {
    let mut server = server();
    server.handle(&request("createSurface", json!({ "surfaceId": "s3" })));
    server.handle(&request(
        "updateComponents",
        json!({
            "surfaceId": "s3",
            "components": [{
                "component": "Button",
                "child": "press",
                "action": { "name": "required" },
            }],
        }),
    ));
    // The channel's **render**: the door's own headless one.
    let mut render = Headless::new(server.shared_session(), server.outbound());
    let picture = render.render().expect("the picture is drawn");
    assert!(picture.len() > 8, "the picture holds bytes");
    // And the click finds the button the **door** drew.
    let answer = render.click(Point::new(1.0, 1.0)).expect("the click is answered");
    assert!(!answer["clicked"].is_null(), "the button was clicked: {answer}");
    // And the action is on the wire, for the agent.
    let outbound = server.take_outbound();
    assert_eq!(outbound.len(), 1, "one message is not a reply: {outbound:?}");
    assert_eq!(outbound[0]["method"], json!("action"), "{outbound:?}");
    assert_eq!(outbound[0]["params"]["version"], json!("1.0"), "{outbound:?}");
    let action = &outbound[0]["params"]["action"];
    assert_eq!(action["name"], json!("required"), "{action}");
    assert_eq!(action["surfaceId"], json!("s3"), "{action}");
    assert!(action["sourceComponentId"].is_string(), "{action}");
    assert!(action["timestamp"].is_string(), "{action}");
    assert!(action["context"].is_object(), "{action}");
}
