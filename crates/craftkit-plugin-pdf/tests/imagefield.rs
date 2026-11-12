//! The **image field** that is put on a page.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-imagefield-{name}-{}", std::process::id()));
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

/// **An image field is put on the page.** It is a widget, it is a push
/// button, and it has no border and no background.
#[test]
fn an_image_field_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("image");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.image.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 130.0, 230.0] }),
        )
        .expect("the image field is added");
    assert!(outcome.changed);
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    let entries = entries_of(&field);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("Annot"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Widget"));
    assert_eq!(entry(&entries, "FT").and_then(|value| value.as_str()), Some("Btn"));
    assert_eq!(entry(&entries, "Ff").and_then(|value| value.as_f64()), Some(65536.0));
    assert_eq!(text_of(entry(&entries, "T").expect("the name")), Some("Image1".to_string()));
    // The **mark**: no border and no background, and the field says
    // that it is **icon only**.
    let mark = dictionary_of(entry(&entries, "MK").expect("the mark"));
    assert!(entry(&mark, "BC").is_none(), "there is no border");
    assert!(entry(&mark, "BG").is_none(), "there is no background");
    assert_eq!(entry(&mark, "TP").and_then(|value| value.as_f64()), Some(1.0));
}

/// **The action** of the field is the tree's own script, and it asks
/// the reader for a picture.
#[test]
fn the_action_asks_for_a_picture() {
    let mut session = session();
    let directory = directory("action");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.image.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 130.0, 230.0] }),
        )
        .expect("the image field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_object(&session, object).expect("the field is there");
    let action = dictionary_of(entry(&entries_of(&field), "A").expect("the action"));
    assert_eq!(entry(&action, "S").and_then(|value| value.as_str()), Some("JavaScript"));
    assert_eq!(
        text_of(entry(&action, "JS").expect("the script")),
        Some("event.target.buttonImportIcon();".to_string())
    );
}

/// **The name is free,** and the tree's own base is `Image`.
#[test]
fn the_name_is_free() {
    let mut session = session();
    let directory = directory("names");
    let page = a_page(&mut session, &directory);
    let first = session
        .execute(
            "pdf.field.image.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 130.0, 230.0] }),
        )
        .expect("the image field is added");
    assert_eq!(first.data["name"].as_str(), Some("Image1"));
    let second = session
        .execute(
            "pdf.field.image.add",
            &json!({ "page": page, "object": 11, "rect": [100.0, 300.0, 130.0, 330.0] }),
        )
        .expect("the image field is added");
    assert_eq!(second.data["name"].as_str(), Some("Image2"));
}

/// **The drawing of an image field** is empty: the picture is the
/// field.
#[test]
fn the_drawing_is_empty() {
    let mut session = session();
    let directory = directory("drawing");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute(
            "pdf.field.image.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 130.0, 230.0],
                "appearance_to": to,
            }),
        )
        .expect("the image field is added");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(!bytes.contains("rg"), "there is no background: {bytes}");
    assert!(!bytes.contains("RG"), "there is no border: {bytes}");
}
