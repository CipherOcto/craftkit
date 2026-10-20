//! JSON-RPC 2.0, framed one message per line.
//!
//! Only the parts MCP uses: requests, responses, errors, and notifications.
//! Batching is deliberately **not** supported — it is rare in practice, and a
//! batch that fails to parse must not take the whole exchange with it.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json, json};

/// The largest message accepted, in bytes.
///
/// A limit on **text**. The parser then turns that text into a tree, and the
/// tree is up to [`craftkit_core::JSON_EXPANSION`] times bigger — a JSON
/// number is two bytes of text and thirty-two bytes of memory. So this is
/// the limit that keeps one message's parse inside
/// [`craftkit_core::MAX_JSON_MEMORY_BYTES`], and it is the same number the
/// control channel and the CLI use, because it is the same question. §64.
///
/// It used to be 4 MiB, which was four times the memory budget and was not
/// derived from anything.
pub const MAX_MESSAGE: usize = craftkit_core::MAX_JSON_TEXT_BYTES;

/// A request or a notification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    /// Absent for a notification, which expects no reply.
    #[serde(default)]
    pub id: Option<Json>,
    pub method: String,
    #[serde(default = "empty_object")]
    pub params: Json,
}

fn empty_object() -> Json {
    Json::Object(Map::new())
}

impl Request {
    pub fn new(id: i64, method: &str, params: Json) -> Self {
        Self { jsonrpc: "2.0".to_string(), id: Some(Json::from(id)), method: method.to_string(), params }
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Whether this is a well-formed JSON-RPC 2.0 request.
    pub fn check(&self) -> Result<(), String> {
        if self.jsonrpc != "2.0" {
            return Err(format!("`jsonrpc` must be \"2.0\", not `{}`", self.jsonrpc));
        }
        if self.method.trim().is_empty() {
            return Err("`method` is empty".to_string());
        }
        Ok(())
    }
}

/// A reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    /// `None` for an error reply, which uses `error` instead.
    pub id: Json,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    pub fn result(id: Json, result: Json) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
    }

    pub fn failure(id: Json, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError { code, message: message.into(), data: None }),
        }
    }

    /// The line that goes on the wire.
    pub fn to_line(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Json>,
}

/// The codes this server uses.
///
/// Only the standard ones plus one craftkit-specific code, because an agent that
/// gets "invalid params" learns nothing about *which* parameter. The extra code
/// carries the engine's stable `kind` in `data`, so a client can branch on it
/// without parsing a message.
pub mod code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;

    /// craftkit's own: the request was valid but the engine refused it.
    ///
    /// `data.kind` holds one of the engine's stable failure kinds.
    pub const CRAFTKIT_ERROR: i64 = -32000;
}

/// Why a line could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("the line is {0} bytes; the limit is {MAX_MESSAGE}")]
    TooLong(usize),
    #[error("this is not valid JSON: {0}")]
    NotJson(String),
    #[error("this is not a JSON-RPC object: {0}")]
    NotAnObject(String),
}

/// Parse one line into a request.
pub fn parse(line: &str) -> Result<Request, Error> {
    if line.len() > MAX_MESSAGE {
        return Err(Error::TooLong(line.len()));
    }
    let value: Json = serde_json::from_str(line).map_err(|e| Error::NotJson(e.to_string()))?;
    if !value.is_object() {
        return Err(Error::NotAnObject(kind_of(&value).to_string()));
    }
    serde_json::from_value(value).map_err(|e| Error::NotAnObject(e.to_string()))
}

/// The word used in a message to name a JSON type.
pub fn kind_of(v: &Json) -> &'static str {
    match v {
        Json::Null => "null",
        Json::Bool(_) => "a bool",
        Json::Number(_) => "a number",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}

