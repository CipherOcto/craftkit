//! The **language server**: `rust-analyzer`, asked about the tree.
//!
//! The tree is a Rust tree, and a Rust tree has a language server: a
//! program that knows what every name means, where every item was declared,
//! and what the compiler thinks of every line. An agent that greps for a
//! name is asking a question the server has already answered — and a grep
//! cannot tell a *declaration* from a *use*.
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
//! # The two doors
//!
//! The **survey** (`docs/survey-lsp.md`) measured what a question used to
//! cost: the server was loaded **again for every question**, ten to thirty
//! seconds and a whole round trip, against two hundredths of a second for
//! a grep. Two doors close that gap, and they are the same idea twice:
//!
//! - **a batch**: many questions in **one** load of the tree. The questions
//!   are written one after another, and the tree is read once.
//! - **a server that stays**: the tree is loaded once, and the server keeps
//!   the picture of it while it is being asked. The next question is
//!   answered in **milliseconds**, and the server stops itself when it is
//!   left alone.
//!
//! # The rules
//!
//! - A question about a **place** takes a file, a line and a column,
//!   counted from **one**, and the column is counted in
//!   **characters** — not bytes, and not UTF-16 units: the file is
//!   read, and the position is found in the text as a person counts
//!   it.
//! - A file is named by its path **in the tree**: `crates/craftkit-xlsx/src/lib.rs`.
//! - The **first load is slow**: the server reads the whole tree and
//!   builds its picture of it. The command says so, so that a reader does
//!   not think it has hung.
//! - A server that is **not there** is an error with the command that
//!   installs it, and not a crash: `rustup component add rust-analyzer`.
//! - The server that **stays** is a **unix** thing: it listens on a socket,
//!   and a machine without a unix socket is told so, and not given a
//!   half-answer.
//!
//! # What this does not do
//!
//! - It does not **check** the tree: the gate does that, and the gate
//!   is `cargo xtask ci`.
//! - It does not **edit** anything: the server is asked, and the answer
//!   is printed.
//! - It does not **keep** anything: the server that stays holds the tree
//!   in memory, and it lets go of it when it is left alone.

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

/// The queries that ask about a **place**: a file, a line and a column.
const PLACE_QUERIES: [&str; 3] = ["definition", "references", "hover"];

/// How long the server that **stays** waits before it stops itself, in
/// seconds.
const DEFAULT_IDLE: u64 = 600;

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let Some(query) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    match query {
        "batch" => return the_batch(root, &args[1..]),
        "serve" => return the_server(root, &args[1..]),
        "stop" => return the_stop(root),
        _ => {}
    }
    if !QUERIES.contains(&query) {
        return Err(usage());
    }
    let request = the_request(root, query, &args[1..])?;
    let answers = ask_many(root, std::slice::from_ref(&request))?;
    let Some(answer) = answers.into_iter().next() else {
        return Err("the server gave no answer".to_string());
    };
    print_answer(query, &answer);
    Ok(crate::Outcome::Ran)
}

/// The **usage** of this command.
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
         The batch, and the server that stays:\n  \
         batch <question>...                  many questions in one load of the tree\n  \
         serve [--idle-seconds <n>]           the server that stays; it stops itself when alone\n  \
         stop                                  stop the server that stays\n\n\
         A file is a path in the tree; a line and a column are counted from one.\n\
         The server is `{SERVER}`; it is installed with `rustup component add {SERVER}`."
    )
}

/// The **questions** of a batch, written one after another.
///
/// A question is named by its first word, and it takes as many arguments as
/// it needs: three for a place, one for a file or a text. The words are
/// read off the list, so a path with a colon in it is still a path.
fn the_questions(args: &[String]) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut questions = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let query = args[index].clone();
        if !QUERIES.contains(&query.as_str()) {
            return Err(format!("`{query}` is not a question the server answers"));
        }
        let arity = if PLACE_QUERIES.contains(&query.as_str()) { 3 } else { 1 };
        let end = index + 1 + arity;
        if end > args.len() {
            return Err(format!("the question `{query}` needs {arity} argument(s)"));
        }
        questions.push((query, args[index + 1..end].to_vec()));
        index = end;
    }
    Ok(questions)
}

