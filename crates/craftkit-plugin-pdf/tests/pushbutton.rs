//! The **push button** that is put on a page.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-pushbutton-{name}-{}", std::process::id()));
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

/// **A push button is put on the page.** It is a widget, it is a
/// button, and it carries a caption.
#[test]
fn a_push_button_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("button");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.button.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 200.0, 240.0],
                "caption": "Go",
            }),
        )
        .expect("the button is added");
    assert!(outcome.changed);
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    let entries = entries_of(&field);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("Annot"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Widget"));
    assert_eq!(entry(&entries, "FT").and_then(|value| value.as_str()), Some("Btn"));
    // The tree's own flag of a push button.
    assert_eq!(entry(&entries, "Ff").and_then(|value| value.as_f64()), Some(65536.0));
    assert_eq!(
        text_of(entry(&entries, "DA").expect("the default appearance")),
        Some("/Helv 0 Tf 0 g".to_string())
    );
    assert_eq!(text_of(entry(&entries, "T").expect("the name")), Some("Button1".to_string()));
    // The **caption** and the **background**.
    let mark = dictionary_of(entry(&entries, "MK").expect("the colours"));
    assert_eq!(text_of(entry(&mark, "CA").expect("the caption")), Some("Go".to_string()));
    assert_eq!(numbers_of(entry(&mark, "BG").expect("the background")), vec![0.86, 0.86, 0.86]);
}

/// **The name is free,** and the tree's own base is `Button`.
#[test]
fn the_name_is_free() {
    let mut session = session();
    let directory = directory("names");
    let page = a_page(&mut session, &directory);
    let first = session
        .execute(
            "pdf.field.button.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 200.0, 240.0], "caption": "Go" }),
        )
        .expect("the button is added");
    assert_eq!(first.data["name"].as_str(), Some("Button1"));
    let second = session
        .execute(
            "pdf.field.button.add",
            &json!({ "page": page, "object": 11, "rect": [100.0, 300.0, 200.0, 340.0], "caption": "Go" }),
        )
        .expect("the button is added");
    assert_eq!(second.data["name"].as_str(), Some("Button2"));
}

/// **The caption is centred** in the button, and its size is the
/// tree's own.
#[test]
fn the_caption_is_centred() {
    let mut session = session();
    let directory = directory("caption");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute(
            "pdf.field.button.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 200.0, 240.0],
                "caption": "Go",
                "appearance_to": to,
            }),
        )
        .expect("the button is added");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("(Go) Tj"), "the caption is drawn: {bytes}");
    // The size is six tenths of the height of the button, and never
    // more than fourteen points: the height is forty, so the size is
    // fourteen.
    assert!(bytes.contains("/Helv 14 Tf"), "the size is the tree's: {bytes}");
    // The width of the caption is the tree's own — the "G" is six
    // hundred and eighty units of a thousand of an em and the "o" is
    // five hundred and fifty-six — so the caption starts at
    // (100 - 17.304) / 2.
    assert!(bytes.contains("41.348"), "the caption is centred: {bytes}");
    assert!(bytes.contains("15.1"), "the caption is centred: {bytes}");
}

/// **The form holds the field** and the page holds it too.
#[test]
fn the_form_and_the_page_hold_the_field() {
    let mut session = session();
    let directory = directory("holds");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.button.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 200.0, 240.0], "caption": "Go" }),
        )
        .expect("the button is added");
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

/// **A name that is taken is refused,** and so is a name with a period
/// in it.
#[test]
fn a_bad_name_is_refused() {
    let mut session = session();
    let directory = directory("badname");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.button.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 200.0, 240.0],
                "caption": "Go",
                "name": "a.b",
            }),
        )
        .expect_err("the name is refused");
    assert!(error.to_string().contains("period"), "the error says why: {error}");
    session
        .execute(
            "pdf.field.button.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 200.0, 240.0],
                "caption": "Go",
                "name": "Go",
            }),
        )
        .expect("the button is added");
    let error = session
        .execute(
            "pdf.field.button.add",
            &json!({
                "page": page,
                "object": 11,
                "rect": [100.0, 300.0, 200.0, 340.0],
                "caption": "Go",
                "name": "Go",
            }),
        )
        .expect_err("the name is refused");
    assert!(error.to_string().contains("already exists"), "the error says why: {error}");
}
