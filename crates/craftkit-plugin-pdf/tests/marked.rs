//! The **marked content** of a tagged file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-marked-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page: two marked contents, one whose
/// properties are named and one whose properties are written where
/// they are used.
fn the_content() -> String {
    let mut content = String::new();
    content.push_str("/Span /MC0 BDC\nBT (the heading) Tj ET\nEMC\n");
    content.push_str("/P << /MCID 1 >> BDC\nBT (the paragraph) Tj ET\nEMC\n");
    content
}

/// A **tagged** file: a heading and a paragraph, each with its marked
/// content.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let content = the_content();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /Properties << /MC0 << /MCID 0 >> >> >> /StructParents 0 >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, "<< /Type /StructTreeRoot /K 6 0 R >>".to_string()),
        (6, "<< /S /Document /Lang (en-GB) /Pg 3 0 R /K [7 0 R 8 0 R] >>".to_string()),
        (7, "<< /S /H1 /T (the heading) /P 6 0 R /Pg 3 0 R /K 0 >>".to_string()),
        (8, "<< /S /P /P 6 0 R /Pg 3 0 R /K << /MCID 1 >> >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 9\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// A file that is **not tagged**.
fn a_plain_file() -> Vec<u8> {
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
fn load(session: &mut Session, name: &str, file: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **marked content** of the file, as the command reads it.
fn marked_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.marked.read", &json!({})).expect("the content is read");
    outcome.data.clone()
}

/// The text under each element of the structure tree is read.
#[test]
fn the_text_under_each_element_is_read() {
    let mut session = session();
    load(&mut session, "tagged", a_file());
    let marked = marked_of(&mut session);
    let list = marked["marked"].as_array().expect("the elements are a list");
    assert_eq!(list.len(), 3, "the file has three elements");
    assert_eq!(list[0]["kind"], json!("Document"), "the first is the document");
    assert_eq!(list[0]["language"], json!("en-GB"), "and it says its language");
    assert_eq!(list[1]["kind"], json!("H1"), "the second is a heading");
    assert_eq!(list[1]["title"], json!("the heading"), "and it is named");
    assert_eq!(list[1]["text"], json!("the heading"), "and the text under it is read");
    assert_eq!(list[2]["kind"], json!("P"), "the third is a paragraph");
    assert_eq!(list[2]["text"], json!("the paragraph"), "and its text is read too");
}

/// The properties of a marked content may be **named**, and the name
/// is looked for in the page's resources.
#[test]
fn the_properties_of_a_marked_content_may_be_named() {
    let mut session = session();
    load(&mut session, "named", a_file());
    let marked = marked_of(&mut session);
    let list = marked["marked"].as_array().expect("the elements are a list");
    // The heading names its properties (/MC0), and the paragraph
    // writes them where they are used.
    assert_eq!(list[1]["text"], json!("the heading"), "the named properties are read");
    assert_eq!(list[2]["text"], json!("the paragraph"), "and the written ones are read");
}

/// A file that is **not tagged** has no marked content.
#[test]
fn a_file_that_is_not_tagged_has_no_marked_content() {
    let mut session = session();
    load(&mut session, "plain", a_plain_file());
    let marked = marked_of(&mut session);
    assert_eq!(marked["count"], json!(0), "the file has no marked content");
    assert_eq!(marked["marked"].as_array().map(Vec::len), Some(0), "and there are none");
}

/// An element may be asked for **by kind**, and one that is not there
/// is refused.
#[test]
fn an_element_may_be_asked_for_by_kind() {
    let mut session = session();
    load(&mut session, "kind", a_file());
    let outcome = session.execute("pdf.marked.read", &json!({ "kind": "H1" })).expect("the element is read");
    assert_eq!(outcome.data["count"], json!(1), "the element is read by its kind");
    assert_eq!(outcome.data["marked"][0]["text"], json!("the heading"), "and it says its text");
    let missing = session.execute("pdf.marked.read", &json!({ "kind": "Figure" }));
    assert!(missing.is_err(), "an element that is not there is refused");
    let message = missing.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("Figure"), "and it says which one: {message}");
}
