//! The **watermark** of a page: a text drawn over it.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-watermark-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file of PDF with a page and a line of **content** on it.
fn a_file() -> Vec<u8> {
    let content = String::from("0 0 1 rg 10 10 100 100 re f\n");
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}endstream", content.len())),
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

/// Load the file, give the **page** and the **bytes** of its content.
fn a_page(session: &mut Session, name: &str) -> (u64, String) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
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
    let stream =
        session.document().walk().find(|node| node.kind == "pdf.stream").expect("the content is there");
    let bytes = std::fs::read_to_string(stream.stream.clone().expect("the stream has a file"))
        .expect("the bytes are there");
    (page, bytes)
}

/// **A watermark is drawn on the page.**
#[test]
fn a_watermark_is_drawn_on_the_page() {
    let mut session = session();
    let (page, _) = a_page(&mut session, "drawn");
    let outcome = session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "CONFIDENTIAL", "size": 40.0 }))
        .expect("the watermark is drawn");
    assert_eq!(outcome.data["text"], json!("CONFIDENTIAL"), "the text is said: {outcome:?}");
    let (_, content) = a_page_after(&mut session);
    assert!(content.contains("/Subtype /Watermark"), "the watermark is marked as such: {content}");
    assert!(content.contains("/PCHelv 40 Tf"), "and it is set in the tree's Helvetica: {content}");
    assert!(content.contains("(CONFIDENTIAL) Tj"), "and it says its text: {content}");
    assert!(content.contains("0.6 0.6 0.6 rg"), "in the tree's grey: {content}");
}

/// **The watermark is behind the content when it is asked for.**
#[test]
fn the_watermark_is_behind_the_content_when_it_is_asked_for() {
    let mut session = session();
    let (page, _) = a_page(&mut session, "behind");
    session
        .execute(
            "pdf.page.watermark",
            &json!({ "page": page, "text": "DRAFT", "size": 40.0, "behind": true }),
        )
        .expect("the watermark is drawn");
    let (_, content) = a_page_after(&mut session);
    let watermark = content.find("CONFIDENTIAL").or_else(|| content.find("DRAFT")).unwrap_or(0);
    let drawing = content.find(" re f").expect("the drawing is in it");
    assert!(watermark < drawing, "the watermark comes first: {content}");
}

/// **The size is the tree's own when it is not asked for.**
#[test]
fn the_size_is_the_trees_own_when_it_is_not_asked_for() {
    let mut session = session();
    let (page, _) = a_page(&mut session, "size");
    let outcome = session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "SECRET" }))
        .expect("the watermark is drawn");
    let size = outcome.data["size"].as_f64().expect("the size is a number");
    assert!((6.0..=300.0).contains(&size), "the size is the tree's: {outcome:?}");
    let (_, content) = a_page_after(&mut session);
    assert!(content.contains("/PCHelv "), "and the file sets the size: {content}");
    assert!(content.contains("Tf"), "and the file sets the size: {content}");
}

/// **The page's resources carry the tree's Helvetica.**
#[test]
fn the_pages_resources_carry_the_trees_helvetica() {
    let mut session = session();
    let (page, _) = a_page(&mut session, "resources");
    session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "COPY", "size": 20.0 }))
        .expect("the watermark is drawn");
    let node = session.document().node(page).expect("the page is there");
    let entries = node.data["entries"].as_array().expect("the entries are a list");
    let resources =
        entries.iter().find(|entry| entry[0] == json!("Resources")).expect("the resources are there");
    let text = resources.to_string();
    assert!(text.contains("PCHelv"), "the font is there: {text}");
    assert!(text.contains("ExtGState"), "and the graphics state: {text}");
    assert!(text.contains("PCGS50"), "with the tree's name for the opacity: {text}");
}

/// **A watermark with no text is refused.**
#[test]
fn a_watermark_with_no_text_is_refused() {
    let mut session = session();
    let (page, _) = a_page(&mut session, "empty");
    let refusal = session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "  " }))
        .expect_err("the watermark is refused");
    let message = refusal.to_string();
    assert!(message.contains("text"), "the refusal says so: {message}");
}

/// **An opacity outside the scale is refused.**
#[test]
fn an_opacity_outside_the_scale_is_refused() {
    let mut session = session();
    let (page, _) = a_page(&mut session, "opacity");
    let refusal = session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "COPY", "opacity": 1.5 }))
        .expect_err("the opacity is refused");
    let message = refusal.to_string();
    assert!(message.contains("opacity"), "the refusal says so: {message}");
}

/// The **bytes** of the page's content, after the watermark is drawn.
fn a_page_after(session: &mut Session) -> (u64, String) {
    let stream =
        session.document().walk().find(|node| node.kind == "pdf.stream").expect("the content is there");
    let bytes = std::fs::read_to_string(stream.stream.clone().expect("the stream has a file"))
        .expect("the bytes are there");
    (stream.id, bytes)
}