/// The **request** a question is: the method of the protocol, and its
/// parameters.
///
/// Every question is turned into a request **here**, so that the batch, the
/// one-shot answer and the server that stays all ask the server the same
/// question, and a bad place is refused before anything is loaded.
fn the_request(root: &Path, query: &str, args: &[String]) -> Result<(String, Value), String> {
    match query {
        "symbols" => {
            let Some(text) = args.first() else { return Err(usage()) };
            let text = text.trim();
            if text.is_empty() {
                return Err("a symbol search needs something to search for".to_string());
            }
            Ok(("workspace/symbol".to_string(), json!({ "query": text })))
        }
        "diagnostics" => {
            let Some(file) = args.first() else { return Err(usage()) };
            let path = the_file(root, file)?;
            Ok(("textDocument/diagnostic".to_string(), json!({ "textDocument": { "uri": uri_of(&path) } })))
        }
        "definition" | "references" | "hover" => {
            let (Some(file), Some(line), Some(column)) = (args.first(), args.get(1), args.get(2)) else {
                return Err(usage());
            };
            let path = the_file(root, file)?;
            let line = number_of(line, "a line")?;
            let column = number_of(column, "a column")?;
            let (line, column) = position_of(&path, line, column)?;
            let method = match query {
                "definition" => "textDocument/definition",
                "references" => "textDocument/references",
                _ => "textDocument/hover",
            };
            let mut params = json!({ "textDocument": { "uri": uri_of(&path) }, "position": { "line": line, "character": column } });
            if query == "references" {
                params["context"] = json!({ "includeDeclaration": true });
            }
            Ok((method.to_string(), params))
        }
        _ => Err(usage()),
    }
}

/// The **batch**: many questions, one load of the tree.
fn the_batch(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let questions = the_questions(args)?;
    if questions.is_empty() {
        return Err("a batch needs at least one question".to_string());
    }
    let mut requests = Vec::new();
    let mut labels = Vec::new();
    for (query, question_args) in &questions {
        requests.push(the_request(root, query, question_args)?);
        labels.push((query.clone(), the_label(query, question_args)));
    }
    let answers = ask_many(root, &requests)?;
    for ((query, label), answer) in labels.iter().zip(answers.iter()) {
        println!("=== {label}");
        print_answer(query, answer);
    }
    Ok(crate::Outcome::Ran)
}

/// The **label** of a question: what it says, so that the answers of a
/// batch can be read one under another.
fn the_label(query: &str, args: &[String]) -> String {
    match query {
        "symbols" => format!("symbols {}", args.first().map(String::as_str).unwrap_or_default()),
        "diagnostics" => format!("diagnostics {}", args.first().map(String::as_str).unwrap_or_default()),
        _ => format!(
            "{query} {} {} {}",
            args.first().map(String::as_str).unwrap_or_default(),
            args.get(1).map(String::as_str).unwrap_or_default(),
            args.get(2).map(String::as_str).unwrap_or_default()
        ),
    }
}

/// Ask the questions, and get the answers.
///
/// The server that **stays** is asked first, if there is one: it holds the
/// picture of the tree already, and the answer comes in milliseconds. If
/// there is none, one is started, and if that does not work the questions
/// are asked of a server of this command's own — which is the old way, and
/// the slow one, and the answer is the same.
fn ask_many(root: &Path, requests: &[(String, Value)]) -> Result<Vec<Value>, String> {
    #[cfg(unix)]
    {
        if let Some(answers) = daemon::ask(root, requests)? {
            return Ok(answers);
        }
    }
    let mut session = Session::start(root)?;
    let mut answers = Vec::new();
    for (method, params) in requests {
        answers.push(session.ask(method, params.clone())?);
    }
    session.finish()?;
    Ok(answers)
}

