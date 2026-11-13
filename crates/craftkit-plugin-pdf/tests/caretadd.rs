//! The **caret** on a page: the tree's own inserted-text mark.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-caretadd-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with a **page** on it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len())),
    ];
    for (number, body) in &bodies {
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
fn a_page(session: &mut Session, directory: &str) -> u64 {
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

/// The **entries** of the dictionary of a node.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    node.data["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(
    entries: &'a [(String, craftkit_sdk::serde_json::Value)],
    key: &str,
) -> Option<&'a craftkit_sdk::serde_json::Value> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// **A caret is put on a page:** its rectangle, its colour and the page
/// that names it.
#[test]
fn a_caret_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("caret");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 110.0, 720.0],
            }),
        )
        .expect("the caret is added");
    assert!(outcome.changed);
    let caret = outcome.data["id"].as_u64().expect("the caret is there");
    let node = session.document().node(caret).cloned().expect("the caret is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Caret")), "the caret is a caret");
    assert_eq!(entry(&entries, "Rect"), Some(&json!([100.0, 700.0, 110.0, 720.0])), "the rectangle");
    // The **tree's own** blue.
    assert_eq!(entry(&entries, "C"), Some(&json!([0.0, 0.47, 0.84])), "the colour is the tree's");
    // The **symbol** is the tree's own: none.
    assert_eq!(entry(&entries, "Sy"), Some(&json!("None")), "the symbol is the tree's");
}

/// **The corners of the rectangle** may come in any order.
#[test]
fn the_corners_may_come_in_any_order() {
    let mut session = session();
    let directory = directory("corners");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [110.0, 720.0, 100.0, 700.0],
            }),
        )
        .expect("the caret is added");
    assert_eq!(outcome.data["rect"], json!([100.0, 700.0, 110.0, 720.0]), "the rectangle");
}

/// **A caret that is too small** is refused.
#[test]
fn a_caret_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("small");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 100.5, 700.0],
            }),
        )
        .expect_err("the caret is refused");
    assert!(error.to_string().contains("a point wide"), "the error says why: {error}");
}

/// **A rectangle that is not four numbers** is refused.
#[test]
fn a_rectangle_that_is_not_four_numbers_is_refused() {
    let mut session = session();
    let directory = directory("rect");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0],
            }),
        )
        .expect_err("the caret is refused");
    assert!(error.to_string().contains("four numbers"), "the error says why: {error}");
}

/// **A colour outside zero and one** is refused.
#[test]
fn a_colour_outside_zero_and_one_is_refused() {
    let mut session = session();
    let directory = directory("colour");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 110.0, 720.0],
                "colour": [0.0, 0.47, 2.0],
            }),
        )
        .expect_err("the caret is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}

/// **The text that was inserted** is the contents of the annotation.
#[test]
fn the_text_is_the_contents() {
    let mut session = session();
    let directory = directory("contents");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 110.0, 720.0],
                "contents": "inserted",
            }),
        )
        .expect("the caret is added");
    let caret = outcome.data["id"].as_u64().expect("the caret is there");
    let node = session.document().node(caret).cloned().expect("the caret is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Contents"), Some(&json!({ "string": "inserted" })), "the text is there");
}
