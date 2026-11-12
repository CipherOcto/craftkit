//! The **appearance** of the mark of a redaction: the outline of what
//! it covers.

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
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-redactappearance-{name}-{}", std::process::id()));
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

/// The **region** the text is under.
fn region() -> craftkit_sdk::serde_json::Value {
    json!([0.0, 690.0, 595.0, 720.0])
}

/// A **mark** on a page: the node of it.
fn a_mark(session: &mut Session, name: &str) -> u64 {
    let page = a_page(session, name);
    let outcome = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
            }),
        )
        .expect("the mark is added");
    outcome.data["id"].as_u64().expect("the mark is there")
}

/// **The mark is drawn:** the outline of the text it covers.
#[test]
fn the_mark_is_drawn() {
    let mut session = session();
    let mark = a_mark(&mut session, "redact");
    let directory = directory("drawing");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tree's own** red, and the line is one point wide.
    assert!(bytes.contains("0.89 0.13 0.13 RG\n1 w\n"), "the mark has its colour: {bytes}");
    // The **outline** of the quad: the upper left, the upper right,
    // the lower right and the lower left.
    assert!(bytes.contains(" m "), "the outline is drawn: {bytes}");
    assert!(bytes.contains(" l "), "the outline is drawn: {bytes}");
    assert!(bytes.ends_with("h S\n"), "the outline is closed and stroked: {bytes}");
}

/// **The fill** of the mark comes when the redaction is applied, and
/// not before: the drawing has no fill of its own.
#[test]
fn the_fill_comes_later() {
    let mut session = session();
    let mark = a_mark(&mut session, "fill");
    let directory = directory("fill");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(!bytes.contains(" f\n"), "the mark is not filled: {bytes}");
}

/// **A mark without a colour** is the tree's own red.
#[test]
fn a_mark_without_a_colour_is_the_trees_red() {
    let mut session = session();
    let mark = a_mark(&mut session, "nocolour");
    session
        .execute("pdf.entry.remove", &json!({ "object": mark, "key": "C" }))
        .expect("the colour is taken away");
    let directory = directory("red");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("0.89 0.13 0.13 RG"), "the mark is the tree's red: {bytes}");
}

/// **A mark without its quads** is refused.
#[test]
fn a_mark_without_its_quads_is_refused() {
    let mut session = session();
    let mark = a_mark(&mut session, "noquads");
    session
        .execute("pdf.entry.remove", &json!({ "object": mark, "key": "QuadPoints" }))
        .expect("the quads are taken away");
    let directory = directory("noquads");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
