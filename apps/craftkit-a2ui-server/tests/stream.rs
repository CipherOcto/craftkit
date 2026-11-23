//! The **door's own stream** and the **control channel**.
//!
//! These are the two doors an agent uses, and both are tested here
//! because a door that is opened and then closed is a door that is not
//! there.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use craftkit_a2ui_server::render::Headless;
use craftkit_a2ui_server::{Server, serve_with};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Point};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_a2ui::CRAFTKIT_PLUGIN_ANCHOR;

/// A door over the battery's own contributions.
fn server() -> Server {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    Server::new(ecosystem)
}

/// The queue of the messages that are not replies.
fn queue() -> std::sync::mpsc::Sender<Option<Json>> {
    let (sender, _reader) = std::sync::mpsc::channel();
    sender
}

/// A **sink** the test can read: the bytes the door wrote.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut held = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        held.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Sink {
    /// The lines the door wrote.
    fn lines(&self) -> Vec<Json> {
        let held = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        String::from_utf8_lossy(&held)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| craftkit_sdk::serde_json::from_str::<Json>(line).expect("the answer is JSON"))
            .collect()
    }
}

/// The door's own stream: a **request** in, an **answer** out.
///
/// The stream is a pipe, so the test writes the requests, closes the
/// writing end, and reads what the door said.
fn ask_the_stream(requests: &str) -> Vec<Json> {
    let server = server();
    let Ok((reader, writer)) = std::io::pipe() else {
        panic!("the pipe is made");
    };
    let reader = BufReader::new(reader);
    let sink = Sink::default();
    let requests = requests.to_string();
    let writing = std::thread::spawn(move || {
        let mut writer = writer;
        let _ = writer.write_all(requests.as_bytes());
    });
    serve_with(server, reader, sink.clone());
    let _ = writing.join();
    sink.lines()
}

/// The control channel is **open**: a client that connects is answered.
///
/// The channel's server owns the thread that listens, and it stops
/// listening when it is dropped — so the door has to **hold** it.
#[test]
fn the_control_channel_is_open() {
    let server = server();
    let render = Arc::new(Mutex::new(Headless::new(server.shared_session(), queue())));
    let options = craftkit_control::Options {
        token: "a-token".to_string(),
        address: std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
        announce: None,
        render: Some(render),
    };
    let channel = craftkit_control::Server::serve_shared(server.shared_session(), options)
        .expect("the channel is open");
    let address = channel.address();
    // The **client**: a connection with the token, and a ping.
    let mut stream = TcpStream::connect(address).expect("the channel is listening");
    let request = json!({ "id": 1, "token": "a-token", "method": "ping", "params": {} });
    let _ = writeln!(stream, "{request}");
    let _ = stream.flush();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let _ = reader.read_line(&mut line);
    let answer: Json = craftkit_sdk::serde_json::from_str(line.trim()).expect("the answer is JSON");
    assert_eq!(answer["id"], json!(1), "{answer}");
}

/// A request that is a **notification** is not answered: an answer would
/// be a reply to nothing.
#[test]
fn a_notification_is_not_answered() {
    let answers = ask_the_stream("{\"method\": \"createSurface\", \"params\": { \"surfaceId\": \"s1\" }}\n");
    assert!(answers.is_empty(), "a notification is not answered: {answers:?}");
}

/// And a request with an id **is** answered.
#[test]
fn a_request_is_answered() {
    let answers =
        ask_the_stream("{\"id\": 1, \"method\": \"createSurface\", \"params\": { \"surfaceId\": \"s1\" }}\n");
    assert_eq!(answers.len(), 1, "{answers:?}");
    assert_eq!(answers[0]["result"]["surfaceId"], json!("s1"), "{answers:?}");
}

/// A line that is over the limit is **refused**, and the door says so.
#[test]
fn an_over_long_line_is_refused() {
    let long = "x".repeat(craftkit_a2ui_server::MAX_LINE + 16);
    let answers =
        ask_the_stream(&format!("{{\"id\": 1, \"method\": \"ping\", \"params\": {{\"x\": \"{long}\"}}}}\n"));
    assert!(answers.is_empty(), "the line was refused and the door stopped: {answers:?}");
}

