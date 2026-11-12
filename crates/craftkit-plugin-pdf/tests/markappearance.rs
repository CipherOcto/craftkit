//! The **appearance** of the mark of Fill and Sign: the check, the
//! cross, the dot and the line.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-markappearance-{name}-{}", std::process::id()));
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

/// A **mark** on a page: the node of it.
fn a_mark(session: &mut Session, directory: &str, mark: &str) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.mark.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 130.0, 730.0],
                "mark": mark,
            }),
        )
        .expect("the mark is added");
    outcome.data["id"].as_u64().expect("the mark is there")
}

/// **The check is drawn:** a mark of three points.
#[test]
fn the_check_is_drawn() {
    let mut session = session();
    let directory = directory("check");
    let mark = a_mark(&mut session, &directory, "check");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The rectangle is thirty wide and thirty high, so the line is a
    // twelfth of the smaller of them: three and six tenths of a point.
    assert!(bytes.contains("0 0 0 RG\n3.6 w 1 J\n"), "the mark has its colour: {bytes}");
    // The three points of the check: a fifteenth of the width along,
    // two fifths down, and then up to the top.
    assert!(bytes.contains("104.5 715 m 112 706 l 126.4 725.5 l S"), "the check is drawn: {bytes}");
}

/// **The cross is drawn:** two lines that cross.
#[test]
fn the_cross_is_drawn() {
    let mut session = session();
    let directory = directory("cross");
    let mark = a_mark(&mut session, &directory, "cross");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(
        bytes.contains("105.4 705.4 m 124.6 724.6 l 105.4 724.6 m 124.6 705.4 l S"),
        "the cross is drawn: {bytes}"
    );
}

/// **The dot is drawn:** a filled circle.
#[test]
fn the_dot_is_drawn() {
    let mut session = session();
    let directory = directory("dot");
    let mark = a_mark(&mut session, &directory, "dot");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The radius is three tenths of the smaller of the sides: nine.
    assert!(bytes.contains("0 0 0 rg\n"), "the dot has its colour: {bytes}");
    assert!(bytes.contains(" c\n"), "the dot is a circle: {bytes}");
    assert!(bytes.ends_with("f\n"), "the dot is filled: {bytes}");
}

/// **The line is drawn:** a line across the middle of the rectangle.
#[test]
fn the_line_is_drawn() {
    let mut session = session();
    let directory = directory("line");
    let mark = a_mark(&mut session, &directory, "line");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The line is two points wide — the height of the rectangle, and
    // never more than two.
    assert!(bytes.contains("0 0 0 RG\n2 w 1 J\n"), "the line has its width: {bytes}");
    assert!(bytes.contains("100 715 m 130 715 l S"), "the line is drawn: {bytes}");
}

/// **A mark without a colour** is the tree's own black.
#[test]
fn a_mark_without_a_colour_is_black() {
    let mut session = session();
    let directory = directory("nocolour");
    let mark = a_mark(&mut session, &directory, "check");
    session
        .execute("pdf.entry.remove", &json!({ "object": mark, "key": "C" }))
        .expect("the colour is taken away");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect("the mark is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("0 0 0 RG"), "the mark is the tree's black: {bytes}");
}

/// **A mark with a name that is not one of the tree's four** is
/// refused.
#[test]
fn a_mark_with_an_unknown_name_is_refused() {
    let mut session = session();
    let directory = directory("unknown");
    let mark = a_mark(&mut session, &directory, "check");
    session
        .execute("pdf.entry.set", &json!({ "object": mark, "key": "Name", "value": "PCSwoosh" }))
        .expect("the name is changed");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": mark, "to": to }))
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
