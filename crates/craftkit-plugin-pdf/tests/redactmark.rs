//! The **mark of a redaction** on a page.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-redactmark-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file of PDF with a line of **text** on its page.
fn a_file() -> Vec<u8> {
    let content = String::from("BT /F1 12 Tf 72 700 Td (Hello world) Tj ET\n");
    let widths: Vec<String> = (32..=126).map(|_| "500".to_string()).collect();
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}endstream", content.len())),
        (5, format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 32 /LastChar 126 /Widths [{}] /Encoding /WinAnsiEncoding >>",
            widths.join(" ")
        )),
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

/// Load the file and give the **page** it has.
fn a_page(session: &mut Session, name: &str) -> u64 {
    let directory = directory(name);
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

/// The **region** the text is under.
fn region() -> craftkit_sdk::serde_json::Value {
    json!([0.0, 690.0, 595.0, 720.0])
}

/// **The mark is put on the page:** the quads of the text it covers and
/// the tree's own colours.
#[test]
fn the_mark_is_put_on_the_page() {
    let mut session = session();
    let page = a_page(&mut session, "redact");
    let outcome = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
            }),
        )
        .expect("the mark is added");
    assert!(outcome.changed);
    let mark = outcome.data["id"].as_u64().expect("the mark is there");
    let node = session.document().node(mark).cloned().expect("the mark is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Redact")), "the mark is a redaction");
    // The **quads** are the lines of the text under the region.
    let quads = entry(&entries, "QuadPoints").expect("the mark has quads");
    assert_eq!(quads.as_array().map(|list| list.len()), Some(8), "the quads are there");
    // The **tree's own** red, and the tree's own black fill.
    assert_eq!(entry(&entries, "C"), Some(&json!([0.89, 0.13, 0.13])), "the colour is the tree's");
    assert_eq!(entry(&entries, "IC"), Some(&json!([0.0, 0.0, 0.0])), "the fill is the tree's");
}

/// **The rectangle** of the mark is the bounds of its quads, and it is
/// not grown.
#[test]
fn the_rectangle_is_the_bounds_of_the_quads() {
    let mut session = session();
    let page = a_page(&mut session, "rect");
    let outcome = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
            }),
        )
        .expect("the mark is added");
    let mark = outcome.data["id"].as_u64().expect("the mark is there");
    let node = session.document().node(mark).cloned().expect("the mark is there");
    let entries = entries_of(&node);
    let quads = entry(&entries, "QuadPoints").expect("the mark has quads");
    let numbers: Vec<f64> = quads
        .as_array()
        .map(|list| list.iter().filter_map(|value| value.as_f64()).collect())
        .unwrap_or_default();
    let mut left = f64::INFINITY;
    let mut bottom = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut top = f64::NEG_INFINITY;
    for index in (0..4).rev() {
        left = left.min(numbers[index * 2]);
        bottom = bottom.min(numbers[index * 2 + 1]);
        right = right.max(numbers[index * 2]);
        top = top.max(numbers[index * 2 + 1]);
    }
    let rect = entry(&entries, "Rect").expect("the mark has a rectangle");
    assert_eq!(rect.as_array().map(|list| list.len()), Some(4), "the rectangle is there");
    let rectangle: Vec<f64> = rect
        .as_array()
        .map(|list| list.iter().filter_map(|value| value.as_f64()).collect())
        .unwrap_or_default();
    assert_eq!(rectangle[0], left, "the left is the left of the quads");
    assert_eq!(rectangle[1], bottom, "the bottom is the bottom of the quads");
    assert_eq!(rectangle[2], right, "the right is the right of the quads");
    assert_eq!(rectangle[3], top, "the top is the top of the quads");
}

/// **The overlay text** is written over the area, and the file says how
/// it is set.
#[test]
fn the_overlay_text_is_written() {
    let mut session = session();
    let page = a_page(&mut session, "overlay");
    let outcome = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
                "text": "taken away",
                "size": 14.0,
                "align": 1,
                "repeat": true,
            }),
        )
        .expect("the mark is added");
    let mark = outcome.data["id"].as_u64().expect("the mark is there");
    let node = session.document().node(mark).cloned().expect("the mark is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "OverlayText"), Some(&json!({ "string": "taken away" })), "the text is there");
    // The **appearance of the overlay** is the tree's own string.
    assert_eq!(
        entry(&entries, "DA"),
        Some(&json!("0.89 0.13 0.13 rg /Helv 14 Tf")),
        "the appearance is the tree's own"
    );
    assert_eq!(entry(&entries, "Q"), Some(&json!(1)), "the alignment is the tree's");
    assert_eq!(entry(&entries, "Repeat"), Some(&json!(true)), "the text repeats");
}

/// **A region with no text under it** is refused.
#[test]
fn a_region_with_no_text_under_it_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "notext");
    let error = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": [0.0, 100.0, 595.0, 120.0],
            }),
        )
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("no text under the region"), "the error says why: {error}");
}

/// **A colour outside zero and one** is refused.
#[test]
fn a_colour_outside_zero_and_one_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "colour");
    let error = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
                "colour": [0.89, 0.13, 1.5],
            }),
        )
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}

/// **A size outside the tree's bounds** is refused.
#[test]
fn a_size_outside_the_trees_bounds_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "size");
    let error = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
                "size": 500.0,
            }),
        )
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("four hundred"), "the error says why: {error}");
}

/// **An alignment that is not one of the tree's three** is refused.
#[test]
fn an_alignment_that_is_not_known_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, "align");
    let error = session
        .execute(
            "pdf.annot.redact.add",
            &json!({
                "page": page,
                "object": 10,
                "region": region(),
                "align": 7.0,
            }),
        )
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("0, 1 or 2"), "the error says why: {error}");
}
