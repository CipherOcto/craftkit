//! The **predictors**: the second step that sits on top of a filter.
//!
//! A predictor undoes a step that was made after the encoding: the
//! bytes of an image are stored as the difference from the byte before
//! them, so that they compress. The command undoes it after the filter,
//! and the PDF says which one a stream is in.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-predictors-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stream** node whose bytes are in a file, with the dictionary it
/// carries, and the file.
fn a_stream(
    session: &mut Session,
    directory: &str,
    name: &str,
    bytes: &[u8],
    entries: Vec<(String, craftkit_sdk::serde_json::Value)>,
) -> (u64, String) {
    let path = format!("{directory}/{name}");
    std::fs::write(&path, bytes).expect("the bytes are written");
    let mut list = Vec::new();
    for (key, value) in entries {
        list.push(json!([key, value]));
    }
    session
        .execute("pdf.stream.add", &json!({ "object": 1, "entries": list, "stream": path.clone() }))
        .expect("the stream is added");
    let id = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .map(|node| node.id)
        .expect("the stream is there");
    (id, path)
}

/// The **width** of the picture, as the dictionary says it.
fn width(columns: usize) -> (String, craftkit_sdk::serde_json::Value) {
    ("DecodeParms".to_string(), json!([["Columns", columns], ["Colors", 1], ["BitsPerComponent", 8]]))
}

/// The bytes a **decode** wrote.
fn decoded(path: &str) -> Vec<u8> {
    std::fs::read(path).expect("the decoded bytes are on the disk")
}

/// The **PNG** predictors, undone.
///
/// The row says how it was made: the first is the difference from the
/// byte to the left and the second from the byte above.
#[test]
fn a_png_predictor_is_undone() {
    let directory = directory("png");
    let mut session = session();
    // The row [1, 2, 3, 4] and the row [5, 6, 7, 8], the first made
    // with the difference from the left and the second with the
    // difference from the row above.
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"01010101010204040404", vec![width(4)]);
    let out = format!("{directory}/out.bin");
    session
        .execute(
            "pdf.stream.decode",
            &json!({ "stream": id, "filter": "ASCIIHexDecode", "predictor": 15, "to": out.clone() }),
        )
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), vec![1, 2, 3, 4, 5, 6, 7, 8], "the rows are as they were");
}

/// The **TIFF** predictor, undone.
#[test]
fn a_tiff_predictor_is_undone() {
    let directory = directory("tiff");
    let mut session = session();
    // The row [10, 20, 30, 40] and the differences from the byte to
    // the left.
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"0a0a0a0a", vec![width(4)]);
    let out = format!("{directory}/out.bin");
    session
        .execute(
            "pdf.stream.decode",
            &json!({ "stream": id, "filter": "ASCIIHexDecode", "predictor": 2, "to": out.clone() }),
        )
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), vec![10, 20, 30, 40], "the row is as it was");
}

/// The **dictionary** says which predictor a stream is in, and the
/// command reads it when nobody says otherwise.
#[test]
fn the_dictionary_names_the_predictor() {
    let directory = directory("named");
    let mut session = session();
    // The same bytes as the PNG test, written as the digits of a byte
    // in hex.
    let (id, _path) = a_stream(
        &mut session,
        &directory,
        "in.txt",
        b"01010101010204040404",
        vec![
            ("Filter".to_string(), json!("ASCIIHexDecode")),
            (
                "DecodeParms".to_string(),
                json!([["Predictor", 15], ["Columns", 4], ["Colors", 1], ["BitsPerComponent", 8]]),
            ),
        ],
    );
    let out = format!("{directory}/out.bin");
    session
        .execute("pdf.stream.decode", &json!({ "stream": id, "to": out.clone() }))
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), vec![1, 2, 3, 4, 5, 6, 7, 8], "the rows are as they were");
}

/// A predictor of **none** leaves the bytes as they are.
#[test]
fn a_predictor_of_none_leaves_the_bytes() {
    let directory = directory("none");
    let mut session = session();
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"01020304", vec![width(4)]);
    let out = format!("{directory}/out.bin");
    session
        .execute(
            "pdf.stream.decode",
            &json!({ "stream": id, "filter": "ASCIIHexDecode", "predictor": 1, "to": out.clone() }),
        )
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), vec![1, 2, 3, 4], "the bytes are as they were");
}

/// The **Paeth** guess: the row says the fifth way, and the bytes are
/// the difference from the guess.
#[test]
fn the_paeth_guess_is_undone() {
    let directory = directory("paeth");
    let mut session = session();
    // The row [10, 20, 30, 40] as it is, and the row [10, 21, 30, 40]
    // made with the guess.
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"000a141e280400010000", vec![width(4)]);
    let out = format!("{directory}/out.bin");
    session
        .execute(
            "pdf.stream.decode",
            &json!({ "stream": id, "filter": "ASCIIHexDecode", "predictor": 15, "to": out.clone() }),
        )
        .expect("the stream is decoded");
    assert_eq!(decoded(&out), vec![10, 20, 30, 40, 10, 21, 30, 40], "the rows are as they were");
}

/// A predictor the kit does not know is **refused by name**, and so is
/// a row that is shorter than the picture is wide.
#[test]
fn a_predictor_that_cannot_be_undone_is_refused() {
    let directory = directory("refused");
    let mut session = session();
    let (id, _path) = a_stream(&mut session, &directory, "in.txt", b"01020304", vec![width(4)]);
    let out = format!("{directory}/out.bin");
    let refused = session.execute(
        "pdf.stream.decode",
        &json!({ "stream": id, "filter": "ASCIIHexDecode", "predictor": 7, "to": out.clone() }),
    );
    let message = match refused {
        Ok(_) => panic!("a predictor the kit does not know is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains('7'), "the refusal says why: {message}");
    // A row of four bytes when the picture is eight wide.
    let (short, _path) = a_stream(&mut session, &directory, "short.txt", b"01020304", vec![width(8)]);
    let refused = session.execute(
        "pdf.stream.decode",
        &json!({ "stream": short, "filter": "ASCIIHexDecode", "predictor": 15, "to": out.clone() }),
    );
    assert!(refused.is_err(), "a row that is shorter than it says is refused");
}
