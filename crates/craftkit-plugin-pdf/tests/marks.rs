//! The **marks** of a page: the watermark, the header, the footer and
//! the background — and the taking of them away.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-marks-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
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

/// Load the file and give the **page**.
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

/// The **bytes** of the page's content.
fn the_content(session: &mut Session) -> String {
    let stream =
        session.document().walk().find(|node| node.kind == "pdf.stream").expect("the content is there");
    std::fs::read_to_string(stream.stream.clone().expect("the stream has a file"))
        .expect("the bytes are there")
}

/// **A watermark is taken away.**
#[test]
fn a_watermark_is_taken_away() {
    let mut session = session();
    let page = a_page(&mut session, "watermark");
    session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "CONFIDENTIAL", "size": 40.0 }))
        .expect("the watermark is drawn");
    let outcome = session
        .execute("pdf.page.marks.remove", &json!({ "page": page, "kind": "watermark" }))
        .expect("the mark is taken away");
    assert_eq!(outcome.data["removed"], json!(1), "one mark is taken: {outcome:?}");
    let content = the_content(&mut session);
    assert!(!content.contains("CONFIDENTIAL"), "the watermark is gone: {content}");
    assert!(content.contains("10 10 100 100 re f"), "and the page's own content is there: {content}");
}

/// **A header and a footer are taken away together.**
#[test]
fn a_header_and_a_footer_are_taken_away_together() {
    let mut session = session();
    let page = a_page(&mut session, "header");
    session
        .execute("pdf.page.header", &json!({ "page": page, "left": "HEAD" }))
        .expect("the header is drawn");
    session
        .execute("pdf.page.footer", &json!({ "page": page, "left": "FOOT" }))
        .expect("the footer is drawn");
    let outcome = session
        .execute("pdf.page.marks.remove", &json!({ "page": page, "kind": "headerfooter" }))
        .expect("the marks are taken away");
    assert_eq!(outcome.data["removed"], json!(2), "both marks are taken: {outcome:?}");
    let content = the_content(&mut session);
    assert!(!content.contains("HEAD"), "the header is gone: {content}");
    assert!(!content.contains("FOOT"), "and the footer: {content}");
}

/// **A mark of another kind is left alone.**
#[test]
fn a_mark_of_another_kind_is_left_alone() {
    let mut session = session();
    let page = a_page(&mut session, "other");
    session
        .execute("pdf.page.watermark", &json!({ "page": page, "text": "KEEP", "size": 40.0 }))
        .expect("the watermark is drawn");
    let outcome = session
        .execute("pdf.page.marks.remove", &json!({ "page": page, "kind": "background" }))
        .expect("the marks are taken away");
    assert_eq!(outcome.data["removed"], json!(0), "no mark is taken: {outcome:?}");
    let content = the_content(&mut session);
    assert!(content.contains("KEEP"), "the watermark is still there: {content}");
}

/// **A kind that is not one of the tree's own is refused.**
#[test]
fn a_kind_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "kind");
    let refusal = session
        .execute("pdf.page.marks.remove", &json!({ "page": page, "kind": "stamp" }))
        .expect_err("the kind is refused");
    let message = refusal.to_string();
    assert!(message.contains("kind"), "the refusal says so: {message}");
}
