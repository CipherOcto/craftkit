//! The **compare** that marks the differences: the highlights over the
//! text that changed and the notes where text was deleted.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-markcmp-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file of PDF whose page shows the **pieces** of text it is given.
fn a_file(pieces: &[&str]) -> Vec<u8> {
    let mut content = String::from("BT /F1 12 Tf 72 700 Td");
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            content.push_str(" 0 -14 Td");
        }
        content.push_str(&format!(" ({piece}) Tj"));
    }
    content.push_str(" ET\n");
    let widths: Vec<String> = (32..=126).map(|_| "500".to_string()).collect();
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}endstream", content.len())),
        (5, format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 32 /LastChar 126 /Widths [{}] /Encoding /WinAnsiEncoding >>",
            widths.join(" ")
        )),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 6\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Write a file of this test's own and give its path.
fn a_file_at(name: &str, pieces: &[&str]) -> String {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(pieces)).expect("the file is written");
    path
}

/// Load a file into the document of the session: it is the **new** file
/// of the comparison.
fn load(session: &mut Session, name: &str, pieces: &[&str]) {
    let directory = directory(name);
    let path = format!("{directory}/new.pdf");
    std::fs::write(&path, a_file(pieces)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **page** of the document of the session.
fn the_page(session: &mut Session) -> u64 {
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

/// The **marks** the comparison made.
fn marks_of(session: &mut Session, old: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("pdf.compare.mark", &json!({ "old": old })).expect("the differences are marked");
    outcome.data.clone()
}

/// **The text that changed is marked with a highlight.**
#[test]
fn the_text_that_changed_is_marked_with_a_highlight() {
    let mut session = session();
    load(&mut session, "changed", &["alpha", "gamma"]);
    let old = a_file_at("changed-old", &["alpha", "beta"]);
    let answer = marks_of(&mut session, &old);
    assert_eq!(answer["count"], json!(1), "one difference is marked: {answer:?}");
    let marked = answer["marked"].as_array().expect("the marks are a list");
    assert_eq!(marked[0]["subtype"], json!("Highlight"), "it is a highlight: {answer:?}");
    assert_eq!(marked[0]["kind"], json!("replaced"), "of what was replaced: {answer:?}");
    assert_eq!(marked[0]["colour"], json!([0.2, 0.45, 0.95]), "in the tree's blue: {answer:?}");
    let contents = marked[0]["contents"].as_str().expect("the contents are a string");
    assert!(contents.contains("Replaced"), "and it says what happened: {contents}");
}

/// **The text that was inserted is marked with a highlight.**
#[test]
fn the_text_that_was_inserted_is_marked_with_a_highlight() {
    let mut session = session();
    load(&mut session, "inserted", &["alpha", "beta"]);
    let old = a_file_at("inserted-old", &["alpha"]);
    let answer = marks_of(&mut session, &old);
    assert_eq!(answer["count"], json!(1), "one difference is marked: {answer:?}");
    let marked = answer["marked"].as_array().expect("the marks are a list");
    assert_eq!(marked[0]["subtype"], json!("Highlight"), "it is a highlight: {answer:?}");
    assert_eq!(marked[0]["kind"], json!("inserted"), "of what was inserted: {answer:?}");
    assert_eq!(marked[0]["colour"], json!([0.2, 0.75, 0.3]), "in the tree's green: {answer:?}");
}

/// **The text that was deleted is marked with a note.**
#[test]
fn the_text_that_was_deleted_is_marked_with_a_note() {
    let mut session = session();
    load(&mut session, "deleted", &["alpha"]);
    let old = a_file_at("deleted-old", &["alpha", "beta"]);
    let answer = marks_of(&mut session, &old);
    assert_eq!(answer["count"], json!(1), "one difference is marked: {answer:?}");
    let marked = answer["marked"].as_array().expect("the marks are a list");
    assert_eq!(marked[0]["subtype"], json!("Text"), "it is a note: {answer:?}");
    assert_eq!(marked[0]["kind"], json!("deleted"), "of what was deleted: {answer:?}");
    assert_eq!(marked[0]["colour"], json!([0.9, 0.25, 0.25]), "in the tree's red: {answer:?}");
    let contents = marked[0]["contents"].as_str().expect("the contents are a string");
    assert!(contents.contains("Deleted"), "and it says what happened: {contents}");
}

/// **The mark is an annotation of the page.**
#[test]
fn the_mark_is_an_annotation_of_the_page() {
    let mut session = session();
    load(&mut session, "annotation", &["alpha", "gamma"]);
    let page = the_page(&mut session);
    let old = a_file_at("annotation-old", &["alpha", "beta"]);
    marks_of(&mut session, &old);
    let annotations =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    let list = annotations.data["annotations"].as_array().expect("the annotations are a list");
    assert_eq!(list.len(), 1, "the page names the mark: {annotations:?}");
}

/// **Two files with the same text have nothing to mark.**
#[test]
fn two_files_with_the_same_text_have_nothing_to_mark() {
    let mut session = session();
    load(&mut session, "same", &["alpha", "beta"]);
    let old = a_file_at("same-old", &["alpha", "beta"]);
    let answer = marks_of(&mut session, &old);
    assert_eq!(answer["count"], json!(0), "there is nothing to mark: {answer:?}");
}
