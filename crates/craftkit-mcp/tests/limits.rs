//! The **limits of the MCP server**: the numbers it will not go past.
//!
//! A limit is a number the server keeps. The numbers that come in are
//! from outside — a client, an agent — so they are checked before they
//! are used, and a limit that no test names is a limit nothing proves
//! holds.

use craftkit_mcp::Server;
use craftkit_mcp::prompts::MAX_SUGGESTIONS;
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

/// The **suggestions** of an argument are cut at `MAX_SUGGESTIONS`, and
/// the answer says how many there were.
///
/// The suggestions are live — they are the nodes of the session, read
/// now — so a session with more than a hundred nodes has more than a
/// hundred suggestions to offer. The cap is the protocol's own, and the
/// count the client is told is the count of everything that **matched**,
/// so a client knows that the list it has is not the whole of the
/// truth.
#[test]
fn the_suggestions_are_cut_at_max_suggestions() {
    let mut session = session();
    for index in 0..150 {
        session
            .execute("prose.document.add", &json!({ "name": format!("the document {index}") }))
            .expect("the document is added");
    }
    let nodes = session.document().nodes().count();
    assert!(nodes > MAX_SUGGESTIONS, "there are more nodes than the cap: {nodes}");
    let mut server = Server::new(session);
    let answer = ask(
        &mut server,
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": "add-paragraph" },
            "argument": { "name": "document", "value": "" },
        }),
    );
    let completion = &answer["result"]["completion"];
    let values = completion["values"].as_array().expect("there are values");
    assert_eq!(values.len(), MAX_SUGGESTIONS, "the list is cut at the cap: {answer:?}");
    assert_eq!(
        completion["total"].as_u64(),
        Some(nodes as u64),
        "and the count says how many there were: {answer:?}"
    );
    assert_eq!(completion["hasMore"], json!(true), "and the answer says there are more: {answer:?}");
}

/// A session with **fewer** nodes than `MAX_SUGGESTIONS` is not cut.
///
/// This is the other half of the test above: a cap that cut everything
/// is not a cap, it is a wall.
#[test]
fn a_few_suggestions_are_not_cut() {
    let mut session = session();
    for index in 0..3 {
        session
            .execute("prose.document.add", &json!({ "name": format!("the document {index}") }))
            .expect("the document is added");
    }
    let nodes = session.document().nodes().count();
    assert!(nodes < MAX_SUGGESTIONS, "there are fewer nodes than the cap: {nodes}");
    let mut server = Server::new(session);
    let answer = ask(
        &mut server,
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": "add-paragraph" },
            "argument": { "name": "document", "value": "" },
        }),
    );
    let completion = &answer["result"]["completion"];
    let values = completion["values"].as_array().expect("there are values");
    assert_eq!(values.len(), nodes, "every match is sent: {answer:?}");
    assert_eq!(completion["total"].as_u64(), Some(nodes as u64), "and the count says so: {answer:?}");
    assert_eq!(completion["hasMore"], json!(false), "and there are no more: {answer:?}");
}
