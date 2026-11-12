//! The **text markup** of a document: the highlight, the underline, the
//! strikeout and the squiggly underline.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-markup-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file of PDF with a line of **text** on its page.
fn a_file() -> Vec<u8> {
    let content = String::from("BT /F1 12 Tf 72 700 Td (Hello world) Tj ET\n");
    let widths: Vec<String> = (32..=126).map(|_| "500".to_string()).collect();
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}endstream", content.len())),
        (5, format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 32 /LastChar 126 /Widths [{}] /Encoding /WinAnsiEncoding >>",
            widths.join(" ")
        )),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 6\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **page** it has.
fn a_page(session: &mut Session, name: &str) -> u64 {
    let directory = directory(name);
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

/// The **entries** of the annotation the page names.
fn the_entries_of(session: &mut Session, page: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let annotations =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    let list = annotations.data["annotations"].as_array().expect("the annotations are a list");
    let id = list.first().expect("there is an annotation")["id"].as_u64().expect("the annotation has an id");
    let node = session.document().node(id).expect("the annotation is in the document");
    node.data["entries"].as_array().expect("the entries are a list").clone()
}

/// The **region** the text is under.
fn region() -> craftkit_sdk::serde_json::Value {
    json!([0.0, 690.0, 595.0, 720.0])
}

/// **A highlight is written over the text.**
#[test]
fn a_highlight_is_written_over_the_text() {
    let mut session = session();
    let page = a_page(&mut session, "highlight");
    let outcome = session
        .execute(
            "pdf.markup.add",
            &json!({ "page": page, "object": 9, "kind": "highlight", "region": region() }),
        )
        .expect("the text is marked");
    assert_eq!(outcome.data["subtype"], json!("Highlight"), "it is a highlight: {outcome:?}");
    let quads = outcome.data["quads"].as_array().expect("the quads are a list");
    assert_eq!(quads.len(), 1, "there is one line of text: {outcome:?}");
    let quad = quads[0].as_array().expect("a quad is a list");
    assert_eq!(quad.len(), 8, "a quad has eight numbers: {outcome:?}");
    assert_eq!(outcome.data["colour"], json!([1.0, 0.94, 0.0]), "and the tree's colour: {outcome:?}");
}

/// **A highlight multiplies over the text.**
#[test]
fn a_highlight_multiplies_over_the_text() {
    let mut session = session();
    let page = a_page(&mut session, "multiply");
    session
        .execute(
            "pdf.markup.add",
            &json!({ "page": page, "object": 9, "kind": "highlight", "region": region() }),
        )
        .expect("the text is marked");
    let entries = the_entries_of(&mut session, page);
    let has = |key: &str| entries.iter().any(|entry| entry[0] == json!(key));
    assert!(has("QuadPoints"), "the quads are in it: {entries:?}");
    assert!(has("BM"), "and the blend mode: {entries:?}");
    let blend = entries.iter().find(|entry| entry[0] == json!("BM")).expect("the blend mode");
    assert_eq!(blend[1], json!("Multiply"), "and it multiplies: {entries:?}");
}

/// **An underline is written over the text.**
#[test]
fn an_underline_is_written_over_the_text() {
    let mut session = session();
    let page = a_page(&mut session, "underline");
    let outcome = session
        .execute(
            "pdf.markup.add",
            &json!({ "page": page, "object": 9, "kind": "underline", "region": region() }),
        )
        .expect("the text is marked");
    assert_eq!(outcome.data["subtype"], json!("Underline"), "it is an underline: {outcome:?}");
    assert_eq!(outcome.data["colour"], json!([0.0, 0.47, 0.84]), "and the tree's colour: {outcome:?}");
}

/// **The page names the markup.**
#[test]
fn the_page_names_the_markup() {
    let mut session = session();
    let page = a_page(&mut session, "named");
    session
        .execute(
            "pdf.markup.add",
            &json!({ "page": page, "object": 9, "kind": "squiggly", "region": region() }),
        )
        .expect("the text is marked");
    let annotations =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    let list = annotations.data["annotations"].as_array().expect("the annotations are a list");
    assert_eq!(list.len(), 1, "the page names it: {annotations:?}");
    assert_eq!(list[0]["Subtype"], json!("Squiggly"), "and it is the squiggly one: {annotations:?}");
}

/// **A region with no text under it is refused.**
#[test]
fn a_region_with_no_text_under_it_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "empty");
    let refusal = session
        .execute(
            "pdf.markup.add",
            &json!({ "page": page, "object": 9, "kind": "highlight", "region": [0.0, 0.0, 10.0, 10.0] }),
        )
        .expect_err("there is no text there");
    let message = refusal.to_string();
    assert!(message.contains("no text"), "the refusal says so: {message}");
}

/// **A kind that is not one of the tree's own is refused.**
#[test]
fn a_kind_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "kind");
    let refusal = session
        .execute(
            "pdf.markup.add",
            &json!({ "page": page, "object": 9, "kind": "marker", "region": region() }),
        )
        .expect_err("the kind is refused");
    let message = refusal.to_string();
    assert!(message.contains("kind"), "the refusal says so: {message}");
}
