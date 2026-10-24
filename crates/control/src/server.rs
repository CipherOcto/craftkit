//! The server: accept lines, dispatch them, answer.
//!
//! # The shape of it
//!
//! A small thread per connection, one `Mutex<Session>` shared between them, and
//! the lock held for the duration of exactly one command.
//!
//! That is the whole concurrency story, deliberately. A queue in front of a
//! single owner thread buys ordering and avoids lock poisoning at the cost of a
//! channel, a second thread and a shutdown protocol — worth it when commands
//! are long and re-entrant, which none of these are. What matters is the one
//! property the lock gives: two commands never interleave, so a
//! clone-before-edit snapshot can never be observed half-written.

use std::io::{BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use craftkit_engine::Session;
use serde_json::{Value as Json, json};

use crate::protocol::{MAX_LINE, Method, ProtocolError, Request, Response, parse_request};
use crate::token::{self, Token};

/// How many connections may be open at once.
///
/// Small, because every connection is a thread and the point of the channel is
/// local scripting, not serving. A connection beyond the limit is closed
/// immediately rather than queued.
pub const MAX_CONNECTIONS: usize = 8;

/// How long a connection may sit idle before it is dropped, in seconds.
///
/// Bounded so an abandoned connection does not hold a thread for the lifetime
/// of the application.
pub const IDLE_TIMEOUT_SECS: u64 = 300;

/// Answers a `ui.*` request against the live session.
///
/// Borrowed rather than owned because the session is already borrowed: the
/// handler draws from the document and must not be able to keep it. It runs on
/// a connection's own thread while the session lock is held, so it must not
/// call back into the server either.
pub type UiHandler = dyn Fn(&mut Session, &Request) -> Response + Send + Sync;

/// The owned form, for [`ServerOptions`].
pub type SharedUiHandler = Arc<UiHandler>;

/// Told that the session may have changed.
///
/// It is a hook rather than a dependency for the same reason [`UiHandler`] is: a
/// window that draws the session needs to hear about a command that ran on
/// another thread, and this crate must not know what a window is.
///
/// Called *after* the session lock is released and only for a request that can
/// change the document, so a handler is free to read the session without
/// deadlocking against it.
pub type ChangeHandler = dyn Fn() + Send + Sync;

/// The owned form, for [`ServerOptions`].
pub type SharedChangeHandler = Arc<ChangeHandler>;

/// How to start a server.
#[derive(Clone)]
pub struct ServerOptions {
    /// The port. `0` asks the operating system for a free one, which is what a
    /// test or a script should use.
    pub port: u16,
    /// The token every connection must present.
    pub token: Token,
    /// Whether to bind beyond loopback. **Off by default and there is no reason
    /// to turn it on**: a control channel on a routable interface is remote
    /// code execution with a JSON syntax.
    pub allow_remote: bool,
    /// Answers `ui.*`, or refuses to.
    ///
    /// This is a callback rather than a dependency on purpose. `craftkit-ui` is
    /// layer 6 and this crate is layer 5, and the layering rules forbid a lower
    /// layer reaching upwards for convenience — which they should, because the
    /// day the control channel can draw, the control channel can no longer be
    /// tested without a display.
    ///
    /// So the channel defines the extension point and the application fills it
    /// in. A process with no UI leaves it unset, and `ui.*` is refused with a
    /// reason instead of pretending to have worked.
    pub ui: Option<SharedUiHandler>,
    /// Told after a request that could have changed the document.
    ///
    /// Without it a host with a window has no way to learn that a command ran on
    /// this crate's thread: the session is behind a mutex, so the window's own
    /// frame loop sees nothing and keeps painting the frame it last drew. That is
    /// not a cosmetic problem — an agent that drives the app over the channel and
    /// then looks at the window is looking at a picture of the past.
    pub after: Option<SharedChangeHandler>,
    /// Where to write the token file, if this server writes one at all.
    ///
    /// `None` means the usual place: [`crate::token_path`], which the
    /// environment can move. A directory of the caller's choosing is what lets
    /// a test keep the file inside a temporary directory it can clean up —
    /// setting an environment variable is `unsafe` in edition 2024, and this
    /// crate forbids `unsafe`.
    pub token_dir: Option<std::path::PathBuf>,
}

impl ServerOptions {
    pub fn new(port: u16, token: Token) -> Self {
        Self { port, token, allow_remote: false, ui: None, after: None, token_dir: None }
    }

    /// Write the token file in `dir` rather than in the usual place.
    ///
    /// For a test, or for a host that keeps everything it writes in one
    /// directory it owns.
    pub fn with_token_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.token_dir = Some(dir.into());
        self
    }

    /// Answer `ui.*` with `handler`.
    ///
    /// The handler runs while the session is locked, on the connection's own
    /// thread, so it may draw from the session but must not call back into the
    /// server.
    pub fn with_ui(
        mut self,
        handler: impl Fn(&mut Session, &Request) -> Response + Send + Sync + 'static,
    ) -> Self {
        self.ui = Some(Arc::new(handler));
        self
    }

    /// Be told when a request may have changed the document.
    ///
    /// Called outside the session lock and **before** the reply is sent, and
    /// only for `engine.execute` and `ui.*` — the methods that can move the
    /// document. A host that draws the session uses this to ask for a
    /// repaint, and the repaint has to be asked for first: a client that
    /// read the reply before the window had redrawn would be looking at the
    /// frame from before the command. §194.
    pub fn with_after(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
        self.after = Some(Arc::new(handler));
        self
    }
}

