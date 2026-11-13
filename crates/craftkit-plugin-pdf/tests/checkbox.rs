//! The **check box** that is put on a page.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-checkbox-{name}-{}", std::process::id()))
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

/// The **node** of an object number.
fn the_object(session: &Session, object: u64) -> Option<craftkit_sdk::Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(object))
        .cloned()
}

/// The **numbers** of a value.
fn numbers_of(value: &craftkit_sdk::serde_json::Value) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
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

/// **A check box is put on the page.** It is a widget, it is a button,
/// and it starts **off**.
#[test]
fn a_check_box_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("box");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0] }),
        )
        .expect("the check box is added");
    assert!(outcome.changed);
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    let entries = entries_of(&field);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("Annot"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Widget"));
    assert_eq!(entry(&entries, "FT").and_then(|value| value.as_str()), Some("Btn"));
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(
        text_of(entry(&entries, "DA").expect("the default appearance")),
        Some("/ZaDb 0 Tf 0 g".to_string())
    );
    assert_eq!(text_of(entry(&entries, "T").expect("the name")), Some("Check Box1".to_string()));
    assert_eq!(numbers_of(entry(&entries, "Rect").expect("the rectangle")), vec![100.0, 200.0, 120.0, 220.0]);
    // The **mark**: the style is the check, and the file says so with
    // the code of the glyph.
    let mark = dictionary_of(entry(&entries, "MK").expect("the colours"));
    assert_eq!(text_of(entry(&mark, "CA").expect("the style")), Some("4".to_string()));
}

/// **The name is free,** and the tree's own base is `Check Box`.
#[test]
fn the_name_is_free() {
    let mut session = session();
    let directory = directory("names");
    let page = a_page(&mut session, &directory);
    let first = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0] }),
        )
        .expect("the check box is added");
    assert_eq!(first.data["name"].as_str(), Some("Check Box1"));
    let second = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 11, "rect": [100.0, 300.0, 120.0, 320.0] }),
        )
        .expect("the check box is added");
    assert_eq!(second.data["name"].as_str(), Some("Check Box2"));
}

/// **A style that is not one of the tree's six is refused.**
#[test]
fn a_bad_style_is_refused() {
    let mut session = session();
    let directory = directory("style");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0], "style": "Tick" }),
        )
        .expect_err("the style is refused");
    assert!(error.to_string().contains("not a style"), "the error says why: {error}");
}

/// **Every style of the tree's own is taken.**
#[test]
fn every_style_is_taken() {
    let mut session = session();
    let directory = directory("styles");
    let page = a_page(&mut session, &directory);
    for (index, style) in ["Check", "Circle", "Cross", "Diamond", "Square", "Star"].iter().enumerate() {
        let object = 10 + index as u64;
        let outcome = session
            .execute(
                "pdf.field.checkbox.add",
                &json!({
                    "page": page,
                    "object": object,
                    "rect": [100.0, 200.0, 120.0, 220.0],
                    "style": style,
                }),
            )
            .expect("the check box is added");
        assert_eq!(outcome.data["style"].as_str(), Some(*style));
    }
}

/// **The two drawings** of the field: the one that says it is off, and
/// the one that says it is on — and the field names both.
#[test]
fn the_two_drawings_are_written() {
    let mut session = session();
    let directory = directory("drawings");
    let page = a_page(&mut session, &directory);
    let on = format!("{directory}/on.txt");
    let off = format!("{directory}/off.txt");
    let outcome = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "on_to": on,
                "off_to": off,
            }),
        )
        .expect("the check box is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    // The **appearance**: the normal one, under `N`, with the two
    // states.
    let appearances = dictionary_of(entry(&entries_of(&field), "AP").expect("the appearance"));
    let normal = dictionary_of(entry(&appearances, "N").expect("the normal appearance"));
    let states: Vec<String> = normal.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(states, vec!["Yes".to_string(), "Off".to_string()]);
    // The **bytes**: the off drawing is the frame, and the on drawing
    // is the frame and the mark.
    let off_bytes = std::fs::read_to_string(&off).expect("the drawing is there");
    let on_bytes = std::fs::read_to_string(&on).expect("the drawing is there");
    assert!(off_bytes.contains("1 1 1 rg"), "the background is filled: {off_bytes}");
    assert!(off_bytes.contains("0.55 0.55 0.55 RG"), "the border is stroked: {off_bytes}");
    assert!(on_bytes.starts_with(&off_bytes), "the on drawing is the frame and the mark: {on_bytes}");
    assert!(on_bytes.contains(" S\n"), "the mark is a stroke: {on_bytes}");
}

