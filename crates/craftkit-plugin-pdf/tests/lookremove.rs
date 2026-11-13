//! The border and the background of a field may be **taken away**.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-lookremove-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a **text field** on a page, with a border and a
/// background.
fn a_file() -> Vec<u8> {
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
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (the name) /Rect [10 700 200 720] \
             /DA (/Helv 12 Tf 0 g) /V (Ada) /P 3 0 R \
             /MK << /BC [1 0 0] /BG [0.9 0.9 0.9] >> /BS << /W 1 /S /S >> >>"
                .to_string(),
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

/// Load the file and give the **widget** it holds, and its directory.
fn load(session: &mut Session, name: &str) -> (u64, String) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let widget = session
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
        .expect("the widget is there");
    (widget, directory)
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

/// The **mark** of the widget: the pairs of it.
fn the_mark(session: &Session, widget: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(widget).expect("the widget is there");
    let entries = entries_of(node);
    let mark = entry(&entries, "MK").expect("the mark is there");
    mark.get("entries")
        .and_then(|entries| entries.as_array())
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

/// **A border of "none"** is taken away: the tree's own rule.
#[test]
fn a_border_of_none_is_taken_away() {
    let mut session = session();
    let (widget, directory) = load(&mut session, "border");
    let outcome = session
        .execute("pdf.field.look.set", &json!({ "field": widget, "border": "none" }))
        .expect("the look is set");
    assert_eq!(outcome.data["border"], json!("none"), "the answer says none");
    // The **mark** has no border colour any more.
    let mark = the_mark(&session, widget);
    assert!(!mark.iter().any(|(key, _)| key == "BC"), "the border colour is gone: {mark:?}");
    // And the **background** is still there.
    assert!(mark.iter().any(|(key, _)| key == "BG"), "the background is still there: {mark:?}");
    // And the drawing has no border.
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }))
        .expect("the appearance is written");
    let drawing = std::fs::read_to_string(format!("{directory}/drawing.txt")).expect("the drawing is there");
    assert!(!drawing.contains(" re S\n"), "there is no border: {drawing}");
    assert!(drawing.contains("0.9 0.9 0.9 rg\n"), "but the background is filled: {drawing}");
}

/// **A background of "none"** is taken away: the tree's own rule.
#[test]
fn a_background_of_none_is_taken_away() {
    let mut session = session();
    let (widget, directory) = load(&mut session, "fill");
    session
        .execute("pdf.field.look.set", &json!({ "field": widget, "fill": "none" }))
        .expect("the look is set");
    let mark = the_mark(&session, widget);
    assert!(!mark.iter().any(|(key, _)| key == "BG"), "the background colour is gone: {mark:?}");
    assert!(mark.iter().any(|(key, _)| key == "BC"), "the border is still there: {mark:?}");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }))
        .expect("the appearance is written");
    let drawing = std::fs::read_to_string(format!("{directory}/drawing.txt")).expect("the drawing is there");
    assert!(!drawing.contains(" re f\n"), "there is no background: {drawing}");
    assert!(drawing.contains("1 0 0 RG\n"), "but the border is stroked: {drawing}");
}

/// **Both of them** may be taken away at once.
#[test]
fn both_of_them_may_be_taken_away() {
    let mut session = session();
    let (widget, _) = load(&mut session, "both");
    session
        .execute("pdf.field.look.set", &json!({ "field": widget, "border": "none", "fill": "none" }))
        .expect("the look is set");
    let mark = the_mark(&session, widget);
    assert!(mark.is_empty(), "the mark is empty: {mark:?}");
}

/// **A look that says nothing** leaves the colours as they were: the
/// tree's own rule.
#[test]
fn a_look_that_says_nothing_leaves_the_colours() {
    let mut session = session();
    let (widget, _) = load(&mut session, "quiet");
    session
        .execute("pdf.field.look.set", &json!({ "field": widget, "width": 2.0 }))
        .expect("the look is set");
    let mark = the_mark(&session, widget);
    assert!(mark.iter().any(|(key, _)| key == "BC"), "the border is still there: {mark:?}");
    assert!(mark.iter().any(|(key, _)| key == "BG"), "and the background: {mark:?}");
}

/// **A colour that is not three numbers and not "none"** is refused.
#[test]
fn a_colour_that_is_nothing_is_refused() {
    let mut session = session();
    let (widget, _) = load(&mut session, "bad");
    let error = session
        .execute("pdf.field.look.set", &json!({ "field": widget, "border": [1.0, 0.0] }))
        .expect_err("the look is refused");
    assert!(error.to_string().contains("three numbers"), "the error says why: {error}");
}
