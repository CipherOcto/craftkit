//! The **information** of a file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-information-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file that says what it is in its information dictionary.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
        (4, "<< /Title (the title) /Author (ann) /Producer (the kit) >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 5\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R /Info 4 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// A file with no information dictionary.
fn a_file_without_information() -> Vec<u8> {
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
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str, bytes: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **information** of a file, as the command reads it.
fn information_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.info.read", &json!({})).expect("the information is read");
    outcome.data.clone()
}

/// The information dictionary of a file is **found**.
#[test]
fn the_information_of_a_file_is_found() {
    let mut session = session();
    load(&mut session, "found", a_file());
    let information = information_of(&mut session);
    assert_eq!(information["has_information"], json!(true), "the file has information");
    assert_eq!(information["Title"], json!("the title"), "the title is the one the file says");
    assert_eq!(information["Author"], json!("ann"), "and the author too");
    assert_eq!(information["Producer"], json!("the kit"), "and the producer");
}

/// An entry the information does not have is **nothing**.
#[test]
fn an_entry_the_information_does_not_have_is_nothing() {
    let mut session = session();
    load(&mut session, "missing", a_file());
    let information = information_of(&mut session);
    assert_eq!(information["Subject"], Json::Null, "there is no subject");
    assert_eq!(information["CreationDate"], Json::Null, "and no date");
}

/// One entry may be read by its **key**.
#[test]
fn one_entry_may_be_read_by_its_key() {
    let mut session = session();
    load(&mut session, "key", a_file());
    let outcome = session.execute("pdf.info.read", &json!({ "key": "Author" })).expect("the entry is read");
    assert_eq!(outcome.data["entries"], json!([["Author", { "string": "ann" }]]), "the author is read");
}

/// A key the information does not have is **refused**.
#[test]
fn a_key_the_information_does_not_have_is_refused() {
    let mut session = session();
    load(&mut session, "refused", a_file());
    let outcome = session.execute("pdf.info.read", &json!({ "key": "Nothing" }));
    assert!(outcome.is_err(), "there is no such entry");
}

/// A file with no information dictionary has **no information**.
#[test]
fn a_file_with_no_information_has_no_information() {
    let mut session = session();
    load(&mut session, "without", a_file_without_information());
    let information = information_of(&mut session);
    assert_eq!(information["has_information"], json!(false), "the file has no information");
    assert_eq!(information["Title"], Json::Null, "and no title");
}
