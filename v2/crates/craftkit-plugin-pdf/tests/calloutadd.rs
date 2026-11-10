//! The **callout** on a page: the tree's own free text with a leader.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-calloutadd-{name}-{}", std::process::id()));
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

/// **A callout is put on a page:** the leader, the intent and the box
/// inside the rectangle.
#[test]
fn a_callout_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("callout");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.callout.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [300.0, 700.0, 400.0, 740.0],
                "knee": [250.0, 680.0],
                "point": [200.0, 650.0],
                "contents": "a note",
            }),
        )
        .expect("the callout is added");
    assert!(outcome.changed);
    let annot = outcome.data["id"].as_u64().expect("the callout is there");
    let node = session.document().node(annot).cloned().expect("the callout is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("FreeText")), "the callout is free text");
    // The **intent** is the tree's own.
    assert_eq!(entry(&entries, "IT"), Some(&json!("FreeTextCallout")), "the intent is the tree's");
    // The **leader** is the three points, and its end is an open arrow.
    assert_eq!(
        entry(&entries, "CL"),
        Some(&json!([200.0, 650.0, 250.0, 680.0, 300.0, 720.0])),
        "the leader is the tree's own"
    );
    assert_eq!(entry(&entries, "LE"), Some(&json!("OpenArrow")), "the arrow is the tree's");
}

/// **The rectangle** is the box and the leader together, grown by half
/// the width of the line, by the size of the arrow and by one point
/// more.
#[test]
fn the_rectangle_is_the_box_and_the_leader() {
    let mut session = session();
    let directory = directory("rect");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.callout.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [300.0, 700.0, 400.0, 740.0],
                "knee": [250.0, 680.0],
                "point": [200.0, 650.0],
                "contents": "a note",
            }),
        )
        .expect("the callout is added");
    // The size of the arrow is nine points, and the pad is half a
    // point and that and one point more: ten and a half. The **box**
    // is not shrunk: only the leader is grown, and the rectangle is
    // the two together. The leader reaches no further right than the
    // box, so the right side of the rectangle is the box's own.
    assert_eq!(outcome.data["rect"], json!([189.5, 639.5, 400.0, 740.0]), "the rectangle is the tree's");
    // And the **inset** of the box inside it.
    assert_eq!(outcome.data["inset"], json!([110.5, 60.5, 0.0, 0.0]), "the inset is the tree's");
}

/// **The leader meets the box** at the middle of the side that faces
/// the knee.
#[test]
fn the_leader_meets_the_box() {
    let mut session = session();
    let directory = directory("attach");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.callout.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [300.0, 700.0, 400.0, 740.0],
                "knee": [250.0, 680.0],
                "point": [200.0, 650.0],
                "contents": "a note",
            }),
        )
        .expect("the callout is added");
    // The knee is to the left of the box, so the leader meets the left
    // side of it, at the middle of its height.
    assert_eq!(outcome.data["attach"], json!([300.0, 720.0]), "the leader meets the left");
}

/// **A box that is too small** is refused.
#[test]
fn a_box_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("small");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.callout.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [300.0, 700.0, 300.5, 700.0],
                "knee": [250.0, 680.0],
                "point": [200.0, 650.0],
                "contents": "a note",
            }),
        )
        .expect_err("the callout is refused");
    assert!(error.to_string().contains("a point wide"), "the error says why: {error}");
}

/// **A knee that is not two numbers** is refused.
#[test]
fn a_knee_that_is_not_two_numbers_is_refused() {
    let mut session = session();
    let directory = directory("knee");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.callout.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [300.0, 700.0, 400.0, 740.0],
                "knee": [250.0],
                "point": [200.0, 650.0],
                "contents": "a note",
            }),
        )
        .expect_err("the callout is refused");
    assert!(error.to_string().contains("two numbers"), "the error says why: {error}");
}
