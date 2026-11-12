//! The **note** that is put on a page.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-commentadd-{name}-{}", std::process::id()));
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

/// The **node** of an id.
fn node_of(session: &Session, id: u64) -> Option<craftkit_sdk::Node> {
    session.document().node(id).cloned()
}

/// **A note is put on a page, and the icon hangs below the point.**
#[test]
fn a_note_is_put_on_a_page_and_the_icon_hangs_below_the_point() {
    let mut session = session();
    let directory = directory("put");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let outcome = session
        .execute(
            "pdf.comment.add",
            &json!({
                "page": page,
                "at": [100.0, 700.0],
                "contents": "A note",
                "author": "Ada",
                "icon": "Note",
                "date": "D:20260101120000",
                "to": to,
            }),
        )
        .expect("the note is put");
    assert_eq!(
        outcome.data["rect"],
        json!([100.0, 680.0, 120.0, 700.0]),
        "the icon hangs below: {outcome:?}"
    );
    let note = node_of(&session, outcome.data["note"].as_u64().expect("the note has an id"))
        .expect("the note is there");
    let entries = entries_of(&note);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Text")), "it is a text annotation: {entries:?}");
    assert_eq!(
        entry(&entries, "Contents").and_then(|text| text["string"].as_str()),
        Some("A note"),
        "the text is said: {entries:?}"
    );
    assert_eq!(
        entry(&entries, "T").and_then(|text| text["string"].as_str()),
        Some("Ada"),
        "the author is said: {entries:?}"
    );
    assert_eq!(entry(&entries, "Name"), Some(&json!("Note")), "the icon is named: {entries:?}");
    assert_eq!(entry(&entries, "Subj"), Some(&json!("Sticky Note")), "the subject is said: {entries:?}");
    assert_eq!(entry(&entries, "F"), Some(&json!(28)), "it prints, and it does not zoom: {entries:?}");
    assert_eq!(entry(&entries, "Open"), Some(&json!(false)), "it is shut: {entries:?}");
    assert_eq!(entry(&entries, "C"), Some(&json!([1.0, 0.82, 0.0])), "the colour is the tree's: {entries:?}");
}

/// **The popup of the note is beside the icon.**
#[test]
fn the_popup_of_the_note_is_beside_the_icon() {
    let mut session = session();
    let directory = directory("popup");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let outcome = session
        .execute("pdf.comment.add", &json!({ "page": page, "at": [100.0, 700.0], "to": to }))
        .expect("the note is put");
    assert_eq!(
        outcome.data["popup_rect"],
        json!([130.0, 580.0, 330.0, 700.0]),
        "the popup is the tree's: {outcome:?}"
    );
    let popup = node_of(&session, outcome.data["popup"].as_u64().expect("the popup has an id"))
        .expect("the popup is there");
    let entries = entries_of(&popup);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Popup")), "it is a popup: {entries:?}");
    assert_eq!(entry(&entries, "Open"), Some(&json!(false)), "it is shut: {entries:?}");
}

/// **The icon is drawn, and it is the tree's drawing.**
#[test]
fn the_icon_is_drawn_and_it_is_the_trees_drawing() {
    let mut session = session();
    let directory = directory("drawn");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    session
        .execute("pdf.comment.add", &json!({ "page": page, "at": [100.0, 700.0], "to": to }))
        .expect("the note is put");
    let icon = std::fs::read_to_string(&to).expect("the icon is there");
    assert!(icon.contains("1 0.82 0 rg"), "the colour is the tree's: {icon}");
    assert!(icon.contains("3 19 m 13 19 l"), "the folded page is drawn: {icon}");
    assert!(icon.contains("0.25 0.25 0.25 RG"), "and the line of it: {icon}");
}

/// **The bubble is drawn for the icon `Comment`.**
#[test]
fn the_bubble_is_drawn_for_the_icon_comment() {
    let mut session = session();
    let directory = directory("bubble");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    session
        .execute(
            "pdf.comment.add",
            &json!({ "page": page, "at": [100.0, 700.0], "icon": "Comment", "to": to }),
        )
        .expect("the note is put");
    let icon = std::fs::read_to_string(&to).expect("the icon is there");
    assert!(icon.contains("3 18.5 m 17 18.5 l"), "the bubble is drawn: {icon}");
}

/// **The note is a comment, and the reading of the comments finds it.**
#[test]
fn the_note_is_a_comment_and_the_reading_of_the_comments_finds_it() {
    let mut session = session();
    let directory = directory("found");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    session
        .execute(
            "pdf.comment.add",
            &json!({ "page": page, "at": [100.0, 700.0], "contents": "A note", "to": to }),
        )
        .expect("the note is put");
    let outcome = session.execute("pdf.comment.read", &json!({})).expect("the comments are read");
    assert_eq!(outcome.data["count"], json!(1), "the note is a comment: {outcome:?}");
    let comment = &outcome.data["comments"][0];
    assert_eq!(comment["subtype"], json!("Text"), "and it is a text annotation: {comment:?}");
    assert_eq!(comment["contents"], json!("A note"), "and it says what it says: {comment:?}");
}

/// **An icon that is not one of the tree's own is refused.**
#[test]
fn an_icon_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let directory = directory("icon");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let refusal = session
        .execute("pdf.comment.add", &json!({ "page": page, "at": [100.0, 700.0], "icon": "Star", "to": to }))
        .expect_err("the icon is refused");
    let message = refusal.to_string();
    assert!(message.contains("icon"), "the refusal says so: {message}");
}

/// **A point that is not two numbers is refused.**
#[test]
fn a_point_that_is_not_two_numbers_is_refused() {
    let mut session = session();
    let directory = directory("point");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let refusal = session
        .execute("pdf.comment.add", &json!({ "page": page, "at": [100.0], "to": to }))
        .expect_err("the point is refused");
    let message = refusal.to_string();
    assert!(message.contains("point"), "the refusal says so: {message}");
}

/// **The file that is written holds the note.**
#[test]
fn the_file_that_is_written_holds_the_note() {
    let mut session = session();
    let directory = directory("written");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    session
        .execute(
            "pdf.comment.add",
            &json!({ "page": page, "at": [100.0, 700.0], "contents": "A note", "author": "Ada", "to": to }),
        )
        .expect("the note is put");
    let path = format!("{directory}/notes.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let file = String::from_utf8_lossy(&std::fs::read(&path).expect("the file is there")).to_string();
    assert!(file.contains("/Subtype /Text"), "the note is in the file: {file}");
    assert!(file.contains("/Subtype /Popup"), "and its popup: {file}");
    assert!(file.contains("3 19 m 13 19 l"), "and the icon: {file}");
    assert!(file.contains("A note"), "and what it says: {file}");
}
