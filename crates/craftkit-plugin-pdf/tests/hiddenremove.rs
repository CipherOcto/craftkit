//! The **hidden information** of a file is taken away.
//!
//! The comments, the attachments and the links are gone from the
//! document.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-hiddenremove-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has four annotations: a link with an action, a
/// note, an attachment and a mark with an action.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>"
                .to_string(),
        ),
        (4, "<< /Type /Annot /Subtype /Link /Rect [10 10 20 20] /AA << /O << /S /GoTo >> >> >>".to_string()),
        (5, "<< /Type /Annot /Subtype /Text /Rect [30 30 40 40] /Contents (a note) >>".to_string()),
        (6, "<< /Type /Annot /Subtype /FileAttachment /Rect [50 50 60 60] >>".to_string()),
        (
            7,
            "<< /Type /Annot /Subtype /Square /Rect [70 70 80 80] /AA << /E << /S /GoTo >> >> >>".to_string(),
        ),
    ] {
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

/// Load a file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// Whether the document still holds the **object** of that number.
fn holds(session: &Session, number: u64) -> bool {
    session
        .document()
        .walk()
        .any(|node| node.data.get("object").and_then(|value| value.as_u64()) == Some(number))
}

/// The **comments** are taken away: the note on the page is gone, and
/// the other annotations stay.
#[test]
fn the_comments_are_taken_away() {
    let mut session = session();
    load(&mut session, "comments");
    let outcome =
        session.execute("pdf.hidden.remove", &json!({ "category": "Comments" })).expect("the comments go");
    // The tree's own rule: a comment is an annotation that is none of
    // a link, a widget, a pop-up, an attachment or a mark -- so the
    // note and the square are both comments.
    assert_eq!(outcome.data["removed"], json!(2), "the note and the square are gone: {outcome:?}");
    assert!(!holds(&session, 5), "and the note is not in the document");
    assert!(!holds(&session, 7), "and the square is not either");
    assert!(holds(&session, 4), "and the link is there");
    assert!(holds(&session, 6), "and the attachment is there");
}

/// The **attachments** are taken away.
#[test]
fn the_attachments_are_taken_away() {
    let mut session = session();
    load(&mut session, "attachments");
    let outcome = session
        .execute("pdf.hidden.remove", &json!({ "category": "Attachments" }))
        .expect("the attachments go");
    assert_eq!(outcome.data["removed"], json!(1), "the attachment is gone: {outcome:?}");
    assert!(!holds(&session, 6), "and it is not in the document");
    assert!(holds(&session, 5), "and the note is there");
}

/// The **links** are taken away, and the **actions** of the
/// annotations that stay go with them.
#[test]
fn the_links_are_taken_away_with_their_actions() {
    let mut session = session();
    load(&mut session, "links");
    let outcome = session
        .execute("pdf.hidden.remove", &json!({ "category": "LinksActionsScripts" }))
        .expect("the links go");
    assert_eq!(outcome.data["removed"], json!(1), "the link is gone: {outcome:?}");
    assert!(!holds(&session, 4), "and it is not in the document");
    assert!(holds(&session, 7), "and the mark that stayed is there");
    let entries = session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|value| value.as_u64()) == Some(7))
        .map(|node| node.data["entries"].as_array().map(|entries| entries.len()).unwrap_or(0))
        .unwrap_or_default();
    // The type, the subtype and the rectangle are left.
    assert_eq!(entries, 3, "and its action is gone: {entries}");
}

/// A category that is **not one of the tree's** is refused, and so is
/// one that is not taken away yet.
#[test]
fn a_category_that_cannot_be_taken_away_is_refused() {
    let mut session = session();
    load(&mut session, "refused");
    let error = session
        .execute("pdf.hidden.remove", &json!({ "category": "Nonsense" }))
        .expect_err("there is no such category");
    assert!(error.to_string().contains("Nonsense"), "the refusal says why: {error}");
    let error = session
        .execute("pdf.hidden.remove", &json!({ "category": "Metadata" }))
        .expect_err("the metadata is not taken away yet");
    assert!(error.to_string().contains("not supported yet"), "the refusal says why: {error}");
}