/// The **file** a name says, and the path it is at.
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

/// Something that can **answer** a question of the server.
///
/// The server of the tree answers for real; the tests answer with a
/// table, so that the shape of the door can be tested without a server.
trait Asker {
    /// Ask the server something, and wait for the answer.
    fn ask(&mut self, method: &str, params: Value) -> Result<Value, String>;
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

impl Asker for Session {
    fn ask(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.request(method, params)
    }
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

/// Print the **answer** of a question.
fn print_answer(query: &str, answer: &Value) {
    match query {
        "hover" => print_hover(answer),
        "diagnostics" => print_diagnostics(answer),
        "symbols" => print_symbols(answer),
        _ => print_places(answer),
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

/// The **hover** of an answer: what the thing is.
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

/// The **complaints** of an answer: what the compiler says about a file.
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

/// The **server that stays**: the tree is loaded once, and the picture of
/// it is kept while the server is asked.
///
/// It is a unix thing: the server listens on a socket in the tree's own
/// build directory, and a machine without a unix socket is told so.
#[cfg(unix)]
mod daemon {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};

    use super::{Asker, DEFAULT_IDLE, TIMEOUT};

    /// The **socket** the server listens on: in the tree's own build
    /// directory, which nobody commits and everybody cleans.
    pub fn socket_path(root: &Path) -> PathBuf {
        root.join("target").join("xtask-lsp.sock")
    }

    /// The **log** of the server: what it did, and what it said.
    fn log_path(root: &Path) -> PathBuf {
        root.join("target").join("xtask-lsp.log")
    }

    /// Ask the questions **through the server that stays**.
    ///
    /// `None` when there is no server and none could be started: the
    /// caller answers the questions itself, the old way, and the answer
    /// is the same.
    pub fn ask(root: &Path, requests: &[(String, Value)]) -> Result<Option<Vec<Value>>, String> {
        let path = socket_path(root);
        if let Some(answers) = ask_the_one_there(&path, requests)? {
            return Ok(Some(answers));
        }
        let mut server = start(root)?;
        let deadline = Instant::now() + Duration::from_secs(TIMEOUT);
        while Instant::now() < deadline {
            // The server that was started has died: the questions are
            // answered here, the old way, and the answer is the same.
            if server
                .try_wait()
                .map_err(|error| format!("the server could not be watched: {error}"))?
                .is_some()
            {
                return Ok(None);
            }
            if let Some(answers) = ask_the_one_there(&path, requests)? {
                return Ok(Some(answers));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        Ok(None)
    }

    /// Ask the server that is **already there**, if there is one.
    fn ask_the_one_there(path: &Path, requests: &[(String, Value)]) -> Result<Option<Vec<Value>>, String> {
        let Ok(mut stream) = UnixStream::connect(path) else { return Ok(None) };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(TIMEOUT)));
        match exchange(&mut stream, requests) {
            Ok(answers) => Ok(Some(answers)),
            // The server went away between the connection and the answer:
            // the caller answers the questions itself.
            Err(_) => Ok(None),
        }
    }

    /// Send the questions, and read the answers: one **line** each.
    pub(super) fn exchange(
        stream: &mut UnixStream,
        requests: &[(String, Value)],
    ) -> Result<Vec<Value>, String> {
        let mut writer = stream.try_clone().map_err(|error| format!("the connection was lost: {error}"))?;
        let mut reader = BufReader::new(stream);
        let mut answers = Vec::new();
        for (method, params) in requests {
            let line = serde_json::to_string(&json!({ "method": method, "params": params }))
                .map_err(|error| format!("the question is not JSON: {error}"))?;
            writer
                .write_all(line.as_bytes())
                .map_err(|error| format!("the question could not be sent: {error}"))?;
            writer.write_all(b"\n").map_err(|error| format!("the question could not be sent: {error}"))?;
            writer.flush().map_err(|error| format!("the question could not be sent: {error}"))?;
            let mut response = String::new();
            let read = reader
                .read_line(&mut response)
                .map_err(|error| format!("the answer could not be read: {error}"))?;
            if read == 0 {
                return Err("the server closed before answering".to_string());
            }
            let answer: Value = serde_json::from_str(response.trim())
                .map_err(|error| format!("the answer is not JSON: {error}"))?;
            if let Some(error) = answer.get("error").and_then(Value::as_str) {
                return Err(error.to_string());
            }
            answers.push(answer.get("answer").cloned().unwrap_or(Value::Null));
        }
        Ok(answers)
    }

    /// Start the server that stays, **detached**: it must outlive the
    /// command that started it.
    fn start(root: &Path) -> Result<std::process::Child, String> {
        let program =
            std::env::current_exe().map_err(|error| format!("the xtask could not be found: {error}"))?;
        if let Some(parent) = log_path(root).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path(root))
            .map_err(|error| format!("the log of the server could not be opened: {error}"))?;
        let errors =
            log.try_clone().map_err(|error| format!("the log of the server could not be copied: {error}"))?;
        // A session of its own, where there is one: the server must not
        // die with the shell that started it.
        let mut command = if setsid_is_there() {
            let mut command = Command::new("setsid");
            command.arg(&program);
            command
        } else {
            Command::new(&program)
        };
        command
            .args(["lsp", "serve", "--idle-seconds", &DEFAULT_IDLE.to_string()])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors))
            .spawn()
            .map_err(|error| format!("the server could not be started: {error}"))
    }

