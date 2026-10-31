//! `craftkit-control` — the **other door**: a session you can drive from
//! another program.
//!
//! # What this is
//!
//! The MCP server (§41) is a door for an **agent**: JSON-RPC over stdio,
//! one tool per command, and the process is the agent's child. A control
//! channel is a door for a **running program**: the program holds the
//! window, and something else — a script, a test, a second program —
//! asks it to do things and asks it what it is.
//!
//! The family's applications have one, and the wire format is theirs:
//! newline-delimited JSON, one object per line, a request and a reply.
//!
//! # The methods
//!
//! | the method | what it does |
//! |---|---|
//! | `ping` | whether the peer is alive |
//! | `engine.describe` | the command catalogue and the kinds |
//! | `engine.execute` | run a command |
//! | `session.state` | what the session is holding |
//! | `diagnostics.list` | what has happened, and what was journalled |
//! | `shutdown` | stop the server |
//!
//! The family's channel also has `diagnostics.crash_reports` and the
//! `ui.*` methods, and **this one does not**: there is no window in this
//! tree (§16), so a `ui` request would be an error rather than a silent
//! no-op. A method that is not there says so, by name.
//!
//! # The token
//!
//! The first request on a connection carries the **launch token**, and a
//! connection that does not have it is closed. The token is compared in
//! a way that does not stop at the first different character, so that
//! the time it takes does not say how much of the token was right.
//!
//! # The line
//!
//! A line is at most a megabyte. A line that is longer is refused
//! rather than read: the reader is hostile input's first line of
//! defence, and a request that is a gigabyte long is not a request.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::io::{BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use craftkit_sdk::Session;
use craftkit_sdk::serde_json::{Map, Value as Json, json};

/// The most a line may be.
///
/// The protocol had this number from the start, and the reader did not
/// use it: a line was read until its newline, however long it was. The
/// number is now the **reader's** bound.
pub const MAX_LINE: usize = 1 << 20;

/// What the channel can ask the **window** for.
///
/// The channel knows the **session**; the window knows the **screen**.
/// So the window gives the channel a way to ask for a picture, and the
/// channel hands the picture to whoever asked for it.
///
/// Without this the channel can change the document and cannot see the
/// result, and an agent that cannot see the result is an agent that
/// has to believe it.
pub trait Render: Send {
    /// The picture of what the program is showing, as a PNG.
    fn render(&mut self) -> Result<Vec<u8>, String>;

    /// **Click** at a point of the world, and say what was clicked.
    ///
    /// The point is in the **world's** units, the way the document is:
    /// a click at a point of the screen is a click at a point of the
    /// world that the camera has already found.
    fn click(&mut self, at: craftkit_sdk::Point) -> Result<Json, String>;
}

/// What a client is asking for.
///
/// A closed set rather than a free-form string, so that a typo is an
/// error that names the methods that exist, instead of a dispatch that
/// silently does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Run a command: `params: { "command": "<id>", "params": { … } }`.
    Execute,
    /// The command catalogue and the kinds.
    Describe,
    /// What the session is holding.
    State,
    /// What has happened.
    Diagnostics,
    /// Whether the peer is alive.
    Ping,
    /// Stop the server.
    Shutdown,
    /// What the program is showing, as a picture.
    UiRender,
    /// Click in the document, at a point of the world.
    UiClick,
}

impl Method {
    /// The method's name on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Execute => "engine.execute",
            Method::Describe => "engine.describe",
            Method::State => "session.state",
            Method::Diagnostics => "diagnostics.list",
            Method::Ping => "ping",
            Method::Shutdown => "shutdown",
            Method::UiRender => "ui.render",
            Method::UiClick => "ui.click",
        }
    }

    /// The method a name is, if it is one.
    pub fn parse(name: &str) -> Option<Method> {
        Some(match name {
            "engine.execute" => Method::Execute,
            "engine.describe" => Method::Describe,
            "session.state" => Method::State,
            "diagnostics.list" => Method::Diagnostics,
            "ping" => Method::Ping,
            "shutdown" => Method::Shutdown,
            "ui.render" => Method::UiRender,
            "ui.click" => Method::UiClick,
            _ => return None,
        })
    }

    /// Every method, for an error message.
    pub fn all() -> &'static [Method] {
        &[
            Method::Execute,
            Method::Describe,
            Method::State,
            Method::Diagnostics,
            Method::Ping,
            Method::Shutdown,
            Method::UiRender,
            Method::UiClick,
        ]
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One request.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Correlates the reply.
    pub id: Json,
    /// The launch token. Only the **first** request on a connection
    /// carries it, which is why it is optional here and mandatory in the
    /// server.
    pub token: Option<String>,
    /// The method.
    pub method: String,
    /// The parameters. An empty object when there are none, because
    /// `{"method":"ping"}` and `{"method":"ping","params":{}}` mean the
    /// same thing to a caller.
    pub params: Json,
}

