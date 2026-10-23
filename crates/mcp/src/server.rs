//! The server: read lines, dispatch, answer.
//!
//! # The methods
//!
//! | Method | What it does |
//! |---|---|
//! | `initialize` | Handshake. Advertises the tools and the resources it serves. |
//! | `notifications/initialized` | Acknowledged by doing nothing — it is a notification. |
//! | `tools/list` | Every tool, generated from the command registry. |
//! | `tools/call` | Runs one tool against the session. |
//! | `resources/list` | Every resource: the state a client can read. |
//! | `resources/templates/list` | Every resource template: a URI with a variable in it. |
//! | `resources/read` | Reads one resource by URI. |
//! | `prompts/list` | Every prompt: the workflows a client can offer. |
//! | `prompts/get` | One prompt's message, with its arguments substituted. |
//! | `ping` | Liveness. |
//! | `shutdown` | Replies, then the caller stops. |
//!
//! Anything else is `METHOD_NOT_FOUND` with the list of what does exist. A
//! guessed method that silently returned `null` is how an agent concludes the
//! server works but the feature does not.
//!
//! # `resources/read` results
//!
//! A resource is *content*, like a tool result: `{"contents":[{"uri":…,"mimeType":…,"text":…}]}`.
//! A URI that names no resource is `RESOURCE_NOT_FOUND` — **not**
//! `METHOD_NOT_FOUND`, because the client asked for something and something
//! was there to ask for. §176.
//!
//! # `tools/call` results
//!
//! An MCP tool result is *content*, and `isError` is how a failure is reported.
//! So a refused command comes back as
//!
//! ```json
//! {"content":[{"type":"text","text":"…"}],"isError":true,"structuredContent":{"kind":"bad-parameter"}}
//! ```
//!
//! `isError: true` means the call failed; the text is still the human-readable
//! reason, and `structuredContent` carries the engine's stable `kind` so a
//! client can branch without reading prose.

use std::io::Write;
use std::sync::{Arc, Mutex};

use craftkit_engine::Session;
use serde_json::{Value as Json, json};

use crate::rpc::{self, Error as RpcParseError, Request, Response, code};
use crate::{prompts, resources, tools};

/// Serve MCP over stdio until the input ends or `shutdown` arrives.
pub fn serve(session: Arc<Mutex<Session>>) -> Result<(), String> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut input = stdin.lock();
    loop {
        // The limit is applied **while reading**: a client that sends a
        // gigabyte without a newline costs this process a buffer, not a
        // gigabyte. The bytes past the limit are counted and dropped, and the
        // rest of the line is still read, so the stream stays in step. §49.
        //
        // The buffer is the *text*. The parse of it is what costs memory, and
        // the message limit is derived from the memory budget so that the
        // parse stays inside it. §64.
        let line = match craftkit_core::read_line_bounded(&mut input, rpc::MAX_MESSAGE) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                // The line is refused with the same words the parser uses, so
                // a client sees one message for one mistake however it was
                // made.
                let too_long = e.too_long_len();
                let message = match too_long {
                    Some(len) => RpcParseError::TooLong(len).to_string(),
                    None => e.to_string(),
                };
                let reply = Response::failure(Json::Null, code::INVALID_REQUEST, message);
                let wire = reply.to_line().map_err(|e| format!("could not encode a reply: {e}"))?;
                writeln!(out, "{wire}").map_err(|e| format!("could not write stdout: {e}"))?;
                out.flush().map_err(|e| format!("could not flush stdout: {e}"))?;
                if too_long.is_some() {
                    // A too-long line is a mistake, not an ending — the
                    // parser's own refusal carries on, and the reader has
                    // drained the rest of the line, so the stream is in step.
                    continue;
                }
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let answer = answer(trimmed, session.as_ref());
        if let Some(reply) = answer.reply {
            let wire = reply.to_line().map_err(|e| format!("could not encode a reply: {e}"))?;
            writeln!(out, "{wire}").map_err(|e| format!("could not write stdout: {e}"))?;
            out.flush().map_err(|e| format!("could not flush stdout: {e}"))?;
        }
        // The reply goes out before the stop, so the client that asked to stop
        // still hears that it was heard. Skipping this leaves a server that
        // acknowledges `shutdown` and then serves the next request — a method
        // that reports success without doing the thing it names, which is the
        // failure mode this project's own `key.remove` doc warns about.
        if answer.stop {
            break;
        }
    }
    Ok(())
}