/// The failures are told apart: a method that is not there, a request
/// that is wrong, and a command that was refused are three things.
#[test]
fn the_failures_are_told_apart() {
    let answers = ask_the_stream(
        "{\"id\": 1, \"method\": \"nope\", \"params\": {}}\n\
         {\"id\": 2, \"method\": \"createSurface\", \"params\": {}}\n\
         {\"id\": 3, \"method\": \"createSurface\", \"params\": { \"surfaceId\": \"s1\", \"catalogId\": \"other\" }}\n",
    );
    assert_eq!(answers.len(), 3, "{answers:?}");
    assert_eq!(answers[0]["error"]["code"], json!(-32601), "no such method: {answers:?}");
    assert_eq!(answers[1]["error"]["code"], json!(-32602), "bad params: {answers:?}");
    assert_eq!(answers[2]["error"]["code"], json!(-32602), "a foreign catalogue: {answers:?}");
}

/// The catalogue file is **created**, and not written over: a path that
/// is already there is refused.
#[test]
fn the_catalogue_file_is_created_and_not_written_over() {
    let path = craftkit_scratch::file("a2ui-catalogue.json").expect("the scratch is made");
    let server = server();
    server.write_catalogue(&path).expect("the catalogue is written");
    let first = std::fs::read_to_string(&path).expect("the file is there");
    assert!(first.contains("catalogId"), "the catalogue is in the file");
    // And the second write is **refused**: the file is the door's, and
    // it does not write over something it did not make.
    let error = server.write_catalogue(&path).expect_err("the file is not written over");
    assert!(error.contains("not written"), "{error}");
}

/// The **click** on the channel is a command, and the action it raises
/// goes on the wire.
#[test]
fn a_click_on_the_channel_raises_an_action() {
    let mut server = server();
    let outbound = server.outbound();
    server.handle(&craftkit_a2ui_server::Request {
        id: Some(json!(1)),
        method: "createSurface".to_string(),
        params: json!({ "surfaceId": "s1" }),
    });
    server.handle(&craftkit_a2ui_server::Request {
        id: Some(json!(2)),
        method: "updateComponents".to_string(),
        params: json!({
            "surfaceId": "s1",
            "components": [{
                "component": "Button",
                "child": "press",
                "action": { "name": "required" },
            }],
        }),
    });
    let mut render = Headless::new(server.shared_session(), outbound);
    let answer = render.click(Point::new(1.0, 1.0)).expect("the click is answered");
    assert!(!answer["clicked"].is_null(), "the button was clicked: {answer}");
    let messages = server.take_outbound();
    assert_eq!(messages.len(), 1, "one message is not a reply: {messages:?}");
    assert_eq!(messages[0]["method"], json!("action"), "{messages:?}");
}

/// And a click on **nothing** is nothing.
#[test]
fn a_click_on_nothing_is_nothing() {
    let server = server();
    let mut render = Headless::new(server.shared_session(), queue());
    let answer = render.click(Point::new(-10.0, -10.0)).expect("the click is answered");
    assert!(answer["clicked"].is_null(), "{answer}");
}

/// The door's **own** session is the channel's: a command that is run
/// over the channel changes what the door is drawing.
#[test]
fn the_channel_serves_the_doors_own_session() {
    let mut server = server();
    server.handle(&craftkit_a2ui_server::Request {
        id: Some(json!(1)),
        method: "createSurface".to_string(),
        params: json!({ "surfaceId": "s1" }),
    });
    let mut render = Headless::new(server.shared_session(), queue());
    let picture = render.render().expect("the picture is drawn");
    assert!(picture.len() > 8, "the picture holds bytes");
    // And the **same** session: the surface is in it.
    let surfaces = server.with_session(|session| session.document().nodes().count());
    assert_eq!(surfaces, 1, "the surface is in the door's own session");
}

/// The reader of the stream is bounded: the limit is written down.
#[test]
fn the_reads_are_bounded() {
    assert_eq!(craftkit_a2ui_server::MAX_LINE, 1024 * 1024, "the limit is written down");
}
