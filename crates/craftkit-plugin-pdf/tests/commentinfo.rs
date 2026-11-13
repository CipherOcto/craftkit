//! The **author**, the **subject** and the **icon** of a comment.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-commentinfo-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with a **note**, a **highlight** and a **locked
/// note** on its page.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Annots [5 0 R 6 0 R 7 0 R] /Resources << >> >>"
                .to_string(),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len()),
        ),
        (
            5,
            "<< /Type /Annot /Subtype /Text /Rect [100 680 120 700] /T (Ada) /Contents (A note) /Name (Note) /C [1 0.82 0] /F 28 >>"
                .to_string(),
        ),
        (
            6,
            "<< /Type /Annot /Subtype /Highlight /Rect [10 100 200 120] /QuadPoints [10 100 200 100 10 120 200 120] /T (Bo) /F 4 >>"
                .to_string(),
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Text /Rect [10 500 30 520] /T (Di) /F 128 >>".to_string(),
        ),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 8\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **page** and the **comments** on it.
fn a_page(session: &mut Session, directory: &str) -> (u64, u64, u64, u64) {
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
    let note = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.object" && node.data["object"].as_u64() == Some(5))
        .map(|node| node.id)
        .expect("the note is there");
    let highlight = session
        .document()
        .walk()
        .find(|node| node.data["object"].as_u64() == Some(6))
        .map(|node| node.id)
        .expect("the highlight is there");
    let locked = session
        .document()
        .walk()
        .find(|node| node.data["object"].as_u64() == Some(7))
        .map(|node| node.id)
        .expect("the locked note is there");
    (page, note, highlight, locked)
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

/// **The author and the subject of a comment are changed.**
#[test]
fn the_author_and_the_subject_of_a_comment_are_changed() {
    let mut session = session();
    let directory = directory("author");
    let (_, note, _, _) = a_page(&mut session, &directory);
    session
        .execute("pdf.comment.info", &json!({ "comment": note, "author": "Cy", "subject": "A question" }))
        .expect("the comment is changed");
    let node = session.document().node(note).expect("the note is there");
    let entries = entries_of(node);
    assert_eq!(
        entry(&entries, "T").and_then(|text| text["string"].as_str()),
        Some("Cy"),
        "the author is said: {entries:?}"
    );
    assert_eq!(entry(&entries, "Subj"), Some(&json!("A question")), "the subject is said: {entries:?}");
}

/// **An empty author takes the author away.**
#[test]
fn an_empty_author_takes_the_author_away() {
    let mut session = session();
    let directory = directory("empty");
    let (_, note, _, _) = a_page(&mut session, &directory);
    session
        .execute("pdf.comment.info", &json!({ "comment": note, "author": "" }))
        .expect("the comment is changed");
    let node = session.document().node(note).expect("the note is there");
    let entries = entries_of(node);
    assert!(entry(&entries, "T").is_none(), "the author is gone: {entries:?}");
}

/// **The icon of a note is changed, and it is drawn again.**
#[test]
fn the_icon_of_a_note_is_changed_and_it_is_drawn_again() {
    let mut session = session();
    let directory = directory("icon");
    let (_, note, _, _) = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let outcome = session
        .execute("pdf.comment.info", &json!({ "comment": note, "icon": "Comment", "to": to }))
        .expect("the comment is changed");
    assert_eq!(outcome.data["icon"], json!("Comment"), "the icon is said: {outcome:?}");
    let node = session.document().node(note).expect("the note is there");
    let entries = entries_of(node);
    assert_eq!(entry(&entries, "Name"), Some(&json!("Comment")), "the icon is named: {entries:?}");
    // The **icon** is drawn again: the bubble.
    let icon = std::fs::read_to_string(&to).expect("the icon is there");
    assert!(icon.contains("3 18.5 m 17 18.5 l"), "the bubble is drawn: {icon}");
    // And the comment points at the drawing.
    let appearance = entry(&entries, "AP").expect("the appearance is there");
    let number = appearance["entries"][0][1]["ref"][0].as_f64().expect("the drawing is named");
    assert!(number > 0.0, "the drawing is a new one: {appearance:?}");
}

/// **An icon for a comment that is not a note is refused.**
#[test]
fn an_icon_for_a_comment_that_is_not_a_note_is_refused() {
    let mut session = session();
    let directory = directory("notnote");
    let (_, _, highlight, _) = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let refusal = session
        .execute("pdf.comment.info", &json!({ "comment": highlight, "icon": "Note", "to": to }))
        .expect_err("the icon is refused");
    let message = refusal.to_string();
    assert!(message.contains("note"), "the refusal says so: {message}");
}

/// **A locked comment is refused.**
#[test]
fn a_locked_comment_is_refused() {
    let mut session = session();
    let directory = directory("locked");
    let (_, _, _, locked) = a_page(&mut session, &directory);
    let refusal = session
        .execute("pdf.comment.info", &json!({ "comment": locked, "author": "Cy" }))
        .expect_err("the comment is locked");
    let message = refusal.to_string();
    assert!(message.contains("locked"), "the refusal says so: {message}");
}

/// **An icon that is not one of the tree's own is refused.**
#[test]
fn an_icon_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let directory = directory("badicon");
    let (_, note, _, _) = a_page(&mut session, &directory);
    let to = format!("{directory}/icon.bin");
    let refusal = session
        .execute("pdf.comment.info", &json!({ "comment": note, "icon": "Star", "to": to }))
        .expect_err("the icon is refused");
    let message = refusal.to_string();
    assert!(message.contains("icon"), "the refusal says so: {message}");
}
