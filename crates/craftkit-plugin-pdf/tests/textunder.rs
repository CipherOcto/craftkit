//! The **text under a region**.
//!
//! A region is a rectangle of the page, and the text of a page has a
//! box: this is what a region covers.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-textunder-{name}-{}", std::process::id()))
        .expect("the directory is made");
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

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str, content: &str, font: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content, font)).expect("the file is written");
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

/// The **pieces** of text under a region.
fn under(session: &mut Session, page: u64, rect: Json) -> Vec<Json> {
    let outcome = session
        .execute("pdf.page.text.under", &json!({ "page": page, "rect": rect }))
        .expect("the text under the region is read");
    outcome.data["pieces"].as_array().expect("the pieces are a list").clone()
}

/// A font with a **descriptor**: the box of its text is known.
fn a_font(widths: &str) -> String {
    format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths {widths} \
         /FontDescriptor << /Ascent 800 /Descent -200 >> >>"
    )
}

/// The text **under** a region is the text the region covers.
#[test]
fn the_text_under_a_region_is_the_text_it_covers() {
    let mut session = session();
    let page = load(&mut session, "covered", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", &a_font("[500]"));
    let pieces = under(&mut session, page, json!([95, 195, 110, 210]));
    assert_eq!(pieces.len(), 1, "the region covers the text: {pieces:?}");
    assert_eq!(pieces[0]["text"], json!("A"), "and the text is the text: {pieces:?}");
    assert_eq!(pieces[0]["box"], json!([100.0, 197.6, 106.0, 209.6]), "and its box is its box: {pieces:?}");
}

/// A **neighbour** that only touches an edge survives: more than a
/// fifth of the text, or a point, must be under the region.
#[test]
fn a_neighbour_that_only_touches_an_edge_survives() {
    let mut session = session();
    let page = load(&mut session, "edge", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", &a_font("[500]"));
    let pieces = under(&mut session, page, json!([106, 195, 120, 210]));
    assert!(pieces.is_empty(), "the text only touches the region: {pieces:?}");
}

/// A box with **nothing in it** is under the region when it begins
/// inside it.
#[test]
fn a_box_with_nothing_in_it_begins_inside() {
    let mut session = session();
    let page = load(&mut session, "degenerate", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", &a_font("[0]"));
    let pieces = under(&mut session, page, json!([95, 195, 110, 210]));
    assert_eq!(pieces.len(), 1, "the text begins inside the region: {pieces:?}");
    let outside = under(&mut session, page, json!([200, 195, 300, 210]));
    assert!(outside.is_empty(), "and it does not when it begins elsewhere: {outside:?}");
}

/// A region that is **not a rectangle** is refused.
#[test]
fn a_region_that_is_not_a_rectangle_is_refused() {
    let mut session = session();
    let page = load(&mut session, "refused", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", &a_font("[500]"));
    let error = session
        .execute("pdf.page.text.under", &json!({ "page": page, "rect": [1, 2, 3] }))
        .expect_err("three numbers are not a rectangle");
    assert!(error.to_string().contains("rectangle"), "the refusal says why: {error}");
}
