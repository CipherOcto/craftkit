//! The **appearance** of a text box: the text of it, in the tree's
//! Helvetica.

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
        std::env::temp_dir().join(format!("craftkit-v2-freetextappearance-{name}-{}", std::process::id()));
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

/// A **text box** on a page: the node of it.
fn a_text_box(session: &mut Session, directory: &str, contents: &str) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": contents,
            }),
        )
        .expect("the text box is added");
    outcome.data["id"].as_u64().expect("the text box is there")
}

/// **The text box is drawn:** the text of it, in the tree's Helvetica.
#[test]
fn the_text_box_is_drawn() {
    let mut session = session();
    let directory = directory("freetext");
    let box_of = a_text_box(&mut session, &directory, "a note");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect("the text box is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The text is **clipped** to the box, and set in the tree's
    // Helvetica at the tree's own size.
    assert!(bytes.contains("100 700 200 60 re W n"), "the text is clipped: {bytes}");
    assert!(bytes.contains("BT\n/Helv 12 Tf\n0 0 0 rg\n"), "the text is set: {bytes}");
    // The text sits two points inside the box, and a tenth of the size
    // of the font below the top of it.
    assert!(bytes.contains("1 0 0 1 102 747.2 Tm (a note) Tj"), "the text is drawn: {bytes}");
    assert!(bytes.ends_with("ET\n"), "the text is done: {bytes}");
}

/// **The font** of the text is in the resources of the drawing: the
/// tree's own Helvetica.
#[test]
fn the_font_is_in_the_resources() {
    let mut session = session();
    let directory = directory("font");
    let box_of = a_text_box(&mut session, &directory, "a note");
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect("the text box is drawn");
    let appearance = outcome.data["appearance"].as_u64().expect("the drawing is there");
    let node = session.document().node(appearance).cloned().expect("the drawing is there");
    let entries = entries_of(&node);
    let resources = entry(&entries, "Resources").expect("the resources are there");
    let fonts = dictionary_of(resources);
    let font = fonts.iter().find(|(key, _)| key == "Font").map(|(_, value)| value).expect("a font");
    let names = dictionary_of(font);
    let helv = names.iter().find(|(key, _)| key == "Helv").map(|(_, value)| value).expect("the font");
    let base = dictionary_of(helv);
    assert!(
        base.iter().any(|(key, value)| key == "BaseFont" && value.as_str() == Some("Helvetica")),
        "the font is the tree's"
    );
}

/// **A background** is filled over the box.
#[test]
fn a_background_is_filled() {
    let mut session = session();
    let directory = directory("background");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "background": [1.0, 1.0, 0.8],
            }),
        )
        .expect("the text box is added");
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect("the text box is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("1 1 0.8 rg\n100 700 200 60 re f\n"), "the background is filled: {bytes}");
}

/// **A border** is stroked inside the box.
#[test]
fn a_border_is_stroked() {
    let mut session = session();
    let directory = directory("border");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "width": 2.0,
            }),
        )
        .expect("the text box is added");
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect("the text box is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The border is two points wide and it is inside the box, and the
    // text is four points from the edge.
    assert!(bytes.contains("2 w\n101 701 198 58 re S\n"), "the border is stroked: {bytes}");
    assert!(bytes.contains("1 0 0 1 104 745.2 Tm"), "the text is inside the border: {bytes}");
}

/// **The text is centred** when the box says so.
#[test]
fn the_text_is_centred() {
    let mut session = session();
    let directory = directory("centre");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.freetext.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 760.0],
                "contents": "a note",
                "align": 1,
            }),
        )
        .expect("the text box is added");
    let box_of = outcome.data["id"].as_u64().expect("the text box is there");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect("the text box is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The width of "a note" at twelve points is 33.804, and the text
    // is centred in the 196 points of the box.
    assert!(bytes.contains("1 0 0 1 183.098 747.2 Tm"), "the text is centred: {bytes}");
}

/// **The text is wrapped** at the width of the box, and a word that
/// does not fit is left long.
#[test]
fn the_text_is_wrapped() {
    let mut session = session();
    let directory = directory("wrap");
    let text = "aaaaaaaaaa bbbbbbbbbb cccccccccc dddddddddd eeeeeeeeee ffffffffff gggggggggg hhhhhhhhhh";
    let box_of = a_text_box(&mut session, &directory, text);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect("the text box is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // Two words fit on a line and three do not, so the text is four
    // lines, and the last of them is at 704.
    let lines = bytes.lines().filter(|line| line.ends_with("Tj")).count();
    assert_eq!(lines, 4, "the text is four lines: {bytes}");
    assert!(bytes.contains("(aaaaaaaaaa bbbbbbbbbb) Tj"), "the first line: {bytes}");
    assert!(bytes.contains("(hhhhhhhhhh) Tj"), "the last line: {bytes}");
}

/// **A text box without its rectangle** is refused.
#[test]
fn a_text_box_without_its_rectangle_is_refused() {
    let mut session = session();
    let directory = directory("norect");
    let box_of = a_text_box(&mut session, &directory, "a note");
    session
        .execute("pdf.entry.remove", &json!({ "object": box_of, "key": "Rect" }))
        .expect("the rectangle is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": box_of, "to": to }))
        .expect_err("the text box is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
