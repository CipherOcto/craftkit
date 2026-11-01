//! The **object stream**: the objects a file keeps in one place.
//!
//! An object stream holds a file's objects so that they may be
//! compressed together. Its dictionary says how many objects it holds
//! and where the first one's data begins, and the header says where
//! each one is.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-objstm-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// An **object stream** with the bytes it holds, and the id.
fn an_object_stream(
    session: &mut Session,
    directory: &str,
    name: &str,
    bytes: &[u8],
    count: u64,
    first: u64,
) -> u64 {
    let path = format!("{directory}/{name}");
    std::fs::write(&path, bytes).expect("the bytes are written");
    session
        .execute(
            "pdf.stream.add",
            &json!({
                "object": 1,
                "entries": [["Type", "ObjStm"], ["N", count], ["First", first]],
                "stream": path,
            }),
        )
        .expect("the stream is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .map(|node| node.id)
        .expect("the stream is there")
}

/// The **objects** a stream holds.
fn objects_of(session: &mut Session, stream: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("pdf.objstm.read", &json!({ "stream": stream })).expect("the objects are read");
    outcome.data["objects"].as_array().expect("the objects are a list").clone()
}

/// An object stream's **objects** are read, each with its number.
#[test]
fn the_objects_of_an_object_stream_are_read() {
    let directory = directory("objects");
    let mut session = session();
    // The header says "object 5 is at 0 and object 6 is at 17", and the
    // first object's data begins at byte 9 — which is the header.
    let bytes = b"5 0 6 17 << /Type /Page >><< /Type /Pages >>";
    let stream = an_object_stream(&mut session, &directory, "objects.bin", bytes, 2, 9);
    let objects = objects_of(&mut session, stream);
    assert_eq!(objects.len(), 2, "the stream holds two objects");
    assert_eq!(objects[0]["object"], json!(5), "the first is object five");
    assert_eq!(objects[0]["value"], json!([["/Type", "/Page"]]), "and it says what it is");
    assert_eq!(objects[1]["object"], json!(6), "the second is object six");
    assert_eq!(objects[1]["value"], json!([["/Type", "/Pages"]]), "and it says what it is");
}

/// An object may be a **value** and not a dictionary: a number, a
/// name, a string, an array, a boolean, null, or a **reference** to
/// another object.
#[test]
fn the_values_of_an_object_are_read() {
    let directory = directory("values");
    let mut session = session();
    let bytes = b"5 0 6 4 7 10 8 33 (a) /Name [1 2 true null 11 0 R] true";
    let stream = an_object_stream(&mut session, &directory, "values.bin", bytes, 4, 18);
    let objects = objects_of(&mut session, stream);
    assert_eq!(objects[0]["value"], json!("a"), "a string is a string");
    assert_eq!(objects[1]["value"], json!("/Name"), "a name is a name");
    assert_eq!(
        objects[2]["value"],
        json!([1.0, 2.0, true, null, { "ref": [11.0, 0.0] }]),
        "an array holds five values"
    );
    assert_eq!(objects[3]["value"], json!(true), "a boolean is a boolean");
}

/// A stream that does not say what it holds is **refused**, and so is
/// one whose header does not hold the objects it says.
#[test]
fn an_object_stream_that_cannot_be_read_is_refused() {
    let directory = directory("refused");
    let mut session = session();
    // A stream with no count and no first offset.
    let path = format!("{directory}/plain.bin");
    std::fs::write(&path, b"5 0 ").expect("the bytes are written");
    session
        .execute("pdf.stream.add", &json!({ "object": 1, "entries": [["Type", "ObjStm"]], "stream": path }))
        .expect("the stream is added");
    let stream = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .map(|node| node.id)
        .expect("the stream is there");
    let refused = session.execute("pdf.objstm.read", &json!({ "stream": stream }));
    assert!(refused.is_err(), "a stream that says nothing about itself is refused");
    // A stream that says it holds four objects and holds one.
    let bytes = b"5 0 << /Type /Page >>";
    let stream = an_object_stream(&mut session, &directory, "short.bin", bytes, 4, 6);
    let refused = session.execute("pdf.objstm.read", &json!({ "stream": stream }));
    assert!(refused.is_err(), "a stream that does not hold what it says is refused");
}
