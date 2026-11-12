//! The **look** of a field.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-look-{name}-{}", std::process::id()));
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

/// The **node** of an object number.
fn the_object(session: &Session, object: u64) -> Option<craftkit_sdk::Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(object))
        .cloned()
}

/// The **text** of a value.
fn text_of(value: &craftkit_sdk::serde_json::Value) -> Option<String> {
    match value {
        craftkit_sdk::serde_json::Value::String(text) => Some(text.clone()),
        craftkit_sdk::serde_json::Value::Object(map) => {
            map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string())
        }
        _ => None,
    }
}

/// The **numbers** of a value.
fn numbers_of(value: &craftkit_sdk::serde_json::Value) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
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

/// A **text field** on a page: the node of it, and the object number
/// it holds.
fn a_text_field(session: &mut Session, directory: &str) -> (u64, u64) {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0] }),
        )
        .expect("the field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let node = the_object(session, object).expect("the field is there");
    (node.id, object)
}

/// **The colours of the field** are set: the border and the fill.
#[test]
fn the_colours_are_set() {
    let mut session = session();
    let directory = directory("colours");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute(
            "pdf.field.look.set",
            &json!({ "field": field, "border": [1.0, 0.0, 0.0], "fill": [0.5, 0.5, 0.5] }),
        )
        .expect("the colours are set");
    let node = the_object(&session, object).expect("the field is there");
    let mark = dictionary_of(entry(&entries_of(&node), "MK").expect("the mark"));
    assert_eq!(numbers_of(entry(&mark, "BC").expect("the border")), vec![1.0, 0.0, 0.0]);
    assert_eq!(numbers_of(entry(&mark, "BG").expect("the fill")), vec![0.5, 0.5, 0.5]);
}

/// **A colour** is three numbers between zero and one, and anything
/// else is refused.
#[test]
fn a_bad_colour_is_refused() {
    let mut session = session();
    let directory = directory("badcolour");
    let (field, _) = a_text_field(&mut session, &directory);
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "border": [1.0, 0.0] }))
        .expect_err("the colour is refused");
    assert!(error.to_string().contains("three numbers"), "the error says why: {error}");
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "border": [1.0, 0.0, 2.0] }))
        .expect_err("the colour is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}

/// **The line** of the field has a width and a style, and the dashed
/// style has the tree's own dashes.
#[test]
fn the_line_is_set() {
    let mut session = session();
    let directory = directory("line");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.look.set", &json!({ "field": field, "width": 3, "style": "Dashed" }))
        .expect("the line is set");
    let node = the_object(&session, object).expect("the field is there");
    let line = dictionary_of(entry(&entries_of(&node), "BS").expect("the line"));
    assert_eq!(entry(&line, "W").and_then(|value| value.as_f64()), Some(3.0));
    assert_eq!(entry(&line, "S").and_then(|value| value.as_str()), Some("D"));
    assert_eq!(entry(&line, "D").and_then(|value| value.as_array().cloned()), Some(vec![json!(3)]));
    // A width that is too wide is refused.
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "width": 20 }))
        .expect_err("the width is refused");
    assert!(error.to_string().contains("width of a line"), "the error says why: {error}");
    // A style that is not one of the tree's five is refused.
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "style": "Wavy" }))
        .expect_err("the style is refused");
    assert!(error.to_string().contains("style of a line"), "the error says why: {error}");
}

/// **The text of the field** has a font, a size and a colour, and they
/// are in the default appearance.
#[test]
fn the_text_is_set() {
    let mut session = session();
    let directory = directory("text");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute(
            "pdf.field.look.set",
            &json!({ "field": field, "font": "Times", "font_size": 16, "text_colour": [0.2, 0.4, 0.6] }),
        )
        .expect("the text is set");
    let node = the_object(&session, object).expect("the field is there");
    let da =
        text_of(entry(&entries_of(&node), "DA").expect("the default appearance")).expect("the appearance");
    assert!(da.contains("/TiRo"), "the font is the tree's: {da}");
    assert!(da.contains("16 Tf"), "the size is the tree's: {da}");
    assert!(da.contains("0.2 0.4 0.6 rg"), "the colour is the tree's: {da}");
    // A size that is too big is refused.
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "font_size": 200 }))
        .expect_err("the size is refused");
    assert!(error.to_string().contains("size of the text"), "the error says why: {error}");
    // A font that is not one of the tree's three is refused.
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "font": "Garamond" }))
        .expect_err("the font is refused");
    assert!(error.to_string().contains("not a font"), "the error says why: {error}");
}

/// **Only a check box** has a mark.
#[test]
fn only_a_check_box_has_a_mark() {
    let mut session = session();
    let directory = directory("mark");
    let (field, _) = a_text_field(&mut session, &directory);
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "check_style": "Star" }))
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("check style"), "the error says why: {error}");
    // A **check box** may have one.
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 20, "rect": [100.0, 300.0, 120.0, 320.0] }),
        )
        .expect("the check box is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let box_node = the_object(&session, object).expect("the check box is there");
    session
        .execute("pdf.field.look.set", &json!({ "field": box_node.id, "check_style": "Star" }))
        .expect("the mark is set");
    let node = the_object(&session, object).expect("the check box is there");
    let mark = dictionary_of(entry(&entries_of(&node), "MK").expect("the mark"));
    assert_eq!(text_of(entry(&mark, "CA").expect("the style")), Some("H".to_string()));
}

/// **The rectangle** of the field is set, and one that is too small is
/// refused.
#[test]
fn the_rectangle_is_set() {
    let mut session = session();
    let directory = directory("rect");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.look.set", &json!({ "field": field, "rect": [300.0, 240.0, 100.0, 200.0] }))
        .expect("the rectangle is set");
    let node = the_object(&session, object).expect("the field is there");
    let rect = numbers_of(entry(&entries_of(&node), "Rect").expect("the rectangle"));
    assert_eq!(rect, vec![100.0, 200.0, 300.0, 240.0]);
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "rect": [100.0, 200.0, 102.0, 202.0] }))
        .expect_err("the rectangle is refused");
    assert!(error.to_string().contains("too small"), "the error says why: {error}");
}

/// **A locked field** refuses every change.
#[test]
fn a_locked_field_refuses() {
    let mut session = session();
    let directory = directory("locked");
    let (field, _) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "locked": true }))
        .expect("the field is locked");
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": field, "width": 2 }))
        .expect_err("the change is refused");
    assert!(error.to_string().contains("locked"), "the error says why: {error}");
}

/// **A node that is not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    let directory = directory("notafield");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": page, "width": 2 }))
        .expect_err("the page is refused");
    assert!(error.to_string().contains("not a field"), "the error says why: {error}");
}