impl Request {
    /// A request for a method.
    pub fn new(id: i64, method: Method) -> Request {
        Request { id: Json::from(id), token: None, method: method.as_str().to_string(), params: json!({}) }
    }

    /// The request with parameters.
    pub fn with_params(mut self, params: Json) -> Request {
        self.params = params;
        self
    }

    /// The request with the token on it.
    pub fn with_token(mut self, token: impl Into<String>) -> Request {
        self.token = Some(token.into());
        self
    }

    /// A request read from a line.
    pub fn from_line(line: &str) -> Result<Request, ProtocolError> {
        if line.len() > MAX_LINE {
            return Err(ProtocolError::TooLong(line.len()));
        }
        let value: Json = craftkit_sdk::serde_json::from_str(line)
            .map_err(|error| ProtocolError::NotJson(error.to_string()))?;
        let Json::Object(object) = value else {
            return Err(ProtocolError::NotAnObject(line.chars().take(40).collect()));
        };
        let method = object
            .get("method")
            .and_then(|method| method.as_str())
            .ok_or(ProtocolError::NoMethod)?
            .to_string();
        let id = object.get("id").cloned().unwrap_or(Json::Null);
        let token = object.get("token").and_then(|token| token.as_str()).map(|token| token.to_string());
        let params = match object.get("params") {
            Some(params) if !params.is_null() => params.clone(),
            _ => json!({}),
        };
        Ok(Request { id, token, method, params })
    }

    /// The line the request goes out as.
    pub fn to_line(&self) -> Result<String, ProtocolError> {
        let mut object = Map::new();
        object.insert("id".to_string(), self.id.clone());
        if let Some(token) = &self.token {
            object.insert("token".to_string(), Json::String(token.clone()));
        }
        object.insert("method".to_string(), Json::String(self.method.clone()));
        object.insert("params".to_string(), self.params.clone());
        craftkit_sdk::serde_json::to_string(&Json::Object(object))
            .map_err(|error| ProtocolError::NotJson(error.to_string()))
    }
}

/// One reply.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The request's id.
    pub id: Json,
    /// Whether it worked.
    pub ok: bool,
    /// One line for a person.
    pub message: Option<String>,
    /// What the caller gets back.
    pub data: Option<Json>,
    /// A stable, machine-readable failure kind.
    pub kind: Option<String>,
}

impl Response {
    /// A reply that worked.
    pub fn ok(id: &Json, message: impl Into<String>, data: Option<Json>) -> Response {
        Response { id: id.clone(), ok: true, message: Some(message.into()), data, kind: None }
    }

    /// A reply that did not.
    pub fn error(id: &Json, kind: &str, message: impl Into<String>) -> Response {
        Response {
            id: id.clone(),
            ok: false,
            message: Some(message.into()),
            data: None,
            kind: Some(kind.to_string()),
        }
    }

    /// The line the reply goes out as.
    pub fn to_line(&self) -> Result<String, ProtocolError> {
        let mut object = Map::new();
        object.insert("id".to_string(), self.id.clone());
        object.insert("ok".to_string(), Json::Bool(self.ok));
        if let Some(message) = &self.message {
            object.insert("message".to_string(), Json::String(message.clone()));
        }
        if let Some(data) = &self.data {
            object.insert("data".to_string(), data.clone());
        }
        if let Some(kind) = &self.kind {
            object.insert("kind".to_string(), Json::String(kind.clone()));
        }
        craftkit_sdk::serde_json::to_string(&Json::Object(object))
            .map_err(|error| ProtocolError::NotJson(error.to_string()))
    }
}

