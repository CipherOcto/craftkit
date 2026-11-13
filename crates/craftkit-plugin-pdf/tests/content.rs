//! The **content stream**: what a page says to draw.
//!
//! A page's content is a stream of operands and operators: `10 20 m`
//! moves the pen, `100 100 200 200 re` draws a rectangle, and `f`
//! fills what the pen has drawn. The command reads the stream and says
//! what it says, in the order the stream says it.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-content-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stream** node whose bytes are in a file, and the id.
fn a_stream(session: &mut Session, directory: &str, name: &str, bytes: &[u8]) -> u64 {
    let path = format!("{directory}/{name}");
    std::fs::write(&path, bytes).expect("the bytes are written");
    // The number is **free**: one number is one object.
    let number = a_free_number(session);
    session
        .execute(
            "pdf.stream.add",
            &json!({ "object": number, "entries": [["Type", "XObject"]], "stream": path }),
        )
        .expect("the stream is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .map(|node| node.id)
        .expect("the stream is there")
}

/// The **operations** a stream holds.
fn operations_of(session: &mut Session, stream: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("pdf.content.operators", &json!({ "stream": stream })).expect("the content is read");
    outcome.data["operations"].as_array().expect("the operations are a list").clone()
}

/// A stream's **operators** are read, with the operands each one was
/// given.
/// A **free** object number: one number is one object.
fn a_free_number(session: &Session) -> u64 {
    session.document().walk().filter(|node| node.kind == "pdf.stream" || node.kind == "pdf.object").count()
        as u64
        + 1
}

#[test]
fn the_operators_of_a_stream_are_read() {
    let directory = directory("operators");
    let mut session = session();
    let stream = a_stream(&mut session, &directory, "content.txt", b"10 20 m 100 100 200 200 re f");
    let operations = operations_of(&mut session, stream);
    assert_eq!(operations.len(), 3, "the stream has three operations");
    assert_eq!(operations[0]["operator"], json!("m"), "the pen moves");
    assert_eq!(operations[0]["operands"], json!([10.0, 20.0]), "with the point it moves to");
    assert_eq!(operations[1]["operator"], json!("re"), "and a rectangle is drawn");
    assert_eq!(operations[1]["operands"], json!([100.0, 100.0, 200.0, 200.0]), "with its four numbers");
    assert_eq!(operations[2]["operator"], json!("f"), "and the path is filled");
    assert_eq!(operations[2]["operands"], json!([]), "with nothing of its own");
}

/// A **name** and a **string** are operands too, and an array is a
/// list of them.
#[test]
fn the_operands_of_an_operator_are_read() {
    let directory = directory("operands");
    let mut session = session();
    let stream = a_stream(&mut session, &directory, "content.txt", b"/F1 12 Tf [(a) -250 (b)] TJ (Hello) Tj");
    let operations = operations_of(&mut session, stream);
    assert_eq!(operations[0]["operator"], json!("Tf"), "the font is set");
    assert_eq!(operations[0]["operands"], json!(["/F1", 12.0]), "with the font and the size");
    assert_eq!(operations[1]["operator"], json!("TJ"), "the text is shown");
    assert_eq!(
        operations[1]["operands"],
        json!([[{ "string": "a" }, -250.0, { "string": "b" }]]),
        "with the array of pieces"
    );
    assert_eq!(operations[2]["operands"], json!([{ "string": "Hello" }]), "and the plain text");
}

/// A **stream** that says something the kit does not know is refused
/// by name, and so is one that is not a stream at all.
#[test]
fn a_content_stream_that_cannot_be_read_is_refused() {
    let directory = directory("refused");
    let mut session = session();
    let stream = a_stream(&mut session, &directory, "content.txt", b"10 20 zz");
    let refused = session.execute("pdf.content.operators", &json!({ "stream": stream }));
    let message = match refused {
        Ok(_) => panic!("an operator the kit does not know is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("zz"), "the refusal says why: {message}");
    // A stream that ends with operands and no operator is not a
    // content stream either.
    let stream = a_stream(&mut session, &directory, "half.txt", b"10 20");
    let refused = session.execute("pdf.content.operators", &json!({ "stream": stream }));
    assert!(refused.is_err(), "a stream that ends with operands is refused");
}
