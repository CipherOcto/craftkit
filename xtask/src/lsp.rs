//! The **language server**: `rust-analyzer`, asked about the tree.
//!
//! The tree is a Rust tree, and a Rust tree has a language server: a
//! program that knows what every name means, where every item was
//! declared, and what the compiler thinks of every line. An agent that
//! greps for a name is asking a question the server has already
//! answered — and a grep cannot tell a *declaration* from a *use*.
//!
//! This command is the door to that server. It speaks the **language
//! server protocol** over the server's standard input and output, and
//! it answers four questions about the tree:
//!
//! - **where is this thing declared** (`definition`),
//! - **what else mentions it** (`references`),
//! - **what is it** (`hover`),
//! - **what does the compiler say about this file** (`diagnostics`),
//!
//! and one about the tree as a whole (**what is called this**, in
//! `symbols`).
//!
//! # The rules
//!
//! - The server is **not run twice**: one server answers every
//!   question of one invocation, and it is shut down when the command
//!   is done.
//! - A question about a **place** takes a file, a line and a column,
//!   counted from **one**, and the column is counted in
//!   **characters** — not bytes, and not UTF-16 units: the file is
//!   read, and the position is found in the text as a person counts
//!   it.
//! - A file is named by its path **in the tree**: `crates/craftkit-xlsx/src/lib.rs`.
//! - The **first load is slow**: the server reads the whole tree and
//!   builds its picture of it, and that is minutes the first time and
//!   seconds after that. The command says so, so that a reader does
//!   not think it has hung.
//! - A server that is **not there** is an error with the command that
//!   installs it, and not a crash: `rustup component add rust-analyzer`.
//!
//! # What this does not do
//!
//! - It does not **check** the tree: the gate does that, and the gate
//!   is `cargo xtask ci`.
//! - It does not **edit** anything: the server is asked, and the answer
//!   is printed.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// The **name** of the server: the program that answers.
const SERVER: &str = "rust-analyzer";

/// How long the server may take to answer a question, in seconds.
///
/// The first load of the tree is the slow one, and a server that is
/// building its picture of the tree answers nothing until it has it.
const TIMEOUT: u64 = 600;

/// The **queries** this command answers.
const QUERIES: [&str; 5] = ["definition", "references", "hover", "diagnostics", "symbols"];

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let Some(query) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    if !QUERIES.contains(&query) {
        return Err(usage());
    }
    let rest = &args[1..];
    match query {
        "symbols" => {
            let Some(text) = rest.first() else { return Err(usage()) };
            let query = text.trim();
            if query.is_empty() {
                return Err("a symbol search needs something to search for".to_string());
            }
            let mut session = Session::start(root)?;
            let answer = session.request("workspace/symbol", json!({ "query": query }))?;
            session.finish()?;
            print_symbols(&answer);
        }
        "diagnostics" => {
            let Some(file) = rest.first() else { return Err(usage()) };
            let path = the_file(root, file)?;
            let mut session = Session::start(root)?;
            let answer = session
                .request("textDocument/diagnostic", json!({ "textDocument": { "uri": uri_of(&path) } }))?;
            session.finish()?;
            print_diagnostics(&answer);
        }
        _ => {
            let (Some(file), Some(line), Some(column)) = (rest.first(), rest.get(1), rest.get(2)) else {
                return Err(usage());
            };
            let path = the_file(root, file)?;
            let line = number_of(line, "a line")?;
            let column = number_of(column, "a column")?;
            let (line, column) = position_of(&path, line, column)?;
            let mut session = Session::start(root)?;
            let answer = match query {
                "definition" => session.request(
                    "textDocument/definition",
                    json!({ "textDocument": { "uri": uri_of(&path) }, "position": { "line": line, "character": column } }),
                )?,
                "references" => session.request(
                    "textDocument/references",
                    json!({
                        "textDocument": { "uri": uri_of(&path) },
                        "position": { "line": line, "character": column },
                        "context": { "includeDeclaration": true },
                    }),
                )?,
                _ => session.request(
                    "textDocument/hover",
                    json!({ "textDocument": { "uri": uri_of(&path) }, "position": { "line": line, "character": column } }),
                )?,
            };
            session.finish()?;
            match query {
                "hover" => print_hover(&answer),
                _ => print_places(&answer),
            }
        }
    }
    Ok(crate::Outcome::Ran)
}

/// The usage of this command.
fn usage() -> String {
    format!(
        "usage: cargo xtask lsp <query>\n\n\
         The language server: rust-analyzer, asked about the tree.\n\n\
         Queries:\n  \
         definition  <file> <line> <column>   where the thing at that place is declared\n  \
         references  <file> <line> <column>   what else mentions it\n  \
         hover       <file> <line> <column>   what it is\n  \
         diagnostics <file>                    what the compiler says about the file\n  \
         symbols     <text>                    what is called that\n\n\
         A file is a path in the tree; a line and a column are counted from one.\n\
         The server is `{SERVER}`; it is installed with `rustup component add {SERVER}`."
    )
}

