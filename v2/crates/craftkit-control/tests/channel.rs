//! The channel, over a real socket: a session on one end and a program
//! on the other.
//!
//! What is proved here: the door works, the token is the door's lock,
//! and a method that does not exist is named rather than ignored.

use std::net::SocketAddr;

use craftkit_control::{Client, ControlError, Method, Options, Server};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// A server, and the address it is listening on.
fn a_server(token: &str) -> (Server, SocketAddr) {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    let session: Session = ecosystem.session();
    let options =
        Options { token: token.to_string(), address: SocketAddr::from(([127, 0, 0, 1], 0)), announce: None };
    let server = Server::serve(session, options).expect("the server serves");
    let address = server.address();
    (server, address)
}

/// A client with the right token.
fn a_client(address: SocketAddr, token: &str) -> Client {
    Client::connect(address, token).expect("the client connects")
}

/// The door works.
#[test]
fn a_client_can_ping() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    let answer = client.call(Method::Ping, json!({})).expect("the ping is answered");
    assert_eq!(answer["pong"], json!(true));
    drop(client);
    drop(server);
}

/// A client can **run a command**, and the session changes.
#[test]
fn a_client_can_run_a_command() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    let answer = client
        .call(Method::Execute, json!({ "command": "tool.hold", "params": { "tool": "a tool" } }))
        .expect("the command runs");
    assert!(answer["message"].is_string(), "the command said something: {answer}");
    let state = client.call(Method::State, json!({})).expect("the state is reported");
    assert!(state["history"].is_object(), "the state is reported: {state}");
    drop(client);
    drop(server);
}

/// A command that **asks** while it runs can be answered over the
/// channel, the same way the MCP door answers it.
#[test]
fn a_command_that_asks_can_be_answered_over_the_channel() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    let asked = client
        .call(Method::Execute, json!({ "command": "doc.ask", "params": { "question": "why?" } }))
        .expect("the command asks");
    assert!(asked["needs_input"].is_object(), "the command is waiting: {asked}");
    assert_eq!(asked["needs_input"]["name"], json!("answer"));
    let answered = client
        .call(
            Method::Execute,
            json!({ "command": "doc.ask", "params": { "question": "why?", "answer": "because" } }),
        )
        .expect("the answer is taken");
    assert_eq!(answered["changed"], json!(true), "the command went on");
    let state = client.call(Method::State, json!({})).expect("the state is reported");
    assert_eq!(state["nodes"], json!(1), "the note is in the document");
    drop(client);
    drop(server);
}

/// The catalogue is the **command list**, the way it is everywhere else.
#[test]
fn the_catalogue_is_the_command_list() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    let answer = client.call(Method::Describe, json!({})).expect("the catalogue is described");
    let commands = answer["commands"].as_array().expect("a list of commands");
    assert!(!commands.is_empty(), "there are commands");
    assert!(commands.iter().any(|command| command["id"] == json!("tool.hold")), "and tool.hold is one");
    assert!(answer["kinds"].as_array().is_some(), "the kinds are there too");
    drop(client);
    drop(server);
}

/// The **state** says what the session is holding.
#[test]
fn the_state_is_reported() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    client
        .call(
            Method::Execute,
            json!({ "command": "doc.ask", "params": { "question": "why?", "answer": "because" } }),
        )
        .expect("the command runs");
    let state = client.call(Method::State, json!({})).expect("the state is reported");
    assert_eq!(state["nodes"], json!(1), "the note is in the document");
    assert!(state["history"]["states"].as_u64().expect("a number") >= 1, "the history is there");
    drop(client);
    drop(server);
}

/// The **diagnostics** say what has happened.
#[test]
fn the_diagnostics_are_reported() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    client
        .call(Method::Execute, json!({ "command": "tool.hold", "params": { "tool": "a tool" } }))
        .expect("the command runs");
    let diagnostics = client.call(Method::Diagnostics, json!({})).expect("the diagnostics are reported");
    assert!(diagnostics["log"].as_array().is_some(), "the log is there");
    drop(client);
    drop(server);
}

/// A client with the **wrong token** is refused.
#[test]
fn a_client_with_the_wrong_token_is_refused() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "wrong");
    let error = client.call(Method::Ping, json!({})).expect_err("the token is wrong");
    assert!(matches!(error, ControlError::Refused { .. }), "{error}");
    drop(client);
    drop(server);
}

/// A method that **does not exist** is named, and the methods that do
/// are listed.
#[test]
fn a_method_that_does_not_exist_is_named() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    let answer = client.send("{\"id\": 1, \"method\": \"engine.explode\"}").expect("the answer comes");
    assert_eq!(answer["ok"], json!(false));
    assert_eq!(answer["kind"], json!("unknown-method"));
    let message = answer["message"].as_str().expect("a message");
    assert!(message.contains("engine.explode"), "{message}");
    assert!(message.contains("ping"), "and it says what there is: {message}");
    drop(client);
    drop(server);
}

/// A line that is **not JSON** is answered with an error and not with a
/// closed connection: the client can carry on.
#[test]
fn a_line_that_is_not_json_is_answered() {
    let (server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    let answer = client.send("not json at all").expect("the answer comes");
    assert_eq!(answer["ok"], json!(false));
    assert_eq!(answer["kind"], json!("protocol"));
    drop(client);
    drop(server);
}

/// The server can be **stopped**.
#[test]
fn the_server_can_be_stopped() {
    let (mut server, address) = a_server("secret");
    let mut client = a_client(address, "secret");
    client.call(Method::Ping, json!({})).expect("the ping is answered");
    server.shutdown();
}

/// A line that is **longer than the protocol allows** is refused: the
/// reader counts the bytes as it reads them, so a peer that never ends
/// its line cannot make the server allocate without limit.
#[test]
fn a_line_that_is_too_long_is_refused() {
    use std::io::{BufRead, Write};
    let (_server, address) = a_server("secret");
    let mut stream = std::net::TcpStream::connect(address).expect("the connection is made");
    // The line is longer than the protocol's limit, and it never ends.
    let mut line = vec![b'x'; craftkit_control::MAX_LINE + 1024];
    line.push(b'\n');
    stream.write_all(&line).expect("the bytes are written");
    stream.flush().expect("the bytes are flushed");
    let mut answer = String::new();
    let read = std::io::BufReader::new(stream).read_line(&mut answer).expect("the answer is read");
    // The server either refuses the line or closes the connection: what
    // it does not do is read the whole line.
    assert!(read == 0 || answer.contains("longer"), "the answer was: {answer:?}");
}
