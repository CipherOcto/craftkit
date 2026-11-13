//! The **appearance** of a markup: the highlight, the underline, the
//! strikeout and the squiggly underline.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-markupappearance-{name}-{}", std::process::id()))
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

/// The **quad** of a line of text: the upper left, the upper right,
/// the lower left and the lower right — the tree's own order, and the
/// order the file says.
const QUAD: [f64; 8] = [100.0, 800.0, 200.0, 800.0, 100.0, 780.0, 200.0, 780.0];

/// A **markup** on a page: the node of it.
fn a_markup(session: &mut Session, directory: &str, subtype: &str, object: u64) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": object,
                "subtype": subtype,
                "rect": [100.0, 780.0, 200.0, 800.0],
            }),
        )
        .expect("the markup is added");
    let markup = outcome.data["id"].as_u64().expect("the markup is there");
    session
        .execute("pdf.entry.set", &json!({ "object": markup, "key": "QuadPoints", "value": QUAD }))
        .expect("the markup has its quads");
    session
        .execute("pdf.entry.set", &json!({ "object": markup, "key": "C", "value": [1.0, 1.0, 0.0] }))
        .expect("the markup has its colour");
    markup
}

/// **A highlight is drawn:** a filled quadrilateral over the text.
#[test]
fn a_highlight_is_filled() {
    let mut session = session();
    let directory = directory("highlight");
    let markup = a_markup(&mut session, &directory, "Highlight", 10);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect("the markup is drawn");
    assert!(outcome.changed);
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The colour of the highlight is the colour of the markup.
    assert!(bytes.contains("1 1 0 rg"), "the highlight has its colour: {bytes}");
    // The quadrilateral of the quad, and it is filled.
    assert!(
        bytes.contains("100 800 m 200 800 l 200 780 l 100 780 l h f"),
        "the highlight covers the text: {bytes}"
    );
    assert_eq!(outcome.data["multiply"], json!(true), "the highlight multiplies");
}

/// **A highlight multiplies** over the text, and the file says so.
#[test]
fn a_highlight_multiplies() {
    let mut session = session();
    let directory = directory("multiply");
    let markup = a_markup(&mut session, &directory, "Highlight", 10);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect("the markup is drawn");
    // The **graphics state** of the drawing says so.
    let appearance = outcome.data["appearance"].as_u64().expect("the drawing is there");
    let node = session.document().node(appearance).cloned().expect("the drawing is there");
    let dictionary = entries_of(&node);
    let resources = entry(&dictionary, "Resources").expect("the resources are there");
    let states = dictionary_of(resources);
    let state = states.iter().find(|(key, _)| key == "ExtGState").map(|(_, value)| value).expect("a state");
    let blend = dictionary_of(state);
    let mode = blend.iter().find(|(key, _)| key == "BM").map(|(_, value)| value).expect("a blend mode");
    assert_eq!(mode, &json!("Multiply"), "the highlight multiplies");
    // And the drawing uses it.
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.starts_with("/GS0 gs\n"), "the state is used: {bytes}");
}

/// **An underline is a line** across the text, eight percent of the way
/// up, and seven percent of the height of the text wide.
#[test]
fn an_underline_is_a_line() {
    let mut session = session();
    let directory = directory("underline");
    let markup = a_markup(&mut session, &directory, "Underline", 10);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect("the markup is drawn");
    assert_eq!(outcome.data["multiply"], json!(false), "an underline does not multiply");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The text is twenty high, so the line is 1.4 wide, and it lies at
    // 780 + 20 × 0.08 = 781.6.
    assert!(bytes.contains("1 1 0 RG\n1.4 w\n"), "the line has its width: {bytes}");
    assert!(bytes.contains("100 781.6 m 200 781.6 l S"), "the line is under the text: {bytes}");
}

/// **A strikeout is a line** across the text, forty two percent of the
/// way up.
#[test]
fn a_strikeout_is_higher() {
    let mut session = session();
    let directory = directory("strikeout");
    let markup = a_markup(&mut session, &directory, "StrikeOut", 10);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect("the markup is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // 780 + 20 × 0.42 = 788.4.
    assert!(bytes.contains("100 788.4 m 200 788.4 l S"), "the line is over the text: {bytes}");
}

/// **A squiggly is a zigzag** along the bottom of the text.
#[test]
fn a_squiggly_is_a_zigzag() {
    let mut session = session();
    let directory = directory("squiggly");
    let markup = a_markup(&mut session, &directory, "Squiggly", 10);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect("the markup is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The text is twenty high, so the zigzag is 1 wide and the round
    // joins are on.
    assert!(bytes.contains("1 1 0 RG\n1 w 1 j\n"), "the zigzag has its width: {bytes}");
    // The zigzag starts at the top of the text and falls to the
    // bottom, and it rises by twice the rise, which is six percent of
    // the height of the text.
    assert!(bytes.contains("100 782.4 m"), "the zigzag starts at the top: {bytes}");
    assert!(bytes.contains("105 780 l"), "the zigzag falls: {bytes}");
    assert!(bytes.contains("110 782.4 l"), "the zigzag rises: {bytes}");
    assert!(bytes.ends_with("S\n"), "the zigzag is stroked: {bytes}");
}

/// **A markup without a colour** is refused: a mark in no colour marks
/// nothing.
#[test]
fn a_markup_without_a_colour_is_refused() {
    let mut session = session();
    let directory = directory("nocolour");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 10,
                "subtype": "Highlight",
                "rect": [100.0, 780.0, 200.0, 800.0],
            }),
        )
        .expect("the markup is added");
    let markup = outcome.data["id"].as_u64().expect("the markup is there");
    session
        .execute("pdf.entry.set", &json!({ "object": markup, "key": "QuadPoints", "value": QUAD }))
        .expect("the markup has its quads");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect_err("the markup is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **A markup whose quads are not whole quads** is refused.
#[test]
fn quads_that_are_not_whole_are_refused() {
    let mut session = session();
    let directory = directory("badquads");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 10,
                "subtype": "Highlight",
                "rect": [100.0, 780.0, 200.0, 800.0],
            }),
        )
        .expect("the markup is added");
    let markup = outcome.data["id"].as_u64().expect("the markup is there");
    session
        .execute(
            "pdf.entry.set",
            &json!({ "object": markup, "key": "QuadPoints", "value": [100.0, 800.0, 200.0, 800.0, 100.0, 780.0] }),
        )
        .expect("the markup has its quads");
    session
        .execute("pdf.entry.set", &json!({ "object": markup, "key": "C", "value": [1.0, 1.0, 0.0] }))
        .expect("the markup has its colour");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect_err("the markup is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **A markup without quads** is refused: a mark over nothing marks
/// nothing.
#[test]
fn a_markup_without_quads_is_refused() {
    let mut session = session();
    let directory = directory("noquads");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 10,
                "subtype": "Underline",
                "rect": [100.0, 780.0, 200.0, 800.0],
            }),
        )
        .expect("the markup is added");
    let markup = outcome.data["id"].as_u64().expect("the markup is there");
    session
        .execute("pdf.entry.set", &json!({ "object": markup, "key": "C", "value": [1.0, 1.0, 0.0] }))
        .expect("the markup has its colour");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": markup, "to": to }))
        .expect_err("the markup is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
