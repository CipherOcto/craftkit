//! The **header** and the **footer** of a page: the text at the top of
//! it and at the bottom of it.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-header-{name}-{}", std::process::id()))
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

/// **A header is drawn at the top of the page.**
#[test]
fn a_header_is_drawn_at_the_top_of_the_page() {
    let mut session = session();
    let page = a_page(&mut session, "header");
    session
        .execute("pdf.page.header", &json!({ "page": page, "left": "Report" }))
        .expect("the header is drawn");
    let content = the_content(&mut session);
    assert!(content.contains("/Subtype /Header"), "the header is marked as such: {content}");
    assert!(content.contains("(Report) Tj"), "and it says its text: {content}");
    assert!(content.contains("1 0 0 1 72 799.6"), "and it hangs below the top margin: {content}");
}

/// **A footer stands on the bottom margin.**
#[test]
fn a_footer_stands_on_the_bottom_margin() {
    let mut session = session();
    let page = a_page(&mut session, "footer");
    session
        .execute("pdf.page.footer", &json!({ "page": page, "left": "Page" }))
        .expect("the footer is drawn");
    let content = the_content(&mut session);
    assert!(content.contains("(Page) Tj"), "it says its text: {content}");
    assert!(content.contains("1 0 0 1 72 36"), "and it stands on the bottom margin: {content}");
}

/// **The tokens of the page number are expanded.**
#[test]
fn the_tokens_of_the_page_number_are_expanded() {
    let mut session = session();
    let page = a_page(&mut session, "tokens");
    session
        .execute("pdf.page.header", &json!({ "page": page, "centre": "<<1>> of <<n>>" }))
        .expect("the header is drawn");
    let content = the_content(&mut session);
    assert!(content.contains("(1 of 1) Tj"), "the tokens are expanded: {content}");
}

/// **The date token is expanded.**
#[test]
fn the_date_token_is_expanded() {
    let mut session = session();
    let page = a_page(&mut session, "date");
    session
        .execute("pdf.page.footer", &json!({ "page": page, "right": "<<yyyy-mm-dd>>", "date": [2026, 3, 7] }))
        .expect("the footer is drawn");
    let content = the_content(&mut session);
    assert!(content.contains("(2026-03-07) Tj"), "the date is said: {content}");
}

/// **The Bates number is the tree's own.**
#[test]
fn the_bates_number_is_the_trees_own() {
    let mut session = session();
    let page = a_page(&mut session, "bates");
    session
        .execute("pdf.page.footer", &json!({ "page": page, "right": "<<Bates Number#6#100#ABC#-X>>" }))
        .expect("the footer is drawn");
    let content = the_content(&mut session);
    assert!(content.contains("(ABC000100-X) Tj"), "the number is the tree's: {content}");
}

/// **A header with no text is refused.**
#[test]
fn a_header_with_no_text_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "empty");
    let refusal =
        session.execute("pdf.page.header", &json!({ "page": page })).expect_err("the header is refused");
    let message = refusal.to_string();
    assert!(message.contains("text"), "the refusal says so: {message}");
}

/// **A size that is not in the scale is refused.**
#[test]
fn a_size_that_is_not_in_the_scale_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "size");
    let refusal = session
        .execute("pdf.page.header", &json!({ "page": page, "left": "x", "size": 500.0 }))
        .expect_err("the size is refused");
    let message = refusal.to_string();
    assert!(message.contains("size"), "the refusal says so: {message}");
}
