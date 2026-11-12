//! The **appearance** of a callout: the leader line and the text box
//! of it.

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
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-calloutappearance-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with a **page** on it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len())),
    ];
    for (number, body) in &bodies {
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

/// Load the file and give the **page**.
fn a_page(session: &mut Session, directory: &str) -> u64 {
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

/// A **callout** on a page: the node of it.
fn a_callout(session: &mut Session, directory: &str) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.callout.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [300.0, 700.0, 400.0, 740.0],
                "knee": [250.0, 680.0],
                "point": [200.0, 650.0],
                "contents": "a note",
            }),
        )
        .expect("the callout is added");
    outcome.data["id"].as_u64().expect("the callout is there")
}

/// **The callout is drawn:** the leader line, the box and the text.
#[test]
fn the_callout_is_drawn() {
    let mut session = session();
    let directory = directory("callout");
    let callout = a_callout(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": callout, "to": to }))
        .expect("the callout is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **leader** is the three points of the `/CL`: the point, the
    // knee and the place where it meets the box.
    assert!(bytes.contains("200 650 m\n250 680 l\n300 720 l\nS\n"), "the leader is drawn: {bytes}");
    // The **box** is inside the rectangle by the `/RD`, and the text is
    // clipped to it.
    assert!(bytes.contains("300 700 100 40 re W n"), "the box is the tree's: {bytes}");
    // The **text** sits three points inside the box — two points of
    // padding and the width of the border — and a tenth of the size of
    // the font below the top of it.
    assert!(bytes.contains("1 0 0 1 303 726.2 Tm (a note) Tj"), "the text is drawn: {bytes}");
    // And the **rectangle of the drawing** is the rectangle of the
    // annotation — the box and the leader together — and not the box
    // the text is in.
    assert_eq!(outcome.data["rect"], json!([189.5, 639.5, 400.0, 740.0]), "the rectangle is the tree's");
}

/// **The arrowhead** is drawn at the point the leader starts at, and
/// it is **open**: the tree's own `/LE`.
#[test]
fn the_arrowhead_is_open_and_at_the_point() {
    let mut session = session();
    let directory = directory("arrow");
    let callout = a_callout(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": callout, "to": to }))
        .expect("the callout is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The arrowhead is three points — the two wings and the tip — and
    // the tip is the point the leader starts at: the tree's own.
    let arrow = bytes
        .lines()
        .find(|line| line.ends_with(" l 200 650 l S") || line.contains(" 200 650 l "))
        .unwrap_or_else(|| panic!("the arrowhead is drawn: {bytes}"));
    assert!(arrow.ends_with(" S"), "the arrowhead is open: {arrow}");
}

/// **The border** of the box is stroked inside it.
#[test]
fn the_border_of_the_box_is_stroked() {
    let mut session = session();
    let directory = directory("border");
    let callout = a_callout(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": callout, "to": to }))
        .expect("the callout is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The border is one point wide — the width of the line of the
    // leader — and it is inside the box.
    assert!(bytes.contains("1 w\n300.5 700.5 99 39 re S\n"), "the border is stroked: {bytes}");
}

/// **A callout without its inset** cannot be drawn: the tree's own
/// rule, because the box of the text is inside the rectangle by the
/// `/RD`.
#[test]
fn a_callout_without_its_inset_is_refused() {
    let mut session = session();
    let directory = directory("noinset");
    let callout = a_callout(&mut session, &directory);
    session
        .execute("pdf.entry.remove", &json!({ "object": callout, "key": "RD" }))
        .expect("the inset is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": callout, "to": to }))
        .expect_err("the callout is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
