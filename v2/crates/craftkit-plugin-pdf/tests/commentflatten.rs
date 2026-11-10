//! The **flattening** of the comments of a file: they are drawn into
//! the pages they are on, and taken away.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-commentflat-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// The **content** of the appearance of a comment.
const APPEARANCE_CONTENT: &str = "0 1 0 rg 0 0 20 20 re f\n";

/// A file of PDF with a **comment** on its page, with its popup and a
/// reply to it, with a link and with a comment that is hidden.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Annots [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R] /Resources << >> >>"
                .to_string(),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len()),
        ),
        (
            5,
            "<< /Type /Annot /Subtype /Text /Rect [100 200 120 220] /AP << /N 10 0 R >> /T (Ada) /Contents (A note) >>"
                .to_string(),
        ),
        (6, "<< /Type /Annot /Subtype /Popup /Rect [100 200 400 400] /Parent 5 0 R /Subj (Note) >>".to_string()),
        (
            7,
            "<< /Type /Annot /Subtype /Text /Rect [10 500 30 520] /IRT 5 0 R /T (Cy) /Contents (A reply) >>"
                .to_string(),
        ),
        (8, "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] >>".to_string()),
        (
            9,
            "<< /Type /Annot /Subtype /Text /Rect [300 300 320 320] /AP << /N 10 0 R >> /F 2 /T (Di) >>"
                .to_string(),
        ),
        (
            10,
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >> /Length {} >>\nstream\n{APPEARANCE_CONTENT}endstream",
                APPEARANCE_CONTENT.len()
            ),
        ),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 11\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 11 /Root 1 0 R >>\nstartxref\n");
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

/// The **node** of an object number.
fn node_of(session: &Session, number: u64) -> Option<craftkit_sdk::Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(number))
        .cloned()
}

/// The **text** of every stream of the file.
fn every_stream(session: &Session) -> String {
    let mut out = String::new();
    for node in session.document().walk() {
        if node.kind != "pdf.stream" {
            continue;
        }
        let Some(path) = node.stream.clone() else { continue };
        let Ok(bytes) = std::fs::read_to_string(&path) else { continue };
        out.push_str(&bytes);
        out.push('\n');
    }
    out
}

/// **The comments are drawn and taken away.**
#[test]
fn the_comments_are_drawn_and_taken_away() {
    let mut session = session();
    let page = a_page(&mut session, "drawn");
    let outcome =
        session.execute("pdf.comment.flatten", &json!({ "page": page })).expect("the comments are flattened");
    assert_eq!(outcome.data["drawn"], json!(1), "one appearance is drawn: {outcome:?}");
    assert_eq!(outcome.data["taken"], json!(3), "the comment, its popup and its reply: {outcome:?}");
    assert_eq!(outcome.data["left"], json!(1), "the hidden comment is left: {outcome:?}");
    // The **drawing**: the tree's name and the tree's matrix.
    let content = every_stream(&session);
    assert!(content.contains("1 0 0 1 100 200 cm"), "the matrix is the tree's: {content}");
    assert!(content.contains("/PCFl0 Do"), "the appearance is drawn: {content}");
    // The **comment**, its **popup** and its **reply** are gone.
    assert!(node_of(&session, 5).is_none(), "the comment is taken away");
    assert!(node_of(&session, 6).is_none(), "the popup goes with it");
    assert!(node_of(&session, 7).is_none(), "and so does the reply");
}

/// **A link is left alone.**
#[test]
fn a_link_is_left_alone() {
    let mut session = session();
    let page = a_page(&mut session, "link");
    session.execute("pdf.comment.flatten", &json!({ "page": page })).expect("the comments are flattened");
    assert!(node_of(&session, 8).is_some(), "the link is still there");
}

/// **A hidden comment is left alone.**
#[test]
fn a_hidden_comment_is_left_alone() {
    let mut session = session();
    let page = a_page(&mut session, "hidden");
    session.execute("pdf.comment.flatten", &json!({ "page": page })).expect("the comments are flattened");
    assert!(node_of(&session, 9).is_some(), "the hidden comment is still there");
}

/// **The page keeps the annotations that are left.**
#[test]
fn the_page_keeps_the_annotations_that_are_left() {
    let mut session = session();
    let page = a_page(&mut session, "annots");
    session.execute("pdf.comment.flatten", &json!({ "page": page })).expect("the comments are flattened");
    let page_node = node_of(&session, 3).expect("the page is there");
    let annots = page_node.data["entries"]
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    if pair.first()?.as_str()? != "Annots" {
                        return None;
                    }
                    pair.get(1).cloned()
                })
                .next()
        })
        .and_then(|annots| annots.as_array().cloned())
        .unwrap_or_default();
    let numbers: Vec<f64> = annots.iter().filter_map(|annot| annot["ref"][0].as_f64()).collect();
    assert_eq!(numbers, vec![8.0, 9.0], "the link and the hidden comment are left: {numbers:?}");
}
