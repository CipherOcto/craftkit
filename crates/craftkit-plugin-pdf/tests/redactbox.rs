//! The **box** of a redaction is drawn into the page.
//!
//! The tree's own rule: the mark is gone when it is applied, and a
//! box in its fill colour is drawn in its place.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-redactbox-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page paints the given content.
fn a_file(content: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>".to_string()),
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

/// The **text** of the content of the page, after the change.
fn text_of(directory: &str) -> String {
    let bytes = std::fs::read(format!("{directory}/streams/4.bin")).expect("the stream is there");
    String::from_utf8_lossy(&bytes).to_string()
}

/// The box is drawn in the **fill colour of the mark**, and the
/// tree's default is black.
#[test]
fn the_box_is_drawn_in_the_colour_of_the_mark() {
    let mut session = session();
    let (page, directory) = load(&mut session, "colour", "1 0 0 RG 10 10 m 20 20 l S");
    let outcome = session
        .execute("pdf.redact.box", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the box is drawn");
    assert_eq!(outcome.data["color"], json!([0.0, 0.0, 0.0]), "the colour is the tree's: {outcome:?}");
    let written = text_of(&directory);
    assert!(written.contains("0 0 0 rg"), "the colour is written: {written}");
    assert!(written.contains("40 40 120 120 re"), "the box is the region: {written}");
    assert!(written.contains('f'), "and it is filled: {written}");
}

/// A **colour of its own** is written as it is given.
#[test]
fn a_colour_of_its_own_is_written_as_it_is_given() {
    let mut session = session();
    let (page, directory) = load(&mut session, "own", "1 0 0 RG 10 10 m 20 20 l S");
    session
        .execute(
            "pdf.redact.box",
            &json!({ "page": page, "rect": [0, 0, 10, 10], "color": [0.5, 0.25, 0.125] }),
        )
        .expect("the box is drawn");
    let written = text_of(&directory);
    assert!(written.contains("0.5 0.25 0.125 rg"), "the colour is written: {written}");
}

/// A colour that is **not three numbers** is refused by name.
#[test]
fn a_colour_that_is_not_three_numbers_is_refused() {
    let mut session = session();
    let (page, _directory) = load(&mut session, "bad", "1 0 0 RG 10 10 m 20 20 l S");
    let outcome = session
        .execute("pdf.redact.box", &json!({ "page": page, "rect": [0, 0, 10, 10], "color": [0.5, 0.5] }));
    assert!(outcome.is_err(), "the colour is refused: {outcome:?}");
    let outcome = session.execute(
        "pdf.redact.box",
        &json!({ "page": page, "rect": [0, 0, 10, 10], "color": [0.5, 0.5, 2.0] }),
    );
    assert!(outcome.is_err(), "and a colour out of range is refused: {outcome:?}");
}

/// The box is drawn where the **region** is: the region is in the
/// space of the page, and the path of the box is written in the space
/// of the content.
#[test]
fn the_box_is_drawn_where_the_region_is() {
    let mut session = session();
    // The content leaves a transform behind: the path of the box is
    // written in the space of the content, and the box itself is in
    // the space of the page.
    let (page, directory) = load(&mut session, "space", "q 2 0 0 2 0 0 cm 1 0 0 RG 10 10 m 20 20 l S");
    session
        .execute("pdf.redact.box", &json!({ "page": page, "rect": [40, 40, 160, 160] }))
        .expect("the box is drawn");
    let written = text_of(&directory);
    assert!(written.contains("20 20 60 60 re"), "the path is in the space of the content: {written}");
}
