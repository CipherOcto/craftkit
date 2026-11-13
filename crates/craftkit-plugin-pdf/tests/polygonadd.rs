//! The **polygon** on a page, and the tree's connected lines.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-polygonadd-{name}-{}", std::process::id()))
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

/// The **entries** of the dictionary of a node.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    node.data["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(
    entries: &'a [(String, craftkit_sdk::serde_json::Value)],
    key: &str,
) -> Option<&'a craftkit_sdk::serde_json::Value> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **corners** of the tree's own test polygon.
const CORNERS: [[f64; 2]; 4] = [[100.0, 700.0], [300.0, 700.0], [300.0, 800.0], [100.0, 800.0]];

/// **A polygon is put on a page:** its corners, its colour and the page
/// that names it.
#[test]
fn a_polygon_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("polygon");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polygon",
                "vertices": CORNERS,
            }),
        )
        .expect("the polygon is added");
    assert!(outcome.changed);
    let polygon = outcome.data["id"].as_u64().expect("the polygon is there");
    let node = session.document().node(polygon).cloned().expect("the polygon is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Polygon")), "the polygon is a polygon");
    assert_eq!(
        entry(&entries, "Vertices"),
        Some(&json!([100.0, 700.0, 300.0, 700.0, 300.0, 800.0, 100.0, 800.0])),
        "the corners are the tree's own"
    );
    assert_eq!(entry(&entries, "C"), Some(&json!([0.89, 0.13, 0.13])), "the colour is the tree's");
    // The page **names** the polygon.
    let page_node = session.document().node(page).cloned().expect("the page is there");
    let page_entries = entries_of(&page_node);
    let annots = entry(&page_entries, "Annots").expect("the page names the polygon");
    assert_eq!(annots.as_array().map(|list| list.len()), Some(1), "the page names the polygon");
}

/// **A connected line** is the same thing left open.
#[test]
fn a_connected_line_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("polyline");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polyline",
                "vertices": [[100.0, 700.0], [300.0, 800.0]],
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line is there");
    let node = session.document().node(line).cloned().expect("the line is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("PolyLine")), "the line is a connected line");
}

/// **The rectangle** of a polygon is the bounds of its corners, grown
/// by half the width of the line and by one point more.
#[test]
fn the_rectangle_is_the_bounds_of_the_corners() {
    let mut session = session();
    let directory = directory("rect");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polygon",
                "vertices": CORNERS,
            }),
        )
        .expect("the polygon is added");
    assert_eq!(outcome.data["rect"], json!([98.0, 698.0, 302.0, 802.0]), "the rectangle");
}

/// **A cloudy border** is the tree's own, and the rectangle grows by
/// one and a half cloud radii: the radius is six points plus one and a
/// half times the width.
#[test]
fn a_cloud_grows_the_rectangle() {
    let mut session = session();
    let directory = directory("cloud");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polygon",
                "vertices": CORNERS,
                "cloud": true,
            }),
        )
        .expect("the polygon is added");
    let polygon = outcome.data["id"].as_u64().expect("the polygon is there");
    let node = session.document().node(polygon).cloned().expect("the polygon is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "IT"), Some(&json!("PolygonCloud")), "the intent is the tree's");
    let border = entry(&entries, "BE").expect("the border is cloudy");
    assert!(
        border["entries"]
            .as_array()
            .map(|entries| entries.iter().any(|entry| entry[0] == json!("S") && entry[1] == json!("C")))
            .unwrap_or(false),
        "the border says so: {border}"
    );
    // The radius of a cloud is nine, and the rectangle grows by one and
    // a half of it and by the two points of the line.
    assert_eq!(outcome.data["rect"], json!([84.5, 684.5, 315.5, 815.5]), "the rectangle");
}

/// **A polygon with two corners** is refused: the tree's own rule.
#[test]
fn a_polygon_with_two_corners_is_refused() {
    let mut session = session();
    let directory = directory("twocorners");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polygon",
                "vertices": [[100.0, 700.0], [300.0, 800.0]],
            }),
        )
        .expect_err("the polygon is refused");
    assert!(error.to_string().contains("at least 3 points"), "the error says why: {error}");
}

/// **A cloud on a connected line** is refused: the tree's own rule.
#[test]
fn a_cloud_on_a_connected_line_is_refused() {
    let mut session = session();
    let directory = directory("cloudline");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polyline",
                "vertices": [[100.0, 700.0], [300.0, 800.0]],
                "cloud": true,
            }),
        )
        .expect_err("the line is refused");
    assert!(error.to_string().contains("no cloudy border"), "the error says why: {error}");
}

/// **A polygon that is too small** is refused.
#[test]
fn a_polygon_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("small");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "polygon",
                "vertices": [[100.0, 700.0], [100.5, 700.0], [100.0, 700.5]],
            }),
        )
        .expect_err("the polygon is refused");
    assert!(error.to_string().contains("a point wide"), "the error says why: {error}");
}

/// **A kind that is not one of the tree's two** is refused.
#[test]
fn a_kind_that_is_not_known_is_refused() {
    let mut session = session();
    let directory = directory("kind");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.polygon.add",
            &json!({
                "page": page,
                "object": 10,
                "kind": "blob",
                "vertices": CORNERS,
            }),
        )
        .expect_err("the polygon is refused");
    assert!(error.to_string().contains("polygon` or `polyline"), "the error says why: {error}");
}

/// **The fill** of a polygon is written when the caller says one.
#[test]
fn the_fill_is_written_when_it_is_said() {
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
    let node = session.document().node(polygon).cloned().expect("the polygon is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "IC"), Some(&json!([0.0, 0.0, 1.0])), "the fill is there");
}