/// **The mark of a circle** is a circle, and the mark of a star is a
/// polygon of ten points.
#[test]
fn the_mark_is_the_shape_of_the_style() {
    let mut session = session();
    let directory = directory("marks");
    let page = a_page(&mut session, &directory);
    let on = format!("{directory}/on.txt");
    let off = format!("{directory}/off.txt");
    session
        .execute(
            "pdf.field.checkbox.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "style": "Circle",
                "on_to": on,
                "off_to": off,
            }),
        )
        .expect("the check box is added");
    let bytes = std::fs::read_to_string(&on).expect("the drawing is there");
    assert!(bytes.contains(" c "), "the circle is drawn with curves: {bytes}");
    let on = format!("{directory}/star.txt");
    let off = format!("{directory}/star-off.txt");
    session
        .execute(
            "pdf.field.checkbox.add",
            &json!({
                "page": page,
                "object": 20,
                "rect": [100.0, 300.0, 120.0, 320.0],
                "style": "Star",
                "on_to": on,
                "off_to": off,
            }),
        )
        .expect("the check box is added");
    let bytes = std::fs::read_to_string(&on).expect("the drawing is there");
    assert!(bytes.contains("h f"), "the star is a filled polygon: {bytes}");
}

/// **One drawing without the other is refused.**
#[test]
fn one_drawing_without_the_other_is_refused() {
    let mut session = session();
    let directory = directory("onedrawing");
    let page = a_page(&mut session, &directory);
    let on = format!("{directory}/on.txt");
    let error = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0], "on_to": on }),
        )
        .expect_err("the drawing is refused");
    assert!(error.to_string().contains("two drawings"), "the error says why: {error}");
}

/// **The form holds the field** and the page holds it too.
#[test]
fn the_form_and_the_page_hold_the_field() {
    let mut session = session();
    let directory = directory("holds");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0] }),
        )
        .expect("the check box is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    // The **form**.
    let catalog = session
        .document()
        .walk()
        .find(|node| {
            entries_of(node).iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
        })
        .cloned()
        .expect("the catalog is there");
    let catalog_entries = entries_of(&catalog);
    let acro = entry(&catalog_entries, "AcroForm").expect("the form");
    let number = acro
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
        .expect("the form is an object");
    let form = the_object(&session, number).expect("the form is there");
    let fields = entry(&entries_of(&form), "Fields")
        .and_then(|fields| fields.as_array().cloned())
        .expect("the fields");
    assert_eq!(fields.len(), 1);
    // The **page**.
    let page_node = session.document().node(page).cloned().expect("the page is there");
    let annots = entry(&entries_of(&page_node), "Annots")
        .and_then(|annots| annots.as_array().cloned())
        .expect("the annotations");
    assert_eq!(annots.len(), 1);
    let number = annots[0]
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
        .expect("the reference is there");
    assert_eq!(number, object);
}

/// **The value command turns the box on,** and it writes the state as
/// a **name** — the value of a button is a name in the standard, and
/// not a text.
#[test]
fn the_value_command_turns_the_box_on() {
    let mut session = session();
    let directory = directory("value");
    let page = a_page(&mut session, &directory);
    let on = format!("{directory}/on.txt");
    let off = format!("{directory}/off.txt");
    let outcome = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "on_to": on,
                "off_to": off,
            }),
        )
        .expect("the check box is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    session
        .execute("pdf.field.set", &json!({ "field": field.id, "value": "Yes" }))
        .expect("the value is put");
    let field = the_object(&session, object).expect("the field is there");
    let field_entries = entries_of(&field);
    let value = entry(&field_entries, "V").expect("the value");
    assert_eq!(value, &json!("Yes"), "the value is a name and not a text");
    // A state the field does not have is refused.
    let error = session
        .execute("pdf.field.set", &json!({ "field": field.id, "value": "Maybe" }))
        .expect_err("the value is refused");
    assert!(error.to_string().contains("not one of the states"), "the error says why: {error}");
}
