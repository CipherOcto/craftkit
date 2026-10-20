//! The server: read lines, dispatch, answer.
//!
//! # The methods
//!
//! | Method | What it does |
//! |---|---|
//! | `initialize` | Handshake. Advertises tools and nothing else. |
//! | `notifications/initialized` | Acknowledged by doing nothing — it is a notification. |
//! | `tools/list` | Every tool, generated from the command registry. |
//! | `tools/call` | Runs one tool against the session. |
//! | `ping` | Liveness. |
//! | `shutdown` | Replies, then the caller stops. |
//!
//! Anything else is `METHOD_NOT_FOUND` with the list of what does exist. A
//! guessed method that silently returned `null` is how an agent concludes the
//! server works but the feature does not.
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
use crate::tools;

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
        "shutdown" => Response::result(id, json!({})),
        other => Response::failure(
            id,
            code::METHOD_NOT_FOUND,
            format!("`{other}` is not a method; try initialize, tools/list, tools/call, ping or shutdown"),
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

    match name {
        "craftkit_status" => {
            let Ok(guard) = session.lock() else {
                return tool_error(id, "poisoned", "the session is unavailable", None);
            };
            tool_ok(id, guard.snapshot_json())
        }
        "craftkit_commands" => tool_ok(id, craftkit_engine::catalogue()),
        other => Response::failure(
            id.clone(),
            code::METHOD_NOT_FOUND,
            format!("`{other}` is not a tool; call tools/list"),
        ),
    }
}

/// Run a command and wrap the outcome as tool content.
fn run(id: &Json, session: &Mutex<Session>, command: &str, arguments: &Json) -> Response {
    let outcome = {
        match session.lock() {
            Ok(mut guard) => guard.execute(command, arguments),
            Err(_) => {
                return tool_error(
                    id,
                    "poisoned",
                    "another command panicked and the session is unavailable",
                    None,
                );
            }
        }
    };
    match outcome {
        Ok(o) => tool_ok(id, o.data.clone()),
        Err(e) => tool_error(id, e.kind(), &e.to_string(), e.command()),
    }
}

fn tool_ok(id: &Json, data: Json) -> Response {
    Response::result(
        id.clone(),
        json!({
            "content": [{
                "type": "text",
                "text": summarise(&data),
            }],
            "isError": false,
            "structuredContent": data,
        }),
    )
}

fn tool_error(id: &Json, kind: &str, message: &str, command: Option<&str>) -> Response {
    Response::result(
        id.clone(),
        json!({
            "content": [{ "type": "text", "text": message }],
            "isError": true,
            "structuredContent": { "kind": kind, "command": command, "message": message },
        }),
    )
}

/// One readable line for the text content.
///
/// MCP clients show this to a person, so it must be prose. The machine-readable
/// answer is in `structuredContent`.
fn summarise(data: &Json) -> String {
    let line = match data {
        Json::Null => "done".to_string(),
        Json::Object(map) => {
            let parts: Vec<String> = map.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            if parts.is_empty() { "done".to_string() } else { parts.join(", ") }
        }
        other => other.to_string(),
    };
    truncate(&line, MAX_TEXT)
}

/// How long the human-readable content may be.
///
/// Bounded because it is shown in a chat transcript. The full answer is always
/// in `structuredContent`, so truncating here loses presentation, never
/// information.
const MAX_TEXT: usize = 400;

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut cut: String = s.chars().take(max).collect();
    cut.push('…');
    cut
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

    #[test]
    fn tools_list_contains_every_command() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        let list = r["result"]["tools"].as_array().expect("tools");
        assert!(list.len() > 10, "{} tools", list.len());
        assert!(list.iter().any(|t| t["name"] == json!("craftkit_node_add")));
        assert!(list.iter().any(|t| t["name"] == json!("craftkit_status")));
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
