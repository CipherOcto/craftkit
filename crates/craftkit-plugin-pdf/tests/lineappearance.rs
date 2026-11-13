//! The **appearance** of a line: the line itself and its arrowheads.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-lineappearance-{name}-{}", std::process::id()))
        .expect("the directory is made");
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

/// A **line** on a page: the node of it.
fn a_line(session: &mut Session, directory: &str, arrow: bool) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [300.0, 700.0],
                "arrow": arrow,
            }),
        )
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the line is there")
}

/// **A line is drawn:** its two points, its colour and its width.
#[test]
fn a_line_is_drawn() {
    let mut session = session();
    let directory = directory("line");
    let line = a_line(&mut session, &directory, false);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect("the line is drawn");
    assert!(outcome.changed);
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tree's own** red, and the width of two points.
    assert!(bytes.contains("0.89 0.13 0.13 RG"), "the line has its colour: {bytes}");
    assert!(bytes.contains("2 w 1 J 1 j"), "the line has its width: {bytes}");
    // The **two points** of the line.
    assert!(bytes.contains("100 700 m 300 700 l S"), "the line is drawn: {bytes}");
}

/// **An arrow** is drawn at the end of the line: two wings at thirty
/// degrees, twelve points long — the tree's own size.
#[test]
fn an_arrow_is_drawn() {
    let mut session = session();
    let directory = directory("arrow");
    let line = a_line(&mut session, &directory, true);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect("the line is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The arrowhead is at the end of the line, and its wings are twelve
    // points long: twelve × cos 30° = 10.3923 to the left, and six
    // points up and down.
    assert!(bytes.contains("289.6077 694 m 300 700 l 289.6077 706 l S"), "the arrow is drawn: {bytes}");
}

/// **A closed arrow** is filled, and the fill of the line is the colour
/// of the line — the tree's own rule.
#[test]
fn a_closed_arrow_is_filled() {
    let mut session = session();
    let directory = directory("closed");
    let line = a_line(&mut session, &directory, false);
    session
        .execute("pdf.entry.set", &json!({ "object": line, "key": "LE", "value": ["None", "ClosedArrow"] }))
        .expect("the line has a closed arrow");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect("the line is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The fill of the line is the colour of the line.
    assert!(bytes.contains("0.89 0.13 0.13 rg"), "the fill is the colour of the line: {bytes}");
    assert!(bytes.ends_with("h B\n"), "the arrow is filled: {bytes}");
}

/// **A line without a colour** is refused.
#[test]
fn a_line_without_a_colour_is_refused() {
    let mut session = session();
    let directory = directory("nocolour");
    let line = a_line(&mut session, &directory, false);
    session
        .execute("pdf.entry.remove", &json!({ "object": line, "key": "C" }))
        .expect("the colour is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect_err("the line is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **A line without its two points** is refused.
#[test]
fn a_line_without_its_points_is_refused() {
    let mut session = session();
    let directory = directory("npoints");
    let line = a_line(&mut session, &directory, false);
    session
        .execute("pdf.entry.remove", &json!({ "object": line, "key": "L" }))
        .expect("the points are taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect_err("the line is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **An end of the line that the tree does not know** is refused: the
/// tree draws three and no more.
#[test]
fn an_end_that_is_not_known_is_refused() {
    let mut session = session();
    let directory = directory("badend");
    let line = a_line(&mut session, &directory, false);
    session
        .execute("pdf.entry.set", &json!({ "object": line, "key": "LE", "value": ["None", "Squiggly"] }))
        .expect("the line has an end");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect_err("the line is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
