//! The **line** on a page.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-lineadd-{name}-{}", std::process::id()))
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

/// **A line is put on a page:** the annotation, its two points and the
/// page that names it.
#[test]
fn a_line_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("line");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [300.0, 700.0],
            }),
        )
        .expect("the line is added");
    assert!(outcome.changed);
    let line = outcome.data["id"].as_u64().expect("the line is there");
    let node = session.document().node(line).cloned().expect("the line is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Line")), "the line is a line");
    assert_eq!(entry(&entries, "L"), Some(&json!([100.0, 700.0, 300.0, 700.0])), "the two points");
    // The **tree's own** colour and width: the red, and two points.
    assert_eq!(entry(&entries, "C"), Some(&json!([0.89, 0.13, 0.13])), "the colour is the tree's");
    let border = entry(&entries, "BS").expect("the line has a border");
    assert!(
        border["entries"]
            .as_array()
            .map(|entries| entries.iter().any(|entry| entry[0] == json!("W") && entry[1] == json!(2.0)))
            .unwrap_or(false),
        "the width is the tree's: {border}"
    );
    // The page **names** the line.
    let page_node = session.document().node(page).cloned().expect("the page is there");
    let page_entries = entries_of(&page_node);
    let annots = entry(&page_entries, "Annots").expect("the page names the line");
    assert_eq!(annots.as_array().map(|list| list.len()), Some(1), "the page names the line");
}

/// **The rectangle** of the line is the bounds of its two points, grown
/// by half the width of the line and by one point more.
#[test]
fn the_rectangle_is_the_bounds_of_the_points() {
    let mut session = session();
    let directory = directory("rect");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [300.0, 700.0],
            }),
        )
        .expect("the line is added");
    // Half the width is one point, and one point more: two points.
    assert_eq!(outcome.data["rect"], json!([98.0, 698.0, 302.0, 702.0]), "the rectangle");
}

/// **An arrow** is the tree's own, and the rectangle grows by its size.
#[test]
fn an_arrow_grows_the_rectangle() {
    let mut session = session();
    let directory = directory("arrow");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [300.0, 700.0],
                "arrow": true,
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line is there");
    let node = session.document().node(line).cloned().expect("the line is there");
    let entries = entries_of(&node);
    // The tree's own arrow: none at the start, an open arrow at the end.
    assert_eq!(entry(&entries, "LE"), Some(&json!(["None", "OpenArrow"])), "the arrow is the tree's");
    // The size of the arrow is six points plus three times the width:
    // twelve, and the rectangle grows by that and by one more.
    assert_eq!(outcome.data["rect"], json!([86.0, 686.0, 314.0, 714.0]), "the rectangle");
}

/// **A line that is too short** is refused.
#[test]
fn a_line_that_is_too_short_is_refused() {
    let mut session = session();
    let directory = directory("short");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [100.5, 700.0],
            }),
        )
        .expect_err("the line is refused");
    assert!(error.to_string().contains("at least a point"), "the error says why: {error}");
}

/// **A point that is not two numbers** is refused.
#[test]
fn a_point_that_is_not_two_numbers_is_refused() {
    let mut session = session();
    let directory = directory("point");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0],
                "to": [300.0, 700.0],
            }),
        )
        .expect_err("the line is refused");
    assert!(error.to_string().contains("two numbers"), "the error says why: {error}");
}

/// **A colour outside zero and one** is refused.
#[test]
fn a_colour_outside_zero_and_one_is_refused() {
    let mut session = session();
    let directory = directory("colour");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [300.0, 700.0],
                "colour": [1.0, 0.0, 2.0],
            }),
        )
        .expect_err("the line is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}

/// **A width below zero** is the tree's own zero.
#[test]
fn a_width_below_zero_is_zero() {
    let mut session = session();
    let directory = directory("width");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.line.add",
            &json!({
                "page": page,
                "object": 10,
                "from": [100.0, 700.0],
                "to": [300.0, 700.0],
                "width": -5.0,
            }),
        )
        .expect("the line is added");
    assert_eq!(outcome.data["width"], json!(0.0), "the width is the tree's zero");
    // And the rectangle grows by one point only.
    assert_eq!(outcome.data["rect"], json!([99.0, 699.0, 301.0, 701.0]), "the rectangle");
}
