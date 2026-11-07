//! The **place** of the text of a page.
//!
//! The place is the text matrix of the text, turned by the transform
//! of the page.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-textplaces-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given, and the extra entries
/// the page is given.
fn a_file(content: &str, extra: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] {extra} /Contents 4 0 R >>")),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
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
    file.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str, content: &str, extra: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content, extra)).expect("the file is written");
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

/// The **pieces** of text the page shows.
fn pieces_of(session: &mut Session, page: u64) -> Vec<Json> {
    let outcome = session.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    outcome.data["pieces"].as_array().expect("the pieces are a list").clone()
}

/// The place of the text is the **text matrix** it was shown at.
#[test]
fn the_place_of_the_text_is_the_text_matrix() {
    let mut session = session();
    let page = load(&mut session, "matrix", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (Hello) Tj ET", "");
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(100.0), "the text begins where the matrix says: {pieces:?}");
    assert_eq!(pieces[0]["y"], json!(200.0), "and it is as high as the matrix says: {pieces:?}");
}

/// The **transform of the page** moves the text: the `cm` operator is
/// the tree's own rule.
#[test]
fn the_transform_of_the_page_moves_the_text() {
    let mut session = session();
    let page = load(&mut session, "cm", "2 0 0 2 10 20 cm BT /F1 12 Tf 1 0 0 1 100 200 Tm (Hello) Tj ET", "");
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(210.0), "the transform is applied: {pieces:?}");
    assert_eq!(pieces[0]["y"], json!(420.0), "in both directions: {pieces:?}");
}

/// The `q` and the `Q` **keep** the transform, and put it back.
#[test]
fn the_transform_is_kept_and_put_back() {
    let mut session = session();
    let page = load(
        &mut session,
        "q",
        "q 2 0 0 2 10 20 cm BT /F1 12 Tf 1 0 0 1 100 200 Tm (Inside) Tj ET Q \
         BT /F1 12 Tf 1 0 0 1 100 200 Tm (Outside) Tj ET",
        "",
    );
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 2, "the page shows two pieces: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(210.0), "the transform is in force: {pieces:?}");
    assert_eq!(pieces[1]["x"], json!(100.0), "and it is put back: {pieces:?}");
    assert_eq!(pieces[1]["y"], json!(200.0), "and the text is where it was: {pieces:?}");
}

/// The **matrix of the page** is the transform the page begins with.
#[test]
fn the_matrix_of_the_page_is_the_transform_it_begins_with() {
    let mut session = session();
    let page = load(
        &mut session,
        "pagematrix",
        "BT /F1 12 Tf 1 0 0 1 100 200 Tm (Hello) Tj ET",
        "/Matrix [1 0 0 1 5 7]",
    );
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(105.0), "the page's own matrix is applied: {pieces:?}");
    assert_eq!(pieces[0]["y"], json!(207.0), "in both directions: {pieces:?}");
}

/// The **rise** moves the text up from the line it is on.
#[test]
fn the_rise_moves_the_text_up() {
    let mut session = session();
    let page = load(&mut session, "rise", "BT /F1 12 Tf 1 0 0 1 100 200 Tm 10 Ts (Hello) Tj ET", "");
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(100.0), "the text is where the matrix says: {pieces:?}");
    assert_eq!(pieces[0]["y"], json!(210.0), "and it is ten higher: {pieces:?}");
}