/// Why a line could not be turned into a request or a reply.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolError {
    /// The line was longer than the limit.
    TooLong(usize),
    /// The line was not JSON.
    NotJson(String),
    /// The line was JSON, but not an object.
    NotAnObject(String),
    /// The object had no method.
    NoMethod,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::TooLong(len) => write!(f, "the line is {len} bytes; the limit is {MAX_LINE}"),
            ProtocolError::NotJson(error) => write!(f, "this is not valid JSON: {error}"),
            ProtocolError::NotAnObject(text) => write!(f, "this line is not a request object: {text}"),
            ProtocolError::NoMethod => write!(f, "the request has no method"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Why the channel could not do what it was asked.
#[derive(Debug)]
pub enum ControlError {
    /// The socket said no.
    Io(std::io::Error),
    /// The line was not a request.
    Protocol(ProtocolError),
    /// The server answered with a failure.
    Refused {
        /// The machine-readable kind.
        kind: String,
        /// What it said.
        message: String,
    },
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::Io(error) => write!(f, "the channel said no: {error}"),
            ControlError::Protocol(error) => write!(f, "{error}"),
            ControlError::Refused { kind, message } => write!(f, "{kind}: {message}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<std::io::Error> for ControlError {
    fn from(error: std::io::Error) -> Self {
        ControlError::Io(error)
    }
}

impl From<ProtocolError> for ControlError {
    fn from(error: ProtocolError) -> Self {
        ControlError::Protocol(error)
    }
}

/// How the server is set up.
#[derive(Clone)]
pub struct Options {
    /// The launch token. The client has to have it.
    pub token: String,
    /// The address to listen on. `127.0.0.1:0` asks the system for a
    /// free port, which is what a test wants.
    pub address: SocketAddr,
    /// Where to write the address and the token, so that another
    /// program can find them.
    pub announce: Option<PathBuf>,
    /// The **window**, if the program has one.
    ///
    /// It is how the channel can show what the program is showing: the
    /// session is what the channel changes, and the window is what the
    /// person — or the agent — looks at.
    pub render: Option<Arc<Mutex<dyn Render + Send>>>,
}

impl std::fmt::Debug for Options {
    /// The token is **not** printed.
    ///
    /// A token in a log is a token in the world, and the token is the
    /// one thing that decides who may drive the program.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("token", &if self.token.is_empty() { "none" } else { "a token" })
            .field("address", &self.address)
            .field("announce", &self.announce)
            .field("render", &self.render.as_ref().map(|_| "a window"))
            .finish()
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            token: String::new(),
            address: SocketAddr::from(([127, 0, 0, 1], 0)),
            announce: None,
            render: None,
        }
    }
}

/// The server: a session, and a door to it.
pub struct Server {
    /// The address it is listening on.
    address: SocketAddr,
    /// Whether it should stop.
    stop: Arc<AtomicBool>,
    /// The thread that is serving.
    handle: Option<JoinHandle<()>>,
}

impl Server {
    /// Serve a session, and say where it is listening.
    pub fn serve(session: Session, options: Options) -> Result<Server, ControlError> {
        Server::serve_shared(Arc::new(Mutex::new(session)), options)
    }

    /// Serve a session that the caller **keeps**.
    ///
    /// The session is behind a lock, and the caller holds the same
    /// lock: a program that has a session of its own — a window, say —
    /// can open a channel over it, and a command that an agent runs
    /// over the channel changes the document the program is showing.
    ///
    /// Without this the channel can only serve a session of its own,
    /// and a program with a session already has two documents and one
    /// of them is a copy.
    pub fn serve_shared(session: Arc<Mutex<Session>>, options: Options) -> Result<Server, ControlError> {
        let listener = TcpListener::bind(options.address)?;
        let address = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let token = options.token.clone();
        let render = options.render.clone();

        if let Some(path) = &options.announce {
            let announcement = json!({
                "address": address.to_string(),
                "token": token,
                "pid": std::process::id(),
            });
            let text = craftkit_sdk::serde_json::to_string_pretty(&announcement)
                .unwrap_or_else(|_| "{}".to_string());
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, text)?;
        }