/// The `initialize` result.
pub fn initialize_result() -> Json {
    json!({
        "protocolVersion": crate::PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": crate::SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_round_trips() {
        let r = parse(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#).expect("parses");
        assert_eq!(r.id, Some(json!(1)));
        assert_eq!(r.method, "tools/list");
        assert_eq!(r.check(), Ok(()));
    }

    #[test]
    fn a_notification_has_no_id_and_gets_no_reply() {
        let r = parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).expect("parses");
        assert!(r.is_notification());
    }

    #[test]
    fn params_default_to_an_object() {
        let r = parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).expect("parses");
        assert_eq!(r.params, json!({}));
    }

    #[test]
    fn a_wrong_protocol_version_is_refused() {
        let r = parse(r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#).expect("parses");
        assert!(r.check().unwrap_err().contains("2.0"));
    }

    #[test]
    fn an_empty_method_is_refused() {
        let r = parse(r#"{"jsonrpc":"2.0","id":1,"method":"  "}"#).expect("parses");
        assert!(r.check().is_err());
    }

    #[test]
    fn a_missing_jsonrpc_field_is_refused_at_parse_time() {
        assert!(matches!(parse(r#"{"id":1,"method":"ping"}"#), Err(Error::NotAnObject(_))));
    }

    #[test]
    fn non_objects_are_refused_by_name() {
        assert!(matches!(parse("[]"), Err(Error::NotAnObject(m)) if m == "an array"));
        assert!(matches!(parse("7"), Err(Error::NotAnObject(m)) if m == "a number"));
        assert!(matches!(parse("\"x\""), Err(Error::NotAnObject(m)) if m == "a string"));
        assert!(matches!(parse("null"), Err(Error::NotAnObject(m)) if m == "null"));
    }

    /// The message limit is the tree's limit, not a number of its own.
    ///
    /// The three readers of untrusted JSON had three unrelated limits, and
    /// the largest of them was thirty-two times looser than the memory it
    /// was supposed to bound. §64.
    #[test]
    fn the_message_limit_is_the_tree_s_json_limit() {
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same trick the model's limit test uses.
        let (message, expansion, budget) =
            (MAX_MESSAGE, craftkit_core::JSON_EXPANSION, craftkit_core::MAX_JSON_MEMORY_BYTES);
        assert_eq!(message, craftkit_core::MAX_JSON_TEXT_BYTES);
        assert!(
            message * expansion <= budget,
            "a message of {message} bytes can cost {} bytes",
            message * expansion
        );
    }

    #[test]
    fn malformed_json_is_refused() {
        assert!(matches!(parse("{ nope"), Err(Error::NotJson(_))));
    }

    #[test]
    fn an_oversized_line_is_refused_before_it_is_parsed() {
        let big = format!("{{\"a\":\"{}\"}}", "x".repeat(MAX_MESSAGE));
        assert!(matches!(parse(&big), Err(Error::TooLong(_))));
    }

    #[test]
    fn a_result_reply_has_no_error_and_an_error_reply_has_no_result() {
        let ok = Response::result(json!(1), json!({ "a": 1 })).to_line().expect("json");
        assert!(ok.contains("\"result\""), "{ok}");
        assert!(!ok.contains("\"error\""), "{ok}");

        let bad = Response::failure(json!(1), code::METHOD_NOT_FOUND, "no").to_line().expect("json");
        assert!(bad.contains("\"error\""), "{bad}");
        assert!(!bad.contains("\"result\""), "{bad}");
    }

    #[test]
    fn an_error_reply_can_carry_a_stable_kind() {
        let mut e = RpcError { code: code::CRAFTKIT_ERROR, message: "refused".into(), data: None };
        e.data = Some(json!({ "kind": "bad-parameter" }));
        let line = Response { jsonrpc: "2.0".into(), id: json!(3), result: None, error: Some(e) }
            .to_line()
            .expect("json");
        assert!(line.contains("\"kind\":\"bad-parameter\""), "{line}");
    }

    #[test]
    fn the_initialize_result_advertises_tools_and_names_the_server() {
        let r = initialize_result();
        assert_eq!(r["protocolVersion"], json!(crate::PROTOCOL_VERSION));
        assert!(r["capabilities"]["tools"].is_object(), "{r}");
        assert_eq!(r["serverInfo"]["name"], json!("craftkit"));
    }
}
