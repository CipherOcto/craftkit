//! The **properties** of a field.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-properties-{name}-{}", std::process::id()));
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

/// **The name of a field** is set, and a name that is empty or holds a
/// period is refused — and so is a name another field has.
#[test]
fn the_name_is_set() {
    let mut session = session();
    let directory = directory("name");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "name": "TheName" }))
        .expect("the name is set");
    let node = the_object(&session, object).expect("the field is there");
    assert_eq!(text_of(entry(&entries_of(&node), "T").expect("the name")), Some("TheName".to_string()));
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": field, "name": "a.b" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("period"), "the error says why: {error}");
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": field, "name": "  " }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("empty"), "the error says why: {error}");
}

/// **The tooltip** is set, and an empty one takes it away.
#[test]
fn the_tooltip_is_set() {
    let mut session = session();
    let directory = directory("tooltip");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "tooltip": "the tip" }))
        .expect("the tooltip is set");
    let node = the_object(&session, object).expect("the field is there");
    assert_eq!(text_of(entry(&entries_of(&node), "TU").expect("the tooltip")), Some("the tip".to_string()));
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "tooltip": "" }))
        .expect("the tooltip is taken away");
    let node = the_object(&session, object).expect("the field is there");
    assert!(entry(&entries_of(&node), "TU").is_none(), "the tooltip is gone");
}

/// **The most characters** a field holds is set, and zero takes it
/// away.
#[test]
fn the_maximum_length_is_set() {
    let mut session = session();
    let directory = directory("maxlen");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "max_len": 20 }))
        .expect("the limit is set");
    let node = the_object(&session, object).expect("the field is there");
    assert_eq!(entry(&entries_of(&node), "MaxLen").and_then(|value| value.as_f64()), Some(20.0));
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "max_len": 0 }))
        .expect("the limit is taken away");
    let node = the_object(&session, object).expect("the field is there");
    assert!(entry(&entries_of(&node), "MaxLen").is_none(), "the limit is gone");
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": field, "max_len": -3 }))
        .expect_err("the limit is refused");
    assert!(error.to_string().contains("whole number"), "the error says why: {error}");
}

/// **The options of a choice** are set, and a choice may not be left
/// with no option at all.
#[test]
fn the_options_are_set() {
    let mut session = session();
    let directory = directory("options");
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
    let field = the_object(&session, object).expect("the field is there").id;
    session
        .execute(
            "pdf.field.properties.set",
            &json!({ "field": field, "options": ["the second", "the third"] }),
        )
        .expect("the options are set");
    let node = the_object(&session, object).expect("the field is there");
    let options = entry(&entries_of(&node), "Opt")
        .and_then(|options| options.as_array().cloned())
        .expect("the options");
    let texts: Vec<String> = options.iter().filter_map(text_of).collect();
    assert_eq!(texts, vec!["the second".to_string(), "the third".to_string()]);
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": field, "options": [] }))
        .expect_err("the options are refused");
    assert!(error.to_string().contains("at least one option"), "the error says why: {error}");
}

/// **The alignment** of the text of a field is set, and it is 0, 1 or
/// 2.
#[test]
fn the_alignment_is_set() {
    let mut session = session();
    let directory = directory("quadding");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "quadding": 1 }))
        .expect("the alignment is set");
    let node = the_object(&session, object).expect("the field is there");
    assert_eq!(entry(&entries_of(&node), "Q").and_then(|value| value.as_f64()), Some(1.0));
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": field, "quadding": 3 }))
        .expect_err("the alignment is refused");
    assert!(error.to_string().contains("alignment"), "the error says why: {error}");
}

/// **The value the reset of the form restores** is set, and an empty
/// one takes it away.
#[test]
fn the_default_value_is_set() {
    let mut session = session();
    let directory = directory("default");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "default_value": "the words" }))
        .expect("the default is set");
    let node = the_object(&session, object).expect("the field is there");
    assert_eq!(text_of(entry(&entries_of(&node), "DV").expect("the default")), Some("the words".to_string()));
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "default_value": "" }))
        .expect("the default is taken away");
    let node = the_object(&session, object).expect("the field is there");
    assert!(entry(&entries_of(&node), "DV").is_none(), "the default is gone");
}

/// **Only a text field** holds more than one line of text.
#[test]
fn only_a_text_field_is_multiline() {
    let mut session = session();
    let directory = directory("multiline");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "multiline": true }))
        .expect("the field is multiline");
    let node = the_object(&session, object).expect("the field is there");
    let flags = entry(&entries_of(&node), "Ff").and_then(|value| value.as_f64()).unwrap_or(0.0);
    assert_eq!(flags, 4096.0);
    // A **button** is not a text field.
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.checkbox.add",
            &json!({ "page": page, "object": 20, "rect": [100.0, 300.0, 120.0, 320.0] }),
        )
        .expect("the check box is added");
    let button = the_object(&session, outcome.data["object"].as_u64().expect("the object is there"))
        .expect("the check box is there")
        .id;
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": button, "multiline": true }))
        .expect_err("the flag is refused");
    assert!(error.to_string().contains("text field"), "the error says why: {error}");
}

/// **A locked field** takes only unlocking.
#[test]
fn a_locked_field_takes_only_unlocking() {
    let mut session = session();
    let directory = directory("locked");
    let (field, object) = a_text_field(&mut session, &directory);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "locked": true }))
        .expect("the field is locked");
    // The **lock** is on the widget, and the widget of a text field is
    // the field itself.
    let node = the_object(&session, object).expect("the field is there");
    let flags = entry(&entries_of(&node), "F").and_then(|value| value.as_f64()).unwrap_or(0.0);
    assert_eq!(flags, 132.0);
    // Every other change is refused.
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": field, "tooltip": "the tip" }))
        .expect_err("the change is refused");
    assert!(error.to_string().contains("locked"), "the error says why: {error}");
    // And the unlocking is not.
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "locked": false }))
        .expect("the field is unlocked");
    let node = the_object(&session, object).expect("the field is there");
    let flags = entry(&entries_of(&node), "F").and_then(|value| value.as_f64()).unwrap_or(0.0);
    assert_eq!(flags, 4.0);
    session
        .execute("pdf.field.properties.set", &json!({ "field": field, "tooltip": "the tip" }))
        .expect("the tooltip is set");
}

/// **A node that is not a field** is refused.
#[test]
fn a_node_that_is_not_a_field_is_refused() {
    let mut session = session();
    let directory = directory("notafield");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute("pdf.field.properties.set", &json!({ "field": page, "tooltip": "the tip" }))
        .expect_err("the page is refused");
    assert!(error.to_string().contains("not a field"), "the error says why: {error}");
}