/// A running server.
#[derive(Debug)]
pub struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    /// The port actually bound. Different from the requested one when `0` was
    /// asked for.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Ask the server to stop and wait for it.
    pub fn stop(mut self) {
        self.shutdown();
    }

    /// Block until something asks the server to stop.
    ///
    /// For a process whose whole job is to serve — a host whose window could
    /// not open still has a control channel, and an agent that was told it may
    /// drive the process should not find it gone.
    ///
    /// Returns when the accept loop has finished, which happens when
    /// [`Server::stop`] is called or a `shutdown` request arrives. It is
    /// interruptible only by the process being signalled, which is what a
    /// supervising shell expects.
    pub fn join(&mut self) {
        loop {
            match &self.worker {
                None => return,
                Some(worker) if worker.is_finished() => {
                    self.worker = None;
                    return;
                }
                Some(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Nudge the accept loop with a throwaway connection so it notices.
        let _ = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, self.port)));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Bind a port and start serving.
///
/// Returns the bound port, which is how a caller that asked for `0` finds out
/// where to connect.
pub fn serve(session: Arc<Mutex<Session>>, options: ServerOptions) -> Result<(u16, Server), String> {
    let listener = bind(options.port, options.allow_remote)?;
    let ui = options.ui.clone();
    let after = options.after.clone();
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let stop = Arc::new(AtomicBool::new(false));
    let open = Arc::new(AtomicU64::new(0));
    let token = Arc::new(options.token);

    let worker_stop = Arc::clone(&stop);
    let worker_session = Arc::clone(&session);
    let worker_token = Arc::clone(&token);
    let worker = std::thread::spawn(move || {
        accept_loop(listener, worker_stop, worker_session, worker_token, open, ui, after);
    });

    Ok((port, Server { port, stop, worker: Some(worker) }))
}

/// Bind and serve, for a caller that only wants a running server.
pub fn serve_with(session: Arc<Mutex<Session>>, options: ServerOptions) -> Result<Server, String> {
    serve(session, options).map(|(_, server)| server)
}

fn bind(port: u16, allow_remote: bool) -> Result<TcpListener, String> {
    let addr = if allow_remote {
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, port))
    } else {
        SocketAddr::from((Ipv4Addr::LOCALHOST, port))
    };
    TcpListener::bind(addr).map_err(|e| format!("could not bind {addr}: {e}"))
}

