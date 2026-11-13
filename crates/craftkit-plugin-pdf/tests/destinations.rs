//! The **named destinations** of a file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-destinations-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file that names two destinations.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /Names 5 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
        (4, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
        (5, "<< /Dests 6 0 R >>"),
        (6, "<< /Names [(the first) [3 0 R /XYZ 0 842 0] (the second) [4 0 R /XYZ 0 842 0]] >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 7\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **destinations** of a file, as the command reads them.
fn destinations_of(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("pdf.dests.read", &json!({})).expect("the destinations are read");
    outcome.data["destinations"].as_array().expect("the destinations are a list").clone()
}

/// The named destinations of a file are **read**.
#[test]
fn the_named_destinations_are_read() {
    let mut session = session();
    load(&mut session, "found");
    let destinations = destinations_of(&mut session);
    assert_eq!(destinations.len(), 2, "there are two destinations");
    assert_eq!(destinations[0]["name"], json!("the first"), "the first is named");
    assert_eq!(destinations[0]["page"], json!(3), "and it points at the third object");
    assert_eq!(destinations[1]["page"], json!(4), "and the second at the fourth");
}

/// One destination may be read by its **name**.
#[test]
fn one_destination_may_be_read_by_its_name() {
    let mut session = session();
    load(&mut session, "one");
    let outcome =
        session.execute("pdf.dests.read", &json!({ "name": "the second" })).expect("the destination is read");
    assert_eq!(outcome.data["count"], json!(1), "one destination is read");
    assert_eq!(outcome.data["destinations"][0]["page"], json!(4), "and it is the one that was named");
}

/// A name the file does not use is **refused**.
#[test]
fn a_name_the_file_does_not_use_is_refused() {
    let mut session = session();
    load(&mut session, "refused");
    let outcome = session.execute("pdf.dests.read", &json!({ "name": "nothing" }));
    assert!(outcome.is_err(), "there is no such destination");
}

/// A file that names no destination has **none**.
#[test]
fn a_file_that_names_no_destination_has_none() {
    let mut session = session();
    let directory = directory("without");
    let path = format!("{directory}/file.pdf");
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
    file.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let destinations = destinations_of(&mut session);
    assert_eq!(destinations.len(), 0, "the file has no named destinations");
}