        let thread_stop = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if thread_stop.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let session = Arc::clone(&session);
                let token = token.clone();
                let stop = Arc::clone(&thread_stop);
                let render = render.clone();
                std::thread::spawn(move || {
                    let _ = serve_connection(stream, session, token, stop, render);
                });
            }
        });

        Ok(Server { address, stop, handle: Some(handle) })
    }

    /// The address it is listening on.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Stop the server.
    ///
    /// The listening socket is woken with a connection of its own, so
    /// that the accept loop ends without polling and without a timer.
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Serve one connection, until the client goes away.
fn serve_connection(
    stream: TcpStream,
    session: Arc<Mutex<Session>>,
    token: String,
    stop: Arc<AtomicBool>,
    render: Option<Arc<Mutex<dyn Render + Send>>>,
) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(stream);
    let mut authenticated = false;
    for line in craftkit_sdk::lines::lines_bounded(reader, MAX_LINE) {
        let line = line.map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))?;
        let request = match Request::from_line(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = Response::error(&Json::Null, "protocol", error.to_string());
                writeln!(writer, "{}", response.to_line().unwrap_or_default())?;
                writer.flush()?;
                continue;
            }
        };
        // The token is on the first request of the connection, and only
        // there.
        if !authenticated {
            let given = request.token.clone().unwrap_or_default();
            if !same(&given, &token) {
                let response = Response::error(&request.id, "unauthorized", "the token is not the one");
                writeln!(writer, "{}", response.to_line().unwrap_or_default())?;
                writer.flush()?;
                return Ok(());
            }
            authenticated = true;
        }
        let response = answer(&request, &session, &stop, &render);
        writeln!(writer, "{}", response.to_line().unwrap_or_default())?;
        writer.flush()?;
        if request.method == Method::Shutdown.as_str() {
            break;
        }
    }
    Ok(())
}

/// Whether two strings are the same, without stopping at the first
/// difference.
fn same(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.bytes().zip(right.bytes()) {
        difference |= a ^ b;
    }
    difference == 0
}

/// Answer one request.
fn answer(
    request: &Request,
    session: &Arc<Mutex<Session>>,
    stop: &Arc<AtomicBool>,
    render: &Option<Arc<Mutex<dyn Render + Send>>>,
) -> Response {
    let Some(method) = Method::parse(&request.method) else {
        let names: Vec<&str> = Method::all().iter().map(|method| method.as_str()).collect();
        return Response::error(
            &request.id,
            "unknown-method",
            format!("there is no method `{}`; the methods are {}", request.method, names.join(", ")),
        );
    };
    match method {
        Method::Ping => Response::ok(&request.id, "pong", Some(json!({ "pong": true }))),
        Method::UiClick => {
            let Some(render) = render else {
                return Response::error(&request.id, "no-window", "this program has no window to click in");
            };
            let x = request.params.get("x").and_then(|x| x.as_f64());
            let y = request.params.get("y").and_then(|y| y.as_f64());
            let (Some(x), Some(y)) = (x, y) else {
                return Response::error(
                    &request.id,
                    "bad-request",
                    "a click is a point: `params.x` and `params.y` are the world's units",
                );
            };
            if !x.is_finite() || !y.is_finite() {
                return Response::error(
                    &request.id,
                    "bad-request",
                    "a click is a point in the world's units",
                );
            }
            let Ok(mut render) = render.lock() else {
                return Response::error(&request.id, "internal", "the window is poisoned");
            };
            match render.click(craftkit_sdk::Point::new(x, y)) {
                Ok(answer) => Response::ok(&request.id, "the click", Some(answer)),
                Err(message) => Response::error(&request.id, "failed", message),
            }
        }
        Method::UiRender => {
            // The picture is what the program is showing, and the
            // program is the only one that knows: the channel asks it
            // and hands the answer over.
            let Some(render) = render else {
                return Response::error(&request.id, "no-window", "this program has no window to show");
            };
            let Ok(mut render) = render.lock() else {
                return Response::error(&request.id, "internal", "the window is poisoned");
            };
            match render.render() {
                Ok(bytes) => Response::ok(
                    &request.id,
                    "the picture",
                    Some(json!({
                        "format": "png",
                        "bytes": bytes.len(),
                        "data": craftkit_sdk::bytes::encode(&bytes),
                    })),
                ),
                Err(message) => Response::error(&request.id, "failed", message),
            }
        }
        Method::Describe => {
            let Ok(session) = session.lock() else {
                return Response::error(&request.id, "internal", "the session is poisoned");
            };
            let commands: Vec<Json> = session
                .registry()
                .all()
                .iter()
                .map(|command| {
                    json!({
                        "id": command.id,
                        "label": command.label,
                        "area": command.area,
                        "capability": command.capability,
                        "doc": command.doc,
                        "shortcut": command.shortcut,
                    })
                })
                .collect();
            let kinds: Vec<Json> = session
                .kinds()
                .all()
                .iter()
                .map(|kind| json!({ "kind": kind.kind, "label": kind.label }))
                .collect();
            Response::ok(&request.id, "the catalogue", Some(json!({ "commands": commands, "kinds": kinds })))
        }
        Method::Execute => {
            let Some(command) = request.params.get("command").and_then(|command| command.as_str()) else {
                return Response::error(
                    &request.id,
                    "bad-request",
                    "the request has no command: `params.command` is the command's id",
                );
            };
            let params = request.params.get("params").cloned().unwrap_or_else(|| json!({}));
            let Ok(mut session) = session.lock() else {
                return Response::error(&request.id, "internal", "the session is poisoned");
            };
            // A command that is **waiting** for an answer: the answer
            // comes in the parameters, under the question's name. That is
            // what the MCP door does too, and the two doors answer the
            // same way on purpose.
            let waiting = session
                .pending()
                .filter(|pending| pending.id == command)
                .and_then(|pending| pending.question().cloned())
                .and_then(|question| params.get(&question.name).cloned());
            let result = match waiting {
                Some(answer) => session.answer(answer),
                None => session.execute(command, &params),
            };
            match result {
                Ok(outcome) => Response::ok(
                    &request.id,
                    outcome.message.clone(),
                    Some(json!({
                        "message": outcome.message,
                        "data": outcome.data,
                        "changed": outcome.changed,
                        "needs_input": outcome.needs_input.map(|question| question.to_json()),
                    })),
                ),
                Err(error) => Response::error(&request.id, "refused", error.to_string()),
            }
        }
        Method::State => {
            let Ok(session) = session.lock() else {
                return Response::error(&request.id, "internal", "the session is poisoned");
            };
            let nodes = session.document().nodes.len();
            let selection: Vec<Json> = session
                .selection()
                .iter()
                .map(|selected| json!({ "node": selected.node, "from": selected.from, "to": selected.to }))
                .collect();
            let pending = session.pending().map(|pending| {
                json!({
                    "id": pending.id,
                    "asked": pending.asked.len(),
                    "answers": pending.answers.len(),
                })
            });
            Response::ok(
                &request.id,
                "the state",
                Some(json!({
                    "nodes": nodes,
                    "selection": selection,
                    "history": {
                        "current": session.history().current(),
                        "states": session.history().len(),
                    },
                    "pending": pending,
                })),
            )
        }
        Method::Diagnostics => {
            let Ok(session) = session.lock() else {
                return Response::error(&request.id, "internal", "the session is poisoned");
            };
            Response::ok(
                &request.id,
                "the diagnostics",
                Some(json!({
                    "log": session.log(),
                    "journal": session.journal().len(),
                })),
            )
        }
        Method::Shutdown => {
            stop.store(true, Ordering::Relaxed);
            Response::ok(&request.id, "stopping", Some(json!({ "stopping": true })))
        }
    }
}

