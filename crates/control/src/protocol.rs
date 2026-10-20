//! The wire format: one JSON object per line.
//!
//! Everything here is pure — parsing and formatting, no I/O — so the protocol
//! can be tested exhaustively without opening a socket.

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// The largest request line accepted, in bytes.
///
/// A cap is not politeness. Without one, one line from a client that has gone
/// wrong fills memory until the process is killed — and this process holds the
/// user's document.
///
/// A limit on **text**. The parser then turns that text into a tree, and the
/// tree is up to [`craftkit_core::JSON_EXPANSION`] times bigger — a JSON
/// number is two bytes of text and thirty-two bytes of memory. So this is
/// the limit that keeps one request's parse inside
/// [`craftkit_core::MAX_JSON_MEMORY_BYTES`], and it is the same number the
/// MCP server and the CLI use, because it is the same question. §64.
pub const MAX_LINE: usize = craftkit_core::MAX_JSON_TEXT_BYTES;

/// What a client is asking for.
///
/// A closed set rather than a free-form string, so a typo is an error that
/// names the method that does exist, instead of a dispatch that silently does
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Run a command. `params: { "command": "<id>", "params": { … } }`
    Execute,
    /// The full command catalogue as JSON.
    Describe,
    /// The session's current state as JSON.
    State,
    /// What the evaluator has to say about the current plan.
    Diagnostics,
    /// A crash report, or the list of them.
    Crashes,
    /// A UI interaction. Only meaningful when a window exists; without one it
    /// is an error, not a silent no-op.
    Ui,
    /// Whether the peer is alive.
    Ping,
    /// Close the connection.
    Shutdown,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Execute => "engine.execute",
            Method::Describe => "engine.describe",
            Method::State => "session.state",
            Method::Diagnostics => "diagnostics.list",
            Method::Crashes => "diagnostics.crash_reports",
            Method::Ui => "ui",
            Method::Ping => "ping",
            Method::Shutdown => "shutdown",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "engine.execute" => Method::Execute,
            "engine.describe" => Method::Describe,
            "session.state" => Method::State,
            "diagnostics.list" => Method::Diagnostics,
            "diagnostics.crash_reports" => Method::Crashes,
            "ui" | "ui.click" | "ui.screenshot" | "ui.render" => Method::Ui,
            "ping" => Method::Ping,
            "shutdown" => Method::Shutdown,
            _ => return None,
        })
    }

    /// Every method, for an error message and for the docs.
    pub fn all() -> &'static [Method] {
        &[
            Method::Execute,
            Method::Describe,
            Method::State,
            Method::Diagnostics,
            Method::Crashes,
            Method::Ui,
            Method::Ping,
            Method::Shutdown,
        ]
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a `ui.*` request is asking for.
///
/// The four spellings `ui`, `ui.render`, `ui.screenshot` and `ui.click` all parse
/// to [`Method::Ui`], which made the spelling look decorative: the host read the
/// action from `params.action` and ignored the method the caller had actually
/// typed. `{"method":"ui.click","params":{"x":400,"y":12}}` rendered a 1180×720
/// PNG and answered `rendered 1180x720` — `ok: true`, no error, and no click.
/// [`Request::ui_action`] is where the method name wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    /// Draw the shell and return the frame.
    Render,
    /// Draw the shell and say so more carefully, because the frame is not the
    /// window's pixels.
    Screenshot,
    /// Report the widgets on screen and their names.
    Widgets,
    /// Report which widget is at a point. It does **not** activate it.
    Click,
}

