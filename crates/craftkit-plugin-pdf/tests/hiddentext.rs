//! The **text of a page that is not seen**.
//!
//! The text drawn in the invisible mode, and the text that is wholly
//! off the page, are not seen.

use craftkit_sdk::serde_json::json;
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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-hiddentext-{name}-{}", std::process::id()))
        .expect("the directory is made");
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
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (
            5,
            "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths [500] \
             /FontDescriptor << /Ascent 800 /Descent -200 >> >>"
                .to_string(),
        ),
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

/// The **pieces** of text that are not seen.
fn hidden_of(session: &mut Session, page: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("pdf.page.text.hidden", &json!({ "page": page })).expect("the text is read");
    outcome.data["pieces"].as_array().expect("the pieces are a list").clone()
}

/// A text drawn in the **invisible** mode is not seen.
#[test]
fn a_text_in_the_invisible_mode_is_not_seen() {
    let mut session = session();
    let page = load(&mut session, "invisible", "BT /F1 12 Tf 3 Tr 1 0 0 1 100 200 Tm (A) Tj ET");
    let pieces = hidden_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the text is not seen: {pieces:?}");
    assert_eq!(pieces[0]["mode"], json!(3), "and the mode says why: {pieces:?}");
    assert_eq!(pieces[0]["why"], json!("invisible"), "and the answer says so too: {pieces:?}");
}

/// A text drawn in the mode that only **clips** is not seen either.
#[test]
fn a_text_in_the_clipping_mode_is_not_seen() {
    let mut session = session();
    let page = load(&mut session, "clip", "BT /F1 12 Tf 7 Tr 1 0 0 1 100 200 Tm (A) Tj ET");
    let pieces = hidden_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the text is not seen: {pieces:?}");
    assert_eq!(pieces[0]["mode"], json!(7), "and the mode says why: {pieces:?}");
}

/// A text that is **wholly off the page** is not seen.
#[test]
fn a_text_wholly_off_the_page_is_not_seen() {
    let mut session = session();
    let page = load(&mut session, "off", "BT /F1 12 Tf 1 0 0 1 1000 200 Tm (A) Tj ET");
    let pieces = hidden_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the text is not seen: {pieces:?}");
    assert_eq!(pieces[0]["why"], json!("off the page"), "and the answer says why: {pieces:?}");
}

/// A text that is **seen** is not reported: the ordinary text of a
/// page is not hidden.
#[test]
fn a_text_that_is_seen_is_not_reported() {
    let mut session = session();
    let page = load(&mut session, "seen", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET");
    let pieces = hidden_of(&mut session, page);
    assert!(pieces.is_empty(), "the text is seen: {pieces:?}");
}
