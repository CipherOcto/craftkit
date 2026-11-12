//! The **embedded file** of a document: the file a file carries, named
//! in the tree of names.

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

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-embeddedfileadd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with a **page** on it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len())),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 5\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **page**.
fn a_page(session: &mut Session, directory: &str) -> u64 {
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    session
        .document()
        .walk()
        .find(|node| {
            node.data["entries"]
                .as_array()
                .map(|entries| {
                    entries.iter().any(|entry| {
                        let Some(pair) = entry.as_array() else { return false };
                        pair.first().and_then(|key| key.as_str()) == Some("Type")
                            && pair.get(1).and_then(|value| value.as_str()) == Some("Page")
                    })
                })
                .unwrap_or(false)
        })
        .map(|node| node.id)
        .expect("the page is there")
}

/// An **attachment** on a page: the file specification of it.
fn a_specification(session: &mut Session, directory: &str, name: &str) -> u64 {
    let page = a_page(session, directory);
    let source = format!("{directory}/attached.txt");
    std::fs::write(&source, "hello").expect("the file is written");
    let outcome = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [200.0, 700.0],
                "name": name,
                "path": source,
                "to": format!("{directory}/attached.bin"),
            }),
        )
        .expect("the attachment is added");
    outcome.data["specification"].as_u64().expect("the file specification is there")
}

/// **The file is named in the document**, and the document's reader
/// finds it.
#[test]
fn the_file_is_named_in_the_document() {
    let mut session = session();
    let directory = directory("named");
    let specification = a_specification(&mut session, &directory, "notes.txt");
    let outcome = session
        .execute("pdf.embeddedfile.add", &json!({ "specification": specification }))
        .expect("the file is named");
    assert!(outcome.changed);
    // The **name** of the file is the file's own name.
    assert_eq!(outcome.data["name"], json!("notes.txt"), "the name is the file's own");
    // And the document's reader finds it.
    let read = session.execute("pdf.attachments.read", &json!({})).expect("the attachments are read");
    let attachments = read.data["attachments"].as_array().expect("the attachments are there");
    assert_eq!(attachments.len(), 1, "the file is carried by the document");
    assert_eq!(attachments[0]["name"], json!("notes.txt"), "the file is there");
    assert_eq!(attachments[0]["bytes"], json!(5), "the bytes are there");
}

/// **A name the caller gives** is the name the file is listed under in
/// the tree of names — and the **file** keeps its own name: the
/// tree's own rule.
#[test]
fn a_name_the_caller_gives_is_used() {
    let mut session = session();
    let directory = directory("named2");
    let specification = a_specification(&mut session, &directory, "notes.txt");
    let outcome = session
        .execute(
            "pdf.embeddedfile.add",
            &json!({ "specification": specification, "name": "Read me first.txt" }),
        )
        .expect("the file is named");
    assert_eq!(outcome.data["name"], json!("Read me first.txt"), "the name is the caller's");
    let read = session.execute("pdf.attachments.read", &json!({})).expect("the attachments are read");
    let attachments = read.data["attachments"].as_array().expect("the attachments are there");
    assert_eq!(attachments.len(), 1, "the file is carried by the document");
    // And the file is listed under **its own** name: the name of the
    // file specification is the name of the attachment.
    assert_eq!(attachments[0]["name"], json!("notes.txt"), "the file keeps its own name");
}

/// **A name that is already there** names the file again, and does not
/// make a second entry: the tree's own rule.
#[test]
fn a_name_that_is_there_names_the_file_again() {
    let mut session = session();
    let directory = directory("again");
    let specification = a_specification(&mut session, &directory, "notes.txt");
    session
        .execute("pdf.embeddedfile.add", &json!({ "specification": specification }))
        .expect("the file is named");
    session
        .execute("pdf.embeddedfile.add", &json!({ "specification": specification }))
        .expect("the file is named again");
    let read = session.execute("pdf.attachments.read", &json!({})).expect("the attachments are read");
    let attachments = read.data["attachments"].as_array().expect("the attachments are there");
    assert_eq!(attachments.len(), 1, "there is one entry: {attachments:?}");
}

/// **A node that is not a file specification** is refused.
#[test]
fn a_node_that_is_not_a_file_specification_is_refused() {
    let mut session = session();
    let directory = directory("notaspec");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute("pdf.embeddedfile.add", &json!({ "specification": page }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("not a file specification"), "the error says why: {error}");
}

/// **A file with no name** is refused: the tree's own rule.
#[test]
fn a_file_with_no_name_is_refused() {
    let mut session = session();
    let directory = directory("noname");
    let specification = a_specification(&mut session, &directory, "notes.txt");
    session
        .execute("pdf.entry.remove", &json!({ "object": specification, "key": "F" }))
        .expect("the name is taken away");
    session
        .execute("pdf.entry.remove", &json!({ "object": specification, "key": "UF" }))
        .expect("the name is taken away");
    let error = session
        .execute("pdf.embeddedfile.add", &json!({ "specification": specification }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("needs a name"), "the error says why: {error}");
}
