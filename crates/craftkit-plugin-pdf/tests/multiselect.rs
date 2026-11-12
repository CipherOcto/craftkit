//! A list that may hold **more than one** option: the tree's own
//! array of values and array of indices.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-multiselect-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a **choice** on a page, whose flags may say that it
/// may hold more than one option.
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
             /DA (/Helv 12 Tf 0 g) /Opt [[(fr) (France)] [(de) (Germany)] [(es) (Spain)]] {flags} /P 3 0 R >>"
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

/// **A list that may hold more than one** holds an **array** of the
/// values and an **array** of the indices: the tree's own rule.
#[test]
fn a_list_that_may_hold_more_holds_an_array() {
    let mut session = session();
    let widget = load(&mut session, "multi", "/Ff 2097152");
    let outcome = session
        .execute("pdf.field.set", &json!({ "field": widget, "values": ["fr", "es"] }))
        .expect("the field is filled");
    assert_eq!(outcome.data["values"], json!(["fr", "es"]), "the values are the tree's");
    assert_eq!(outcome.data["index"], json!([0, 2]), "and the indices are the tree's");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(
        entry(&entries, "V"),
        Some(&json!([{ "string": "fr" }, { "string": "es" }])),
        "the field holds an array"
    );
    assert_eq!(entry(&entries, "I"), Some(&json!([0, 2])), "and the indices are an array");
}

/// **The display texts** may be given, and the field holds the export
/// values: the tree's own rule.
#[test]
fn the_display_texts_may_be_given() {
    let mut session = session();
    let widget = load(&mut session, "display", "/Ff 2097152");
    session
        .execute("pdf.field.set", &json!({ "field": widget, "values": ["Germany", "Spain"] }))
        .expect("the field is filled");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(
        entry(&entries, "V"),
        Some(&json!([{ "string": "de" }, { "string": "es" }])),
        "the field holds the export values"
    );
    assert_eq!(entry(&entries, "I"), Some(&json!([1, 2])), "and the indices are of those options");
}

/// **A field that takes a single value** refuses to take more than
/// one: the tree's own rule.
#[test]
fn a_field_that_takes_one_value_refuses_two() {
    let mut session = session();
    let widget = load(&mut session, "single", "");
    let error = session
        .execute("pdf.field.set", &json!({ "field": widget, "values": ["fr", "es"] }))
        .expect_err("the field is refused");
    assert!(error.to_string().contains("takes a single value"), "the error says why: {error}");
}

/// **One value** is still one value: the field holds a text and not an
/// array, and the index is an array of one: the tree's own rule.
#[test]
fn one_value_is_still_one_value() {
    let mut session = session();
    let widget = load(&mut session, "one", "/Ff 2097152");
    session
        .execute("pdf.field.set", &json!({ "field": widget, "values": ["fr"] }))
        .expect("the field is filled");
    let node = session.document().node(widget).cloned().expect("the widget is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "V"), Some(&json!({ "string": "fr" })), "the field holds a text");
    assert_eq!(entry(&entries, "I"), Some(&json!([0])), "and the index is an array of one");
}
