//! The **trailer** of a PDF file.

use craftkit_sdk::serde_json::{Value as Json, json};
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-trailer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A small **file**: a catalog and a page, and a trailer of its own.
fn a_file(trailer: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 4\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n").as_bytes());
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str, trailer: &str) {
    let directory = directory(name);
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file(trailer)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **trailer** of the document, as the command reads it.
fn trailer(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.trailer.read", &json!({})).expect("the trailer is read");
    outcome.data.clone()
}

/// The **entries** of the trailer, by key.
fn entries_of(session: &mut Session) -> Vec<(String, Json)> {
    trailer(session)["entries"]
        .as_array()
        .expect("the entries are a list")
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
        })
        .collect()
}

/// The **preflight** of the document.
fn preflight(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.preflight", &json!({})).expect("the preflight is done");
    outcome.data.clone()
}

/// The **clauses** the document breaks.
fn clauses(preflight: &Json) -> Vec<String> {
    preflight["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .map(|issue| issue["clause"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The **trailer** of a file is kept in the document.
#[test]
fn the_trailer_of_a_file_is_kept_in_the_document() {
    let mut session = session();
    load(&mut session, "kept", "<< /Size 4 /Root 1 0 R >>");
    let entries = entries_of(&mut session);
    assert!(entries.iter().any(|(key, _)| key == "Root"), "the trailer names the catalog: {entries:?}");
}

/// The trailer is **not** an object of the file.
#[test]
fn the_trailer_is_not_an_object_of_the_file() {
    let mut session = session();
    load(&mut session, "notobject", "<< /Size 4 /Root 1 0 R >>");
    let objects = session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object" || node.kind == "pdf.stream")
        .count();
    assert_eq!(objects, 3, "the file holds three objects and not a fourth");
}

/// A file that is **encrypted** breaks the rule.
#[test]
fn a_file_that_is_encrypted_breaks_the_rule() {
    let mut session = session();
    load(&mut session, "encrypted", "<< /Size 4 /Root 1 0 R /Encrypt 4 0 R >>");
    let preflight = preflight(&mut session);
    assert!(clauses(&preflight).contains(&"6.1.3".to_string()), "the rule is broken: {preflight}");
}

/// A file that is **not** encrypted breaks nothing.
#[test]
fn a_file_that_is_not_encrypted_breaks_nothing() {
    let mut session = session();
    load(&mut session, "plain", "<< /Size 4 /Root 1 0 R >>");
    let preflight = preflight(&mut session);
    assert!(!clauses(&preflight).contains(&"6.1.3".to_string()), "the file is plain: {preflight}");
}

/// The **trailer** says whether the file is encrypted.
#[test]
fn the_trailer_says_whether_the_file_is_encrypted() {
    let mut session = session();
    load(&mut session, "says", "<< /Size 4 /Root 1 0 R /Encrypt 4 0 R >>");
    assert_eq!(trailer(&mut session)["encrypted"], json!(true), "the file is encrypted");
}

/// One **entry** of the trailer is read.
#[test]
fn one_entry_of_the_trailer_is_read() {
    let mut session = session();
    load(&mut session, "one", "<< /Size 4 /Root 1 0 R >>");
    let outcome = session.execute("pdf.trailer.read", &json!({ "key": "Size" })).expect("the entry is read");
    assert_eq!(outcome.data["entries"], json!([["Size", 4.0]]), "the size is the size");
}

/// An entry the trailer does not have is **refused**.
#[test]
fn an_entry_the_trailer_does_not_have_is_refused() {
    let mut session = session();
    load(&mut session, "missing", "<< /Size 4 /Root 1 0 R >>");
    let refused =
        session.execute("pdf.trailer.read", &json!({ "key": "Encrypt" })).expect_err("the entry is refused");
    assert!(refused.to_string().contains("Encrypt"), "the refusal says why: {refused}");
}

/// A document that was not loaded from a file has **no** trailer.
#[test]
fn a_document_that_was_not_loaded_from_a_file_has_no_trailer() {
    let mut session = session();
    let trailer = trailer(&mut session);
    assert_eq!(trailer["entries"], json!([]), "there is no trailer");
    assert_eq!(trailer["encrypted"], json!(false), "and the file is not encrypted");
}
