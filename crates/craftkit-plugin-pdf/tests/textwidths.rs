//! The **width** of the text of a page.
//!
//! The width is the advance of the text: how far it goes along its
//! own line.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-textwidths-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given, and a font whose
/// **widths** are given.
fn a_file(content: &str, widths: Option<&str>) -> Vec<u8> {
    let font = match widths {
        Some(widths) => {
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths {widths} >>"
            )
        }
        None => "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica >>".to_string(),
    };
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, font),
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

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str, content: &str, widths: Option<&str>) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content, widths)).expect("the file is written");
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

/// The **width** of the text is the width of its codes: the tree's
/// own rule.
#[test]
fn the_width_of_the_text_is_the_width_of_its_codes() {
    let mut session = session();
    let page = load(&mut session, "width", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (AB) Tj ET", Some("[500 600]"));
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    // Five hundred and six hundred thousandths of an em, at twelve
    // points: six and a bit.
    assert!(
        (pieces[0]["width"].as_f64().unwrap_or_default() - 13.2).abs() < 1e-9,
        "the text is that wide: {pieces:?}"
    );
}

/// The text **moves on** by its width: the second piece is after the
/// first.
#[test]
fn the_text_moves_on_by_its_width() {
    let mut session = session();
    let page =
        load(&mut session, "moves", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj (B) Tj ET", Some("[500 600]"));
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 2, "the page shows two pieces: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(100.0), "the first is where the matrix says: {pieces:?}");
    assert!(
        (pieces[1]["x"].as_f64().unwrap_or_default() - 106.0).abs() < 1e-9,
        "the second is after the first: {pieces:?}"
    );
}

/// A font with **no widths** has no width: the width of its text is
/// not known, and the text does not move.
#[test]
fn a_font_with_no_widths_has_no_width() {
    let mut session = session();
    let page = load(&mut session, "nowidths", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (AB) Tj ET", None);
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    assert_eq!(pieces[0]["width"], Json::Null, "and the width is not known: {pieces:?}");
}

/// The **character spacing** and the **word spacing** are the tree's
/// own: a space is wider by the word spacing.
#[test]
fn the_spacings_are_the_tree_s_own() {
    let mut session = session();
    let page = load(&mut session, "spacing", "BT /F1 12 Tf 2 Tc 1 0 0 1 100 200 Tm (A) Tj ET", Some("[500]"));
    let pieces = pieces_of(&mut session, page);
    // Five hundred thousandths of an em at twelve points, and two
    // more for the spacing.
    assert!(
        (pieces[0]["width"].as_f64().unwrap_or_default() - 8.0).abs() < 1e-9,
        "the spacing is added: {pieces:?}"
    );
}
