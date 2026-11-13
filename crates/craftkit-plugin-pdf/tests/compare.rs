//! The **compare** of two files of PDF: the differences between their
//! text.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-compare-{name}-{}", std::process::id()))
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
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}endstream", content.len())),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
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

/// The **changes** of a comparison, as the command says them.
fn changes_of(session: &mut Session, old: &str, new: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("pdf.compare", &json!({ "old": old, "new": new })).expect("the files are compared");
    outcome.data.clone()
}

/// **Two files with the same text have no changes.**
#[test]
fn two_files_with_the_same_text_have_no_changes() {
    let mut session = session();
    let old = a_file_at("same-old", &["alpha", "beta"]);
    let new = a_file_at("same-new", &["alpha", "beta"]);
    let answer = changes_of(&mut session, &old, &new);
    assert_eq!(answer["count"], json!(0), "there is no change: {answer:?}");
    assert_eq!(answer["identical"], json!(true), "and the text is the same: {answer:?}");
    assert!(
        answer["report"].as_str().unwrap_or_default().contains("identical"),
        "the report says so: {answer:?}"
    );
}

/// **A text that is not the same is a change.**
#[test]
fn a_text_that_is_not_the_same_is_a_change() {
    let mut session = session();
    let old = a_file_at("changed-old", &["alpha", "beta"]);
    let new = a_file_at("changed-new", &["alpha", "gamma"]);
    let answer = changes_of(&mut session, &old, &new);
    assert_eq!(answer["count"], json!(1), "there is one change: {answer:?}");
    let changes = answer["changes"].as_array().expect("the changes are a list");
    assert_eq!(changes[0]["kind"], json!("replaced"), "the text was replaced: {answer:?}");
    assert_eq!(changes[0]["old"]["text"], json!("beta"), "the old text is said: {answer:?}");
    assert_eq!(changes[0]["new"]["text"], json!("gamma"), "and the new one: {answer:?}");
}

/// **Text that is only in the new file is inserted.**
#[test]
fn text_that_is_only_in_the_new_file_is_inserted() {
    let mut session = session();
    let old = a_file_at("inserted-old", &["alpha"]);
    let new = a_file_at("inserted-new", &["alpha", "beta"]);
    let answer = changes_of(&mut session, &old, &new);
    assert_eq!(answer["count"], json!(1), "there is one change: {answer:?}");
    let changes = answer["changes"].as_array().expect("the changes are a list");
    assert_eq!(changes[0]["kind"], json!("inserted"), "the text was inserted: {answer:?}");
    assert_eq!(changes[0]["new"]["text"], json!("beta"), "and it is said: {answer:?}");
}

/// **Text that is only in the old file is deleted.**
#[test]
fn text_that_is_only_in_the_old_file_is_deleted() {
    let mut session = session();
    let old = a_file_at("deleted-old", &["alpha", "beta"]);
    let new = a_file_at("deleted-new", &["alpha"]);
    let answer = changes_of(&mut session, &old, &new);
    assert_eq!(answer["count"], json!(1), "there is one change: {answer:?}");
    let changes = answer["changes"].as_array().expect("the changes are a list");
    assert_eq!(changes[0]["kind"], json!("deleted"), "the text was deleted: {answer:?}");
    assert_eq!(changes[0]["old"]["text"], json!("beta"), "and it is said: {answer:?}");
}

/// **A file that is not a file of PDF is refused.**
#[test]
fn a_file_that_is_not_a_file_of_pdf_is_refused() {
    let mut session = session();
    let directory = directory("notpdf");
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, b"this is not a file of PDF\n").expect("the file is written");
    let refusal = session
        .execute("pdf.compare", &json!({ "old": path, "new": path }))
        .expect_err("the file is refused");
    let message = refusal.to_string();
    assert!(message.contains("loaded"), "the refusal says so: {message}");
}

/// **A file that is not there is refused.**
#[test]
fn a_file_that_is_not_there_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("pdf.compare", &json!({ "old": "/nowhere/old.pdf", "new": "/nowhere/new.pdf" }))
        .expect_err("the file is refused");
    let message = refusal.to_string();
    assert!(message.contains("read"), "the refusal says so: {message}");
}
