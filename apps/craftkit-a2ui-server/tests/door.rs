//! The **door** — it speaks a2ui, and it refuses what it cannot serve.
//!
//! The test says what the plan says: the method names are **a2ui's**, the
//! handshake is **two-way**, the inline catalogue is published **only** if
//! the agent accepts one, and a catalogue the door does not serve is
//! **refused**.

use craftkit_a2ui_server::{METHODS, Request, Server};
use craftkit_sdk::Ecosystem;
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
