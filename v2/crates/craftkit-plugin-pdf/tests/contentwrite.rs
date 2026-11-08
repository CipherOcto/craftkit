//! The **content stream** written back.
//!
//! The content of a stream is read and written again, and the stream
//! that is written is the stream that was read.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-contentwrite-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stream** node whose bytes are in a file, and the id.
fn a_stream(session: &mut Session, directory: &str, name: &str, bytes: &[u8]) -> u64 {
    let path = format!("{directory}/{name}");
    std::fs::write(&path, bytes).expect("the bytes are written");
    let number = 1000u64 + bytes.len() as u64;
    session
        .execute(
            "pdf.stream.add",
            &json!({ "object": number, "entries": [["Type", "XObject"]], "stream": path }),
        )
        .expect("the stream is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.stream")
        .last()
        .map(|node| node.id)
        .expect("the stream is there")
}

/// The **operations** a stream holds.
fn operations_of(session: &mut Session, stream: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("pdf.content.operators", &json!({ "stream": stream })).expect("the content is read");
    outcome.data["operations"].as_array().expect("the operations are a list").clone()
}

/// The content of a stream is **written back**, and the stream that is
/// written says what the stream said.
#[test]
fn the_content_is_written_back() {
    let mut session = session();
    let directory = directory("written");
    let stream = a_stream(&mut session, &directory, "content.txt", b"10 20 m 100 100 200 200 re f");
    let outcome =
        session.execute("pdf.content.write", &json!({ "stream": stream })).expect("the content is written");
    assert_eq!(outcome.data["operations"], json!(3), "the three operations are written: {outcome:?}");
    let operations = operations_of(&mut session, stream);
    assert_eq!(operations.len(), 3, "the stream still holds three: {operations:?}");
    assert_eq!(operations[0]["operator"], json!("m"), "the first is still the move: {operations:?}");
    assert_eq!(operations[0]["operands"], json!([10.0, 20.0]), "with its point: {operations:?}");
    assert_eq!(operations[2]["operator"], json!("f"), "and the last is still the fill: {operations:?}");
}

/// A **name** is a name and a **string** is a string: the two are two
/// things, and the round trip keeps them apart.
#[test]
fn a_name_is_a_name_and_a_string_is_a_string() {
    let mut session = session();
    let directory = directory("names");
    let stream = a_stream(&mut session, &directory, "content.txt", b"/F1 12 Tf (Hello) Tj");
    session.execute("pdf.content.write", &json!({ "stream": stream })).expect("the content is written");
    let operations = operations_of(&mut session, stream);
    assert_eq!(operations[0]["operands"], json!(["/F1", 12.0]), "the name is a name: {operations:?}");
    assert_eq!(
        operations[1]["operands"],
        json!([{ "string": "Hello" }]),
        "and the string is a string: {operations:?}"
    );
}

/// The **inline image** is kept whole: the bytes of it are the bytes
/// it was.
#[test]
fn the_inline_image_is_kept_whole() {
    let mut session = session();
    let directory = directory("image");
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"BI /W 2 /H 2 /BPC 8 ID \x00\x01\x02\x03 EI\n");
    let stream = a_stream(&mut session, &directory, "content.txt", &bytes);
    session.execute("pdf.content.write", &json!({ "stream": stream })).expect("the content is written");
    let operations = operations_of(&mut session, stream);
    assert_eq!(operations.len(), 1, "the image is one operation: {operations:?}");
    assert_eq!(operations[0]["operator"], json!("BI"), "and it is the image: {operations:?}");
    assert_eq!(operations[0]["inline"], json!(true), "and it is kept: {operations:?}");
    let written = std::fs::read(format!("{directory}/content.txt")).expect("the file is there");
    assert!(
        written.windows(4).any(|window| window == [0x00, 0x01, 0x02, 0x03]),
        "and the bytes of it are the bytes it was: {written:?}"
    );
}

/// A stream that is **filtered** is refused: its bytes are not the
/// content.
#[test]
fn a_filtered_stream_is_refused() {
    let mut session = session();
    let directory = directory("filtered");
    let path = format!("{directory}/content.txt");
    std::fs::write(&path, b"10 20 m").expect("the bytes are written");
    let outcome = session
        .execute(
            "pdf.stream.add",
            &json!({
                "object": 4242,
                "entries": [["Type", "XObject"], ["Filter", "FlateDecode"]],
                "stream": path,
            }),
        )
        .expect("the stream is added");
    let stream = outcome.data["id"].as_u64().expect("the stream has an id");
    let error = session
        .execute("pdf.content.write", &json!({ "stream": stream }))
        .expect_err("a filtered stream is not written");
    assert!(error.to_string().contains("filtered"), "the refusal says why: {error}");
}
