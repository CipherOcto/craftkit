//! The **cross-reference table**: the file's own index of itself.
//!
//! A file does not hold its objects in order; the table says where
//! each one is. There are two kinds: the classic table of twenty-byte
//! entries, and the cross-reference stream that holds the same
//! information in binary.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-xref-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **entries** a file's table holds.
fn entries_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session.execute("pdf.file.xref", &json!({ "path": path })).expect("the table is read");
    outcome.data["entries"].as_array().expect("the entries are a list").clone()
}

/// The **classic table**: twenty-byte entries under the word `xref`.
#[test]
fn the_classic_table_is_read() {
    let directory = directory("classic");
    let mut session = session();
    let mut file = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 2\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    file.extend_from_slice(b"0000000015 00000 n\r\n");
    file.extend_from_slice(b"trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    let path = format!("{directory}/classic.pdf");
    std::fs::write(&path, &file).expect("the file is written");
    let entries = entries_of(&mut session, &path);
    assert_eq!(entries.len(), 2, "the table holds two entries");
    assert_eq!(entries[0]["object"], json!(0), "the first is object zero");
    assert_eq!(entries[0]["kind"], json!("free"), "and it is free");
    assert_eq!(entries[1]["object"], json!(1), "the second is object one");
    assert_eq!(entries[1]["offset"], json!(15), "and it is at byte fifteen");
    assert_eq!(entries[1]["kind"], json!("in-file"), "and it is in the file");
}

/// The **cross-reference stream**: the same table, in binary.
#[test]
fn the_cross_reference_stream_is_read() {
    let directory = directory("stream");
    let mut session = session();
    // Two entries of one byte each: the kind, the offset and the
    // generation.
    let mut file = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let table_at = file.len();
    let data: Vec<u8> = vec![1, 0, 0, 0, 2, 0, 9, 0];
    let dictionary =
        format!("1 0 obj\n<< /Type /XRef /Size 2 /W [1 2 1] /Root 1 0 R /Length {} >>\nstream\n", data.len());
    file.extend_from_slice(dictionary.as_bytes());
    file.extend_from_slice(&data);
    file.extend_from_slice(b"\nendstream\nendobj\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    let path = format!("{directory}/stream.pdf");
    std::fs::write(&path, &file).expect("the file is written");
    let entries = entries_of(&mut session, &path);
    assert_eq!(entries.len(), 2, "the table holds two entries");
    assert_eq!(entries[0]["kind"], json!("in-file"), "the first is in the file");
    assert_eq!(entries[0]["offset"], json!(0), "at byte zero");
    assert_eq!(entries[1]["kind"], json!("compressed"), "the second is compressed");
    assert_eq!(entries[1]["offset"], json!(9), "and it is in the stream nine");
}

/// A file that does not say where its table is is **refused**.
#[test]
fn a_file_without_a_table_is_refused() {
    let directory = directory("refused");
    let mut session = session();
    let path = format!("{directory}/plain.pdf");
    std::fs::write(&path, b"%PDF-1.7\nnothing here\n").expect("the file is written");
    let refused = session.execute("pdf.file.xref", &json!({ "path": path }));
    assert!(refused.is_err(), "a file that does not say where its table is is refused");
}
