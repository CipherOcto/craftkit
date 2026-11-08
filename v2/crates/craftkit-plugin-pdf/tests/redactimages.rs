//! The **images** of a file under a region are taken away.
//!
//! An image of the page that the region hits is not drawn any more.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-redactimages-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page draws an **image** — or a **form**, when the
/// subtype says so.
fn a_file(subtype: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let content = "q 100 0 0 100 50 50 cm /Im1 Do Q";
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /XObject << /Im1 5 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, format!("<< /Type /XObject /Subtype /{subtype} /Width 2 /Height 2 >>")),
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
fn load(session: &mut Session, name: &str, subtype: &str) -> (u64, String) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(subtype)).expect("the file is written");
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

/// An image the region **covers** is not drawn any more.
#[test]
fn an_image_the_region_covers_is_not_drawn() {
    let mut session = session();
    let (page, directory) = load(&mut session, "covered", "Image");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the image is taken away");
    assert_eq!(outcome.data["images"], json!(1), "the image is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(!written.windows(2).any(|window| window == b"Do"), "and it is not drawn: {written:?}");
}

/// An image the region only **hits** is not drawn any more either: the
/// tree clears the covered pixels, and this tree cannot decode an
/// image, so the tree's own rule for an image it cannot read applies.
#[test]
fn an_image_that_only_hits_the_region_is_not_drawn() {
    let mut session = session();
    let (page, directory) = load(&mut session, "hits", "Image");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 60, 60] }))
        .expect("the image is taken away");
    assert_eq!(outcome.data["images"], json!(1), "the image is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(!written.windows(2).any(|window| window == b"Do"), "and it is not drawn: {written:?}");
}

/// An image the region does **not** touch is left alone.
#[test]
fn an_image_the_region_does_not_touch_is_left_alone() {
    let mut session = session();
    let (page, directory) = load(&mut session, "apart", "Image");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [300, 300, 320, 320] }))
        .expect("the region is read");
    assert_eq!(outcome.data["images"], json!(0), "no image is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(written.windows(2).any(|window| window == b"Do"), "and it is still drawn: {written:?}");
}

/// An XObject that is **not an image** is left alone: a form is a
/// drawing of its own.
#[test]
fn an_xobject_that_is_not_an_image_is_left_alone() {
    let mut session = session();
    let (page, directory) = load(&mut session, "form", "Form");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the region is read");
    assert_eq!(outcome.data["images"], json!(0), "no image is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(written.windows(2).any(|window| window == b"Do"), "and it is still drawn: {written:?}");
}
