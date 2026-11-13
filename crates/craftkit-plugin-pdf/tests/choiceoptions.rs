//! The **export value** of an option: what the file holds, and the
//! **display text** the reader sees.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-choiceoptions-{name}-{}", std::process::id()))
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

/// The **text** a value holds.
fn text_of(value: &craftkit_sdk::serde_json::Value) -> Option<String> {
    match value {
        craftkit_sdk::serde_json::Value::String(text) => Some(text.clone()),
        craftkit_sdk::serde_json::Value::Object(map) => {
            map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string())
        }
        _ => None,
    }
}

/// **A drop-down** whose options carry an export value and a display
/// text: the node of it.
fn a_drop_down(
    session: &mut Session,
    directory: &str,
    options: &craftkit_sdk::serde_json::Value,
    value: Option<&str>,
) -> u64 {
    let page = a_page(session, directory);
    let mut params = json!({
        "page": page,
        "object": 10,
        "rect": [100.0, 700.0, 300.0, 720.0],
        "options": options,
        "appearance_to": format!("{directory}/drawing.txt"),
    });
    if let Some(value) = value {
        params["value"] = json!(value);
    }
    let outcome = session.execute("pdf.field.combo.add", &params).expect("the drop-down is added");
    outcome.data["id"].as_u64().expect("the drop-down is there")
}

/// **A list box** whose options carry an export value and a display
/// text: the node of it.
fn a_list_box(
    session: &mut Session,
    directory: &str,
    options: &craftkit_sdk::serde_json::Value,
    value: Option<&str>,
) -> u64 {
    let page = a_page(session, directory);
    let mut params = json!({
        "page": page,
        "object": 10,
        "rect": [100.0, 700.0, 300.0, 760.0],
        "options": options,
        "appearance_to": format!("{directory}/drawing.txt"),
    });
    if let Some(value) = value {
        params["value"] = json!(value);
    }
    let outcome = session.execute("pdf.field.list.add", &params).expect("the list box is added");
    outcome.data["id"].as_u64().expect("the list box is there")
}

/// **An option that carries an export value and a display text** is
/// held by the file as a **pair**, and the value the field holds is the
/// **export value**: the tree's own rule.
#[test]
fn an_option_carries_an_export_value_and_a_display_text() {
    let mut session = session();
    let directory = directory("pair");
    let options = json!([["fr", "France"], ["de", "Germany"]]);
    let drop_down = a_drop_down(&mut session, &directory, &options, Some("fr"));
    let node = session.document().node(drop_down).cloned().expect("the drop-down is there");
    let entries = entries_of(&node);
    let held = entry(&entries, "Opt").and_then(|value| value.as_array().cloned()).expect("the options");
    // The **first** option is a pair: the export value and the display
    // text.
    let first = held[0].as_array().expect("the option is a pair");
    assert_eq!(text_of(&first[0]), Some("fr".to_string()), "the export value is there");
    assert_eq!(text_of(&first[1]), Some("France".to_string()), "the display text is there");
    // And the field holds the **export value**.
    assert_eq!(entry(&entries, "V"), Some(&json!({ "string": "fr" })), "the value is the export value");
}

/// **A value the caller gives as the display text** is held by the
/// field as the **export value**: the tree's own rule.
#[test]
fn a_display_text_is_held_as_the_export_value() {
    let mut session = session();
    let directory = directory("display");
    let options = json!([["fr", "France"], ["de", "Germany"]]);
    let drop_down = a_drop_down(&mut session, &directory, &options, Some("France"));
    let node = session.document().node(drop_down).cloned().expect("the drop-down is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "V"), Some(&json!({ "string": "fr" })), "the value is the export value");
}

/// **An option that is a plain text** is held by the file as that text:
/// the tree's own rule.
#[test]
fn a_plain_option_is_held_as_a_text() {
    let mut session = session();
    let directory = directory("plain");
    let options = json!(["France", "Germany"]);
    let drop_down = a_drop_down(&mut session, &directory, &options, Some("France"));
    let node = session.document().node(drop_down).cloned().expect("the drop-down is there");
    let entries = entries_of(&node);
    let held = entry(&entries, "Opt").and_then(|value| value.as_array().cloned()).expect("the options");
    assert_eq!(text_of(&held[0]), Some("France".to_string()), "the option is a text");
    assert!(held[0].as_array().is_none(), "the option is not a pair");
}

/// **The drawing** of the drop-down shows the **display text** of the
/// option that is chosen, and not the export value: the tree's own
/// rule.
#[test]
fn the_drawing_shows_the_display_text() {
    let mut session = session();
    let directory = directory("drawn");
    let options = json!([["fr", "France"], ["de", "Germany"]]);
    let _ = a_drop_down(&mut session, &directory, &options, Some("fr"));
    let bytes = std::fs::read_to_string(format!("{directory}/drawing.txt")).expect("the drawing is there");
    assert!(bytes.contains("(France) Tj"), "the display text is drawn: {bytes}");
    assert!(!bytes.contains("(fr) Tj"), "the export value is not drawn: {bytes}");
}

/// **The list box** shows the display text of every option, and the
/// band of the one that is chosen is on its **export value**: the
/// tree's own rule.
#[test]
fn the_list_box_shows_the_display_texts() {
    let mut session = session();
    let directory = directory("list");
    let options = json!([["fr", "France"], ["de", "Germany"]]);
    let list = a_list_box(&mut session, &directory, &options, Some("de"));
    let node = session.document().node(list).cloned().expect("the list box is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "V"), Some(&json!({ "string": "de" })), "the value is the export value");
    let bytes = std::fs::read_to_string(format!("{directory}/drawing.txt")).expect("the drawing is there");
    assert!(bytes.contains("(France) Tj"), "the first display text is drawn: {bytes}");
    assert!(bytes.contains("(Germany) Tj"), "the second display text is drawn: {bytes}");
    // And the band of the tree's own colour is behind the chosen row.
    assert_eq!(bytes.matches("0.6 0.75 0.86 rg").count(), 1, "one row is chosen: {bytes}");
}

/// **A value that is not one of the options** is refused.
#[test]
fn a_value_that_is_not_an_option_is_refused() {
    let mut session = session();
    let directory = directory("refused");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 720.0],
                "options": [["fr", "France"]],
                "value": "Spain",
            }),
        )
        .expect_err("the drop-down is refused");
    assert!(error.to_string().contains("not one of the options"), "the error says why: {error}");
}

/// **An option that is not a text and not a pair of texts** is refused.
#[test]
fn an_option_that_is_nothing_is_refused() {
    let mut session = session();
    let directory = directory("nothing");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.combo.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 720.0],
                "options": [["fr", "France", "extra"]],
            }),
        )
        .expect_err("the drop-down is refused");
    assert!(error.to_string().contains("pair of two texts"), "the error says why: {error}");
}