#[allow(clippy::too_many_arguments)]
fn accept_loop(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    session: Arc<Mutex<Session>>,
    token: Arc<Token>,
    open: Arc<AtomicU64>,
    ui: Option<SharedUiHandler>,
    after: Option<SharedChangeHandler>,
) {
    // A short read timeout turns `accept` into a poll, which is what makes
    // `stop` observable without another wake-up mechanism.
    let _ = listener.set_nonblocking(true);

    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                if open.load(Ordering::SeqCst) as usize >= MAX_CONNECTIONS {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    continue;
                }
                open.fetch_add(1, Ordering::SeqCst);
                let session = Arc::clone(&session);
                let token = Arc::clone(&token);
                let open_for_thread = Arc::clone(&open);
                let stop_for_thread = Arc::clone(&stop);
                let ui_for_thread = ui.clone();
                let after_for_thread = after.clone();
                let _ = std::thread::Builder::new().name("craftkit-control".to_string()).spawn(move || {
                    handle(
                        stream,
                        session,
                        token,
                        &stop_for_thread,
                        ui_for_thread.as_deref(),
                        after_for_thread.as_deref(),
                    );
                    open_for_thread.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => {
                log::warn!("control channel: accept failed: {e}");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

/// Serve one connection.
#[allow(clippy::too_many_arguments)]
fn handle(
    stream: TcpStream,
    session: Arc<Mutex<Session>>,
    token: Arc<Token>,
    stop: &AtomicBool,
    ui: Option<&UiHandler>,
    after: Option<&ChangeHandler>,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(IDLE_TIMEOUT_SECS)));
    let peer = stream.try_clone().map_err(|e| e.to_string());
    let mut writer = match peer {
        Ok(w) => w,
        Err(e) => {
            log::warn!("control channel: could not split the connection: {e}");
            return;
        }
    };
    let mut reader = BufReader::new(stream);

    let mut authenticated = false;
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // The limit is applied **while reading**: a peer that sends a
        // gigabyte without a newline costs this process a buffer, not a
        // gigabyte — and this process holds the user's document. The bytes
        // past the limit are counted and dropped; the rest of the line is
        // still read, so the connection is not left half-way through it. §49.
        //
        // The buffer is the *text*. The parse of it is what costs memory, and
        // the line limit is derived from the memory budget so that the parse
        // stays inside it. §64.
        let line = match craftkit_core::read_line_bounded(&mut reader, MAX_LINE) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                if let Some(len) = e.too_long_len() {
                    let _ = send(
                        &mut writer,
                        &Response::error(
                            &Json::Null,
                            "request-too-large",
                            ProtocolError::TooLong(len).to_string(),
                        ),
                    );
                    break;
                }
                // A timeout is the normal end of an idle connection.
                if let craftkit_core::ReadError::Io { kind, message } = &e
                    && *kind != std::io::ErrorKind::WouldBlock
                    && *kind != std::io::ErrorKind::TimedOut
                {
                    log::debug!("control channel: connection ended: {message}");
                }
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request = match parse_request(trimmed) {
            Ok(r) => r,
            Err(ProtocolError::TooLong(n)) => {
                let _ = send(
                    &mut writer,
                    &Response::error(&Json::Null, "request-too-large", ProtocolError::TooLong(n).to_string()),
                );
                break;
            }
            Err(e) => {
                // A malformed line does not end the connection: the peer may
                // simply have sent it by mistake, and the reply tells it so.
                let _ = send(&mut writer, &Response::error(&Json::Null, "bad-request", e.to_string()));
                continue;
            }
        };

        // The token is checked before *anything* is interpreted, once per
        // connection, and a failure closes the connection with no reply: an
        // unauthenticated peer learns only that the port is open.
        if !authenticated {
            let presented = request.token.as_deref().unwrap_or("");
            if !token.matches(presented) {
                log::warn!("control channel: rejected a connection with a bad token");
                break;
            }
            authenticated = true;
        }

        let method = match request.method() {
            Ok(m) => m,
            Err(r) => {
                let _ = send(&mut writer, &r);
                continue;
            }
        };

        let response = answer(&request, session.as_ref(), ui);

        // Announced *before* the reply is sent, and that ordering is the whole
        // point. `answer` has already changed the document by this line; whether
        // the reply then reaches the client is a fact about the socket and says
        // nothing about the document. Telling the host after the send meant a
        // client that disconnected mid-command left the window showing the frame
        // from before that command, silently and permanently.
        //
        // `answer` has also returned, so the session lock is released and a
        // handler is free to read the document it just changed. Only the methods
        // that can change something are announced: `engine.describe` in a loop
        // must not repaint a window.
        if matches!(method, Method::Execute | Method::Ui)
            && let Some(handler) = after
        {
            handler();
        }

        if send(&mut writer, &response).is_err() {
            break;
        }

        if method == Method::Shutdown {
            break;
        }
    }
    let _ = writer.shutdown(std::net::Shutdown::Both);
}

/// Answer one request, holding the session for exactly as long as it takes.
fn answer(request: &Request, session: &Mutex<Session>, ui: Option<&UiHandler>) -> Response {
    // Lock poisoning is reported, never propagated: a panic on another
    // connection's thread must not turn this one into a hang.
    match session.lock() {
        Ok(mut guard) => dispatch_with(&mut guard, request, ui),
        Err(_) => Response::error(
            &request.id,
            "poisoned",
            "another command panicked and the session is unavailable; restart the process",
        ),
    }
}

/// Answer a request against a session, with no I/O anywhere.
pub fn dispatch(session: &mut Session, request: &Request) -> Response {
    dispatch_with(session, request, None)
}

/// [`dispatch`], with the host's `ui.*` handler if it has one.
pub fn dispatch_with(session: &mut Session, request: &Request, ui: Option<&UiHandler>) -> Response {
    let method = match request.method() {
        Ok(m) => m,
        Err(r) => return r,
    };
    match method {
        Method::Execute => execute(session, request),
        Method::Describe => Response::ok(&request.id, "the command catalogue", craftkit_engine::catalogue()),
        Method::State => Response::ok(&request.id, "the session", session.snapshot_json()),
        Method::Diagnostics => match session.execute("diagnostics.list", &Json::Object(Default::default())) {
            Ok(o) => Response::ok(&request.id, o.message, o.data),
            Err(e) => Response::error(&request.id, e.kind(), e.to_string()),
        },
        Method::Crashes => crashes(session, request),
        Method::Ping => Response::ok(&request.id, "pong", json!({ "at": session.at().raw() })),
        // `ui.*` is answered by the host, which is the only thing that knows
        // whether there is a UI. Without one this is a refusal rather than a
        // silent success, because a silent success here is how an agent comes
        // to believe it clicked something.
        Method::Ui => match ui {
            // The handler is host code, not ours, running with the session
            // locked. Without a boundary here a panic in it would drop the
            // connection with no reply at all, and an agent would be left
            // guessing whether its request had arrived.
            //
            // Catching here rather than in `answer` also means the session
            // guard is never dropped during unwinding, so the lock is not
            // poisoned and the next request still works — the same bargain the
            // engine keeps for a panicking command.
            Some(handler) => {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(session, request))) {
                    Ok(response) => response,
                    Err(_) => Response::error(
                        &request.id,
                        "handler-panicked",
                        "the window handler panicked; the session is unchanged and this connection \
                     is still good",
                    ),
                }
            }
            None => Response::error(
                &request.id,
                "no-window",
                "this process has no window, so `ui.*` cannot run. \n\
                 `engine.execute` reaches every command and does everything a click does; \
                 `craftkit-cli snapshot` renders a frame to a PNG without a window.",
            ),
        },
        Method::Shutdown => Response::ok_empty(&request.id, "stopping"),
    }
}

