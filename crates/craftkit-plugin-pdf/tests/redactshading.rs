//! The **shading** of a page under a region is clipped out.
//!
//! A shading fills the clip it is in, so the region is cut out of it.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-redactshading-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page paints a **shading**, and nothing else.
fn a_file(content: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /Shading << /Sh1 5 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, "<< /Type /Shading /ShadingType 2 /ColorSpace /DeviceGray >>".to_string()),
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

/// The **text** of the content of the page, after the change.
fn text_of(directory: &str) -> String {
    String::from_utf8_lossy(&content_of(directory)).to_string()
}

/// A shading under the region is **clipped**: the region is cut out of
/// it.
#[test]
fn a_shading_under_the_region_is_clipped() {
    let mut session = session();
    let (page, directory) = load(&mut session, "shading", "/Sh1 sh");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the shading is clipped");
    assert_eq!(outcome.data["shadings"], json!(1), "the shading is clipped: {outcome:?}");
    let written = text_of(&directory);
    assert!(written.contains("W*"), "the clip is written: {written}");
    assert!(written.contains("sh"), "and the shading is still painted: {written}");
    assert!(
        written.find('q') < written.find("sh") && written.find("sh") < written.rfind('Q'),
        "and it is wrapped in the clip: {written}"
    );
}

/// The clip cuts the **region** out of the shading: the corners of the
/// region are in the path.
#[test]
fn the_clip_cuts_the_region_out() {
    let mut session = session();
    let (page, directory) = load(&mut session, "region", "/Sh1 sh");
    session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the shading is clipped");
    let written = text_of(&directory);
    assert!(written.contains("40 40"), "the corner of the region is in the path: {written}");
    assert!(written.contains("160 160"), "and the other one: {written}");
}

/// The path of the clip is written in the space the shading is
/// **painted** in: the transform of the page is undone first.
#[test]
fn the_path_of_the_clip_is_in_the_space_of_the_shading() {
    let mut session = session();
    let (page, directory) = load(&mut session, "space", "q 2 0 0 2 0 0 cm /Sh1 sh Q");
    session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the shading is clipped");
    let written = text_of(&directory);
    assert!(written.contains("20 20"), "the corner is halved with the transform: {written}");
    assert!(written.contains("80 80"), "and the other one: {written}");
}

/// A page with **no shading** is left alone.
#[test]
fn a_page_with_no_shading_is_left_alone() {
    let mut session = session();
    let (page, directory) = load(&mut session, "plain", "1 0 0 RG 10 10 m 20 20 l S");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the region is read");
    assert_eq!(outcome.data["shadings"], json!(0), "no shading is clipped: {outcome:?}");
    let written = text_of(&directory);
    assert!(!written.contains("W*"), "and nothing is clipped: {written}");
}
