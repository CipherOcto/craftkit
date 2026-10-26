//! End-to-end over stdio: the real binary, a real pipe, real JSON-RPC.
//!
//! Everything else in this crate tests `handle_line`, which is pure and is
//! therefore tested thoroughly. `serve` is the part that reads a pipe, writes
//! replies and decides when to stop, and it had no test at all — `CONTRIBUTING`
//! records it as "the one subcommand no test executes", on the grounds that it
//! blocks on stdio. Blocking on stdio is not a reason a test cannot drive it:
//! that is what a pipe is for.
//!
//! These run in CI. Unlike the window, stdio needs no display.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_craftkit-cli");

/// The running server and the pipe into it.
struct Server {
    child: Child,
    stdin: ChildStdin,
    /// The server's replies, as they arrive.
    ///
    /// A **channel**, not the pipe itself: a pipe has no timeout, so a read
    /// on one waits forever, and a test that waits forever cannot catch a
    /// hang — it *is* the hang. The reader thread does the waiting and the
    /// test waits for the answer with a bound. §224.
    replies: std::sync::mpsc::Receiver<String>,
    /// Where the server's own words go.
    ///
    /// They used to go to `/dev/null`, and a server that died said nothing
    /// about why: the test could only report "a reply to the call", and the
    /// person reading it had nothing to go on. §87.
    log: PathBuf,
}

/// How long the test waits for a reply before it gives up.
const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl Server {
    fn start() -> Self {
        // The folder comes from the tree's one home, so this test cannot be the
        // one that shares a folder with another run. §212.
        let dir = craftkit_core::temp::dir("mcp-stdio").expect("a folder for the server's log");
        let log = dir.join(format!("{:?}.log", std::thread::current().id()));
        // A file, not a pipe: a pipe nobody reads fills up, and then the
        // server blocks writing to it — which would look exactly like the
        // hang this file is trying to catch.
        let err = std::fs::File::create(&log).expect("a log file for the server");
        let mut child = Command::new(BIN)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(err))
            .spawn()
            .expect("the binary that cargo just built");
        let stdin = child.stdin.take().expect("stdin is piped");
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout is piped"));
        // The reader thread. It ends when the pipe ends, and the channel is
        // closed with it, so a server that stops talking is a server whose
        // channel has nothing left in it.
        let (sender, replies) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            while stdout.read_line(&mut line).unwrap_or(0) > 0 {
                let said = std::mem::take(&mut line);
                if sender.send(said).is_err() {
                    return;
                }
            }
        });
        let mut s = Server { child, stdin, replies, log };
        // The handshake every real client sends first, so the rest of the test
        // reads like a session rather than a protocol probe.
        s.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"craftkit-test","version":"1"}}}"#);
        s.reply();
        s
    }

    fn send(&mut self, line: &str) {
        // A write that fails means the server is gone, and the reason it is
        // gone is in its log and its exit status — not in the broken pipe.
        let written = writeln!(self.stdin, "{line}").and_then(|()| self.stdin.flush());
        if let Err(e) = written {
            let complaint = self.complaint();
            panic!("the server stopped listening ({e}) — {complaint}");
        }
    }

    /// The next reply, or `None` if the server has stopped talking.
    ///
    /// The wait is **bounded**: a server that never answers is a failure with
    /// a message, not a test that never ends. §224.
    fn reply(&mut self) -> Option<serde_json::Value> {
        let line = match self.replies.recv_timeout(REPLY_TIMEOUT) {
            Ok(line) => line,
            // The channel is closed: the server stopped talking, and that is
            // not a timeout — it is the answer.
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let complaint = self.complaint();
                panic!("the server did not answer within {REPLY_TIMEOUT:?} — {complaint}");
            }
        };
        Some(serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON: {line:?} ({e})")))
    }

    /// What the server said, and what became of it.
    ///
    /// This is what a failure message is made of: without it, "a reply to the
    /// call" is all anybody has. §87.
    fn complaint(&mut self) -> String {
        let status = self.child.try_wait().ok().flatten();
        let said = std::fs::read_to_string(&self.log).unwrap_or_default();
        let said = said.trim();
        if said.is_empty() {
            format!("the server said nothing; its status is {status:?}")
        } else {
            format!("the server said: {said}; its status is {status:?}")
        }
    }

    /// The next reply, or a panic that says what the server said.
    fn expect_reply(&mut self, what: &str) -> serde_json::Value {
        let reply = self.reply();
        let complaint = self.complaint();
        reply.unwrap_or_else(|| panic!("{what} — {complaint}"))
    }

    /// Wait for the process to end, with the pipe into it still open.
    ///
    /// `Child::wait` closes that pipe, and a server that sees end-of-file on
    /// stdin ends whether or not it was told to — so waiting that way would
    /// prove nothing about the shutdown. This polls instead, and the pipe
    /// into the child stays open the whole time.
    ///
    /// The wait is bounded, because a server that does not end is the failure
    /// this is here to catch, and a test that hangs is not a test that
    /// catches it.
    fn wait_for_exit(&mut self, within: std::time::Duration) -> Option<std::process::ExitStatus> {
        let deadline = std::time::Instant::now() + within;
        while std::time::Instant::now() < deadline {
            if let Some(status) = self.child.try_wait().ok().flatten() {
                return Some(status);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        None
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.log);
    }
}