fn execute(session: &mut Session, request: &Request) -> Response {
    let command = match request.params.get("command").and_then(Json::as_str) {
        Some(c) => c,
        None => {
            return Response::error(
                &request.id,
                "bad-parameter",
                "`command` is required and must be a string; call `engine.describe` for the list",
            );
        }
    };
    let params = request.params.get("params").cloned().unwrap_or(Json::Object(Default::default()));
    if !params.is_object() {
        return Response::error(&request.id, "bad-parameter", "`params` must be an object");
    }
    match session.execute(command, &params) {
        Ok(outcome) => Response::ok(&request.id, outcome.message, outcome.data),
        Err(e) => Response::error(&request.id, e.kind(), e.to_string()),
    }
}

fn crashes(session: &Session, request: &Request) -> Response {
    crashes_in(session, request, &craftkit_engine::crash::report_dir())
}

/// [`crashes`], in a named folder.
///
/// The folder is a parameter for the same reason the engine's own crash
/// functions have `_in` variants: the environment variable that relocates the
/// crash folder is process-wide, so a test that set it would race every other
/// test in the binary. §47.
///
/// The three actions are the engine's, and the answers are the engine's too:
/// this is a transport, not a second implementation. §113.
fn crashes_in(session: &Session, request: &Request, dir: &std::path::Path) -> Response {
    let action = request.params.get("action").and_then(Json::as_str).unwrap_or("list");
    let id = request.params.get("id").and_then(Json::as_str);
    let from = session.document().meta.name.clone();
    match craftkit_engine::crash::action_in(dir, action, id, &from) {
        Ok(data) => {
            let message = match action {
                "show" => format!("report {}", id.unwrap_or("")),
                "replay" => format!(
                    "replayed {} step(s) onto the design the report recorded",
                    data.get("steps").and_then(Json::as_u64).unwrap_or(0)
                ),
                _ => "crash reports".to_string(),
            };
            Response::ok(&request.id, message, data)
        }
        Err(e) => Response::error(&request.id, e.kind(), e.message()),
    }
}

fn send(writer: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    let wire = response.to_wire().map_err(|e| std::io::Error::other(e.to_string()))?;
    writer.write_all(wire.as_bytes())?;
    writer.flush()
}

/// Put the token for `port` where a script can find it.
///
/// Returns the path. Called by the app's `--control` flag; a script reads this
/// file rather than being told the token out of band.
pub fn publish_token(port: u16, token: &Token) -> Result<std::path::PathBuf, String> {
    let path = token::token_path(port);
    token::write_token(&path, token)?;
    Ok(path)
}

/// Bind the channel and publish its token, **in that order**.
///
/// The order is the whole point. The token file is the app's promise that this
/// port answers to this token, and a promise must not be made for a port this
/// process does not hold: a second instance, which cannot bind, would
/// otherwise replace the running instance's token with its own and lock the
/// running one out — every request refused with *"a bad token"*, by an app
/// whose window is right there working. §54.
///
/// The file is also named for the port that was **bound**, not the one that was
/// asked for: with port `0` the operating system picks the port, and a file
/// named `0.token` is a promise about nothing.
pub fn serve_published(
    session: Arc<Mutex<Session>>,
    options: ServerOptions,
) -> Result<(Server, std::path::PathBuf), String> {
    let token = options.token.clone();
    let dir = options.token_dir.clone();
    let requested = options.port;
    // Bind first. If this fails, the token file is never touched.
    let (port, server) = serve(session, options)?;
    let path = write_token_file(dir.as_deref(), port, &token)?;
    // A caller that asked for a specific port and did not get it would be
    // reading a file for a port it does not own.
    if port != requested && requested != 0 {
        return Err(format!("asked for port {requested} and got {port}"));
    }
    Ok((server, path))
}

