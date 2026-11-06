//! The **optional content** of a file.

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
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-optional-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with **layers** in it: one that is shown, one that is not,
/// and one that says how it is shown.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /OCProperties 4 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_string()),
        (4, "<< /OCGs [5 0 R 6 0 R 7 0 R] /D << /BaseState /ON /ON [5 0 R] /OFF [6 0 R] >> >>".to_string()),
        (5, "<< /Type /OCG /Name (the drawing) /Intent /View >>".to_string()),
        (6, "<< /Type /OCG /Name (the notes) /Intent /Design >>".to_string()),
        (7, "<< /Type /OCG /Name (the guide) /Usage << /View << /ViewState /OFF >> >> >>".to_string()),
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

/// A file with **no** optional content.
fn a_plain_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 4\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str, file: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **layers** of the file, as the command reads them.
fn layers_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.layers.read", &json!({})).expect("the layers are read");
    outcome.data.clone()
}

/// The layers of a file are read, and each one says whether it is
/// shown.
#[test]
fn the_layers_of_a_file_are_read() {
    let mut session = session();
    load(&mut session, "layers", a_file());
    let layers = layers_of(&mut session);
    assert_eq!(layers["count"], json!(3), "the file has three layers");
    let list = layers["layers"].as_array().expect("the layers are a list");
    assert_eq!(list[0]["name"], json!("the drawing"), "the first layer is named");
    assert_eq!(list[0]["intent"], json!("View"), "and it says what it is for");
    assert_eq!(list[0]["visible"], json!(true), "and it is shown");
    assert_eq!(list[1]["name"], json!("the notes"), "the second layer is named");
    assert_eq!(list[1]["intent"], json!("Design"), "and it says what it is for");
    assert_eq!(list[1]["visible"], json!(false), "and it is not shown");
    assert_eq!(list[2]["name"], json!("the guide"), "the third layer is named");
    assert_eq!(list[2]["visible"], json!(false), "and it says how it is shown");
}

/// A file with no optional content has no layers.
#[test]
fn a_file_with_no_optional_content_has_no_layers() {
    let mut session = session();
    load(&mut session, "plain", a_plain_file());
    let layers = layers_of(&mut session);
    assert_eq!(layers["count"], json!(0), "the file has no layers");
    assert_eq!(layers["layers"].as_array().map(Vec::len), Some(0), "and there are none");
}

/// A layer may be asked for **by name**, and one that is not there is
/// refused.
#[test]
fn a_layer_may_be_asked_for_by_name() {
    let mut session = session();
    load(&mut session, "named", a_file());
    let outcome =
        session.execute("pdf.layers.read", &json!({ "name": "the notes" })).expect("the layer is read");
    assert_eq!(outcome.data["count"], json!(1), "the layer is read by its name");
    assert_eq!(outcome.data["layers"][0]["visible"], json!(false), "and it says what it is");
    let missing = session.execute("pdf.layers.read", &json!({ "name": "the other" }));
    assert!(missing.is_err(), "a layer that is not there is refused");
    let message = missing.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("the other"), "and it says which one: {message}");
}
