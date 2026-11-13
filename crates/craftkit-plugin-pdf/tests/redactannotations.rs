//! The **annotations under a region are taken away**.
//!
//! The annotations the region overlaps are gone from the document, and
//! the pop-ups that belong to them go with them.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-redactannot-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a page that has **three** annotations: one that the
/// region covers, one that it does not, and the pop-up of the first.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R] >>".to_string(),
        ),
        (4, "<< /Type /Annot /Subtype /Square /Rect [100 100 200 200] >>".to_string()),
        (5, "<< /Type /Annot /Subtype /Square /Rect [400 400 500 500] >>".to_string()),
        (6, "<< /Type /Annot /Subtype /Popup /Rect [100 100 200 200] /Parent 4 0 R >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 7\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str) -> u64 {
    let directory = directory(name);
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

/// The **annotations** the page holds.
fn annotations_of(session: &mut Session, page: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    outcome.data["annotations"].as_array().expect("the annotations are a list").clone()
}

/// Whether the document still holds the **object** of that number.
fn holds(session: &Session, number: u64) -> bool {
    session
        .document()
        .walk()
        .any(|node| node.data.get("object").and_then(|value| value.as_u64()) == Some(number))
}

/// An annotation the region **overlaps** is taken away.
#[test]
fn an_annotation_the_region_overlaps_is_taken_away() {
    let mut session = session();
    let page = load(&mut session, "covered");
    let outcome = session
        .execute("pdf.redact.annotations", &json!({ "page": page, "rect": [90, 90, 210, 210] }))
        .expect("the annotations are taken away");
    assert_eq!(outcome.data["removed"], json!(2), "the annotation and its pop-up: {outcome:?}");
    assert!(!holds(&session, 4), "the annotation is gone: {outcome:?}");
    let annotations = annotations_of(&mut session, page);
    assert_eq!(annotations.len(), 1, "and the page holds one annotation: {annotations:?}");
}

/// An annotation that does **not** overlap the region is left alone.
#[test]
fn an_annotation_that_does_not_overlap_is_left_alone() {
    let mut session = session();
    let page = load(&mut session, "alone");
    session
        .execute("pdf.redact.annotations", &json!({ "page": page, "rect": [300, 300, 350, 350] }))
        .expect("the region is read");
    let annotations = annotations_of(&mut session, page);
    assert_eq!(annotations.len(), 3, "the page holds its three annotations: {annotations:?}");
    assert!(holds(&session, 4), "and the first is there");
    assert!(holds(&session, 6), "and the pop-up is there");
}

/// The **pop-up** of an annotation that is gone goes with it.
#[test]
fn the_pop_up_goes_with_its_annotation() {
    let mut session = session();
    let page = load(&mut session, "popup");
    session
        .execute("pdf.redact.annotations", &json!({ "page": page, "rect": [90, 90, 210, 210] }))
        .expect("the annotations are taken away");
    assert!(!holds(&session, 6), "the pop-up is gone: the annotation it belongs to is gone");
    assert!(holds(&session, 5), "and the annotation that was elsewhere is not");
}

/// A region that is **not a rectangle** is refused.
#[test]
fn a_region_that_is_not_a_rectangle_is_refused() {
    let mut session = session();
    let page = load(&mut session, "refused");
    let error = session
        .execute("pdf.redact.annotations", &json!({ "page": page, "rect": [1, 2, 3] }))
        .expect_err("three numbers are not a rectangle");
    assert!(error.to_string().contains("rectangle"), "the refusal says why: {error}");
}
