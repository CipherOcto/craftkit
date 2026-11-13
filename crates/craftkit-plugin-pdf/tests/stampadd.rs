//! The **stamp** on a page: the tree's own twenty two.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-stampadd-{name}-{}", std::process::id()))
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

/// **The stamp is put on a page:** its name, its colour and the tree's
/// own mark that it is the tree's.
#[test]
fn the_stamp_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("stamp");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": "Approved",
            }),
        )
        .expect("the stamp is added");
    assert!(outcome.changed);
    let stamp = outcome.data["id"].as_u64().expect("the stamp is there");
    let node = session.document().node(stamp).cloned().expect("the stamp is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Stamp")), "the stamp is a stamp");
    assert_eq!(entry(&entries, "Name"), Some(&json!("Approved")), "the name is the tree's");
    // The **tree's own** green for an approval.
    assert_eq!(entry(&entries, "C"), Some(&json!([0.13, 0.55, 0.13])), "the colour is the tree's");
    // The stamp says it is the tree's own.
    assert_eq!(entry(&entries, "PCStamp"), Some(&json!(true)), "the stamp is the tree's");
}

/// **A stamp that asks for a signature** is the tree's own orange.
#[test]
fn a_stamp_that_asks_for_a_signature_is_orange() {
    let mut session = session();
    let directory = directory("signhere");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": "PCSignHere",
            }),
        )
        .expect("the stamp is added");
    let stamp = outcome.data["id"].as_u64().expect("the stamp is there");
    let node = session.document().node(stamp).cloned().expect("the stamp is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "C"), Some(&json!([0.85, 0.35, 0.05])), "the colour is the tree's");
    assert_eq!(outcome.data["group"], json!("signhere"), "the group is the tree's");
}

/// **The line that says who did it** is written when the caller says
/// one.
#[test]
fn the_line_that_says_who_did_it_is_written() {
    let mut session = session();
    let directory = directory("by");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": "PCDynApproved",
                "by": "by A. Reader",
            }),
        )
        .expect("the stamp is added");
    let stamp = outcome.data["id"].as_u64().expect("the stamp is there");
    let node = session.document().node(stamp).cloned().expect("the stamp is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "PCByLine"), Some(&json!({ "string": "by A. Reader" })), "the line is there");
    assert_eq!(outcome.data["group"], json!("dynamic"), "the group is the tree's");
}

/// **A kind that is not one of the tree's twenty two** is refused.
#[test]
fn a_kind_that_is_not_known_is_refused() {
    let mut session = session();
    let directory = directory("kind");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": "Brilliant",
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("not a kind of stamp"), "the error says why: {error}");
}

/// **A stamp that is too small** is refused.
#[test]
fn a_stamp_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("small");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 100.5, 700.0],
                "kind": "Draft",
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("a point wide"), "the error says why: {error}");
}

/// **A colour outside zero and one** is refused.
#[test]
fn a_colour_outside_zero_and_one_is_refused() {
    let mut session = session();
    let directory = directory("colour");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": "Draft",
                "colour": [0.1, 0.3, 1.5],
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}

/// **The caller may say another colour**, and the tree's own is kept
/// when the caller says nothing.
#[test]
fn the_caller_may_say_another_colour() {
    let mut session = session();
    let directory = directory("other");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": "Draft",
                "colour": [0.2, 0.4, 0.6],
            }),
        )
        .expect("the stamp is added");
    let stamp = outcome.data["id"].as_u64().expect("the stamp is there");
    let node = session.document().node(stamp).cloned().expect("the stamp is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "C"), Some(&json!([0.2, 0.4, 0.6])), "the colour is the caller's");
}
