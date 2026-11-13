//! The **filters**: the ways a PDF stream's bytes may be encoded.
//!
//! A stream in a PDF file may be encoded: the `/Filter` entry names
//! the way and the bytes in the file are the encoded ones. The command
//! reads the file, undoes the filter and writes what was inside it.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-filters-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stream** node whose bytes are in a file, and the file.
fn a_stream(session: &mut Session, directory: &str, name: &str, bytes: &[u8]) -> (u64, String) {
    let path = format!("{directory}/{name}");
    std::fs::write(&path, bytes).expect("the bytes are written");
    session
        .execute(
            "pdf.stream.add",
            &json!({ "object": a_free_number(session), "entries": [["Type", "XObject"]], "stream": path.clone() }),
        )
        .expect("the stream is added");
    let id = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .map(|node| node.id)
        .expect("the stream is there");
    (id, path)
}

/// The bytes a **decode** wrote.
fn decoded(path: &str) -> Vec<u8> {
    std::fs::read(path).expect("the decoded bytes are on the disk")
}

/// A **Flate** stream is a zlib stream, and the PNG crate's inflate
/// reads it.
/// A **free** object number: one number is one object.
fn a_free_number(session: &Session) -> u64 {
    session.document().walk().filter(|node| node.kind == "pdf.stream" || node.kind == "pdf.object").count()
        as u64
        + 1
}

#[test]
fn a_flate_stream_is_decoded() {
    let directory = directory("flate");
    let mut session = session();
    // The zlib stream of "Hello".
    let (id, _path) = a_stream(
        &mut session,
        &directory,
        "in.bin",
        &[0x78, 0x9c, 0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00, 0x05, 0x8c, 0x01, 0xf5],
    );
    let out = format!("{directory}/out.bin");
    session
        .execute("pdf.stream.decode", &json!({ "stream": id, "filter": "FlateDecode", "to": out.clone() }))
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), b"Hello".to_vec(), "the stream says Hello");
}

/// The digits of a byte in **hex**, undone.
#[test]
fn an_ascii_hex_stream_is_decoded() {
    let directory = directory("hex");
    let mut session = session();
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"48 65 6c 6c 6f>");
    let out = format!("{directory}/out.bin");
    session
        .execute("pdf.stream.decode", &json!({ "stream": id, "filter": "ASCIIHexDecode", "to": out.clone() }))
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), b"Hello".to_vec(), "the stream says Hello");
}

/// Five digits to four bytes, undone.
#[test]
fn an_ascii85_stream_is_decoded() {
    let directory = directory("85");
    let mut session = session();
    // The ASCII85 of "abcd" is "@:E_W", and the "~>" ends it.
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"@:E_W~>");
    let out = format!("{directory}/out.bin");
    session
        .execute("pdf.stream.decode", &json!({ "stream": id, "filter": "ASCII85Decode", "to": out.clone() }))
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), b"abcd".to_vec(), "the stream says abcd");
}

/// A length byte and the bytes it counts, undone.
#[test]
fn a_run_length_stream_is_decoded() {
    let directory = directory("run");
    let mut session = session();
    // Three bytes, then three of the same byte, and the end.
    let (id, _path) =
        a_stream(&mut session, &directory, "in.bin", &[0x02, b'a', b'b', b'c', 0xfe, b'Z', 0x80]);
    let out = format!("{directory}/out.bin");
    session
        .execute(
            "pdf.stream.decode",
            &json!({ "stream": id, "filter": "RunLengthDecode", "to": out.clone() }),
        )
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), b"abcZZZ".to_vec(), "the stream says abcZZZ");
}

/// **LZW**, with the early change, undone.
#[test]
fn an_lzw_stream_is_decoded() {
    let directory = directory("lzw");
    let mut session = session();
    // The code for "A", the code for "A" again, and the end: three
    // codes of nine bits each.
    let (id, _path) = a_stream(&mut session, &directory, "in.bin", &[0x20, 0x90, 0x60, 0x20]);
    let out = format!("{directory}/out.bin");
    session
        .execute("pdf.stream.decode", &json!({ "stream": id, "filter": "LZWDecode", "to": out.clone() }))
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), b"AA".to_vec(), "the stream says AA");
}

/// The **dictionary** says which filter a stream is in, and the
/// command reads it when nobody says otherwise.
#[test]
fn the_dictionary_names_the_filter() {
    let directory = directory("named");
    let mut session = session();
    let path = format!("{directory}/in.txt");
    std::fs::write(&path, b"48 65 6c 6c 6f>").expect("the bytes are written");
    session
        .execute(
            "pdf.stream.add",
            &json!({
                "object": a_free_number(&session),
                "entries": [["Type", "XObject"], ["Filter", "ASCIIHexDecode"]],
                "stream": path,
            }),
        )
        .expect("the stream is added");
    let id = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .map(|node| node.id)
        .expect("the stream is there");
    let out = format!("{directory}/out.bin");
    session
        .execute("pdf.stream.decode", &json!({ "stream": id, "to": out.clone() }))
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), b"Hello".to_vec(), "the stream says Hello");
}

/// A filter the kit does not know is **refused by name**, and so is a
/// stream that names no filter at all.
#[test]
fn a_filter_that_is_not_known_is_refused() {
    let directory = directory("unknown");
    let mut session = session();
    let (id, _path) = a_stream(&mut session, &directory, "in.bin", b"anything");
    let out = format!("{directory}/out.bin");
    let refused =
        session.execute("pdf.stream.decode", &json!({ "stream": id, "filter": "JBIG2Decode", "to": out }));
    let message = match refused {
        Ok(_) => panic!("a filter the kit does not know is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("JBIG2Decode"), "the refusal says why: {message}");
    let none = session.execute("pdf.stream.decode", &json!({ "stream": id, "to": out }));
    assert!(none.is_err(), "a stream that names no filter is refused");
}
