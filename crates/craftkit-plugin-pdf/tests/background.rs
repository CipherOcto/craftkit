//! The **background** of a page: a colour behind the content of it.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-background-{name}-{}", std::process::id()))
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

/// **A background is drawn behind the content.**
#[test]
fn a_background_is_drawn_behind_the_content() {
    let mut session = session();
    let page = a_page(&mut session, "drawn");
    session
        .execute("pdf.page.background", &json!({ "page": page, "colour": "#ffee00" }))
        .expect("the background is drawn");
    let content = the_content(&mut session);
    assert!(content.contains("/Subtype /Background"), "the background is marked as such: {content}");
    assert!(content.contains("1 0.9333 0 rg"), "and it is the colour that was asked for: {content}");
    assert!(content.contains("0 0 595 842 re f"), "and it covers the page: {content}");
    let background = content.find("re f").expect("the rectangle is in it");
    let drawing = content.find("10 10 100 100 re f").expect("the drawing is in it");
    assert!(background < drawing, "the background comes first: {content}");
}

/// **The page's resources carry the graphics state.**
#[test]
fn the_pages_resources_carry_the_graphics_state() {
    let mut session = session();
    let page = a_page(&mut session, "resources");
    session
        .execute("pdf.page.background", &json!({ "page": page, "opacity": 0.25 }))
        .expect("the background is drawn");
    let node = session.document().node(page).expect("the page is there");
    let entries = node.data["entries"].as_array().expect("the entries are a list");
    let resources =
        entries.iter().find(|entry| entry[0] == json!("Resources")).expect("the resources are there");
    let text = resources.to_string();
    assert!(text.contains("PCGS25"), "the state is there: {text}");
}

/// **An opacity outside the scale is refused.**
#[test]
fn an_opacity_outside_the_scale_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "opacity");
    let refusal = session
        .execute("pdf.page.background", &json!({ "page": page, "opacity": 2.0 }))
        .expect_err("the opacity is refused");
    let message = refusal.to_string();
    assert!(message.contains("opacity"), "the refusal says so: {message}");
}
