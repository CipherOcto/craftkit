//! The **text under a region is removed**.
//!
//! The text is gone from the content of the page, and the rest of the
//! line keeps its place.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-redacttext-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given, and a **font** given.
fn a_file(content: &str, font: &str) -> Vec<u8> {
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
        (5, font.to_string()),
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

/// A font with a **descriptor**: the box of its text is known.
fn a_font(widths: &str) -> String {
    format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths {widths} \
         /FontDescriptor << /Ascent 800 /Descent -200 >> >>"
    )
}

/// Load a file into the document, and give the **page** it holds and
/// the file it came from.
fn load(session: &mut Session, name: &str, content: &str, font: &str) -> (u64, String) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content, font)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = session
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
        .expect("the page is there");
    (page, directory)
}

/// The **pieces** of text the page shows.
fn pieces_of(session: &mut Session, page: u64) -> Vec<Json> {
    let outcome = session.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    outcome.data["pieces"].as_array().expect("the pieces are a list").clone()
}

/// The text under a region is **gone** from the content of the page.
#[test]
fn the_text_under_a_region_is_gone_from_the_page() {
    let mut session = session();
    let (page, directory) =
        load(&mut session, "gone", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj (B) Tj ET", &a_font("[500 500]"));
    let outcome = session
        .execute("pdf.redact.text", &json!({ "page": page, "rect": [95, 195, 106, 210] }))
        .expect("the text is removed");
    assert_eq!(outcome.data["removed"], json!(1), "one piece is removed: {outcome:?}");
    let pieces = pieces_of(&mut session, page);
    assert!(
        pieces.iter().all(|piece| piece["text"] != json!("A")),
        "and the text that was there is not: {pieces:?}"
    );
    assert!(
        pieces.iter().any(|piece| piece["text"] == json!("B")),
        "and the one that was left is: {pieces:?}"
    );
    let written = std::fs::read(format!("{directory}/streams/4.bin")).expect("the stream is there");
    assert!(
        !written.windows(3).any(|window| window == b"(A)"),
        "and the text is gone from the file: {written:?}"
    );
}

/// The rest of the line keeps its **place**: the text that was taken
/// away leaves a space of its own.
#[test]
fn the_rest_of_the_line_keeps_its_place() {
    let mut session = session();
    let (page, _directory) =
        load(&mut session, "place", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj (B) Tj ET", &a_font("[500 500]"));
    session
        .execute("pdf.redact.text", &json!({ "page": page, "rect": [95, 195, 106, 210] }))
        .expect("the text is removed");
    let pieces = pieces_of(&mut session, page);
    let left = pieces.iter().find(|piece| piece["text"] == json!("B")).expect("the text that was left");
    assert_eq!(left["x"], json!(106.0), "and it is where it was: {pieces:?}");
}

/// Text that is **not** under the region is left alone: the file is
/// not written at all.
#[test]
fn text_that_is_not_under_the_region_is_left_alone() {
    let mut session = session();
    let content = "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj (B) Tj ET";
    let (page, directory) = load(&mut session, "alone", content, &a_font("[500]"));
    let outcome = session
        .execute("pdf.redact.text", &json!({ "page": page, "rect": [300, 300, 400, 400] }))
        .expect("the region is read");
    assert_eq!(outcome.data["removed"], json!(0), "nothing is removed: {outcome:?}");
    let written = std::fs::read(format!("{directory}/streams/4.bin")).expect("the stream is there");
    assert!(written.windows(3).any(|window| window == b"(A)"), "and the text is still there: {written:?}");
}

/// A region that is **not a rectangle** is refused.
#[test]
fn a_region_that_is_not_a_rectangle_is_refused() {
    let mut session = session();
    let (page, _directory) =
        load(&mut session, "refused", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", &a_font("[500 500]"));
    let error = session
        .execute("pdf.redact.text", &json!({ "page": page, "rect": [1, 2, 3] }))
        .expect_err("three numbers are not a rectangle");
    assert!(error.to_string().contains("rectangle"), "the refusal says why: {error}");
}