/// Write the token file for `port`, in `dir` if one was given.
fn write_token_file(
    dir: Option<&std::path::Path>,
    port: u16,
    token: &Token,
) -> Result<std::path::PathBuf, String> {
    let path = match dir {
        Some(dir) => token::token_path_in(dir, port),
        None => token::token_path(port),
    };
    token::write_token(&path, token)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(port: u16) -> ServerOptions {
        ServerOptions::new(port, Token::from_text("0123456789abcdef0123").expect("valid"))
    }

    /// Ask for a port, connect, send, read one line.
    fn round_trip(port: u16, lines: &[&str]) -> Vec<String> {
        use std::io::BufRead as _;
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut out = Vec::new();
        for line in lines {
            let mut framed = line.to_string();
            framed.push('\n');
            if stream.write_all(framed.as_bytes()).is_err() {
                break;
            }
            let _ = stream.flush();
            let mut reply = String::new();
            if reader.read_line(&mut reply).is_err() || reply.is_empty() {
                break;
            }
            out.push(reply.trim().to_string());
        }
        out
    }

    #[test]
    fn a_server_binds_a_free_port_when_asked_for_zero() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        assert!(port > 0);
        drop(server);
    }

    /// The hook is the only way a host with a window learns that a command ran
    /// on this crate's thread, so it has to fire for the methods that change
    /// something — and stay quiet for the ones that do not, or a `describe` in
    /// a loop would repaint a window on every iteration.
    #[test]
    fn the_change_hook_fires_for_commands_and_not_for_questions() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let session = Arc::new(Mutex::new(Session::sample()));
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&seen);
        let (port, server) = serve(
            session,
            options(0).with_after(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .expect("bind");

        let replies = round_trip(
            port,
            &[
                r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.describe"}"#,
                r#"{"id":2,"method":"ping"}"#,
                r#"{"id":3,"method":"engine.execute","params":{"command":"node.add","params":{"name":"Hooked","kind":"filter"}}}"#,
                r#"{"id":4,"method":"engine.execute","params":{"command":"node.add","params":{"name":"Also hooked","kind":"cue"}}}"#,
            ],
        );
        assert_eq!(replies.len(), 4, "{replies:?}");
        assert!(replies[2].contains("\"ok\":true"), "{}", replies[2]);
        assert!(replies[3].contains("\"ok\":true"), "{}", replies[3]);
        // Two of the four changed the document; neither of the questions did.
        assert_eq!(seen.load(Ordering::SeqCst), 2, "the hook fired {replies:?}");
        drop(server);
    }

    /// The hook fires **before** the reply is sent, and that order is the
    /// whole point of it: a host that draws the session asks for a repaint,
    /// and a client that read the reply before the window had redrawn would
    /// be looking at the frame from before the command.
    ///
    /// The hook blocks until the test releases it, so the test can look at
    /// the socket while the server is inside the hook and cannot have sent
    /// the reply yet. §194.
    #[test]
    fn the_change_hook_fires_before_the_reply_is_sent() {
        use std::io::BufRead as _;
        use std::sync::mpsc;
        use std::sync::{Condvar, Mutex};
        use std::time::Duration;

        let session = Arc::new(Mutex::new(Session::sample()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let hook_gate = Arc::clone(&gate);
        let (port, server) = serve(
            session,
            options(0).with_after(move || {
                let _ = entered_tx.send(());
                let (lock, condvar) = &*hook_gate;
                let mut released = lock.lock().expect("the gate");
                while !*released {
                    released = condvar.wait(released).expect("the gate");
                }
            }),
        )
        .expect("bind");

        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut request = r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.execute","params":{"command":"node.add","params":{"name":"Ordered","kind":"filter"}}}"#.to_string();
        request.push('\n');
        stream.write_all(request.as_bytes()).expect("write");
        let _ = stream.flush();

        // The server is inside the hook now, so it has not sent the reply.
        entered_rx.recv_timeout(Duration::from_secs(5)).expect("the hook fired");

        stream.set_read_timeout(Some(Duration::from_millis(200))).expect("a read timeout");
        let mut early = String::new();
        let arrived = reader.read_line(&mut early).is_ok() && !early.is_empty();

        // Let the server finish either way, so a failure below does not leave
        // a thread blocked in the hook.
        {
            let (lock, condvar) = &*gate;
            *lock.lock().expect("the gate") = true;
            condvar.notify_all();
        }

        assert!(
            !arrived,
            "the reply was sent before the hook fired, so a client could read a frame from before \
             the command"
        );

        stream.set_read_timeout(Some(Duration::from_secs(5))).expect("a read timeout");
        let mut reply = String::new();
        reader.read_line(&mut reply).expect("the reply arrives");
        assert!(reply.contains("\"ok\":true"), "{reply}");
        drop(server);
    }

    /// A `ui.*` request can change the document too — a click can — so it is
    /// announced like a command rather than silently skipped.
    #[test]
    fn the_change_hook_fires_for_ui_requests() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let session = Arc::new(Mutex::new(Session::sample()));
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&seen);
        let (port, server) = serve(
            session,
            options(0)
                .with_ui(|_session, request| crate::Response::ok(&request.id, "drew", serde_json::json!({})))
                .with_after(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                }),
        )
        .expect("bind");

        let replies = round_trip(port, &[r#"{"id":1,"token":"0123456789abcdef0123","method":"ui.render"}"#]);
        assert_eq!(replies.len(), 1, "{replies:?}");
        assert!(replies[0].contains("drew"), "{}", replies[0]);
        assert_eq!(seen.load(Ordering::SeqCst), 1, "the hook never fired: {replies:?}");
        drop(server);
    }

    #[test]
    fn a_request_with_the_token_is_answered() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies = round_trip(
            port,
            &[
                r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.execute","params":{"command":"node.add","params":{"name":"Via socket","kind":"filter"}}}"#,
            ],
        );
        assert_eq!(replies.len(), 1, "{replies:?}");
        assert!(replies[0].contains("\"ok\":true"), "{}", replies[0]);
        assert!(replies[0].contains("Via socket"), "{}", replies[0]);
        drop(server);
    }

    #[test]
    fn the_token_is_only_needed_on_the_first_request() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies = round_trip(
            port,
            &[r#"{"id":1,"token":"0123456789abcdef0123","method":"ping"}"#, r#"{"id":2,"method":"ping"}"#],
        );
        assert_eq!(replies.len(), 2, "{replies:?}");
        assert!(replies.iter().all(|r| r.contains("\"ok\":true")), "{replies:?}");
        drop(server);
    }

    #[test]
    fn a_connection_without_the_token_is_closed_with_no_reply() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies = round_trip(port, &[r#"{"id":1,"method":"ping"}"#]);
        assert!(replies.is_empty(), "an unauthenticated peer must learn nothing: {replies:?}");
        drop(server);
    }

    #[test]
    fn a_connection_with_the_wrong_token_is_closed() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies =
            round_trip(port, &[r#"{"id":1,"token":"wrong-token-but-long-enough","method":"ping"}"#]);
        assert!(replies.is_empty(), "{replies:?}");
        drop(server);
    }

    /// A token with padding on the end is not the token.
    ///
    /// The comparison folded the length difference into a `u8` accumulator, so
    /// every length difference that was a multiple of 256 was lost and the
    /// token followed by 256 NUL bytes was accepted. This is the level at
    /// which it mattered: the channel answered a peer that had not presented
    /// the token. §63.
    ///
    /// The padding is written as `\u0000`, which is how any JSON encoder
    /// writes a NUL: the channel's own parser refuses a *raw* control
    /// character, so the padding only has to survive the escape to get in.
    #[test]
    fn a_connection_with_a_padded_token_is_closed() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let padded = format!("0123456789abcdef0123{}", r"\u0000".repeat(256));
        let line = format!(r#"{{"id":1,"token":"{padded}","method":"ping"}}"#);
        let replies = round_trip(port, &[&line]);
        assert!(replies.is_empty(), "a padded token was accepted: {replies:?}");
        drop(server);
    }

    #[test]
    fn an_unknown_method_is_answered_with_the_list_of_real_ones() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies =
            round_trip(port, &[r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.eplode"}"#]);
        assert_eq!(replies.len(), 1);
        assert!(replies[0].contains("unknown-method"), "{}", replies[0]);
        drop(server);
    }

    #[test]
    fn a_malformed_line_is_answered_and_the_connection_survives() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies = round_trip(
            port,
            &[
                r#"{"id":1,"token":"0123456789abcdef0123","method":"ping"}"#,
                "{ not json",
                r#"{"id":3,"token":"0123456789abcdef0123","method":"ping"}"#,
            ],
        );
        assert_eq!(replies.len(), 3, "{replies:?}");
        assert!(replies[1].contains("bad-request"), "{}", replies[1]);
        assert!(replies[2].contains("\"ok\":true"), "the connection should have survived: {}", replies[2]);
        drop(server);
    }

    #[test]
    fn an_execute_without_a_command_is_refused_with_a_way_forward() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies =
            round_trip(port, &[r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.execute"}"#]);
        assert!(replies[0].contains("bad-parameter"), "{}", replies[0]);
        assert!(replies[0].contains("engine.describe"), "{}", replies[0]);
        drop(server);
    }

    #[test]
    fn the_catalogue_can_be_fetched_over_the_wire() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies =
            round_trip(port, &[r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.describe"}"#]);
        assert!(replies[0].contains("node.add"), "{}", &replies[0][..replies[0].len().min(200)]);
        drop(server);
    }

    #[test]
    fn a_ui_method_without_a_window_says_so_rather_than_pretending() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies = round_trip(
            port,
            &[r#"{"id":1,"token":"0123456789abcdef0123","method":"ui.click","params":{"x":1,"y":2}}"#],
        );
        assert!(replies[0].contains("no-window"), "{}", replies[0]);
        drop(server);
    }

    #[test]
    fn a_host_handler_answers_ui_instead_of_the_refusal() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let options = options(0).with_ui(|_session, request| {
            Response::ok(&request.id, "handled here", json!({ "by": "the host" }))
        });
        let (port, server) = serve(session, options).expect("bind");
        let replies = round_trip(port, &[r#"{"id":7,"token":"0123456789abcdef0123","method":"ui.render"}"#]);
        assert!(replies[0].contains("\"ok\":true"), "{}", replies[0]);
        assert!(replies[0].contains("handled here"), "{}", replies[0]);
        assert!(replies[0].contains("the host"), "{}", replies[0]);
        assert!(replies[0].contains("\"id\":7"), "the id must come back: {}", replies[0]);
        drop(server);
    }

    /// The crash replay says whether it reproduced the document.
    ///
    /// The consumer of the crash pipeline that nothing tested: the control
    /// channel's `diagnostics.crash_reports` with `action: "replay"`. It used
    /// to answer with a step count and nothing else, so an agent could not
    /// tell a reproduction from a different document. §47.
    #[test]
    fn the_crash_replay_says_whether_it_reproduced_the_document() {
        let dir = std::env::temp_dir()
            .join("craftkit-control-crash-tests")
            .join(format!("replay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");

        let mut session = Session::sample();
        session
            .execute("node.add", &serde_json::json!({ "name": "Before the crash", "kind": "filter" }))
            .expect("a valid command");
        let report = craftkit_engine::crash::build(&session, "key.set", "something went wrong");
        let written = craftkit_engine::crash::write_to(&dir, &report).expect("the report is written");
        let report_id = written.file_name().expect("a name").to_string_lossy().to_string();

        let request =
            Request::new(9, Method::Crashes).with_params(json!({ "action": "replay", "id": report_id }));
        let response = crashes_in(&session, &request, &dir);
        assert!(response.ok, "{response:?}");
        let data = response.data.expect("the reply carries the outcome");
        assert_eq!(data["reproduced"], json!(true), "{data}");
        assert_eq!(data["steps"], json!(1), "{data}");
        assert_eq!(data["differences"], json!([]), "{data}");

        // And a report whose recorded design does not match says so.
        //
        // It has to be its **own** report. This used to write the lying report
        // over the honest one and then ask for the id — which worked only
        // because a second report with the same id overwrote the first, the
        // defect §58 fixed. A test that sets up its scenario with the defect
        // cannot be the test that catches it.
        let mut lying = report.clone();
        lying.id = format!("{report_id}-lying");
        lying.design.meta.name = "lies".to_string();
        let lying_dir = craftkit_engine::crash::write_to(&dir, &lying).expect("the report is written");
        let lying_id = lying_dir.file_name().expect("a name").to_string_lossy().to_string();
        let request =
            Request::new(10, Method::Crashes).with_params(json!({ "action": "replay", "id": lying_id }));
        let response = crashes_in(&session, &request, &dir);
        assert!(response.ok, "{response:?}");
        let data = response.data.expect("the reply carries the outcome");
        assert_eq!(data["reproduced"], json!(false), "{data}");
        assert!(!data["differences"].as_array().expect("an array").is_empty(), "{data}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dispatch_without_a_handler_still_refuses_ui() {
        // `dispatch` is the no-I/O entry point and has no host; it must refuse
        // rather than diverge from the server.
        let mut session = Session::sample();
        let request = Request::new(1, Method::Ui);
        let response = dispatch(&mut session, &request);
        assert!(!response.ok);
        assert_eq!(response.kind, Some("no-window"), "{response:?}");
        assert!(response.message.as_deref().unwrap_or_default().contains("engine.execute"), "{response:?}");
    }

    #[test]
    fn a_handler_that_panics_does_not_take_the_server_down() {
        // The host handler is somebody else's code running on this thread with
        // the session locked. A panic in it must become an error reply, not a
        // dead socket — and the next request must still work.
        let session = Arc::new(Mutex::new(Session::sample()));
        let options = options(0).with_ui(|_session, request| {
            if request.params.get("action").and_then(Json::as_str) == Some("explode") {
                panic!("the host handler panicked");
            }
            Response::ok_empty(&request.id, "fine")
        });
        let (port, server) = serve(session, options).expect("bind");
        let replies = round_trip(
            port,
            &[
                r#"{"id":1,"token":"0123456789abcdef0123","method":"ui.render","params":{"action":"explode"}}"#,
                r#"{"id":2,"token":"0123456789abcdef0123","method":"ping"}"#,
            ],
        );
        assert_eq!(replies.len(), 2, "the connection died: {replies:?}");
        // Whatever happened first, the connection survived; the lock is
        // poisoned either way, so `ping` may fail — what must not happen is a
        // hang or a dropped connection.
        assert!(replies[0].contains("id"), "{}", replies[0]);
        assert!(replies[1].contains("id"), "{}", replies[1]);
        drop(server);
    }

    #[test]
    fn the_ui_refusal_points_at_what_does_work() {
        // An agent that gets "no window" and nothing else will try the window.
        let mut session = Session::sample();
        let response = dispatch(&mut session, &Request::new(1, Method::Ui));
        let message = response.message.unwrap_or_default();
        assert!(message.contains("engine.execute"), "{message}");
        assert!(message.contains("snapshot"), "{message}");
    }

    #[test]
    fn a_bad_command_comes_back_with_its_kind_and_its_command() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let replies = round_trip(
            port,
            &[
                r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.execute","params":{"command":"node.nope"}}"#,
            ],
        );
        assert!(replies[0].contains("unknown-command"), "{}", replies[0]);
        drop(server);
    }

    #[test]
    fn two_connections_share_one_session_consistently() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let (port, server) = serve(Arc::clone(&session), options(0)).expect("bind");
        let a = round_trip(
            port,
            &[
                r#"{"id":1,"token":"0123456789abcdef0123","method":"engine.execute","params":{"command":"node.add","params":{"name":"One","kind":"cue"}}}"#,
            ],
        );
        let b = round_trip(port, &[r#"{"id":1,"token":"0123456789abcdef0123","method":"session.state"}"#]);
        assert!(a[0].contains("\"ok\":true"), "{}", a[0]);
        assert!(b[0].contains("\"One\""), "the second connection must see the first's work: {}", b[0]);
        drop(server);
    }

    #[test]
    fn a_second_bind_to_the_same_port_is_refused_rather_than_stealing_it() {
        let session = Arc::new(Mutex::new(Session::sample()));
        let opts = ServerOptions::new(0, Token::from_text("0123456789abcdef0123").expect("valid"));
        let (port, server) = serve(session, opts).expect("bind");
        let Err(err) = serve(Arc::new(Mutex::new(Session::sample())), options(port)) else {
            panic!("a taken port must be refused rather than silently reused");
        };
        assert!(err.contains("could not bind"), "{err}");
        drop(server);
    }

    /// A second instance must not take the first one's token file.
    ///
    /// The token file is the promise that a port answers to a token, and an
    /// instance that could not bind has no business writing it. It used to: the
    /// file was written *before* the bind, so a second instance on the same
    /// port replaced the running instance's token with its own and every
    /// request to the running app was refused with *"a bad token"* — by an app
    /// whose window was right there, working. §54.
    #[test]
    fn a_second_instance_does_not_take_the_first_ones_token_file() {
        let dir = std::env::temp_dir()
            .join("craftkit-control-tests")
            .join(format!("token-takeover-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");

        let first_token = Token::generate().expect("entropy");
        let (server, path) = serve_published(
            Arc::new(Mutex::new(Session::sample())),
            ServerOptions::new(0, first_token.clone()).with_token_dir(dir.clone()),
        )
        .expect("the first instance");
        let written = std::fs::read_to_string(&path).expect("the first token");
        assert_eq!(written, first_token.as_str(), "the file must hold the token");
        assert!(
            path.ends_with(format!("{}.token", server.port())),
            "the file must be named for the port that was bound, not the one asked for: {}",
            path.display()
        );

        // A second instance on the same port cannot bind.
        let second_token = Token::generate().expect("entropy");
        let err = serve_published(
            Arc::new(Mutex::new(Session::sample())),
            ServerOptions::new(server.port(), second_token.clone()).with_token_dir(dir.clone()),
        )
        .expect_err("the port is taken");
        assert!(err.contains("could not bind"), "{err}");

        // And the running instance's token is still the one in the file.
        let after = std::fs::read_to_string(&path).expect("the token is still there");
        assert_eq!(after, first_token.as_str(), "the second instance took the first one's token file");
    }

    #[test]
    fn the_listener_is_loopback_only_by_default() {
        let listener = bind(0, false).expect("bind");
        let addr = listener.local_addr().expect("addr");
        assert!(addr.ip().is_loopback(), "bound to {addr}");
    }

    #[test]
    fn the_default_is_loopback_and_the_default_is_what_binds() {
        // The failure this catches: `allow_remote` defaulting to `true` binds
        // 0.0.0.0 and makes the control channel remote code execution on every
        // interface, with a JSON syntax. The test above passed a hardcoded
        // `false` instead of the real default, so flipping the default left all
        // 41 tests green. Found by doing exactly that.
        let options = options(0);
        assert!(!options.allow_remote, "the control channel must not bind beyond loopback by default");

        // And not just the field: bind with whatever the default actually is.
        let listener = bind(0, options.allow_remote).expect("bind");
        let addr = listener.local_addr().expect("addr");
        assert!(addr.ip().is_loopback(), "bound to {addr}, which is reachable from another machine");
    }

    #[test]
    fn nothing_in_the_tree_turns_the_control_channel_into_a_remote_one() {
        // The realistic way the boundary goes is not a flipped default but a
        // developer adding the flag where the server is built. This reads the
        // whole workspace, so the flag set to a true value anywhere fails the
        // build.
        let root = workspace_root();
        // Assembled at run time so this file's own source does not contain the
        // literals it is looking for. A test that matches itself is a test that
        // always fails, which is the same species of problem as one that always
        // passes.
        //
        // Both spellings, because both are real: a struct literal sets the field
        // with a colon, and a developer with a `ServerOptions` in hand assigns
        // to it. A first version of this test only looked for the colon, and a
        // probe that used an equals sign walked straight past it.
        let truth = "true";
        let needles = [format!("allow_remote: {truth}"), format!("allow_remote = {truth}")];
        let mut checked = 0usize;
        for dir in ["crates", "apps", "xtask"] {
            let dir = root.join(dir);
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let src = entry.path().join("src");
                let Ok(files) = std::fs::read_dir(&src) else { continue };
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                        continue;
                    }
                    let Ok(text) = std::fs::read_to_string(&path) else { continue };
                    checked += 1;
                    assert!(
                        !needles.iter().any(|n| text.contains(n.as_str())),
                        "{} opens the control channel to the network.\n\
                         A control channel on a routable interface is remote code execution with a\n\
                         JSON syntax. If you have a reason, delete this test and write down what it\n\
                         is in crates/control/src/server.rs, where the field is declared.",
                        path.display()
                    );
                }
            }
        }
        assert!(checked >= 10, "only read {checked} files; the scan is wrong, not the tree");
    }

    /// The workspace root: the directory holding this crate's parent's manifest.
    ///
    /// Found by walking up for the `Cargo.toml` that lists `crates/control`,
    /// rather than assuming a fixed depth.
    fn workspace_root() -> std::path::PathBuf {
        let mut dir: &std::path::Path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        loop {
            let manifest = dir.join("Cargo.toml");
            if manifest.is_file()
                && std::fs::read_to_string(&manifest)
                    .map(|t| t.contains("crates/control") || t.contains("crates/*"))
                    .unwrap_or(false)
            {
                return dir.to_path_buf();
            }
            dir = dir.parent().unwrap_or(dir);
        }
    }
}
