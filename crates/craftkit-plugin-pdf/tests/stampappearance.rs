//! The **appearance** of a stamp: the tag, the frame and the label.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-stampappearance-{name}-{}", std::process::id()))
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

/// A **stamp** on a page: the node of it.
fn a_stamp(session: &mut Session, directory: &str, kind: &str) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.stamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 730.0],
                "kind": kind,
            }),
        )
        .expect("the stamp is added");
    outcome.data["id"].as_u64().expect("the stamp is there")
}

/// **A stamp is drawn:** a rounded frame with the label of the kind in
/// it.
#[test]
fn a_stamp_is_drawn() {
    let mut session = session();
    let directory = directory("stamp");
    let stamp = a_stamp(&mut session, &directory, "Approved");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect("the stamp is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tree's own** green, and the light tint of it.
    assert!(bytes.contains("0.13 0.55 0.13 RG"), "the stamp has its colour: {bytes}");
    // The frame is a rounded one, and it is painted and stroked.
    assert!(bytes.contains(" c "), "the frame is rounded: {bytes}");
    assert!(bytes.contains(" c h B\n"), "the frame is painted and stroked: {bytes}");
    // The **label** of the kind is in the tree's Helvetica bold.
    assert!(bytes.contains("/HelvB "), "the label is in the bold: {bytes}");
    assert!(bytes.contains("(APPROVED) Tj"), "the label is the tree's: {bytes}");
}

/// **A stamp that asks for a signature** is a tag pointing left, and
/// its text is white.
#[test]
fn a_stamp_that_asks_for_a_signature_is_a_tag() {
    let mut session = session();
    let directory = directory("signhere");
    let stamp = a_stamp(&mut session, &directory, "PCSignHere");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect("the stamp is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tag** points left, and it is filled.
    assert!(bytes.contains(" h B\n"), "the tag is filled: {bytes}");
    // The text of it is white.
    assert!(bytes.contains("BT\n1 1 1 rg\n"), "the text is white: {bytes}");
    assert!(bytes.contains("(SIGN HERE) Tj"), "the label is the tree's: {bytes}");
}

/// **The fonts** of the stamp are in the resources: the tree's
/// Helvetica and its bold.
#[test]
fn the_fonts_are_in_the_resources() {
    let mut session = session();
    let directory = directory("fonts");
    let stamp = a_stamp(&mut session, &directory, "Approved");
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect("the stamp is drawn");
    let appearance = outcome.data["appearance"].as_u64().expect("the drawing is there");
    let node = session.document().node(appearance).cloned().expect("the drawing is there");
    let entries = entries_of(&node);
    let resources = entry(&entries, "Resources").expect("the resources are there");
    let fonts = dictionary_of(resources);
    let font = fonts.iter().find(|(key, _)| key == "Font").map(|(_, value)| value).expect("a font");
    let names = dictionary_of(font);
    let bold = names.iter().find(|(key, _)| key == "HelvB").map(|(_, value)| value).expect("the bold");
    assert!(
        dictionary_of(bold)
            .iter()
            .any(|(key, value)| key == "BaseFont" && value.as_str() == Some("Helvetica-Bold")),
        "the bold is the tree's"
    );
}

/// **A stamp with a name that is not one of the tree's** is refused.
#[test]
fn a_stamp_with_an_unknown_name_is_refused() {
    let mut session = session();
    let directory = directory("unknown");
    let stamp = a_stamp(&mut session, &directory, "Approved");
    session
        .execute("pdf.entry.set", &json!({ "object": stamp, "key": "Name", "value": "Brilliant" }))
        .expect("the name is changed");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}

/// **A stamp without its rectangle** is refused.
#[test]
fn a_stamp_without_its_rectangle_is_refused() {
    let mut session = session();
    let directory = directory("norect");
    let stamp = a_stamp(&mut session, &directory, "Approved");
    session
        .execute("pdf.entry.remove", &json!({ "object": stamp, "key": "Rect" }))
        .expect("the rectangle is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
