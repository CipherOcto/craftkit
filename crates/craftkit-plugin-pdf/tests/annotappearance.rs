//! The **appearance** of a shape.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-annotappearance-{name}-{}", std::process::id()))
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

/// The **pairs** of an inline dictionary.
fn dictionary_of(value: &craftkit_sdk::serde_json::Value) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
        })
        .collect()
}

/// A **shape** on a page: the node of it.
fn a_shape(session: &mut Session, directory: &str, subtype: &str, object: u64) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": object,
                "subtype": subtype,
                "rect": [100.0, 200.0, 300.0, 260.0],
            }),
        )
        .expect("the shape is added");
    outcome.data["id"].as_u64().expect("the shape is there")
}

/// Give the shape a **line colour**, as the tree's own shapes have.
fn with_a_line(session: &mut Session, shape: u64) {
    session
        .execute("pdf.entry.set", &json!({ "object": shape, "key": "C", "value": [0.0, 0.0, 0.0] }))
        .expect("the line has a colour");
}

/// **A square is drawn:** the border of it, and the annotation names
/// the drawing.
#[test]
fn a_square_is_drawn() {
    let mut session = session();
    let directory = directory("square");
    let shape = a_shape(&mut session, &directory, "Square", 10);
    with_a_line(&mut session, shape);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": shape, "to": to }))
        .expect("the shape is drawn");
    assert!(outcome.changed);
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The square is a rectangle, and the line is **inset** by half its
    // width — the width is one point, so the inset is half a point.
    assert!(bytes.contains("100.5 200.5 199 59 re"), "the square is drawn: {bytes}");
    assert!(bytes.ends_with("S\n"), "the line is stroked: {bytes}");
    // The **appearance** of the shape.
    let node = session.document().node(shape).cloned().expect("the shape is there");
    let appearances = dictionary_of(entry(&entries_of(&node), "AP").expect("the appearance"));
    assert_eq!(appearances.len(), 1);
    assert_eq!(appearances[0].0, "N");
}

/// **A circle is drawn** with four curves.
#[test]
fn a_circle_is_drawn() {
    let mut session = session();
    let directory = directory("circle");
    let shape = a_shape(&mut session, &directory, "Circle", 10);
    with_a_line(&mut session, shape);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": shape, "to": to }))
        .expect("the shape is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains(" c\n"), "the circle is drawn with curves: {bytes}");
    assert!(bytes.ends_with("h\nS\n"), "the circle is closed and stroked: {bytes}");
}

/// **The colours** of the shape are drawn: the line and the fill, and
/// four numbers are turned into the red, the green and the blue.
#[test]
fn the_colours_are_drawn() {
    let mut session = session();
    let directory = directory("colours");
    let shape = a_shape(&mut session, &directory, "Square", 10);
    session
        .execute("pdf.entry.set", &json!({ "object": shape, "key": "C", "value": [1.0, 0.0, 0.0] }))
        .expect("the line has a colour");
    session
        .execute("pdf.entry.set", &json!({ "object": shape, "key": "IC", "value": [0.0, 0.0, 0.0, 1.0] }))
        .expect("the fill has a colour");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": shape, "to": to }))
        .expect("the shape is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("1 0 0 RG"), "the line has its colour: {bytes}");
    // The cyan, the magenta, the yellow and the black of the fill are
    // turned into the red, the green and the blue of the screen.
    assert!(bytes.contains("0 0 0 rg"), "the fill has its colour: {bytes}");
    // A line and a fill together are painted both ways.
    assert!(bytes.ends_with("B\n"), "the shape is painted and stroked: {bytes}");
}

/// **A dashed line** has the tree's own dashes.
#[test]
fn a_dashed_line_has_its_dashes() {
    let mut session = session();
    let directory = directory("dashed");
    let shape = a_shape(&mut session, &directory, "Square", 10);
    session
        .execute(
            "pdf.entry.set",
            &json!({ "object": shape, "key": "BS", "value": { "entries": [ ["W", 2], ["S", "D"], ["D", [4, 2]] ] } }),
        )
        .expect("the line is dashed");
    with_a_line(&mut session, shape);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": shape, "to": to }))
        .expect("the shape is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("[4 2] 0 d"), "the dashes are the tree's: {bytes}");
    // The line is two points wide, so the inset is a whole point.
    assert!(bytes.contains("101 201 198 58 re"), "the square is inset: {bytes}");
}

/// **The opacity** of the shape is in the graphics state of the
/// drawing.
#[test]
fn the_opacity_is_in_the_graphics_state() {
    let mut session = session();
    let directory = directory("opacity");
    let shape = a_shape(&mut session, &directory, "Square", 10);
    session
        .execute("pdf.entry.set", &json!({ "object": shape, "key": "CA", "value": 0.5 }))
        .expect("the shape has an opacity");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": shape, "to": to }))
        .expect("the shape is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("/GS0 gs"), "the opacity is used: {bytes}");
}

/// **An annotation that is not drawn here** is refused.
#[test]
fn an_annotation_that_is_not_drawn_here_is_refused() {
    let mut session = session();
    let directory = directory("notashape");
    let shape = a_shape(&mut session, &directory, "Text", 10);
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": shape, "to": to }))
        .expect_err("the annotation is refused");
    assert!(error.to_string().contains("not drawn here"), "the error says why: {error}");
}
