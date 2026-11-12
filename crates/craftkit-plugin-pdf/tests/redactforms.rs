//! The **content of a form** under a region is rewritten.
//!
//! A form that a region hits is written again as a new object, with the
//! drawings under the region taken away from it, and it is drawn under a
//! new name — the object the pages share is untouched. That is the
//! tree's own rule.

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

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-redactforms-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page draws a **form**, and the form draws a path.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let content = "q 100 0 0 100 0 0 cm /Fx1 Do Q";
    let form = "0 0 0.5 0.5 re f";
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /XObject << /Fx1 5 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (
            5,
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length {} >>\nstream\n{form}\nendstream",
                form.len()
            ),
        ),
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

/// Load a file into the document, and give the **page** it holds and
/// the folder it came from.
fn load(session: &mut Session, name: &str) -> (u64, String) {
    let directory = directory(name);
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
    (page, directory)
}

/// The **bytes** of the content of the page, after the change.
fn content_of(directory: &str) -> Vec<u8> {
    std::fs::read(format!("{directory}/streams/4.bin")).expect("the stream is there")
}

/// The **streams** of the document, and the bytes of each of them.
fn streams_of(session: &Session) -> Vec<Vec<u8>> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.stream")
        .filter_map(|node| {
            let path = node.stream.clone()?;
            Some(std::fs::read(path).unwrap_or_default())
        })
        .collect()
}

/// A form the region **hits** is written again: the path under the
/// region is gone from its content, and it is drawn under a new name.
#[test]
fn a_form_the_region_hits_is_written_again() {
    let mut session = session();
    let (page, directory) = load(&mut session, "hits");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 50, 50] }))
        .expect("the form is written again");
    assert_eq!(outcome.data["forms"], json!(1), "one form is written again: {outcome:?}");
    // The form is still drawn, under a **new name**.
    let written = content_of(&directory);
    let text = String::from_utf8_lossy(&written).to_string();
    assert!(text.contains("Do"), "the form is still drawn: {text}");
    assert!(text.contains("PCRedacted1"), "under a new name: {text}");
    assert!(!text.contains("Fx1"), "and not under the old one: {text}");
    // The **content** of the new form: the path is gone.
    let streams = streams_of(&session);
    let new_form = streams
        .iter()
        .find(|bytes| !bytes.is_empty() && !String::from_utf8_lossy(bytes).contains(" re "))
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .expect("the new form is there");
    assert!(!new_form.contains("re"), "and the path is gone from it: {new_form}");
}

/// The **object** the file had is untouched: the pages that share it
/// still draw it.
#[test]
fn the_object_the_file_had_is_untouched() {
    let mut session = session();
    let (page, _directory) = load(&mut session, "untouched");
    session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 50, 50] }))
        .expect("the form is written again");
    let streams = streams_of(&session);
    let original = streams
        .iter()
        .find(|bytes| String::from_utf8_lossy(bytes).contains(" re "))
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .expect("the original form is there");
    assert!(original.contains("0.5"), "and it is as it was: {original}");
}

/// A form the region does **not** touch is left alone.
#[test]
fn a_form_the_region_does_not_touch_is_left_alone() {
    let mut session = session();
    let (page, directory) = load(&mut session, "apart");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [300, 300, 320, 320] }))
        .expect("the region is read");
    assert_eq!(outcome.data["forms"], json!(0), "no form is written again: {outcome:?}");
    let written = content_of(&directory);
    let text = String::from_utf8_lossy(&written).to_string();
    assert!(text.contains("Fx1"), "and the form is still drawn as it was: {text}");
}
