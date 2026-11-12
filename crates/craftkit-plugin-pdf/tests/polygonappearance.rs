//! The **appearance** of a polygon: the shape of corners and the
//! cloudy border.

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
        std::env::temp_dir().join(format!("craftkit-v2-polygonappearance-{name}-{}", std::process::id()));
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

/// The **corners** of the tree's own test polygon.
const CORNERS: [[f64; 2]; 4] = [[100.0, 700.0], [300.0, 700.0], [300.0, 800.0], [100.0, 800.0]];

/// A **polygon** on a page: the node of it.
fn a_polygon(session: &mut Session, directory: &str, kind: &str, cloud: bool) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": kind,
                "vertices": CORNERS,
                "cloud": cloud,
            }),
        )
        .expect("the polygon is added");
    outcome.data["id"].as_u64().expect("the polygon is there")
}

/// **A polygon is drawn:** its corners, and it is closed.
#[test]
fn a_polygon_is_drawn() {
    let mut session = session();
    let directory = directory("polygon");
    let polygon = a_polygon(&mut session, &directory, "polygon", false);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": polygon, "to": to }))
        .expect("the polygon is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tree's own** red, and the width of two points.
    assert!(bytes.contains("0.89 0.13 0.13 RG"), "the line has its colour: {bytes}");
    assert!(bytes.contains("2 w 1 J 1 j"), "the line has its width: {bytes}");
    // The **corners** of the shape, and it is closed.
    assert!(
        bytes.contains("100 700 m\n300 700 l\n300 800 l\n100 800 l\nh\n"),
        "the polygon is drawn: {bytes}"
    );
    assert!(bytes.ends_with("S\n"), "the polygon is stroked: {bytes}");
}

/// **A polygon with a fill** is painted and stroked.
#[test]
fn a_polygon_with_a_fill_is_painted() {
    let mut session = session();
    let directory = directory("fill");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polygon",
                "vertices": CORNERS,
                "fill": [0.0, 0.0, 1.0],
            }),
        )
        .expect("the polygon is added");
    let polygon = outcome.data["id"].as_u64().expect("the polygon is there");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": polygon, "to": to }))
        .expect("the polygon is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("0 0 1 rg"), "the fill has its colour: {bytes}");
    assert!(bytes.ends_with("B\n"), "the polygon is painted and stroked: {bytes}");
}

/// **A connected line** is drawn through its corners and left open.
#[test]
fn a_connected_line_is_drawn() {
    let mut session = session();
    let directory = directory("polyline");
    let line = a_polygon(&mut session, &directory, "polyline", false);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect("the line is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("100 700 m\n300 700 l\n300 800 l\n100 800 l\n"), "the line is drawn: {bytes}");
    // The line is **not** closed.
    assert!(!bytes.contains("h\n"), "the line is left open: {bytes}");
}

/// **A cloud** is drawn as a row of bumps along the edges of the shape:
/// the radius of a bump is nine points, and each edge takes as many
/// bumps as fit in it — eleven, six, eleven and six.
#[test]
fn a_cloud_is_drawn() {
    let mut session = session();
    let directory = directory("cloud");
    let polygon = a_polygon(&mut session, &directory, "polygon", true);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": polygon, "to": to }))
        .expect("the polygon is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    let bumps = bytes.lines().filter(|line| line.ends_with(" c")).count();
    assert_eq!(bumps, 34, "the cloud is a row of bumps: {bytes}");
    assert!(bytes.ends_with("S\n"), "the cloud is stroked: {bytes}");
}

/// **A cloud on anything that is not a polygon** is refused: the tree
/// draws a cloudy border for a polygon and for nothing else.
#[test]
fn a_cloud_on_a_square_is_refused() {
    let mut session = session();
    let directory = directory("cloudsquare");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 10,
                "subtype": "Square",
                "rect": [100.0, 700.0, 300.0, 800.0],
            }),
        )
        .expect("the square is added");
    let square = outcome.data["id"].as_u64().expect("the square is there");
    session
        .execute(
            "pdf.entry.set",
            &json!({ "object": square, "key": "BE", "value": { "entries": [ ["S", "C"], ["I", 1] ] } }),
        )
        .expect("the border is cloudy");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": square, "to": to }))
        .expect_err("the square is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **A connected line with a cloudy border** is refused.
#[test]
fn a_cloud_on_a_connected_line_is_refused() {
    let mut session = session();
    let directory = directory("cloudline");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polyline",
                "vertices": CORNERS,
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line is there");
    session
        .execute(
            "pdf.entry.set",
            &json!({ "object": line, "key": "BE", "value": { "entries": [ ["S", "C"], ["I", 1] ] } }),
        )
        .expect("the border is cloudy");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": line, "to": to }))
        .expect_err("the line is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **A polygon without its corners** is refused.
#[test]
fn a_polygon_without_its_corners_is_refused() {
    let mut session = session();
    let directory = directory("nocorners");
    let polygon = a_polygon(&mut session, &directory, "polygon", false);
    session
        .execute("pdf.entry.remove", &json!({ "object": polygon, "key": "Vertices" }))
        .expect("the corners are taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": polygon, "to": to }))
        .expect_err("the polygon is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