/// What one line did.
pub struct Answer {
    /// The line to send back, or `None` for a notification.
    pub reply: Option<Response>,
    /// The client asked the server to stop, so `serve` ends after replying.
    pub stop: bool,
}

/// Answer one line, or `None` for a notification.
///
/// Pure with respect to I/O: it takes the line and the session and returns the
/// reply. Every protocol rule below is therefore testable without a pipe.
pub fn handle_line(line: &str, session: &Mutex<Session>) -> Option<Response> {
    answer(line, session).reply
}

/// [`handle_line`], and whether the client asked the server to stop.
///
/// Kept separate from the public entry point so the tests that already cover
/// the protocol keep testing exactly one function, while `serve` still learns
/// about `shutdown` from the same single parse.
fn answer(line: &str, session: &Mutex<Session>) -> Answer {
    let request = match rpc::parse(line) {
        Ok(r) => r,
        Err(RpcParseError::TooLong(n)) => {
            return Answer {
                reply: Some(Response::failure(
                    Json::Null,
                    code::INVALID_REQUEST,
                    RpcParseError::TooLong(n).to_string(),
                )),
                stop: false,
            };
        }
        Err(e) => {
            return Answer {
                reply: Some(Response::failure(Json::Null, code::PARSE_ERROR, e.to_string())),
                stop: false,
            };
        }
    };

    let id = request.id.clone();
    let is_notification = request.is_notification();

    if let Err(e) = request.check() {
        // A malformed request still gets a reply if it carried an id; a
        // notification never does, even when it is malformed.
        return Answer { reply: id.map(|i| Response::failure(i, code::INVALID_REQUEST, e)), stop: false };
    }

    let stop = request.method == "shutdown";
    let response = dispatch(&request, session);
    Answer { reply: if is_notification { None } else { Some(response) }, stop }
}

fn dispatch(request: &Request, session: &Mutex<Session>) -> Response {
    let id = request.id.clone().unwrap_or(Json::Null);
    match request.method.as_str() {
        "initialize" => Response::result(id, rpc::initialize_result()),
        "notifications/initialized" | "notifications/cancelled" => Response::result(id, json!({})),
        "ping" => Response::result(id, json!({})),
        "tools/list" => Response::result(id, json!({ "tools": tools::tools_json() })),
        "tools/call" => call_tool(&id, request, session),
        "resources/list" => Response::result(id, json!({ "resources": resources::list_json() })),
        "resources/templates/list" => {
            Response::result(id, json!({ "resourceTemplates": resources::templates_json() }))
        }
        "resources/read" => read_resource(&id, request, session),
        "prompts/list" => Response::result(id, json!({ "prompts": prompts::list_json() })),
        "prompts/get" => get_prompt(&id, request, session),
        "shutdown" => Response::result(id, json!({})),
        other => Response::failure(
            id,
            code::METHOD_NOT_FOUND,
            format!(
                "`{other}` is not a method; try initialize, tools/list, tools/call, resources/list, \
                 resources/read, prompts/list, prompts/get, ping or shutdown"
            ),
        ),
    }
}

fn call_tool(id: &Json, request: &Request, session: &Mutex<Session>) -> Response {
    let Some(name) = request.params.get("name").and_then(Json::as_str) else {
        return Response::failure(
            id.clone(),
            code::INVALID_PARAMS,
            "`name` is required and must be a string; call tools/list",
        );
    };
    let arguments = request.params.get("arguments").cloned().unwrap_or(json!({}));
    if !arguments.is_object() {
        return Response::failure(id.clone(), code::INVALID_PARAMS, "`arguments` must be an object");
    }

    if let Some(command) = tools::command_for(name) {
        return run(id, session, command, &arguments);
    }

    // The built-in tools, from the same list `tools/list` advertises. §171.
    if let Some(built_in) = tools::built_in(name) {
        return (built_in.call)(id, &arguments, session);
    }

    Response::failure(id.clone(), code::METHOD_NOT_FOUND, format!("`{name}` is not a tool; call tools/list"))
}