impl UiAction {
    /// The action a `params.action` string names.
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "render" => UiAction::Render,
            "screenshot" => UiAction::Screenshot,
            "widgets" => UiAction::Widgets,
            "click" => UiAction::Click,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            UiAction::Render => "render",
            UiAction::Screenshot => "screenshot",
            UiAction::Widgets => "widgets",
            UiAction::Click => "click",
        }
    }

    /// The spelling of the method that asks for this action on its own, or `None`
    /// for [`UiAction::Widgets`], which has always been spelled
    /// `{"method":"ui","params":{"action":"widgets"}}`.
    pub fn method(self) -> Option<&'static str> {
        Some(match self {
            UiAction::Render => "ui.render",
            UiAction::Screenshot => "ui.screenshot",
            UiAction::Click => "ui.click",
            UiAction::Widgets => return None,
        })
    }
}

impl std::fmt::Display for UiAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Correlates the reply. Not required to be unique; the server does not
    /// care, and requiring uniqueness would make a retry after a timeout
    /// ambiguous.
    #[serde(default)]
    pub id: Json,
    /// The launch token. Only the *first* request on a connection carries it,
    /// which is why it is optional here and mandatory in the server.
    #[serde(default)]
    pub token: Option<String>,
    pub method: String,
    /// Defaults to an **empty object**, not to `null`: `{"method":"ping"}` and
    /// `{"method":"ping","params":{}}` mean the same thing to a caller, and the
    /// second is what dispatch expects to look keys up in.
    #[serde(default = "empty_object")]
    pub params: Json,
}

/// The default for [`Request::params`].
fn empty_object() -> Json {
    Json::Object(Default::default())
}

impl Request {
    pub fn new(id: i64, method: Method) -> Self {
        Self { id: Json::from(id), token: None, method: method.as_str().to_string(), params: empty_object() }
    }

    pub fn with_params(mut self, params: Json) -> Self {
        self.params = params;
        self
    }

    /// The parsed method, or an error naming the ones that exist.
    pub fn method(&self) -> Result<Method, Response> {
        Method::parse(&self.method).ok_or_else(|| {
            Response::error(
                &self.id,
                "unknown-method",
                format!(
                    "`{}` is not a method; try {}",
                    self.method,
                    Method::all().iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ")
                ),
            )
        })
    }

    /// What a `ui.*` request asks for.
    ///
    /// The **method name decides**, and `params.action` is only consulted for the
    /// bare `ui`, which has no spelling of its own. The reverse order is what this
    /// replaces: the action defaulted to `render` and the typed method was never
    /// read, so `ui.click` rendered.
    ///
    /// Asking for two different things at once is refused rather than guessed at.
    /// A caller that writes `ui.click` with `action: "render"` has made a mistake,
    /// and picking either answer silently would hide it.
    ///
    /// # Errors
    ///
    /// Not a `ui.*` method; an `action` that is not a string or not an action
    /// name; or a method and an `action` that disagree. Each names what was asked
    /// for and what was on offer.
    pub fn ui_action(&self) -> Result<UiAction, Response> {
        let bad = |message: String| Response::error(&self.id, "bad-parameter", message);

        // `params` is a JSON object by construction, so a lookup cannot fail.
        let asked = match self.params.get("action") {
            None => None,
            Some(Json::String(s)) => Some(UiAction::parse(s).ok_or_else(|| {
                bad(format!("`action: {s:?}` is not a ui action; try render, screenshot, widgets or click"))
            })?),
            Some(other) => {
                return Err(bad(format!(
                    "`action` must be a string naming one of render, screenshot, widgets or click, not {}",
                    kind_of(other)
                )));
            }
        };

        // What the typed method means. `ui` alone means "whatever `action` says".
        let from_method = match self.method.as_str() {
            "ui.render" => Some(UiAction::Render),
            "ui.screenshot" => Some(UiAction::Screenshot),
            "ui.click" => Some(UiAction::Click),
            "ui" => None,
            other => {
                return Err(Response::error(
                    &self.id,
                    "not-a-ui-method",
                    format!("`{other}` does not take a ui action; try ui.render, ui.screenshot or ui.click"),
                ));
            }
        };

        Ok(match (from_method, asked) {
            (None, None) => UiAction::Render,
            (Some(from_method), None) => from_method,
            (None, Some(asked)) => asked,
            (Some(from_method), Some(asked)) if from_method == asked => from_method,
            (Some(from_method), Some(asked)) => {
                return Err(bad(format!(
                    "`{}` asks to {from_method} and `action` asks to {asked}; send one or the other",
                    self.method
                )));
            }
        })
    }
}

