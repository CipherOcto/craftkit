//! The **validation** of a field.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-validation-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose fields say what they accept.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>"),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (ranged) /Rect [0 0 10 10] /P 3 0 R /AA << /V 8 0 R >> >>",
        ),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (plain) /Rect [0 0 10 10] /P 3 0 R >>"),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (written) /Rect [0 0 10 10] /P 3 0 R /AA << /V << /S /JavaScript /JS (AFRange_Validate(true, 1, true, 100)) >> >> >>",
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (other) /Rect [0 0 10 10] /P 3 0 R /AA << /V 9 0 R >> >>",
        ),
        (8, "<< /S /JavaScript /JS (AFRange_Validate(true, 0, true, 10)) >>"),
        (9, "<< /S /JavaScript /JS (if (AFNumber_Format(2,0,0,\"\",\"\",true) > 0) {} ) >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 10\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 10 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/form.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **node** of a field, by the name it is called.
fn field_of(session: &Session, name: &str) -> u64 {
    session
        .document()
        .walk()
        .find(|node| {
            node.kind == "pdf.object"
                && crate_entries(&node.data)
                    .iter()
                    .any(|(key, value)| key == "T" && text_of(value) == Some(name.to_string()))
        })
        .map(|node| node.id)
        .expect("the field is there")
}

/// The **text** a value holds: the file's own spelling of a text.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **entries** of a node's data, as the crate reads them.
fn crate_entries(data: &Json) -> Vec<(String, Json)> {
    data.get("entries")
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

/// The **validation** of a field, as the command reads it.
fn validation_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.validate", &json!({ "field": field })).expect("the validation is read");
    outcome.data.clone()
}

/// A field says the **range** of values it accepts.
#[test]
fn a_field_says_the_range_it_accepts() {
    let mut session = session();
    load(&mut session, "ranged");
    let field = field_of(&session, "ranged");
    let validation = validation_of(&mut session, field);
    assert_eq!(validation["kind"], json!("range"), "the field accepts a range");
    assert_eq!(validation["greater"], json!(true), "there is a smallest value");
    assert_eq!(validation["min"], json!(0.0), "and it is zero");
    assert_eq!(validation["less"], json!(true), "there is a largest value");
    assert_eq!(validation["max"], json!(10.0), "and it is ten");
}

/// A field that says nothing about what it accepts validates nothing.
#[test]
fn a_field_that_says_nothing_validates_nothing() {
    let mut session = session();
    load(&mut session, "plain");
    let field = field_of(&session, "plain");
    let validation = validation_of(&mut session, field);
    assert_eq!(validation["kind"], json!("nothing"), "there is no validation");
    assert_eq!(validation["min"], Json::Null, "and no smallest value");
}

/// The action may be written where it is used, and not named.
#[test]
fn the_action_may_be_written_in_place() {
    let mut session = session();
    load(&mut session, "written");
    let field = field_of(&session, "written");
    let validation = validation_of(&mut session, field);
    assert_eq!(validation["kind"], json!("range"), "the field accepts a range");
    assert_eq!(validation["min"], json!(1.0), "the smallest is one");
    assert_eq!(validation["max"], json!(100.0), "the largest is a hundred");
}

/// A script this tree does not understand is said so, and is not
/// pretended to be run.
#[test]
fn a_script_that_is_not_understood_is_reported() {
    let mut session = session();
    load(&mut session, "other");
    let field = field_of(&session, "other");
    let validation = validation_of(&mut session, field);
    assert_eq!(validation["kind"], json!("unsupported"), "the tree does not understand it");
    assert!(validation["min"].as_str().unwrap_or_default().contains("AFNumber_Format"), "and it says so");
}

/// A field that is not there is refused.
#[test]
fn a_field_that_is_not_there_is_refused() {
    let mut session = session();
    load(&mut session, "missing");
    let outcome = session.execute("pdf.field.validate", &json!({ "field": 9999 }));
    assert!(outcome.is_err(), "there is no such field");
}