    /// Whether this machine has the command that gives a process a
    /// session of its own.
    fn setsid_is_there() -> bool {
        let Some(path) = std::env::var_os("PATH") else { return false };
        std::env::split_paths(&path).any(|directory| directory.join("setsid").is_file())
    }

    /// **Run** the server that stays: it loads the tree once, and it is
    /// asked until it is left alone.
    pub fn serve(root: &Path, idle_seconds: u64) -> Result<(), String> {
        let path = socket_path(root);
        // Another server may already be serving: then this one has
        // nothing to do.
        if path.exists() {
            if UnixStream::connect(&path).is_ok() {
                return Ok(());
            }
            // A socket with nobody behind it: the server that died left
            // it behind.
            let _ = std::fs::remove_file(&path);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("the build directory could not be made: {error}"))?;
        }
        let listener = UnixListener::bind(&path)
            .map_err(|error| format!("the server could not listen on {}: {error}", path.display()))?;
        println!("the server is listening on {}", path.display());
        let outcome = serve_with(root, &listener, idle_seconds);
        let _ = std::fs::remove_file(&path);
        outcome
    }

    /// The **loop** of the server that stays.
    fn serve_with(root: &Path, listener: &UnixListener, idle_seconds: u64) -> Result<(), String> {
        let mut session = super::Session::start(root)?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("the server could not wait for a question: {error}"))?;
        let mut last = Instant::now();
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    last = Instant::now();
                    if serve_one(&mut session, stream)? {
                        return Ok(());
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                    if last.elapsed() > Duration::from_secs(idle_seconds) {
                        return Ok(());
                    }
                }
                Err(error) => return Err(format!("the server could not accept a question: {error}")),
            }
        }
    }

    /// Answer the questions of **one** connection.
    ///
    /// `true` when the connection asked the server to stop.
    pub(super) fn serve_one(session: &mut dyn Asker, stream: UnixStream) -> Result<bool, String> {
        let mut writer = stream.try_clone().map_err(|error| format!("the connection was lost: {error}"))?;
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let line = line.map_err(|error| format!("the question could not be read: {error}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let request: Value = serde_json::from_str(line.trim())
                .map_err(|error| format!("the question is not JSON: {error}"))?;
            if request.get("stop").is_some() {
                let _ = writeln!(writer, "{}", json!({ "answer": "stopped" }));
                let _ = writer.flush();
                return Ok(true);
            }
            let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            let response = match session.ask(method, params) {
                Ok(answer) => json!({ "answer": answer }),
                Err(error) => json!({ "error": error }),
            };
            writeln!(writer, "{response}")
                .map_err(|error| format!("the answer could not be sent: {error}"))?;
            writer.flush().map_err(|error| format!("the answer could not be sent: {error}"))?;
        }
        Ok(false)
    }

    /// **Stop** the server that stays.
    pub fn stop(root: &Path) -> Result<bool, String> {
        let path = socket_path(root);
        let Ok(mut stream) = UnixStream::connect(&path) else { return Ok(false) };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        writeln!(stream, "{}", json!({ "stop": true }))
            .map_err(|error| format!("the server could not be stopped: {error}"))?;
        stream.flush().map_err(|error| format!("the server could not be stopped: {error}"))?;
        let mut answer = String::new();
        let _ = BufReader::new(stream).read_line(&mut answer);
        Ok(true)
    }
}

