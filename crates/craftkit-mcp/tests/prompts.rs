//! The **prompts** of the MCP server: the workflows a client can offer
//! a user.

use craftkit_mcp::Server;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// A battery, linked in so that a prompt that names one of its
/// commands is served.
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

/// The **list** of the prompts: every workflow the server offers.
#[test]
fn the_prompts_are_listed() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "prompts/list", json!({}));
    let prompts = answer["result"]["prompts"].as_array().expect("there are prompts");
    assert!(prompts.len() >= 3, "there are several: {answer:?}");
    for prompt in prompts {
        assert!(!prompt["name"].as_str().unwrap_or_default().is_empty(), "each has a name");
        assert!(!prompt["title"].as_str().unwrap_or_default().is_empty(), "and a title");
        assert!(!prompt["description"].as_str().unwrap_or_default().is_empty(), "and says what it does");
    }
}

/// The **message** of a prompt is built from the registry: the id, the
/// label and the description of the command it names.
#[test]
fn the_message_comes_from_the_registry() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "prompts/get", json!({ "name": "plan-the-document" }));
    let text = answer["result"]["messages"][0]["content"]["text"].as_str().expect("there is a text");
    assert!(text.contains("doc.plan"), "the message names the command: {text}");
    assert!(text.contains("What the document means"), "and says what the command does: {text}");
}

/// A **prompt** that is fetched with its arguments has them in its
/// message.
#[test]
fn the_arguments_are_in_the_message() {
    let mut server = Server::new(session());
    let answer = ask(
        &mut server,
        "prompts/get",
        json!({ "name": "new-document", "arguments": { "name": "the report" } }),
    );
    let text = answer["result"]["messages"][0]["content"]["text"].as_str().expect("there is a text");
    assert!(text.contains("the report"), "the name is in the message: {text}");
    // An argument that is not given has its **default**.
    let answer = ask(&mut server, "prompts/get", json!({ "name": "new-document" }));
    let text = answer["result"]["messages"][0]["content"]["text"].as_str().expect("there is a text");
    assert!(text.contains("the document"), "the default is in the message: {text}");
}

/// A **required argument** that is left out is refused, and so is a
/// prompt that names nothing.
#[test]
fn a_prompt_that_cannot_be_fetched_is_refused() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "prompts/get", json!({ "name": "add-paragraph" }));
    assert_eq!(answer["error"]["code"], json!(-32602), "the code is the MCP's: {answer:?}");
    assert!(
        answer["error"]["message"].as_str().unwrap_or_default().contains("document"),
        "and the message says which argument: {answer:?}"
    );
    let answer = ask(&mut server, "prompts/get", json!({ "name": "no-such-prompt" }));
    assert_eq!(answer["error"]["code"], json!(-32602), "a prompt that names nothing is refused");
    let answer = ask(&mut server, "prompts/get", json!({}));
    assert_eq!(answer["error"]["code"], json!(-32602), "a missing name is a bad request");
}

/// The **suggestions** of an argument are live: they are the nodes of
/// the session, read now.
#[test]
fn the_suggestions_are_live() {
    let mut session = session();
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let id = outcome.data["id"].as_u64().expect("the document has an id");
    let mut server = Server::new(session);
    let answer = ask(
        &mut server,
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": "add-paragraph" },
            "argument": { "name": "document", "value": "" },
        }),
    );
    let values = answer["result"]["completion"]["values"].as_array().expect("there are values");
    assert!(
        values.iter().any(|value| value.as_str() == Some(&id.to_string())),
        "the node is there: {answer:?}"
    );
    // And they are **filtered** by what has been typed.
    let answer = ask(
        &mut server,
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": "add-paragraph" },
            "argument": { "name": "document", "value": "999" },
        }),
    );
    let values = answer["result"]["completion"]["values"].as_array().expect("there are values");
    assert!(values.is_empty(), "nothing matches: {answer:?}");
    assert_eq!(answer["result"]["completion"]["total"], json!(0), "and the count says so");
}

/// A **value that looks like a placeholder** is left alone: the
/// template is read once, and what is put in it is not read again.
#[test]
fn a_value_that_looks_like_a_placeholder_is_left_alone() {
    let mut server = Server::new(session());
    let answer =
        ask(&mut server, "prompts/get", json!({ "name": "new-document", "arguments": { "name": "{doc}" } }));
    let text = answer["result"]["messages"][0]["content"]["text"].as_str().expect("there is a text");
    assert!(text.contains("{doc}"), "the caller's text is left as it is: {text}");
    assert!(
        !text.contains("Say what the command does"),
        "and the registry's words did not go into the caller's field: {text}"
    );
}

/// The server **answers a shutdown**, and the caller stops.
#[test]
fn a_shutdown_is_answered() {
    let mut server = Server::new(session());
    let answer = ask(&mut server, "shutdown", json!({}));
    assert!(answer["result"].is_object(), "the server answers: {answer:?}");
}
