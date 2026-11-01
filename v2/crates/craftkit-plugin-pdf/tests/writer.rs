//! The **writer**: the document, written out as a PDF file.
//!
//! A file that was read into the document may be written out again —
//! and the file that comes out is read back by the reader, so the two
//! are the same questions asked in the other order.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-writer-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A small **file**: a catalog, a page tree and a page.
fn a_file() -> Vec<u8> {
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

/// The **entries** a file's table holds.
fn entries_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session.execute("pdf.file.xref", &json!({ "path": path })).expect("the table is read");
    outcome.data["entries"].as_array().expect("the entries are a list").clone()
}

/// A file is read into the document and written out again, and the
/// file that comes out has the same objects in it.
#[test]
fn a_file_is_written_out_again() {
    let directory = directory("write");
    let mut session = session();
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let out = format!("{directory}/out.pdf");
    session.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    let entries = entries_of(&mut session, &out);
    // The three objects the document held.
    assert_eq!(entries.len(), 3, "the file holds three entries");
    let numbers: Vec<u64> = entries.iter().filter_map(|entry| entry["object"].as_u64()).collect();
    assert!(numbers.contains(&1), "the catalog is there");
    assert!(numbers.contains(&3), "and the page is there");
    let kinds: Vec<String> =
        entries.iter().filter_map(|entry| entry["kind"].as_str().map(|kind| kind.to_string())).collect();
    assert_eq!(kinds.iter().filter(|kind| *kind == "in-file").count(), 3, "all three are in the file");
}

/// The file that is written out is **read back**: the same objects, in
/// the same places.
#[test]
fn the_file_that_is_written_is_read_back() {
    let directory = directory("round");
    let mut loaded = session();
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    loaded.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let out = format!("{directory}/out.pdf");
    loaded.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    // The file that comes out is read into a document of its own.
    let mut other = session();
    let again = format!("{directory}/again");
    other.execute("pdf.file.load", &json!({ "path": out, "into": again })).expect("the file is loaded again");
    let nodes: Vec<(u64, String)> = other
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object")
        .map(|node| {
            (node.data.get("object").and_then(|number| number.as_u64()).unwrap_or(0), node.name.clone())
        })
        .collect();
    assert_eq!(nodes.len(), 3, "the file holds three objects");
    let refs: Vec<_> = other.document().relations.iter().filter(|relation| relation.kind == "ref").collect();
    assert_eq!(refs.len(), 3, "and three references between them");
}

/// A document with no **catalog** has no root, and the refusal says
/// so.
#[test]
fn a_document_with_no_root_is_refused() {
    let directory = directory("noroot");
    let mut session = session();
    session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Something"]] }))
        .expect("the object is added");
    let refused = session.execute("pdf.file.write", &json!({ "path": format!("{directory}/out.pdf") }));
    let message = match refused {
        Ok(_) => panic!("a document with no root is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("root"), "the refusal says why: {message}");
}