/// The **server that stays**, on a machine without a unix socket.
///
/// The tree is read once per batch there, which is the old way, and the
/// answer is the same.
#[cfg(not(unix))]
fn the_server(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    Err("not supported yet: the server that stays needs a unix socket, and this machine has none".to_string())
}

/// The **server that stays**, on a machine with a unix socket.
#[cfg(unix)]
fn the_server(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let idle = the_idle(args)?;
    daemon::serve(root, idle)?;
    Ok(crate::Outcome::Ran)
}

/// How long the server waits before it stops itself.
#[cfg(unix)]
fn the_idle(args: &[String]) -> Result<u64, String> {
    let mut idle = DEFAULT_IDLE;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--idle-seconds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--idle-seconds needs a number".to_string());
                };
                idle = number_of(value, "the idle time")?;
                index += 2;
            }
            other => return Err(format!("`{other}` is not a flag of the server")),
        }
    }
    Ok(idle)
}

/// **Stop** the server that stays.
#[cfg(unix)]
fn the_stop(root: &Path) -> Result<crate::Outcome, String> {
    if daemon::stop(root)? {
        println!("the server is stopping");
    } else {
        println!("there was no server");
    }
    Ok(crate::Outcome::Ran)
}

/// **Stop** the server that stays, on a machine without a unix socket.
#[cfg(not(unix))]
fn the_stop(_root: &Path) -> Result<crate::Outcome, String> {
    Err("not supported yet: the server that stays needs a unix socket, and this machine has none".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree of its own, with a file in it.
    fn a_tree() -> PathBuf {
        let directory = crate::temp::dir("lsp").expect("the folder is there");
        std::fs::create_dir_all(directory.join("src")).expect("the folder is there");
        std::fs::write(directory.join("src/lib.rs"), "fn one() {}\nfn two() {}\n")
            .expect("the file is written");
        directory
    }

    /// The arguments of a question.
    fn args_of(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| part.to_string()).collect()
    }

    #[test]
    fn a_place_becomes_a_request() {
        let root = a_tree();
        let (method, params) =
            the_request(&root, "hover", &args_of(&["src/lib.rs", "1", "3"])).expect("the request");
        assert_eq!(method, "textDocument/hover");
        assert_eq!(params.pointer("/position/line").and_then(Value::as_u64), Some(0));
        assert_eq!(params.pointer("/position/character").and_then(Value::as_u64), Some(2));
    }

    #[test]
    fn a_reference_asks_for_the_declaration_too() {
        let root = a_tree();
        let (_, params) =
            the_request(&root, "references", &args_of(&["src/lib.rs", "1", "3"])).expect("the request");
        assert_eq!(params.pointer("/context/includeDeclaration").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn a_place_outside_the_file_is_refused() {
        let root = a_tree();
        let error =
            the_request(&root, "hover", &args_of(&["src/lib.rs", "1", "99"])).expect_err("it is refused");
        assert!(error.contains("not in the line"), "{error}");
    }

    #[test]
    fn a_file_outside_the_tree_is_refused() {
        let root = a_tree();
        let error = the_request(&root, "diagnostics", &args_of(&["Cargo.toml"])).expect_err("it is refused");
        assert!(error.contains("not a file of the tree"), "{error}");
    }

    #[test]
    fn a_batch_is_read_by_the_arity_of_its_questions() {
        let questions = the_questions(&args_of(&[
            "hover",
            "src/lib.rs",
            "1",
            "3",
            "diagnostics",
            "src/lib.rs",
            "symbols",
            "two",
        ]))
        .expect("the questions");
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0].0, "hover");
        assert_eq!(questions[0].1.len(), 3);
        assert_eq!(questions[1].0, "diagnostics");
        assert_eq!(questions[1].1.len(), 1);
        assert_eq!(questions[2].0, "symbols");
        assert_eq!(questions[2].1, vec!["two".to_string()]);
    }

    #[test]
    fn a_question_that_is_not_a_question_is_refused() {
        let error = the_questions(&args_of(&["nonsense", "x"])).expect_err("it is refused");
        assert!(error.contains("not a question"), "{error}");
    }

    #[test]
    fn a_question_without_its_arguments_is_refused() {
        let error = the_questions(&args_of(&["hover", "src/lib.rs"])).expect_err("it is refused");
        assert!(error.contains("needs 3 argument"), "{error}");
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use serde_json::{Value, json};
        use std::os::unix::net::UnixStream;

        /// An asker that answers from a **table**, so that the door can
        /// be tested without a language server.
        struct Table {
            answers: Vec<(String, Value)>,
        }

        impl Asker for Table {
            fn ask(&mut self, method: &str, _params: Value) -> Result<Value, String> {
                for (name, answer) in &self.answers {
                    if name == method {
                        return Ok(answer.clone());
                    }
                }
                Err(format!("the table has no answer for `{method}`"))
            }
        }

        #[test]
        fn the_socket_is_in_the_build_directory_of_the_tree() {
            let root = a_tree();
            let path = daemon::socket_path(&root);
            assert_eq!(path, root.join("target/xtask-lsp.sock"));
        }

        #[test]
        fn a_whole_exchange_answers_every_question() {
            let (mut mine, theirs) = UnixStream::pair().expect("the pair is there");
            let table =
                Table { answers: vec![("textDocument/hover".to_string(), json!({ "contents": "a type" }))] };
            let handle = std::thread::spawn(move || {
                let mut server = table;
                daemon::serve_one(&mut server, theirs).expect("the connection is answered")
            });
            let requests = vec![
                ("textDocument/hover".to_string(), json!({ "position": { "line": 1, "character": 2 } })),
                ("textDocument/hover".to_string(), json!({ "position": { "line": 3, "character": 4 } })),
            ];
            let answers = daemon::exchange(&mut mine, &requests).expect("the answers");
            assert_eq!(answers.len(), 2);
            assert_eq!(answers[0].pointer("/contents").and_then(Value::as_str), Some("a type"));
            // The connection is closed before the thread is joined: the
            // server reads until the other end is gone.
            drop(mine);
            assert!(!handle.join().expect("the thread"));
        }

        #[test]
        fn a_question_the_server_refuses_is_an_error() {
            let (mut mine, theirs) = UnixStream::pair().expect("the pair is there");
            let table = Table { answers: Vec::new() };
            let handle = std::thread::spawn(move || {
                let mut server = table;
                daemon::serve_one(&mut server, theirs).expect("the connection is answered")
            });
            let requests = vec![("textDocument/hover".to_string(), json!({}))];
            let error = daemon::exchange(&mut mine, &requests).expect_err("it is refused");
            assert!(error.contains("no answer for"), "{error}");
            drop(mine);
            let _ = handle.join();
        }
    }
}