/// The client: the other end of the door.
pub struct Client {
    /// The socket.
    stream: TcpStream,
    /// The lines that come back.
    reader: BufReader<TcpStream>,
    /// Whether the token has been sent.
    authenticated: bool,
    /// The token.
    token: String,
    /// The next id.
    next: i64,
}

impl Client {
    /// Connect to a server.
    pub fn connect(address: SocketAddr, token: impl Into<String>) -> Result<Client, ControlError> {
        let stream = TcpStream::connect(address)?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Client { stream, reader, authenticated: false, token: token.into(), next: 0 })
    }

    /// Send a line as it is, and read the answer.
    ///
    /// For a client that wants to say something the methods do not
    /// cover — and for the tests, which need to send a method that does
    /// not exist.
    pub fn send(&mut self, line: &str) -> Result<Json, ControlError> {
        // The token goes on the first request of the connection, and the
        // client does not have to remember to put it there.
        let line = if self.authenticated {
            line.to_string()
        } else {
            // A line that is not a request is sent as it is: the server
            // answers it with a protocol error, and that is the answer
            // worth seeing.
            match Request::from_line(line) {
                Ok(mut request) => {
                    self.authenticated = true;
                    request.token = Some(self.token.clone());
                    request.to_line()?
                }
                Err(_) => line.to_string(),
            }
        };
        let mut writer = self.stream.try_clone()?;
        writeln!(writer, "{line}")?;
        writer.flush()?;
        let Some(answer) = craftkit_sdk::lines::read_line_bounded(&mut self.reader, MAX_LINE)
            .map_err(|message| ControlError::Protocol(ProtocolError::NotJson(message)))?
        else {
            return Err(ControlError::Protocol(ProtocolError::NotJson("the answer was empty".to_string())));
        };
        if answer.trim().is_empty() {
            return Err(ControlError::Protocol(ProtocolError::NotJson("the answer was empty".to_string())));
        }
        craftkit_sdk::serde_json::from_str(answer.trim())
            .map_err(|error| ControlError::Protocol(ProtocolError::NotJson(error.to_string())))
    }

    /// Ask for something, and read the answer.
    pub fn call(&mut self, method: Method, params: Json) -> Result<Json, ControlError> {
        self.next += 1;
        let mut request = Request::new(self.next, method).with_params(params);
        if !self.authenticated {
            request = request.with_token(self.token.clone());
            self.authenticated = true;
        }
        let mut writer = self.stream.try_clone()?;
        writeln!(writer, "{}", request.to_line()?)?;
        writer.flush()?;
        let Some(line) = craftkit_sdk::lines::read_line_bounded(&mut self.reader, MAX_LINE)
            .map_err(|message| ControlError::Protocol(ProtocolError::NotJson(message)))?
        else {
            return Err(ControlError::Protocol(ProtocolError::NotJson("the answer was empty".to_string())));
        };
        if line.trim().is_empty() {
            return Err(ControlError::Protocol(ProtocolError::NotJson("the answer was empty".to_string())));
        }
        let answer: Json = craftkit_sdk::serde_json::from_str(line.trim())
            .map_err(|error| ControlError::Protocol(ProtocolError::NotJson(error.to_string())))?;
        let ok = answer.get("ok").and_then(|ok| ok.as_bool()).unwrap_or(false);
        if ok {
            return Ok(answer.get("data").cloned().unwrap_or(Json::Null));
        }
        let kind = answer.get("kind").and_then(|kind| kind.as_str()).unwrap_or("refused").to_string();
        let message =
            answer.get("message").and_then(|message| message.as_str()).unwrap_or("refused").to_string();
        Err(ControlError::Refused { kind, message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_survives_the_wire() {
        let request = Request::new(7, Method::Ping);
        let line = request.to_line().expect("the line is written");
        let read = Request::from_line(&line).expect("the line is read");
        assert_eq!(read, request);
    }

    #[test]
    fn a_request_with_parameters_and_a_token_survives_the_wire() {
        let request = Request::new(7, Method::Execute)
            .with_params(json!({ "command": "node.add" }))
            .with_token("secret");
        let line = request.to_line().expect("the line is written");
        let read = Request::from_line(&line).expect("the line is read");
        assert_eq!(read.method, "engine.execute");
        assert_eq!(read.params["command"], json!("node.add"));
        assert_eq!(read.token.as_deref(), Some("secret"));
    }

    #[test]
    fn a_line_that_is_not_json_is_refused() {
        assert!(matches!(Request::from_line("not json"), Err(ProtocolError::NotJson(_))));
    }

    #[test]
    fn a_line_that_is_not_an_object_is_refused() {
        assert!(matches!(Request::from_line("[1, 2, 3]"), Err(ProtocolError::NotAnObject(_))));
    }

    #[test]
    fn a_line_that_is_too_long_is_refused() {
        let line = "x".repeat(MAX_LINE + 1);
        assert!(matches!(Request::from_line(&line), Err(ProtocolError::TooLong(_))));
    }

    #[test]
    fn a_request_with_no_method_is_refused() {
        assert!(matches!(Request::from_line("{\"id\": 1}"), Err(ProtocolError::NoMethod)));
    }

    #[test]
    fn the_methods_are_named_the_way_the_family_names_them() {
        assert_eq!(Method::Execute.as_str(), "engine.execute");
        assert_eq!(Method::Describe.as_str(), "engine.describe");
        assert_eq!(Method::State.as_str(), "session.state");
        assert_eq!(Method::Diagnostics.as_str(), "diagnostics.list");
        assert_eq!(Method::Ping.as_str(), "ping");
        assert_eq!(Method::Shutdown.as_str(), "shutdown");
        // The family names the window's methods `ui.*`, and this tree
        // has two of the three: a click and a picture. The family's
        // third is a **screenshot** — the picture of the screen and not
        // of the document — and this tree has no screen to take one of.
        assert_eq!(Method::UiClick.as_str(), "ui.click");
        assert_eq!(Method::UiRender.as_str(), "ui.render");
        assert!(Method::parse("ui.screenshot").is_none(), "there is no screen in this tree");
    }

    #[test]
    fn two_tokens_are_compared_without_stopping_early() {
        assert!(same("secret", "secret"));
        assert!(!same("secret", "secre7"));
        assert!(!same("secret", "secr"));
        assert!(!same("", "x"));
    }
}
