//! The **appearance** of the ink: the strokes of a hand.

use craftkit_sdk::serde_json::{Value as Json, json};
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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-inkappearance-{name}-{}", std::process::id()))
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

/// The **strokes** of the tree's own test ink.
const STROKES: [[f64; 2]; 3] = [[100.0, 700.0], [200.0, 750.0], [300.0, 800.0]];

/// **Ink** on a page: the node of it.
fn an_ink(session: &mut Session, directory: &str, strokes: Json) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.ink.add",
            &json!({
                "page": page,
                "object": 10,
                "strokes": strokes,
            }),
        )
        .expect("the ink is added");
    outcome.data["id"].as_u64().expect("the ink is there")
}

/// The **strokes** as the test gives them: one stroke of points.
fn strokes_of(stroke: &[[f64; 2]]) -> Json {
    json!([stroke])
}

/// **The ink is drawn:** its points, in a stroke of their own.
#[test]
fn the_ink_is_drawn() {
    let mut session = session();
    let directory = directory("ink");
    let ink = an_ink(&mut session, &directory, strokes_of(&STROKES));
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": ink, "to": to }))
        .expect("the ink is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tree's own** blue, and the width of two points.
    assert!(bytes.contains("0 0.4 0.87 RG"), "the ink has its colour: {bytes}");
    assert!(bytes.contains("2 w 1 J 1 j"), "the ink has its width: {bytes}");
    // The **points** of the stroke.
    assert!(bytes.contains("100 700 m\n200 750 l\n300 800 l\nS\n"), "the ink is drawn: {bytes}");
}

/// **A stroke of one point** is a dot: the tree draws the point and a
/// hair along it.
#[test]
fn a_stroke_of_one_point_is_a_dot() {
    let mut session = session();
    let directory = directory("dot");
    let ink = an_ink(&mut session, &directory, json!([[[100.0, 700.0]]]));
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": ink, "to": to }))
        .expect("the ink is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("100 700 m\n100.01 700 l\nS\n"), "the dot is drawn: {bytes}");
}

/// **Each stroke is drawn on its own.**
#[test]
fn each_stroke_is_drawn_on_its_own() {
    let mut session = session();
    let directory = directory("strokes");
    let ink = an_ink(
        &mut session,
        &directory,
        json!([[[100.0, 700.0], [200.0, 750.0]], [[300.0, 800.0], [400.0, 850.0]]]),
    );
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": ink, "to": to }))
        .expect("the ink is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(
        bytes.contains("100 700 m\n200 750 l\nS\n300 800 m\n400 850 l\nS\n"),
        "the strokes are drawn one by one: {bytes}"
    );
}

/// **Ink without a colour** is refused.
#[test]
fn ink_without_a_colour_is_refused() {
    let mut session = session();
    let directory = directory("nocolour");
    let ink = an_ink(&mut session, &directory, strokes_of(&STROKES));
    session
        .execute("pdf.entry.remove", &json!({ "object": ink, "key": "C" }))
        .expect("the colour is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": ink, "to": to }))
        .expect_err("the ink is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **Ink without its strokes** is refused.
#[test]
fn ink_without_its_strokes_is_refused() {
    let mut session = session();
    let directory = directory("nostrokes");
    let ink = an_ink(&mut session, &directory, strokes_of(&STROKES));
    session
        .execute("pdf.entry.remove", &json!({ "object": ink, "key": "InkList" }))
        .expect("the strokes are taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": ink, "to": to }))
        .expect_err("the ink is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
