//! The **index** of the option that is chosen: the tree's own `/I`.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-choiceindex-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a **choice** on a page, whose options carry an export
/// value and a display text.
fn a_file(flags: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (
            1,
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DR << /Font << /Helv 6 0 R >> >> >> >>"
                .to_string(),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
             /Resources << /Font << /Helv 6 0 R >> >> /Annots [4 0 R] >>"
                .to_string(),
        ),
        (
            4,
            format!(
                "<< /Type /Annot /Subtype /Widget /FT /Ch /T (choice) /Rect [10 700 200 720] \
             /DA (/Helv 12 Tf 0 g) /V (fr) /Opt [[(fr) (France)] [(de) (Germany)]] {flags} /P 3 0 R >>"
            ),
        ),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
        (6, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 7\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **widget** it holds.
fn load(session: &mut Session, name: &str, flags: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(flags)).expect("the file is written");
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
                        pair.first().and_then(|key| key.as_str()) == Some("Subtype")
                            && pair.get(1).and_then(|value| value.as_str()) == Some("Widget")
                    })
                })
                .unwrap_or(false)
        })
        .map(|node| node.id)
        .expect("the widget is there")
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

/// **The chosen option has its index:** the tree's own `/I`.
#[test]
fn the_chosen_option_has_its_index() {
    let mut session = session();
    let widget = load(&mut session, "index", "");
    let outcome = session
        .execute("pdf.field.set", &json!({ "field": widget, "value": "de" }))
        .expect("the field is filled");
    assert_eq!(outcome.data["index"], json!([1]), "the index is the tree's");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "V"), Some(&json!({ "string": "de" })), "the value is there");
    assert_eq!(entry(&entries, "I"), Some(&json!([1])), "the index is there");
}

/// **A display text** may be given, and the field holds the **export
/// value** with the index of that option: the tree's own rule.
#[test]
fn a_display_text_is_held_as_the_export_value() {
    let mut session = session();
    let widget = load(&mut session, "display", "");
    let outcome = session
        .execute("pdf.field.set", &json!({ "field": widget, "value": "Germany" }))
        .expect("the field is filled");
    assert_eq!(outcome.data["value"], json!("de"), "the value is the export value");
    assert_eq!(outcome.data["index"], json!([1]), "and the index is of that option");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "V"), Some(&json!({ "string": "de" })), "the value is there");
}

/// **An empty value** takes the value **and the index** away: the
/// tree's own rule.
#[test]
fn an_empty_value_takes_the_index_away() {
    let mut session = session();
    let widget = load(&mut session, "empty", "");
    session.execute("pdf.field.set", &json!({ "field": widget, "value": "" })).expect("the field is emptied");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "V"), None, "the value is gone");
    assert_eq!(entry(&entries, "I"), None, "and the index is gone");
}

/// **A value that is not an option** is refused: the tree's own rule.
#[test]
fn a_value_that_is_not_an_option_is_refused() {
    let mut session = session();
    let widget = load(&mut session, "refused", "");
    let error = session
        .execute("pdf.field.set", &json!({ "field": widget, "value": "Spain" }))
        .expect_err("the field is refused");
    assert!(error.to_string().contains("not one of the options"), "the error says why: {error}");
}

/// **A free text** in a drop-down that may be **edited** is allowed,
/// and it has no index: the tree's own rule.
#[test]
fn a_free_text_in_an_editable_drop_down_is_allowed() {
    let mut session = session();
    let widget = load(&mut session, "editable", "/Ff 262144");
    let outcome = session
        .execute("pdf.field.set", &json!({ "field": widget, "value": "Spain" }))
        .expect("the field is filled");
    assert_eq!(outcome.data["value"], json!("Spain"), "the value is what was given");
    assert_eq!(outcome.data["index"], json!([]), "and there is no index");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "I"), None, "there is no index");
}
