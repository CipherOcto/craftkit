//! The **written appearance** of a field.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// A directory of this test's own, so that the files do not mix.
///
/// Every call makes a **directory of its own**: the tests of one file
/// run beside each other, and a directory that two of them shared
/// would be taken away by the second one under the first.
fn directory(name: &str) -> String {
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let number = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-written-{name}-{}-{number}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a **text field** on a page: it has a value, a default
/// appearance and a font in the page's resources.
fn a_file() -> Vec<u8> {
    a_file_with(true)
}

/// A file whose text field holds **no value**.
fn a_file_without_value() -> Vec<u8> {
    a_file_with(false)
}

/// The file, with or without a value in the field.
fn a_file_with(value: bool) -> Vec<u8> {
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
            if value {
                "<< /Type /Annot /Subtype /Widget /FT /Tx /T (the name) /Rect [10 700 200 720] \
             /DA (/Helv 12 Tf 0 g) /V (Ada) /P 3 0 R >>"
                    .to_string()
            } else {
                "<< /Type /Annot /Subtype /Widget /FT /Tx /T (the name) /Rect [10 700 200 720] \
             /DA (/Helv 12 Tf 0 g) /P 3 0 R >>"
                    .to_string()
            },
        ),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
        (6, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
        (7, "<< /Type /Bogus >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 8\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document, and give the **widget** it holds.
fn load(session: &mut Session, name: &str) -> u64 {
    load_with(session, name, a_file())
}

/// Load a file into the document, and give the **widget** it holds.
fn load_with(session: &mut Session, name: &str, file: Vec<u8>) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    the_widget(session)
}

/// The **widget** of the document: the annotation that is a widget.
fn the_widget(session: &Session) -> u64 {
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

/// The **entry** of a node's dictionary, by its key.
fn entry_of(node: &craftkit_sdk::Node, key: &str) -> Option<Json> {
    node.data["entries"]
        .as_array()?
        .iter()
        .filter_map(|entry| entry.as_array())
        .find(|pair| pair.first().and_then(|name| name.as_str()) == Some(key))
        .and_then(|pair| pair.get(1).cloned())
}

/// The appearance of a field is written: the widget names it, and the
/// drawing holds the value.
#[test]
fn the_appearance_of_a_field_is_written() {
    let mut session = session();
    let widget = load(&mut session, "written");
    let directory = directory("written");
    let to = format!("{directory}/appearance.txt");
    let outcome = session
        .execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }))
        .expect("the appearance is written");
    assert!(outcome.data["bytes"].as_u64().unwrap_or_default() > 0, "there are bytes");
    assert_eq!(outcome.data["font"], json!("Helv"), "the font is the one the field says");
    assert_eq!(outcome.data["size"], json!(12.0), "and the size is the one it says");
    // The **drawing**: the value is in it.
    let drawing = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(drawing.contains("(Ada) Tj"), "the value is drawn: {drawing}");
    assert!(drawing.contains("/Helv 12 Tf"), "with the font and the size: {drawing}");
    assert!(drawing.contains("190 20 re W n"), "in the box of the widget: {drawing}");
    // The **widget** names the appearance, under `N`.
    let node = session.document().node(widget).expect("the widget is there");
    let appearances = entry_of(node, "AP").expect("the widget has an appearance");
    let normal = appearances["entries"]
        .as_array()
        .expect("the appearance is a dictionary")
        .iter()
        .filter_map(|entry| entry.as_array())
        .find(|pair| pair.first().and_then(|key| key.as_str()) == Some("N"))
        .and_then(|pair| pair.get(1).cloned())
        .expect("the normal appearance is named");
    let number = normal["ref"][0].as_f64().expect("the appearance is a reference");
    // The **appearance** is a form XObject with the box of the widget.
    let appearance = session
        .document()
        .walk()
        .find(|node| node.data["object"].as_f64() == Some(number))
        .expect("the appearance is there");
    let entries = appearance.data["entries"].as_array().expect("it has a dictionary");
    let find = |key: &str| {
        entries
            .iter()
            .filter_map(|entry| entry.as_array())
            .find(|pair| pair.first().and_then(|name| name.as_str()) == Some(key))
            .and_then(|pair| pair.get(1).cloned())
    };
    assert_eq!(find("Subtype"), Some(json!("Form")), "the appearance is a form");
    assert_eq!(find("Type"), Some(json!("XObject")), "and it is an XObject");
    assert_eq!(find("BBox"), Some(json!([0.0, 0.0, 190.0, 20.0])), "and it is drawn in the box");
}

/// The appearance that is written is **read back**: the tree's own
/// reading of it says what it holds.
#[test]
fn the_appearance_that_is_written_is_read_back() {
    let mut session = session();
    let widget = load(&mut session, "readback");
    let directory = directory("readback");
    let to = format!("{directory}/appearance.txt");
    session
        .execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }))
        .expect("the appearance is written");
    let outcome =
        session.execute("pdf.field.appearance", &json!({ "field": widget })).expect("the appearance is read");
    assert_eq!(outcome.data["has_appearance"], json!(true), "the field has an appearance");
    assert_eq!(outcome.data["box"], json!([0.0, 0.0, 190.0, 20.0]), "and it is drawn in the box");
    let operations = outcome.data["operations"].as_array().expect("the drawing is a list");
    assert!(!operations.is_empty(), "and the drawing holds its operators");
}

/// A widget that holds **no value** is refused, and so is a widget
/// that is not a widget.
#[test]
fn a_widget_without_a_value_is_refused() {
    let mut session = session();
    let widget = load_with(&mut session, "novalue", a_file_without_value());
    let directory = directory("novalue");
    let to = format!("{directory}/appearance.txt");
    let outcome = session.execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }));
    assert!(outcome.is_err(), "a widget with no value is refused");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("no value"), "and it says why: {message}");
    // A node that is not a widget is refused too.
    let other = session
        .document()
        .walk()
        .find(|node| node.id != widget && node.kind == "pdf.object")
        .map(|node| node.id)
        .expect("there is another object");
    let outcome = session.execute("pdf.field.appearance.write", &json!({ "widget": other, "to": to }));
    assert!(outcome.is_err(), "a node that is not a widget is refused");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("not a widget"), "and it says why: {message}");
}

/// The file that is written holds the appearance: the widget names it
/// and the drawing is in the file.
#[test]
fn the_file_that_is_written_holds_the_appearance() {
    let mut session = session();
    let widget = load(&mut session, "writtenfile");
    let directory = directory("writtenfile");
    let to = format!("{directory}/appearance.txt");
    session
        .execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }))
        .expect("the appearance is written");
    let path = format!("{directory}/out.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let file = String::from_utf8_lossy(&std::fs::read(&path).expect("the file is there")).to_string();
    assert!(file.contains("/Subtype /Form"), "the appearance is in the file: {file}");
    assert!(file.contains("(Ada) Tj"), "and the drawing is in it: {file}");
    assert!(file.contains("/AP << /N"), "and the widget names it: {file}");
}