/// A JSON value described by its kind, for an error message.
fn kind_of(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}

/// One reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub id: Json,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Json>,
    /// A stable, machine-readable failure kind. An agent branches on this and
    /// never on the message text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
}

impl Response {
    pub fn ok(id: &Json, message: impl Into<String>, data: Json) -> Self {
        Self { id: id.clone(), ok: true, message: Some(message.into()), data: Some(data), kind: None }
    }

    pub fn ok_empty(id: &Json, message: impl Into<String>) -> Self {
        Self { id: id.clone(), ok: true, message: Some(message.into()), data: None, kind: None }
    }

    pub fn error(id: &Json, kind: &'static str, message: impl Into<String>) -> Self {
        Self { id: id.clone(), ok: false, message: Some(message.into()), data: None, kind: Some(kind) }
    }

    /// The line that goes on the wire, without its newline.
    pub fn to_line(&self) -> Result<String, ProtocolError> {
        serde_json::to_string(self).map_err(|e| ProtocolError::Encode(e.to_string()))
    }

    /// The line plus the newline that frames it.
    pub fn to_wire(&self) -> Result<String, ProtocolError> {
        Ok(format!("{}\n", self.to_line()?))
    }
}

/// Why a line could not be turned into a request or a reply.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolError {
    #[error("the line is {0} bytes; the limit is {MAX_LINE}")]
    TooLong(usize),
    #[error("this is not valid JSON: {0}")]
    NotJson(String),
    #[error("this line is not a request object: {0}")]
    NotAnObject(String),
    #[error("the reply could not be encoded: {0}")]
    Encode(String),
}

impl ProtocolError {
    /// Whether the connection should survive. Everything except `TooLong` is a
    /// malformed message from a peer that is otherwise talking correctly.
    pub fn is_fatal(&self) -> bool {
        matches!(self, ProtocolError::TooLong(_))
    }
}

