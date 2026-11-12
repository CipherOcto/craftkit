//! The **drop-down** that is put on a page.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-combobox-{name}-{}", std::process::id()));
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

/// The **texts** of a list of values.
fn texts_of(value: &craftkit_sdk::serde_json::Value) -> Vec<String> {
    value.as_array().map(|list| list.iter().filter_map(text_of).collect()).unwrap_or_default()
}

/// **A drop-down is put on the page.** It is a widget, it is a choice,
/// and it offers the options it was given.
#[test]
fn a_drop_down_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("combo");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first", "the second"],
            }),
        )
        .expect("the drop-down is added");
    assert!(outcome.changed);
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    let entries = entries_of(&field);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("Annot"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Widget"));
    assert_eq!(entry(&entries, "FT").and_then(|value| value.as_str()), Some("Ch"));
    // The tree's own flag of a drop-down.
    assert_eq!(entry(&entries, "Ff").and_then(|value| value.as_f64()), Some(131072.0));
    assert_eq!(
        text_of(entry(&entries, "DA").expect("the default appearance")),
        Some("/Helv 12 Tf 0 g".to_string())
    );
    assert_eq!(text_of(entry(&entries, "T").expect("the name")), Some("Dropdown1".to_string()));
    // The **options**.
    let options =
        entry(&entries, "Opt").and_then(|options| options.as_array().cloned()).expect("the options");
    assert_eq!(texts_of(&json!(options)), vec!["the first".to_string(), "the second".to_string()]);
}

/// **A drop-down that may be edited** carries the tree's own second
/// flag.
#[test]
fn an_editable_drop_down_says_so() {
    let mut session = session();
    let directory = directory("editable");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first"],
                "editable": true,
            }),
        )
        .expect("the drop-down is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    // The drop-down flag and the edit flag.
    assert_eq!(entry(&entries_of(&field), "Ff").and_then(|value| value.as_f64()), Some(393216.0));
}

/// **The options are texts,** and a list that holds something else is
/// refused.
#[test]
fn options_that_are_not_texts_are_refused() {
    let mut session = session();
    let directory = directory("options");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first", 7],
            }),
        )
        .expect_err("the options are refused");
    assert!(error.to_string().contains("list of texts"), "the error says why: {error}");
    let error = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": "the first",
            }),
        )
        .expect_err("the options are refused");
    assert!(error.to_string().contains("list of texts"), "the error says why: {error}");
}

/// **The name is free,** and the tree's own base is `Dropdown`.
#[test]
fn the_name_is_free() {
    let mut session = session();
    let directory = directory("names");
    let page = a_page(&mut session, &directory);
    let first = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first"],
            }),
        )
        .expect("the drop-down is added");
    assert_eq!(first.data["name"].as_str(), Some("Dropdown1"));
    let second = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 11,
                "rect": [100.0, 300.0, 500.0, 340.0],
                "options": ["the first"],
            }),
        )
        .expect("the drop-down is added");
    assert_eq!(second.data["name"].as_str(), Some("Dropdown2"));
}

/// **The value of a drop-down** is one of its options — unless the
/// field may be edited, and then anything may be written in it.
#[test]
fn the_value_must_be_one_of_the_options() {
    let mut session = session();
    let directory = directory("value");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first"],
                "value": "the third",
            }),
        )
        .expect_err("the value is refused");
    assert!(error.to_string().contains("not one of the options"), "the error says why: {error}");
    // An **editable** field may hold anything.
    let outcome = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 11,
                "rect": [100.0, 300.0, 500.0, 340.0],
                "options": ["the first"],
                "value": "something else",
                "editable": true,
            }),
        )
        .expect("the drop-down is added");
    assert_eq!(outcome.data["value"].as_str(), Some("something else"));
}

/// **The drawing** of the field: the frame and the text it holds.
#[test]
fn the_drawing_is_written() {
    let mut session = session();
    let directory = directory("drawing");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first", "the second"],
                "value": "the second",
                "appearance_to": to,
            }),
        )
        .expect("the drop-down is added");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("1 1 1 rg"), "the background is filled: {bytes}");
    assert!(bytes.contains("(the second) Tj"), "the text is drawn: {bytes}");
}

/// **The form holds the field** and the page holds it too.
#[test]
fn the_form_and_the_page_hold_the_field() {
    let mut session = session();
    let directory = directory("holds");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "options": ["the first"],
            }),
        )
        .expect("the drop-down is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    // The **page**.
    let page_node = session.document().node(page).cloned().expect("the page is there");
    let annots = entry(&entries_of(&page_node), "Annots")
        .and_then(|annots| annots.as_array().cloned())
        .expect("the annotations");
    assert_eq!(annots.len(), 1);
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
    let held = fields[0]
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
        .expect("the reference is there");
    assert_eq!(held, object);
}