fn id_of(v: &serde_json::Value) -> i64 {
    v.get("id").and_then(serde_json::Value::as_i64).expect("every reply carries its id")
}

/// The document really changed and the tool really said so.
#[test]
fn a_tool_call_changes_the_document_through_the_real_binary() {
    let mut s = Server::start();

    s.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"craftkit_node_add","arguments":{"name":"EndToEnd","kind":"filter"}}}"#);
    let added = s.expect_reply("a reply to the call");
    assert_eq!(id_of(&added), 2);
    let result = added.get("result").expect("a result, not an error");
    assert_eq!(result["isError"], serde_json::Value::Bool(false), "{added}");
    assert_eq!(result["structuredContent"]["name"], "EndToEnd", "{added}");

    // Independent check: the session says so too, so the test is not just
    // echoing the tool's own answer back at itself.
    s.send(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"craftkit_status","arguments":{}}}"#);
    let status = s.expect_reply("a reply to the status call");
    let text = status["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    assert!(text.contains("EndToEnd"), "the status does not list the node: {text}");
}

/// A refused command is a tool error, not a protocol error, and an unknown tool
/// is the other way round. Mixing those two up is what makes an agent retry a
/// request that can never work.
#[test]
fn refusals_keep_their_protocol_codes() {
    let mut s = Server::start();

    s.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"craftkit_node_remove","arguments":{"node":"nope"}}}"#);
    let bad = s.expect_reply("a reply to the refused command");
    assert_eq!(bad["result"]["isError"], serde_json::Value::Bool(true), "{bad}");
    assert_eq!(bad["result"]["structuredContent"]["kind"], "bad-parameter", "{bad}");

    s.send(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#,
    );
    let missing = s.expect_reply("a reply about the unknown tool");
    assert_eq!(missing["error"]["code"], -32601, "{missing}");

    // Malformed input must not take the server down: the next request still works.
    s.send("this is not json");
    let junk = s.expect_reply("a parse error, not a dead server");
    assert_eq!(junk["error"]["code"], -32700, "{junk}");

    s.send(r#"{"jsonrpc":"2.0","id":4,"method":"ping"}"#);
    assert_eq!(id_of(&s.expect_reply("still alive")), 4);
}

/// A line that is too long is refused, and the server carries on.
///
/// The limit is applied while the line is read, so the reader has to leave
/// the stream in step: the rest of the long line is drained and the next
/// line is read normally. A server that ended here would take the client's
/// whole session with it, and a client that sent one long line by accident
/// would have to start again.
#[test]
fn a_too_long_line_is_refused_and_the_server_carries_on() {
    let mut s = Server::start();

    let huge = format!("{{\"a\":\"{}\"}}", "x".repeat(4 * 1024 * 1024));
    s.send(&huge);
    let refused = s.expect_reply("a refusal, not a dead server");
    assert_eq!(refused["error"]["code"], -32600, "{refused}");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("the limit is"), "{refused}");

    s.send(r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#);
    assert_eq!(id_of(&s.expect_reply("still alive")), 9);
}

/// The defect this file exists for: `serve` said it ended "or `shutdown`
/// arrives", acknowledged `shutdown`, and then served the next request anyway.
/// Kept as an end-to-end test because the unit test cannot see the loop.
#[test]
fn shutdown_ends_the_server_and_the_reply_before_it_is_still_sent() {
    let mut s = Server::start();

    s.send(r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#);
    let ack = s.expect_reply("the acknowledgement is sent before the server stops");
    assert_eq!(id_of(&ack), 2, "{ack}");

    // Nothing more may be answered, and the process must be gone on its own.
    // stdin is held open, so only `shutdown` can end this.
    //
    // The write is **allowed to fail**. A server that has already exited is
    // behaving exactly as it was told, and the broken pipe is the proof of
    // it; the old version panicked on that pipe, so it failed when the
    // server did the right thing and passed when the server was slow.
    let ping = r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#;
    let _ = writeln!(s.stdin, "{ping}");
    let _ = s.stdin.flush();

    // The process must end on its own, with the pipe into it still open.
    let status = s.wait_for_exit(std::time::Duration::from_secs(10));
    assert!(status.is_some(), "the process did not exit after shutdown");
    assert!(s.reply().is_none(), "the server answered after shutdown");
}