/// Read one resource.
///
/// A URI that names nothing is refused with the server's own
/// resource-not-found code, so a client can tell "there is no such resource"
/// from "this server has no resources". §176.
fn read_resource(id: &Json, request: &Request, session: &Mutex<Session>) -> Response {
    let Some(uri) = request.params.get("uri").and_then(Json::as_str) else {
        return Response::failure(
            id.clone(),
            code::INVALID_PARAMS,
            "`uri` is required and must be a string; call resources/list",
        );
    };
    match resources::read(uri, session) {
        Ok((mime, body)) => Response::result(
            id.clone(),
            json!({ "contents": [{ "uri": uri, "mimeType": mime, "text": body.to_string() }] }),
        ),
        Err(resources::ReadError::NotFound(missing)) => Response::failure(
            id.clone(),
            code::RESOURCE_NOT_FOUND,
            format!("no resource at `{missing}`; call resources/list"),
        ),
        Err(resources::ReadError::Session) => {
            Response::failure(id.clone(), code::INTERNAL_ERROR, "the session is unavailable")
        }
    }
}

/// Fetch one prompt's message.
///
/// A prompt that is not there, or one that is missing an argument it needs,
/// is refused with the parameters error — the client asked for something the
/// server does not have, and said so in the wrong shape. §178.
fn get_prompt(id: &Json, request: &Request, session: &Mutex<Session>) -> Response {
    let Some(name) = request.params.get("name").and_then(Json::as_str) else {
        return Response::failure(
            id.clone(),
            code::INVALID_PARAMS,
            "`name` is required and must be a string; call prompts/list",
        );
    };
    let arguments =
        request.params.get("arguments").cloned().unwrap_or_else(|| Json::Object(Default::default()));
    match prompts::message(name, &arguments, session) {
        Ok(message) => Response::result(id.clone(), message),
        Err(prompts::PromptError::NotFound(missing)) => Response::failure(
            id.clone(),
            code::INVALID_PARAMS,
            format!("no prompt named `{missing}`; call prompts/list"),
        ),
        Err(prompts::PromptError::Missing { prompt, argument }) => Response::failure(
            id.clone(),
            code::INVALID_PARAMS,
            format!("the prompt `{prompt}` needs its `{argument}`"),
        ),
    }
}

