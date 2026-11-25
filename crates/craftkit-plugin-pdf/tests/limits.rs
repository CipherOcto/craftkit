//! The **limits of the pdf battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. A PDF file comes from outside
//! — a download, a scan, an agent — so it is read carefully, and a limit
//! that no test names is a limit nothing proves holds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-limits-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The body of a **stream** object: its dictionary and its bytes.
fn stream(dictionary: &str, content: &str) -> String {
    format!("<< {dictionary} /Length {} >>\nstream\n{content}\nendstream", content.len())
}

/// A file whose name tree is **deeper** than the reader reads.
///
/// The tree is a chain of kids, and the scripts are at the top and at the
/// bottom: the top one is read and the bottom one is below the reader's
/// depth.
fn a_file_with_a_deep_name_tree() -> Vec<u8> {
    let mut bodies: Vec<(usize, String)> = Vec::new();
    bodies.push((1, "<< /Type /Catalog /Names 2 0 R >>".to_string()));
    bodies.push((2, "<< /JavaScript 3 0 R >>".to_string()));
    // The chain: object 3 holds the script, and every other object of
    // the chain holds the next one.
    let levels = 12;
    for level in 0..levels {
        let number = 3 + level;
        let body = if level == 0 {
            "<< /Names [(top) 30 0 R] /Kids [4 0 R] >>".to_string()
        } else if level + 1 == levels {
            "<< /Names [(deep) 31 0 R] >>".to_string()
        } else {
            format!("<< /Kids [{} 0 R] >>", number + 1)
        };
        bodies.push((number, body));
    }
    bodies.push((30, stream("/Type /JavaScript", "app.alert('top');")));
    bodies.push((31, stream("/Type /JavaScript", "app.alert('deep');")));
    bodies.sort_by_key(|(number, _)| *number);
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    let size = bodies.len() + 1;
    file.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n").as_bytes());
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str, bytes: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// A name tree that is **deeper** than the reader reads says so.
///
/// The scripts below the reader's depth are in the file and not in the
/// answer, and a file that says more than the document does is a file
/// that lies: the part that was not read is **said**.
#[test]
fn a_name_tree_deeper_than_the_reader_reads_says_so() {
    let mut session = session();
    load(&mut session, "deep", a_file_with_a_deep_name_tree());
    let outcome = session.execute("pdf.scripts.read", &json!({})).expect("the scripts are read");
    let scripts = outcome.data["scripts"].as_array().expect("the scripts are a list");
    assert_eq!(scripts.len(), 1, "the script at the top is read: {outcome:?}");
    let skipped = outcome.data["skipped"].as_array().expect("the skipped are a list");
    assert!(!skipped.is_empty(), "the part that was not read is said: {outcome:?}");
    assert!(
        skipped.iter().any(|note| note.as_str().unwrap_or_default().contains("deeper")),
        "and it says what was not read: {outcome:?}"
    );
}

/// A file whose name tree of **attachments** is deeper than the reader
/// reads.
///
/// The tree is a chain of kids, and the attachments are at the top and
/// at the bottom: the top one is read and the bottom one is below the
/// reader's depth.
fn a_file_with_a_deep_tree_of_attachments() -> Vec<u8> {
    let mut bodies: Vec<(usize, String)> = Vec::new();
    bodies.push((1, "<< /Type /Catalog /Names 2 0 R >>".to_string()));
    bodies.push((2, "<< /EmbeddedFiles 3 0 R >>".to_string()));
    let levels = 12;
    for level in 0..levels {
        let number = 3 + level;
        let body = if level == 0 {
            "<< /Names [(top) 30 0 R] /Kids [4 0 R] >>".to_string()
        } else if level + 1 == levels {
            "<< /Names [(deep) 31 0 R] >>".to_string()
        } else {
            format!("<< /Kids [{} 0 R] >>", number + 1)
        };
        bodies.push((number, body));
    }
    bodies.push((30, "<< /Type /Filespec /F (top.txt) /EF << /F 32 0 R >> >>".to_string()));
    bodies.push((31, "<< /Type /Filespec /F (deep.txt) /EF << /F 33 0 R >> >>".to_string()));
    bodies.push((32, stream("/Type /EmbeddedFile", "the top one")));
    bodies.push((33, stream("/Type /EmbeddedFile", "the deep one")));
    bodies.sort_by_key(|(number, _)| *number);
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    let size = bodies.len() + 1;
    file.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n").as_bytes());
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// The attachments of a file whose tree is **deeper** than the reader
/// reads say so.
///
/// The attachments below the reader's depth are in the file and not in
/// the answer, and the answer says that it did not read them all.
#[test]
fn a_tree_of_attachments_deeper_than_the_reader_reads_says_so() {
    let mut session = session();
    load(&mut session, "deep-attachments", a_file_with_a_deep_tree_of_attachments());
    let outcome = session.execute("pdf.attachments.read", &json!({})).expect("the attachments are read");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// Form data of **more bytes** than the limit allows is refused.
///
/// The file comes from outside, and a file of eight megabytes and more
/// is refused rather than read.
#[test]
fn form_data_over_the_limit_is_refused() {
    let mut session = session();
    let directory = directory("formdata");
    let path = format!("{directory}/data.json");
    let bytes = vec![b'x'; 8 * 1024 * 1024 + 1];
    std::fs::write(&path, &bytes).expect("the file is written");
    let error =
        session.execute("pdf.form.data.import", &json!({ "path": path })).expect_err("that file is too big");
    assert!(error.to_string().contains("bytes long"), "{error}");
}
