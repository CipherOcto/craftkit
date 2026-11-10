//! The **text box** on a page: the tree's own free text.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-freetextadd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
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

/// **A text box is put on a page:** its text, its appearance and the
/// page that names it.
#[test]
fn a_text_box_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("freetext");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
            }),
        )
        .expect("the text box is added");
    assert!(outcome.changed);
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let node = session.document().node(box_of).cloned().expect("the text box is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("FreeText")), "the box is free text");
    assert_eq!(entry(&entries, "Contents"), Some(&json!("a note")), "the text is there");
    // The **appearance of the text** is the tree's own string.
    assert_eq!(
        entry(&entries, "DA"),
        Some(&json!("0 0 0 rg /Helv 12 Tf")),
        "the appearance is the tree's own"
    );
    // The **alignment** is left, the tree's own zero.
    assert_eq!(entry(&entries, "Q"), Some(&json!(0)), "the alignment is the tree's");
}

/// **The background** of the box is written when the caller says one.
#[test]
fn the_background_is_written_when_it_is_said() {
    let mut session = session();
    let directory = directory("background");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "background": [1.0, 1.0, 0.8],
            }),
        )
        .expect("the text box is added");
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let node = session.document().node(box_of).cloned().expect("the text box is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "C"), Some(&json!([1.0, 1.0, 0.8])), "the background is there");
}

/// **The size and the colour** the caller says are the tree's own
/// string.
#[test]
fn the_size_and_the_colour_are_the_trees_own() {
    let mut session = session();
    let directory = directory("size");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "size": 18.0,
                "colour": [1.0, 0.0, 0.0],
            }),
        )
        .expect("the text box is added");
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let node = session.document().node(box_of).cloned().expect("the text box is there");
    let entries = entries_of(&node);
    assert_eq!(
        entry(&entries, "DA"),
        Some(&json!("1 0 0 rg /Helv 18 Tf")),
        "the appearance is the tree's own"
    );
}

/// **A box that is too small** is refused.
#[test]
fn a_box_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("small");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 100.5, 700.0],
                "contents": "a note",
            }),
        )
        .expect_err("the text box is refused");
    assert!(error.to_string().contains("a point wide"), "the error says why: {error}");
}

/// **A size outside the tree's bounds** is refused.
#[test]
fn a_size_outside_the_trees_bounds_is_refused() {
    let mut session = session();
    let directory = directory("bigsize");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "size": 500.0,
            }),
        )
        .expect_err("the text box is refused");
    assert!(error.to_string().contains("four hundred"), "the error says why: {error}");
}

/// **An alignment that is not one of the tree's three** is refused.
#[test]
fn an_alignment_that_is_not_known_is_refused() {
    let mut session = session();
    let directory = directory("align");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "align": 5.0,
            }),
        )
        .expect_err("the text box is refused");
    assert!(error.to_string().contains("0, 1 or 2"), "the error says why: {error}");
}

/// **A width below zero** is the tree's own zero.
#[test]
fn a_width_below_zero_is_zero() {
    let mut session = session();
    let directory = directory("width");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "width": -2.0,
            }),
        )
        .expect("the text box is added");
    assert_eq!(outcome.data["width"], json!(0.0), "the width is the tree's zero");
    // And there is no border in the dictionary.
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let node = session.document().node(box_of).cloned().expect("the text box is there");
    let entries = entries_of(&node);
    assert!(entry(&entries, "BS").is_none(), "there is no border: {entries:?}");
}

/// **A colour outside zero and one** is refused.
#[test]
fn a_colour_outside_zero_and_one_is_refused() {
    let mut session = session();
    let directory = directory("colour");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "colour": [0.0, 0.0, 1.5],
            }),
        )
        .expect_err("the text box is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}
