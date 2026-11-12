//! The **structure** of a file.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-structure-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **tagged** file: a document with a heading, a paragraph and a
/// picture in it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
        (4, "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /P 3 0 R >>"),
        (5, "<< /Type /StructTreeRoot /K 6 0 R >>"),
        (6, "<< /S /Document /Lang (en-GB) /K [7 0 R 8 0 R 9 0 R] >>"),
        (7, "<< /S /H1 /T (the heading) /P 6 0 R >>"),
        (8, "<< /S /P /P 6 0 R >>"),
        (9, "<< /S /Figure /Alt (a picture of a cat) /P 6 0 R >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 10\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 10 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **structure** of a file, as the command reads it.
fn structure_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.structure.read", &json!({})).expect("the structure is read");
    outcome.data.clone()
}

/// The structure tree of a **tagged** file is read.
#[test]
fn the_structure_tree_of_a_tagged_file_is_read() {
    let mut session = session();
    load(&mut session, "tagged");
    let structure = structure_of(&mut session);
    assert_eq!(structure["tagged"], json!(true), "the file is tagged");
    let root = &structure["structure"];
    assert_eq!(root["kind"], json!("Document"), "the root is the document");
    assert_eq!(root["language"], json!("en-GB"), "and it says its language");
    let children = root["children"].as_array().expect("the children are a list");
    assert_eq!(children.len(), 3, "there are three elements");
    assert_eq!(children[0]["kind"], json!("H1"), "the first is a heading");
    assert_eq!(children[0]["title"], json!("the heading"), "and it is named");
    assert_eq!(children[2]["alt"], json!("a picture of a cat"), "and the picture has its alt text");
}

/// A file with **no structure tree** is not tagged.
#[test]
fn a_file_with_no_structure_tree_is_not_tagged() {
    let mut session = session();
    let directory = directory("untagged");
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
    let structure = structure_of(&mut session);
    assert_eq!(structure["tagged"], json!(false), "the file is not tagged");
    assert_eq!(structure["structure"], Json::Null, "and there is no tree");
}