/// The file a name says, and the path it is at.
///
/// The path is inside the tree: a name that is not a path in the tree
/// is refused, so that a typo is an error and not a file somewhere
/// else.
fn the_file(root: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a file has a name".to_string());
    }
    let path = root.join(name);
    if !path.is_file() {
        return Err(format!("`{name}` is not a file of the tree"));
    }
    Ok(path)
}

/// A number a name says.
fn number_of(text: &str, what: &str) -> Result<u64, String> {
    text.trim().parse::<u64>().map_err(|_| format!("{what} is a number: `{}` is not one", text.trim()))
}

/// The **place** of a line and a column, in the units the protocol
/// uses: the line from zero, and the column in **UTF-16 units**.
///
/// The command counts from one, in characters; the protocol counts
/// from zero, in the units a string is measured in. The file is read,
/// and the place is found in the text.
fn position_of(path: &Path, line: u64, column: u64) -> Result<(u64, u64), String> {
    let text = crate::tree::read_rust(path, crate::tree::Subject::Everything)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let text = text.replace("\r\n", "\n");
    for (index, text_line) in text.split('\n').enumerate() {
        let current = index as u64 + 1;
        if current == line {
            let characters: Vec<char> = text_line.chars().collect();
            let count = characters.len() as u64;
            if column == 0 || column > count {
                return Err(format!(
                    "the column {column} is not in the line {line}: the line has {count} character(s)"
                ));
            }
            let units: u64 = characters[..(column - 1) as usize].iter().map(|c| c.len_utf16() as u64).sum();
            return Ok((line - 1, units));
        }
    }
    Err(format!("the file has no line {line}"))
}

/// The **URI** of a file, as the protocol names it.
fn uri_of(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    format!("file://{text}")
}

/// The **server**: the process that answers.
struct Session {
    child: Child,
    reader: BufReader<ChildStdout>,
    next_id: i64,
    /// The **watchdog**: the thread that stops a server that does not
    /// answer. It is told when the answer has come.
    watchdog: Option<(std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>)>,
}

