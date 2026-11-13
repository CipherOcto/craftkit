//! The **pixels of an image** under a region are cleared.
//!
//! An image the region only hits is not taken away: its covered pixels
//! are cleared, it is written again as a new object, and it is drawn
//! under a new name — the object the pages share is untouched. That is
//! the tree's own rule.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-redactpixels-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page draws an **image** of four by four grey pixels.
///
/// The image is drawn in a hundred points square at the origin, and
/// every pixel of it is white.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let content = "q 100 0 0 100 0 0 cm /Im1 Do Q";
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
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    // The **image**: four by four grey pixels, and every one of them is
    // white.
    offsets.push(file.len());
    file.extend_from_slice(
        b"5 0 obj\n<< /Type /XObject /Subtype /Image /Width 4 /Height 4 \
          /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 16 >>\nstream\n",
    );
    file.extend_from_slice(&[0xff; 16]);
    file.extend_from_slice(b"\nendstream\nendobj\n");
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
fn streams_of(session: &Session, directory: &str) -> Vec<(u64, Vec<u8>)> {
    let mut out: Vec<(u64, Vec<u8>)> = Vec::new();
    for node in session.document().walk() {
        if node.kind != "pdf.stream" {
            continue;
        }
        let Some(path) = node.stream.clone() else { continue };
        let bytes = std::fs::read(&path).unwrap_or_default();
        out.push((node.id, bytes));
    }
    let _ = directory;
    out
}

/// An image the region only **hits** has its covered pixels cleared,
/// and it is drawn under a new name.
#[test]
fn an_image_that_only_hits_the_region_has_its_pixels_cleared() {
    let mut session = session();
    let (page, directory) = load(&mut session, "hits");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 50, 50] }))
        .expect("the image is cleared");
    assert_eq!(outcome.data["cleared"], json!(1), "one image is cleared: {outcome:?}");
    assert_eq!(outcome.data["images"], json!(0), "and none is taken away: {outcome:?}");
    // The image is still drawn, under a **new name**: the object the
    // pages share keeps the name it had.
    let written = content_of(&directory);
    let text = String::from_utf8_lossy(&written).to_string();
    assert!(text.contains("Do"), "the image is still drawn: {text}");
    assert!(text.contains("PCRedacted1"), "under a new name: {text}");
    assert!(!text.contains("Im1"), "and not under the old one: {text}");
    // The **pixels** of the new image: the four under the region are
    // black, and the other twelve are white.
    let streams = streams_of(&session, &directory);
    let cleared = streams
        .iter()
        .find(|(_, bytes)| bytes.len() == 16 && bytes.contains(&0))
        .map(|(_, bytes)| bytes.clone())
        .expect("the new image is there");
    let mut expected = vec![0xff; 16];
    for at in [8usize, 9, 12, 13] {
        expected[at] = 0;
    }
    assert_eq!(cleared, expected, "and the covered pixels are black: {cleared:?}");
}

/// An image the region **covers** is taken away: the tree's own rule.
#[test]
fn an_image_the_region_covers_is_taken_away() {
    let mut session = session();
    let (page, directory) = load(&mut session, "covered");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 100, 100] }))
        .expect("the image is taken away");
    assert_eq!(outcome.data["images"], json!(1), "the image is taken away: {outcome:?}");
    assert_eq!(outcome.data["cleared"], json!(0), "and nothing is cleared: {outcome:?}");
    let written = content_of(&directory);
    assert!(!written.windows(2).any(|window| window == b"Do"), "and it is not drawn: {written:?}");
}

/// An image the region does **not** touch is left alone.
#[test]
fn an_image_the_region_does_not_touch_is_left_alone() {
    let mut session = session();
    let (page, directory) = load(&mut session, "apart");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [300, 300, 320, 320] }))
        .expect("the region is read");
    assert_eq!(outcome.data["images"], json!(0), "no image is taken away: {outcome:?}");
    assert_eq!(outcome.data["cleared"], json!(0), "and nothing is cleared: {outcome:?}");
    let written = content_of(&directory);
    assert!(written.windows(2).any(|window| window == b"Do"), "and it is still drawn: {written:?}");
}

/// The **object** the file had is untouched: the pages that share it
/// still draw it.
#[test]
fn the_object_the_file_had_is_untouched() {
    let mut session = session();
    let (page, directory) = load(&mut session, "untouched");
    session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 50, 50] }))
        .expect("the image is cleared");
    // The **original** image is still sixteen white bytes.
    let streams = streams_of(&session, &directory);
    let original = streams
        .iter()
        .find(|(_, bytes)| bytes.len() == 16 && bytes.iter().all(|byte| *byte == 0xff))
        .map(|(_, bytes)| bytes.clone())
        .expect("the original image is there");
    assert_eq!(original, vec![0xff; 16], "and it is as it was: {original:?}");
}
