//! The **place** of the text on a page.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-places-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given.
fn a_file(content: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>".to_string()),
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
fn load(session: &mut Session, name: &str, content: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content)).expect("the file is written");
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

/// The **text** of the page, as the command reads it.
fn text_of(session: &mut Session, page: u64) -> Json {
    let outcome = session.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    outcome.data.clone()
}

/// The place of a text is where the **matrix** says it begins.
#[test]
fn the_place_of_a_text_is_where_the_matrix_says() {
    let mut session = session();
    let page = load(&mut session, "matrix", "BT 1 0 0 1 72 700 Tm (hello) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 1, "there is one piece");
    assert_eq!(pieces[0]["x"], json!(72.0), "and it is where the matrix says");
    assert_eq!(pieces[0]["y"], json!(700.0), "in both directions");
}

/// A **move** puts the text somewhere else.
#[test]
fn a_move_puts_the_text_somewhere_else() {
    let mut session = session();
    let page = load(&mut session, "move", "BT 72 700 Td (hello) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces[0]["x"], json!(72.0), "the move puts the text across");
    assert_eq!(pieces[0]["y"], json!(700.0), "and up");
}

/// A second piece is **moved** from the first: the place of a piece
/// is the place the pen is at.
#[test]
fn a_second_piece_is_moved_from_the_first() {
    let mut session = session();
    let page = load(&mut session, "second", "BT 72 700 Td (one) Tj 0 -14 Td (two) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 2, "there are two pieces");
    assert_eq!(pieces[0]["y"], json!(700.0), "the first is where the pen was put");
    assert_eq!(pieces[1]["y"], json!(686.0), "and the second is where it was moved to");
}

/// Text that is **not placed** has no place: the file did not say
/// where it is.
#[test]
fn text_that_is_not_placed_has_no_place() {
    let mut session = session();
    let page = load(&mut session, "unplaced", "BT (hello) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces[0]["text"], json!("hello"), "the text is read");
    assert_eq!(pieces[0]["x"], Json::Null, "and it has no place");
    assert_eq!(pieces[0]["y"], Json::Null, "in either direction");
}