/// Parse one line into a request.
pub fn parse_request(line: &str) -> Result<Request, ProtocolError> {
    if line.len() > MAX_LINE {
        return Err(ProtocolError::TooLong(line.len()));
    }
    let value: Json = serde_json::from_str(line).map_err(|e| ProtocolError::NotJson(e.to_string()))?;
    if !value.is_object() {
        return Err(ProtocolError::NotAnObject(match &value {
            Json::Array(_) => "an array".to_string(),
            Json::String(_) => "a string".to_string(),
            Json::Number(_) => "a number".to_string(),
            Json::Bool(_) => "a bool".to_string(),
            Json::Null => "null".to_string(),
            Json::Object(_) => String::new(),
        }));
    }
    serde_json::from_value(value).map_err(|e| ProtocolError::NotAnObject(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The line limit is the tree's limit, not a number of its own. §64.
    #[test]
    fn the_line_limit_is_the_tree_s_json_limit() {
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same trick the model's limit test uses.
        let (line, expansion, budget) =
            (MAX_LINE, craftkit_core::JSON_EXPANSION, craftkit_core::MAX_JSON_MEMORY_BYTES);
        assert_eq!(line, craftkit_core::MAX_JSON_TEXT_BYTES);
        assert!(line * expansion <= budget, "a line of {line} bytes can cost {} bytes", line * expansion);
    }

    #[test]
    fn a_request_round_trips() {
        let line = r#"{"id":1,"method":"engine.execute","params":{"command":"node.add"}}"#;
        let req = parse_request(line).expect("parses");
        assert_eq!(req.id, json!(1));
        assert_eq!(req.method(), Ok(Method::Execute));
        assert_eq!(req.params["command"], json!("node.add"));
    }

    #[test]
    fn an_unknown_method_is_refused_with_the_list_of_real_ones() {
        let req = Request { id: json!(7), token: None, method: "engine.eplode".into(), params: Json::Null };
        let err = req.method().expect_err("refused");
        assert_eq!(err.id, json!(7));
        assert_eq!(err.kind, Some("unknown-method"));
        assert!(err.message.expect("message").contains("engine.execute"));
    }

    #[test]
    fn the_ui_aliases_all_reach_one_method() {
        for name in ["ui", "ui.click", "ui.screenshot", "ui.render"] {
            assert_eq!(Method::parse(name), Some(Method::Ui), "{name}");
        }
        assert_eq!(Method::parse("ui.frobnicate"), None);
    }

    #[test]
    fn every_method_round_trips_through_its_own_name() {
        for m in Method::all() {
            assert_eq!(Method::parse(m.as_str()), Some(*m), "{m}");
        }
    }

    #[test]
    fn a_reply_carries_the_id_even_when_it_failed() {
        let r = Response::error(&json!(42), "bad-parameter", "nope");
        let wire = r.to_line().expect("json");
        assert!(wire.contains("\"id\":42"), "{wire}");
        assert!(wire.contains("\"ok\":false"), "{wire}");
        assert!(wire.contains("\"kind\":\"bad-parameter\""), "{wire}");
    }

    #[test]
    fn a_successful_reply_is_compact_and_carries_its_data() {
        let r = Response::ok(&json!(1), "added", json!({ "id": 4 }));
        let line = r.to_line().expect("json");
        assert!(!line.contains('\n'), "the newline belongs to `to_wire`, not `to_line`");
        assert!(line.contains("\"data\":{\"id\":4}"), "{line}");
        assert!(!line.contains("\"kind\""), "a success has no kind: {line}");
    }

    #[test]
    fn the_wire_form_ends_in_exactly_one_newline() {
        let wire = Response::ok_empty(&json!(0), "pong").to_wire().expect("wire");
        assert!(wire.ends_with('\n'));
        assert_eq!(wire.matches('\n').count(), 1);
    }

    #[test]
    fn a_request_may_omit_id_and_params() {
        let req = parse_request(r#"{"method":"ping"}"#).expect("parses");
        assert_eq!(req.id, Json::Null);
        assert_eq!(req.params, Json::Object(Default::default()));
    }

    #[test]
    fn a_request_without_a_method_is_refused_rather_than_defaulted() {
        let err = parse_request(r#"{"id":1}"#).expect_err("refused");
        assert!(matches!(err, ProtocolError::NotAnObject(_)), "{err}");
    }

    #[test]
    fn non_objects_are_refused_by_name() {
        assert!(matches!(parse_request("[1,2]"), Err(ProtocolError::NotAnObject(m)) if m == "an array"));
        assert!(matches!(parse_request("\"x\""), Err(ProtocolError::NotAnObject(m)) if m == "a string"));
        assert!(matches!(parse_request("42"), Err(ProtocolError::NotAnObject(m)) if m == "a number"));
        assert!(matches!(parse_request("null"), Err(ProtocolError::NotAnObject(m)) if m == "null"));
    }

    #[test]
    fn malformed_json_is_refused_with_the_parser_message() {
        let err = parse_request("{ nope").expect_err("refused");
        assert!(matches!(err, ProtocolError::NotJson(_)));
        assert!(err.to_string().contains("not valid JSON"), "{err}");
    }

    #[test]
    fn an_oversized_line_is_refused_before_it_is_parsed() {
        let big = format!("{{\"a\":\"{}\"}}", "x".repeat(MAX_LINE));
        let err = parse_request(&big).expect_err("refused");
        assert!(matches!(err, ProtocolError::TooLong(_)));
        assert!(err.is_fatal(), "a client this confused should be disconnected");
    }

    #[test]
    fn a_malformed_line_is_not_fatal() {
        assert!(!ProtocolError::NotJson(String::new()).is_fatal());
        assert!(!ProtocolError::NotAnObject(String::new()).is_fatal());
    }

    #[test]
    fn an_empty_line_is_ignored_by_the_caller_not_rejected_here() {
        // A trailing newline at the end of a stream produces an empty final
        // line; the server skips it rather than answering it.
        assert!(parse_request("").is_err());
    }

    /// The defect this whole module exists to prevent, pinned as the first test
    /// it applies to: the method name is not decoration.
    ///
    /// It used to be. `ui.click` parsed to `Method::Ui`, the host then read the
    /// action from `params.action`, and the typed method was never looked at —
    /// so the request an agent is told to send answered `rendered 1180x720`,
    /// `ok: true`, having clicked nothing.
    #[test]
    fn the_ui_action_comes_from_the_method_name() {
        for (method, action, expected) in [
            ("ui.click", None, UiAction::Click),
            ("ui.render", None, UiAction::Render),
            ("ui.screenshot", None, UiAction::Screenshot),
            // A bare `ui` has no spelling of its own, so it is the action that
            // decides, and a render is the sensible default.
            ("ui", None, UiAction::Render),
            ("ui", Some("widgets"), UiAction::Widgets),
            ("ui", Some("click"), UiAction::Click),
            // Spelling out an action that agrees changes nothing.
            ("ui.click", Some("click"), UiAction::Click),
        ] {
            let mut request = Request::new(1, Method::Ui);
            request.method = method.to_string();
            if let Some(action) = action {
                request.params = serde_json::json!({ "action": action });
            }
            assert_eq!(
                request.ui_action().map_err(|r| r.message.unwrap_or_default()),
                Ok(expected),
                "{method} with action {action:?}"
            );
        }
    }

    /// A caller who writes `ui.click` and `action: "render"` has made a mistake.
    /// Guessing would hide it; either way one of the two things it said is ignored.
    #[test]
    fn a_method_that_disagrees_with_its_ui_action_is_refused() {
        let mut request = Request::new(1, Method::Ui);
        request.method = "ui.click".to_string();
        request.params = serde_json::json!({ "action": "render" });

        let response = request.ui_action().expect_err("refused");
        assert!(!response.ok, "a disagreement must not be reported as success");
        let message = response.message.unwrap_or_default();
        assert!(message.contains("click"), "{message}");
        assert!(message.contains("render"), "{message}");
    }

    #[test]
    fn hostile_ui_actions_are_refused_by_name() {
        let cases = [
            // Not an action.
            ("ui", json!("destroy"), "destroy"),
            // Not a string at all: indexed safely, never a panic.
            ("ui", json!(7), "a number"),
            ("ui", json!(["click"]), "an array"),
            ("ui", json!(null), "null"),
            // A ui action asked of a method that is not a ui one.
            ("ping", json!("click"), "does not take a ui action"),
        ];
        for (method, action, expected) in cases {
            let mut request = Request::new(1, Method::Ui);
            request.method = method.to_string();
            request.params = serde_json::json!({ "action": action });
            let response = request.ui_action().expect_err("refused");
            let message = response.message.unwrap_or_default();
            assert!(message.contains(expected), "{method} with {action}: {message}");
        }
    }

    /// Every action that has a method spelling round-trips through that spelling.
    /// `widgets` is the honest exception and says so, rather than being given a
    /// `ui.widgets` method that [`Method::parse`] does not accept.
    #[test]
    fn a_ui_action_named_by_a_method_means_that_action() {
        for action in [UiAction::Render, UiAction::Screenshot, UiAction::Click] {
            let method = action.method().expect("has a spelling");
            let mut request = Request::new(1, Method::Ui);
            request.method = method.to_string();
            assert_eq!(request.ui_action(), Ok(action), "{method}");
        }
        assert_eq!(UiAction::Widgets.method(), None);
        assert_eq!(UiAction::parse("widgets"), Some(UiAction::Widgets));
    }
}
