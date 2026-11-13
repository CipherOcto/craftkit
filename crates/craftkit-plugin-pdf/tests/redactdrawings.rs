//! The **drawings under a region are taken away**.
//!
//! The inline images and the paths the region covers are gone from the
//! content of the page.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-redactdrawings-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given.
fn a_file(content: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
    ] {
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

/// Load a file into the document, and give the **page** it holds and
/// the file it came from.
fn load(session: &mut Session, name: &str, content: &str) -> (u64, String) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content)).expect("the file is written");
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

/// An **inline image** under the region is taken away: the bytes of it
/// are gone.
#[test]
fn an_inline_image_under_the_region_is_taken_away() {
    let mut session = session();
    let content = "q 100 0 0 100 50 50 cm BI /W 2 /H 2 /BPC 8 ID \u{0}\u{1}\u{2}\u{3} EI Q";
    let (page, directory) = load(&mut session, "image", content);
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the image is taken away");
    assert_eq!(outcome.data["images"], json!(1), "one image is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(
        !written.windows(4).any(|window| window == [0x00, 0x01, 0x02, 0x03]),
        "and the bytes of it are gone: {written:?}"
    );
}

/// A **path** the region covers is taken away.
#[test]
fn a_path_the_region_covers_is_taken_away() {
    let mut session = session();
    let content = "q 1 0 0 1 100 100 cm 10 10 50 50 re f Q";
    let (page, directory) = load(&mut session, "path", content);
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [100, 100, 170, 170] }))
        .expect("the path is taken away");
    assert_eq!(outcome.data["paths"], json!(1), "one path is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(!written.windows(2).any(|window| window == b"re"), "and the path is gone: {written:?}");
}

/// A path that only **hits** the region is left alone: the tree clips
/// it, and that is not here.
#[test]
fn a_path_that_only_hits_the_region_is_left_alone() {
    let mut session = session();
    let content = "q 1 0 0 1 100 100 cm 10 10 50 50 re f Q";
    let (page, directory) = load(&mut session, "hits", content);
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [100, 100, 120, 170] }))
        .expect("the region is read");
    assert_eq!(outcome.data["paths"], json!(0), "no path is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(written.windows(2).any(|window| window == b"re"), "and the path is still there: {written:?}");
}

/// A path that **clips** keeps its clip and paints nothing.
#[test]
fn a_path_that_clips_keeps_its_clip() {
    let mut session = session();
    let content = "q 1 0 0 1 100 100 cm 10 10 50 50 re W f Q";
    let (page, directory) = load(&mut session, "clip", content);
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [100, 100, 170, 170] }))
        .expect("the path is taken away");
    assert_eq!(outcome.data["paths"], json!(1), "one path is taken away: {outcome:?}");
    let written = content_of(&directory);
    assert!(written.windows(2).any(|window| window == b"re"), "and the clip is kept: {written:?}");
    assert!(!written.windows(1).any(|window| window == b"f"), "and the path paints nothing: {written:?}");
}