/// Run a command and wrap the outcome as tool content.
fn run(id: &Json, session: &Mutex<Session>, command: &str, arguments: &Json) -> Response {
    let outcome = {
        match session.lock() {
            Ok(mut guard) => guard.execute(command, arguments),
            Err(_) => {
                return rpc::tool_error(
                    id,
                    "poisoned",
                    "another command panicked and the session is unavailable",
                    None,
                );
            }
        }
    };
    match outcome {
        Ok(o) => rpc::tool_ok(id, o.data.clone()),
        Err(e) => rpc::tool_error(id, e.kind(), &e.to_string(), e.command()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Arc<Mutex<Session>> {
        Arc::new(Mutex::new(Session::sample()))
    }

    fn ask(line: &str) -> Json {
        let s = session();
        let reply = handle_line(line, &s).expect("a reply");
        serde_json::from_str(&reply.to_line().expect("json")).expect("parses back")
    }

    /// The defect: `serve` promised to end "or `shutdown` arrives" and did not.
    /// `handle_line` cannot report that, so the promise was written in a doc
    /// comment and tested nowhere, and the server answered `shutdown` with a
    /// success and then went on serving.
    #[test]
    fn shutdown_asks_the_server_to_stop_rather_than_only_saying_ok() {
        let s = session();
        let stop = answer(r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#, &s);
        assert!(stop.stop, "shutdown did not ask to stop");
        assert!(stop.reply.is_some(), "the client that asked to stop gets no acknowledgement");

        // Everything else must leave the server running, or this becomes a
        // method that stops the server on a typo.
        for line in [
            r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"p","version":"1"}}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        ] {
            assert!(!answer(line, &session()).stop, "{line} should not stop the server");
        }

        // A notification never gets a reply — including a shutdown sent as one,
        // which is why the stop has to travel separately from the reply.
        let quiet = answer(r#"{"jsonrpc":"2.0","method":"shutdown"}"#, &session());
        assert!(quiet.reply.is_none(), "a notification must stay silent");
        assert!(quiet.stop, "a shutdown notification still asked to stop");
    }

    #[test]
    fn initialize_advertises_tools() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        assert_eq!(r["result"]["protocolVersion"], json!(crate::PROTOCOL_VERSION));
        assert!(r["result"]["capabilities"]["tools"].is_object(), "{r}");
    }

    /// The server serves resources, and says so in the handshake.
    ///
    /// The capability follows the list: it is advertised exactly when there
    /// is a resource to ask for, so a client never has to try to find out.
    /// §176.
    #[test]
    fn initialize_advertises_the_resources_it_serves() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        assert!(r["result"]["capabilities"]["resources"].is_object(), "{r}");
        assert!(resources::serves_any(), "the server advertises resources and serves none");
    }

    /// Every resource the server advertises can be read.
    ///
    /// The advertisement and the dispatch are one list, so a resource cannot
    /// be advertised without being readable. §171's rule, applied to the half
    /// of the protocol that was missing. §176.
    #[test]
    fn every_advertised_resource_can_be_read() {
        let session = session();
        let listed = ask(r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#);
        let list = listed["result"]["resources"].as_array().expect("resources");
        assert!(!list.is_empty(), "the server advertises no resources");
        for resource in list {
            let uri = resource["uri"].as_str().expect("a uri");
            let line =
                format!(r#"{{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{{"uri":"{uri}"}}}}"#);
            let reply = handle_line(&line, &session).expect("a reply");
            let text = reply.to_line().expect("json");
            let read: Json = serde_json::from_str(&text).expect("parses back");
            assert!(read.get("result").is_some(), "{uri} is advertised and cannot be read: {text}");
            assert_eq!(read["result"]["contents"][0]["uri"], json!(uri), "{text}");
        }
    }

    /// A URI that names no resource is refused **as a missing resource**.
    ///
    /// `-32601` would say "this server has no such method", and a client
    /// reading that would conclude the server has no resources at all — and
    /// stop asking. §176.
    #[test]
    fn an_unknown_resource_is_refused_with_its_own_code() {
        let r =
            ask(r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"craftkit://nope"}}"#);
        assert_eq!(r["error"]["code"], json!(code::RESOURCE_NOT_FOUND), "{r}");
        assert_eq!(r["error"]["code"], json!(-32002), "{r}");

        // And a missing `uri` is a bad request, not a missing resource.
        let r = ask(r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{}}"#);
        assert_eq!(r["error"]["code"], json!(code::INVALID_PARAMS), "{r}");
    }

    /// The templates are advertised with the protocol's own spelling.
    #[test]
    fn the_templates_are_advertised() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"resources/templates/list"}"#);
        let list = r["result"]["resourceTemplates"].as_array().expect("templates");
        assert!(!list.is_empty(), "the server advertises no templates");
        assert!(list.iter().any(|t| t["uriTemplate"] == json!("craftkit://node/{id}")), "{list:?}");
    }

    /// The server serves prompts, and says so in the handshake.
    #[test]
    fn initialize_advertises_the_prompts_it_serves() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        assert!(r["result"]["capabilities"]["prompts"].is_object(), "{r}");
    }

    /// Every prompt the server advertises can be fetched.
    ///
    /// The advertisement and the dispatch are one list, so a prompt cannot be
    /// advertised without being fetchable. §171's rule. §178.
    #[test]
    fn every_advertised_prompt_can_be_fetched() {
        let session = session();
        let listed = ask(r#"{"jsonrpc":"2.0","id":1,"method":"prompts/list"}"#);
        let list = listed["result"]["prompts"].as_array().expect("prompts");
        assert!(!list.is_empty(), "the server advertises no prompts");
        for prompt in list {
            let name = prompt["name"].as_str().expect("a name");
            // The arguments come from the advertisement, so the test also
            // checks that the arguments a prompt advertises are the ones it
            // needs.
            let mut arguments = serde_json::Map::new();
            for argument in prompt["arguments"].as_array().expect("arguments") {
                let argument_name = argument["name"].as_str().expect("a name");
                let value = match argument_name {
                    "kind" => "filter",
                    "name" => "Probe",
                    "node" => "1",
                    "path" => "amount",
                    "value" => "1",
                    "time" => "0",
                    other => panic!("the prompt `{name}` has an argument `{other}` this test does not know"),
                };
                arguments.insert(argument_name.to_string(), json!(value));
            }
            let arguments = serde_json::Value::Object(arguments);
            let line = format!(
                r#"{{"jsonrpc":"2.0","id":2,"method":"prompts/get","params":{{"name":"{name}","arguments":{arguments}}}}}"#
            );
            let reply = handle_line(&line, &session).expect("a reply");
            let text = reply.to_line().expect("json");
            let got: Json = serde_json::from_str(&text).expect("parses back");
            assert!(got.get("result").is_some(), "{name} is advertised and cannot be fetched: {text}");
            assert_eq!(
                got["result"]["messages"][0]["role"],
                json!("user"),
                "a prompt's message is a user message: {text}"
            );
        }
    }

    /// A prompt that is not there is refused, and so is one that is missing
    /// an argument it needs.
    #[test]
    fn a_prompt_that_cannot_be_fetched_says_so() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"prompts/get","params":{"name":"nope"}}"#);
        assert_eq!(r["error"]["code"], json!(code::INVALID_PARAMS), "{r}");

        // `kind` is required, and the prompt says so.
        let r = ask(r#"{"jsonrpc":"2.0","id":2,"method":"prompts/get","params":{"name":"add-node"}}"#);
        assert_eq!(r["error"]["code"], json!(code::INVALID_PARAMS), "{r}");

        // And a call with no name at all is a bad request.
        let r = ask(r#"{"jsonrpc":"2.0","id":3,"method":"prompts/get","params":{}}"#);
        assert_eq!(r["error"]["code"], json!(code::INVALID_PARAMS), "{r}");
    }

    /// Every tool the server advertises can be called.
    ///
    /// The defect, in one repo of the family: `tools/list` advertised five
    /// tools and `tools/call` answered to seventeen, so a client that
    /// discovers tools the way MCP clients do never saw the other twelve.
    ///
    /// The advertisement and the dispatch are one list now, so this cannot
    /// drift; the test is what says so. §171.
    #[test]
    fn every_advertised_tool_can_be_called() {
        let session = session();
        for tool in tools::all() {
            let line = format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{}","arguments":{{}}}}}}"#,
                tool.name
            );
            let reply = handle_line(&line, &session).expect("a reply");
            let text = reply.to_line().expect("json");
            assert!(
                !text.contains("is not a tool"),
                "{} is advertised and cannot be called: {text}",
                tool.name
            );
        }
    }

    #[test]
    fn tools_list_contains_every_command() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        let list = r["result"]["tools"].as_array().expect("tools");
        assert!(list.len() > 10, "{} tools", list.len());
        assert!(list.iter().any(|t| t["name"] == json!("craftkit_node_add")));
        assert!(list.iter().any(|t| t["name"] == json!("craftkit_status")));
    }

    /// The crash reports are a tool an agent can call.
    ///
    /// §113: the tree writes a crash report with the design that crashed and
    /// the commands that ran, and the CLI and the control channel could both
    /// read them — but the MCP server had no way to see one, and the MCP
    /// server is the front end whose reader is an agent.
    #[test]
    fn the_crash_reports_are_a_tool_an_agent_can_call() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        let list = r["result"]["tools"].as_array().expect("tools");
        assert!(list.iter().any(|t| t["name"] == json!("craftkit_crashes")), "{list:?}");

        // And it answers: the list of the reports this machine has.
        let r = ask(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"craftkit_crashes","arguments":{"action":"list"}}}"#,
        );
        assert_eq!(r["result"]["isError"], json!(false), "{r}");
        let content = r["result"]["structuredContent"].as_object().expect("content");
        assert!(content.contains_key("ids"), "{content:?}");
    }

    /// An action the tree does not have is refused, in the words the engine
    /// uses.
    ///
    /// §113: the MCP server and the control channel answer with the same
    /// words now, because they ask the same engine the same question.
    #[test]
    fn a_crash_action_that_does_not_exist_is_refused() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"craftkit_crashes","arguments":{"action":"delete"}}}"#,
        );
        assert_eq!(r["result"]["isError"], json!(true), "{r}");
        assert_eq!(r["result"]["structuredContent"]["kind"], json!("bad-parameter"));
        let text = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("not a crash action"), "{text}");
    }

    #[test]
    fn a_tool_call_runs_a_real_command() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"craftkit_node_add","arguments":{"name":"From MCP","kind":"filter"}}}"#,
        );
        assert_eq!(r["result"]["isError"], json!(false), "{r}");
        assert_eq!(r["result"]["structuredContent"]["name"], json!("From MCP"));
        let text = r["result"]["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("From MCP"), "{text}");
    }

    #[test]
    fn a_refused_command_is_a_tool_error_not_a_protocol_error() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"craftkit_node_add","arguments":{"name":"X","kind":"nonsense"}}}"#,
        );
        // The call succeeded; the *command* failed. An agent distinguishes the
        // two by this flag.
        assert!(r.get("error").is_none(), "{r}");
        assert_eq!(r["result"]["isError"], json!(true), "{r}");
        assert_eq!(r["result"]["structuredContent"]["kind"], json!("bad-parameter"));
        assert_eq!(r["result"]["structuredContent"]["command"], json!("node.add"));
    }

    #[test]
    fn an_unknown_parameter_is_refused_rather_than_ignored() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"craftkit_node_add","arguments":{"name":"X","kind":"cue","colour":"red"}}}"#,
        );
        assert_eq!(r["result"]["isError"], json!(true), "{r}");
        let text = r["result"]["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("colour"), "{text}");
    }

    #[test]
    fn an_unknown_tool_is_method_not_found() {
        let r =
            ask(r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"craftkit_teleport"}}"#);
        assert_eq!(r["error"]["code"], json!(code::METHOD_NOT_FOUND));
        assert!(r["error"]["message"].as_str().unwrap_or_default().contains("tools/list"));
    }

    #[test]
    fn a_tool_call_without_a_name_says_what_to_do() {
        let r = ask(r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{}}"#);
        assert_eq!(r["error"]["code"], json!(code::INVALID_PARAMS));
        assert!(r["error"]["message"].as_str().unwrap_or_default().contains("tools/list"));
    }

    #[test]
    fn non_object_arguments_are_refused() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"craftkit_status","arguments":[1,2]}}"#,
        );
        assert_eq!(r["error"]["code"], json!(code::INVALID_PARAMS));
    }

    #[test]
    fn an_unknown_method_lists_the_real_ones() {
        let r = ask(r#"{"jsonrpc":"2.0","id":8,"method":"tools/teleport"}"#);
        assert_eq!(r["error"]["code"], json!(code::METHOD_NOT_FOUND));
        let msg = r["error"]["message"].as_str().unwrap_or_default();
        for m in ["initialize", "tools/list", "tools/call", "ping", "shutdown"] {
            assert!(msg.contains(m), "`{m}` is missing from: {msg}");
        }
    }

    #[test]
    fn a_notification_gets_no_reply_at_all() {
        let s = session();
        assert!(handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, &s).is_none());
    }

    #[test]
    fn a_malformed_notification_also_gets_no_reply() {
        let s = session();
        assert!(handle_line(r#"{"jsonrpc":"2.0","method":""}"#, &s).is_none());
    }

    #[test]
    fn malformed_json_is_a_parse_error_with_a_null_id() {
        let r = ask("{ not json");
        assert_eq!(r["error"]["code"], json!(code::PARSE_ERROR));
        assert_eq!(r["id"], Json::Null);
    }

    #[test]
    fn a_wrong_protocol_version_is_an_invalid_request() {
        let r = ask(r#"{"jsonrpc":"1.0","id":9,"method":"ping"}"#);
        assert_eq!(r["error"]["code"], json!(code::INVALID_REQUEST));
        assert_eq!(r["id"], json!(9));
    }

    #[test]
    fn the_status_tool_reports_the_session_without_a_command() {
        let r = ask(r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"craftkit_status"}}"#);
        assert_eq!(r["result"]["isError"], json!(false), "{r}");
        assert_eq!(r["result"]["structuredContent"]["name"], json!("Sample plan"));
    }

    #[test]
    fn the_commands_tool_returns_the_catalogue() {
        let r =
            ask(r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"craftkit_commands"}}"#);
        assert_eq!(r["result"]["isError"], json!(false));
        assert!(r["result"]["structuredContent"].is_array());
    }

    #[test]
    fn two_calls_in_one_session_share_the_document() {
        let s = session();
        let a = handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"craftkit_node_add","arguments":{"name":"Shared","kind":"cue"}}}"#,
            &s,
        )
        .expect("reply");
        assert_eq!(a.result.expect("result")["isError"], json!(false));

        let b = handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"craftkit_status"}}"#,
            &s,
        )
        .expect("reply");
        let text = b.result.expect("result")["content"][0]["text"].as_str().unwrap_or_default().to_string();
        assert!(text.contains("Shared"), "{text}");
    }

    #[test]
    fn a_long_value_is_truncated_in_the_text_but_not_in_the_structured_copy() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"craftkit_node_add","arguments":{"name":"0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890","kind":"cue"}}}"#,
        );
        let text = r["result"]["content"][0]["text"].as_str().expect("text");
        assert!(
            text.chars().count() <= 401,
            "the text content should be short: {} chars",
            text.chars().count()
        );
    }
}
