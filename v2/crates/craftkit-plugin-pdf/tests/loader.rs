//! The **loader**: a PDF file read into the document.
//!
//! A file is an index and the objects it points at. The loader reads
//! both, and the document holds the objects as **nodes** and the
//! references as **relations**.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-loader-{name}"));
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

/// The **nodes** the document holds after the file is loaded.
fn nodes_of(session: &Session) -> Vec<(u64, String, craftkit_sdk::serde_json::Value)> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object" || node.kind == "pdf.stream")
        .map(|node| {
            let number = node.data.get("object").and_then(|number| number.as_u64()).unwrap_or(0);
            (number, node.kind.clone(), node.data.clone())
        })
        .collect()
}

/// A file is loaded into the document: its objects become nodes.
#[test]
fn a_file_is_loaded_into_the_document() {
    let directory = directory("load");
    let mut session = session();
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    let outcome =
        session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    assert_eq!(outcome.data["objects"], json!(3), "the file holds three objects");
    let nodes = nodes_of(&session);
    assert_eq!(nodes.len(), 3, "and the document holds three nodes");
    let catalog = nodes.iter().find(|(number, _, _)| *number == 1).expect("the catalog is there");
    let entries = catalog.2["entries"].as_array().expect("the catalog has a dictionary");
    let kind = entries
        .iter()
        .filter_map(|entry| entry.as_array())
        .find(|pair| pair.first().and_then(|key| key.as_str()) == Some("Type"))
        .and_then(|pair| pair.get(1).and_then(|value| value.as_str().map(|value| value.to_string())));
    assert_eq!(kind.as_deref(), Some("Catalog"), "the catalog says it is the catalog");
}

/// The **references** become relations, and a reference that points
/// forward is still a relation.
#[test]
fn the_references_of_a_file_become_relations() {
    let directory = directory("relations");
    let mut session = session();
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let document = session.document();
    let refs: Vec<_> = document.relations.iter().filter(|relation| relation.kind == "ref").collect();
    assert_eq!(refs.len(), 3, "the file holds three references");
    // The catalog names the page tree, the page tree names the page,
    // and the page names its parent.
    let numbers: Vec<(u64, u64)> = refs.iter().map(|relation| (relation.from, relation.to)).collect();
    assert!(numbers.contains(&(1, 2)), "the catalog names the page tree");
    assert!(numbers.contains(&(2, 3)), "the page tree names the page");
    assert!(numbers.contains(&(3, 2)), "the page names its parent");
    let key = refs
        .iter()
        .find(|relation| relation.from == 1)
        .and_then(|relation| relation.payload.get("key").and_then(|key| key.as_str()))
        .map(|key| key.to_string());
    assert_eq!(key.as_deref(), Some("Pages"), "and the relation says under which key");
}