impl Session {
    /// Start the server on the tree.
    fn start(root: &Path) -> Result<Session, String> {
        let program = the_program()?;
        eprintln!("the language server is loading the tree; the first load takes a minute");
        let mut child = Command::new(&program)
            .current_dir(root)
            .env("CARGO_NET_OFFLINE", "true")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("the language server could not be started: {error}"))?;
        let Some(stdout) = child.stdout.take() else {
            return Err("the language server has no output".to_string());
        };
        let pid = child.id();
        let mut session = Session { child, reader: BufReader::new(stdout), next_id: 1, watchdog: None };
        session.watchdog = Some(session.watchdog_for(pid));
        session.initialize(root)?;
        session.wait_until_ready()?;
        Ok(session)
    }

    /// Wait until the server has **loaded the tree**.
    ///
    /// The server answers a question before it has loaded the tree, and
    /// its answer is then empty: a symbol search finds nothing in a
    /// tree the server has not read. So the client asks it a question
    /// every tree answers — a search for the functions of the tree — and
    /// waits until the answer is not empty.
    fn wait_until_ready(&mut self) -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(TIMEOUT);
        loop {
            let answer = self.request("workspace/symbol", json!({ "query": "fn" }))?;
            if answer.as_array().is_some_and(|list| !list.is_empty()) {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(format!("the language server did not load the tree in {TIMEOUT} second(s)"));
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    /// The **watchdog**: a thread that stops a server that does not
    /// answer in time.
    ///
    /// The server is stopped by its number, with the `kill` of the
    /// machine — a thread that owns the child cannot wait on it and
    /// answer questions at the same time.
    fn watchdog_for(&self, pid: u32) -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            if receiver.recv_timeout(std::time::Duration::from_secs(TIMEOUT)).is_err() {
                let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
            }
        });
        (sender, handle)
    }

    /// Say hello, and tell the server what the tree is.
    fn initialize(&mut self, root: &Path) -> Result<(), String> {
        let answer = self.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": uri_of(root),
                "workspaceFolders": [{ "uri": uri_of(root), "name": "craftkit" }],
                "capabilities": {
                    "textDocument": {
                        "definition": { "linkSupport": true },
                        "references": {},
                        "hover": { "contentFormat": ["markdown", "plaintext"] },
                        "diagnostic": { "dynamicRegistration": false },
                    },
                    "workspace": { "symbol": {}, "configuration": true },
                },
                "clientInfo": { "name": "craftkit-xtask", "version": env!("CARGO_PKG_VERSION") },
            }),
        )?;
        if answer.is_null() {
            return Err("the language server did not answer the greeting".to_string());
        }
        self.notify("initialized", json!({}))?;
        Ok(())
    }

    /// Ask the server something, and wait for the answer.
    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let Some(message) = self.read()? else {
                return Err(format!("the language server closed before answering `{method}`"));
            };
            // The server asks things too: a setting, a registration. The
            // answers here are the ones every server accepts, and the
            // question of the client is not one of them.
            if let Some(their_id) = message.get("id").cloned() {
                if let Some(method) = message.get("method").and_then(Value::as_str) {
                    let _ = self.answer(their_id, answer_to(method, &message));
                    continue;
                }
                if their_id.as_i64() == Some(id) {
                    if let Some(error) = message.get("error") {
                        let text =
                            error.get("message").and_then(Value::as_str).unwrap_or("the server refused");
                        return Err(format!("the language server refused `{method}`: {text}"));
                    }
                    return Ok(message.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }

    /// Tell the server something, and do not wait.
    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    /// Answer a question of the server.
    fn answer(&mut self, id: Value, result: Value) -> Result<(), String> {
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }

    /// Write one message to the server.
    fn send(&mut self, message: &Value) -> Result<(), String> {
        let body =
            serde_json::to_string(message).map_err(|error| format!("the message is not JSON: {error}"))?;
        let stdin = self.child.stdin.as_mut().ok_or("the language server has no input")?;
        stdin
            .write_all(frame(&body).as_slice())
            .map_err(|error| format!("the message could not be written: {error}"))?;
        stdin.flush().map_err(|error| format!("the message could not be flushed: {error}"))
    }

    /// Read one message from the server.
    fn read(&mut self) -> Result<Option<Value>, String> {
        let Some(body) = read_frame(&mut self.reader)? else { return Ok(None) };
        serde_json::from_str(&body).map(Some).map_err(|error| format!("the answer is not JSON: {error}"))
    }

    /// Shut the server down.
    fn finish(&mut self) -> Result<(), String> {
        if let Some((sender, handle)) = self.watchdog.take() {
            // The answer has come: the watchdog has nothing to do.
            let _ = sender.send(());
            let _ = handle.join();
        }
        let _ = self.notify("shutdown", json!({}));
        let _ = self.notify("exit", json!({}));
        let _ = self.child.kill();
        let _ = self.child.wait();
        Ok(())
    }
}

/// The program that is the server.
///
/// The **toolchain** of the tree holds it — the tree pins its
/// toolchain, and the server is a component of it — and the path is
/// found through `rustup`, so that the server of the toolchain of the
/// tree is the server that answers.
fn the_program() -> Result<PathBuf, String> {
    let output = Command::new("rustup")
        .args(["which", SERVER])
        .output()
        .map_err(|error| format!("`rustup` could not be run: {error}"))?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    Err(format!("the language server is not installed: `rustup component add {SERVER}`"))
}

/// A message, **framed**: the length of it, and then the message.
fn frame(body: &str) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// Read one **framed** message from a reader.
///
/// `None` when the reader is at its end: the server has closed its
/// output, and there is nothing more to read.
fn read_frame<R: BufRead>(reader: &mut R) -> Result<Option<String>, String> {
    let mut length: Option<usize> = None;
    let mut header = Vec::new();
    loop {
        let mut line = Vec::new();
        let read = read_line(reader, &mut line)?;
        if read == 0 {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&line).to_string();
        let text = text.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            break;
        }
        header.push(text.to_string());
    }
    for line in header {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            length = rest.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = length else {
        return Err("the server sent a message with no length".to_string());
    };
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).map_err(|error| format!("the message could not be read: {error}"))?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

/// Read one line into a buffer, and say how many bytes were read.
fn read_line<R: BufRead>(reader: &mut R, out: &mut Vec<u8>) -> Result<usize, String> {
    reader.read_until(b'\n', out).map_err(|error| format!("the message could not be read: {error}"))
}

/// The answer to a question of the **server**.
///
/// The server asks the client two things that matter: the **settings**
/// it should use, and the **registrations** it may make. The settings
/// of this tree are the server's own file — `.rust-analyzer.toml` — so
/// the answer is "nothing", and the server reads its own file. Every
/// other question is answered with nothing, which every server takes
/// for "yes".
fn answer_to(method: &str, message: &Value) -> Value {
    match method {
        "workspace/configuration" => {
            let count = message.pointer("/params/items").and_then(Value::as_array).map(Vec::len).unwrap_or(0);
            Value::Array(vec![Value::Null; count])
        }
        _ => Value::Null,
    }
}

/// The **places** of an answer: where the thing is, or where it is
/// mentioned.
fn print_places(answer: &Value) {
    let places = match answer {
        Value::Null => Vec::new(),
        Value::Array(list) => list.clone(),
        other => vec![other.clone()],
    };
    if places.is_empty() {
        println!("nothing here");
        return;
    }
    for place in places {
        let uri = place.get("targetUri").or_else(|| place.get("uri"));
        let Some(uri) = uri.and_then(Value::as_str) else { continue };
        let line = place.pointer("/targetRange/start/line").or_else(|| place.pointer("/range/start/line"));
        let column =
            place.pointer("/targetRange/start/character").or_else(|| place.pointer("/range/start/character"));
        println!(
            "{}:{}:{}",
            uri.trim_start_matches("file://"),
            line.and_then(Value::as_u64).map(|line| line + 1).unwrap_or(0),
            column.and_then(Value::as_u64).map(|column| column + 1).unwrap_or(0),
        );
    }
}

/// The **text** of an answer about a thing.
fn print_hover(answer: &Value) {
    let text = answer
        .pointer("/contents/value")
        .and_then(Value::as_str)
        .or_else(|| answer.get("contents").and_then(Value::as_str))
        .unwrap_or("");
    if text.trim().is_empty() {
        println!("nothing here");
        return;
    }
    println!("{}", text.trim());
}

/// The **symbols** of an answer: what is called that.
fn print_symbols(answer: &Value) {
    let Some(list) = answer.as_array() else {
        println!("nothing by that name");
        return;
    };
    if list.is_empty() {
        println!("nothing by that name");
        return;
    }
    for symbol in list {
        let name = symbol.get("name").and_then(Value::as_str).unwrap_or("");
        let kind = symbol.get("kind").and_then(Value::as_i64).unwrap_or(0);
        let uri = symbol.pointer("/location/uri").and_then(Value::as_str).unwrap_or("");
        let line = symbol.pointer("/location/range/start/line").and_then(Value::as_u64).unwrap_or(0);
        println!("{} [{}] {}:{}", name, kind, uri.trim_start_matches("file://"), line + 1);
    }
}

/// The **diagnostics** of an answer: what the compiler says.
fn print_diagnostics(answer: &Value) {
    let items = answer.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    if items.is_empty() {
        println!("no complaints");
        return;
    }
    for item in items {
        let message = item.get("message").and_then(Value::as_str).unwrap_or("");
        let severity = item.get("severity").and_then(Value::as_i64).unwrap_or(0);
        let name = match severity {
            1 => "error",
            2 => "warning",
            3 => "information",
            4 => "hint",
            _ => "note",
        };
        let line = item.pointer("/range/start/line").and_then(Value::as_u64).unwrap_or(0);
        let column = item.pointer("/range/start/character").and_then(Value::as_u64).unwrap_or(0);
        println!("{name}: {message} (line {}, column {})", line + 1, column + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_framed() {
        let framed = frame("{\"a\":1}");
        let text = String::from_utf8_lossy(&framed).to_string();
        assert!(text.starts_with("Content-Length: 7\r\n\r\n"), "{text}");
        assert!(text.ends_with("{\"a\":1}"), "{text}");
    }

    #[test]
    fn a_framed_message_is_read() {
        let framed = frame("{\"a\":1}");
        let mut bytes = framed.as_slice();
        let read = read_frame(&mut bytes).expect("the message is read");
        assert_eq!(read.as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn the_end_of_the_output_is_nothing() {
        let mut bytes: &[u8] = b"";
        assert!(read_frame(&mut bytes).expect("the end is read").is_none());
    }

    #[test]
    fn a_place_is_counted_in_characters() {
        // A line with a character outside the basic plane: the protocol
        // counts it as two units, and the command counts it as one
        // character.
        let folder = crate::temp::dir("lsp").expect("the folder is made");
        let file = folder.join("place.rs");
        std::fs::write(&file, "let a = \"\u{1F600}\";\nlet b = 1;\n").expect("the file is written");
        let (line, column) = position_of(&file, 1, 10).expect("the place is found");
        assert_eq!(line, 0, "the line is counted from zero");
        assert_eq!(column, 9, "and the column is in UTF-16 units");
        // After the character outside the basic plane: it is one
        // character and two units.
        let (line, column) = position_of(&file, 1, 11).expect("the place is found");
        assert_eq!(line, 0, "the line is counted from zero");
        assert_eq!(column, 11, "and the character is two units");
    }

    #[test]
    fn a_place_outside_the_file_is_refused() {
        let folder = crate::temp::dir("lsp").expect("the folder is made");
        let file = folder.join("small.rs");
        std::fs::write(&file, "let a = 1;\n").expect("the file is written");
        assert!(position_of(&file, 9, 1).is_err(), "a line that is not there is refused");
        assert!(position_of(&file, 1, 99).is_err(), "a column that is not there is refused");
    }
}
