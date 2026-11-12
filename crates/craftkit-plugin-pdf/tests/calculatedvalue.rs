//! The **value** a field is calculated to hold.

use craftkit_sdk::serde_json::json;
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-calculated-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose field is calculated from two others.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>"),
        (4, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (a) /V (10) /Rect [0 0 10 10] /P 3 0 R >>"),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (b) /V (4) /Rect [0 0 10 10] /P 3 0 R >>"),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (total) /Rect [0 0 10 10] /P 3 0 R /AA << /C 8 0 R >> >>",
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (expression) /Rect [0 0 10 10] /P 3 0 R /AA << /C 9 0 R >> >>",
        ),
        (8, "<< /S /JavaScript /JS (AFSimple_Calculate(\"SUM\", new Array(\"a\", \"b\"))) >>"),
        (9, "<< /S /JavaScript /JS (// BVCALC a + b EVCALC) >>"),
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

/// A file whose field is calculated from a field that is not there.
fn a_file_naming_a_field_that_is_not_there() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R] >>"),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (total) /Rect [0 0 10 10] /P 3 0 R /AA << /C 5 0 R >> >>",
        ),
        (5, "<< /S /JavaScript /JS (AFSimple_Calculate(\"SUM\", new Array(\"nowhere\"))) >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 6\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file of this test's own into the document.
fn load_bytes(session: &mut Session, name: &str, bytes: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/form.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
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
fn text_of(value: &craftkit_sdk::serde_json::Value) -> Option<String> {
    match value {
        craftkit_sdk::serde_json::Value::String(text) => Some(text.clone()),
        craftkit_sdk::serde_json::Value::Object(map) => {
            map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string())
        }
        _ => None,
    }
}

/// The **entries** of a node's data, as the crate reads them.
fn crate_entries(data: &craftkit_sdk::serde_json::Value) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
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

/// The **value** a field is calculated to hold.
fn value_of(session: &mut Session, field: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("pdf.field.value", &json!({ "field": field })).expect("the value is read");
    outcome.data.clone()
}

/// A field is **calculated** to hold the sum of two others.
#[test]
fn a_field_is_calculated_to_hold_the_sum_of_two_others() {
    let mut session = session();
    load(&mut session, "sum");
    let field = field_of(&session, "total");
    let value = value_of(&mut session, field);
    assert_eq!(value["kind"], json!("simple"), "the field is calculated the simple way");
    assert_eq!(value["operation"], json!("SUM"), "and it is a sum");
    assert_eq!(value["value"], json!(14.0), "and it holds fourteen");
}

/// A field that is **not calculated** holds nothing.
#[test]
fn a_field_that_is_not_calculated_holds_nothing() {
    let mut session = session();
    load(&mut session, "none");
    let field = field_of(&session, "a");
    let value = value_of(&mut session, field);
    assert_eq!(value["kind"], json!("nothing"), "there is no calculation");
    assert_eq!(value["value"], json!(null), "and no value");
}

/// An **expression** is not worked out here, and the answer says so.
#[test]
fn an_expression_is_not_worked_out() {
    let mut session = session();
    load(&mut session, "expression");
    let field = field_of(&session, "expression");
    let value = value_of(&mut session, field);
    assert_eq!(value["kind"], json!("notation"), "the field is calculated by an expression");
    assert_eq!(value["value"], json!(null), "and the value is not worked out");
}

/// A field the script names that is **not there** is refused.
#[test]
fn a_field_the_script_names_that_is_not_there_is_refused() {
    let mut session = session();
    load_bytes(&mut session, "nowhere", a_file_naming_a_field_that_is_not_there());
    let field = field_of(&session, "total");
    let outcome = session.execute("pdf.field.value", &json!({ "field": field }));
    assert!(outcome.is_err(), "there is no field called nowhere");
}

/// A field that is **not there** is refused.
#[test]
fn a_field_that_is_not_there_is_refused() {
    let mut session = session();
    load(&mut session, "missing");
    let outcome = session.execute("pdf.field.value", &json!({ "field": 9999 }));
    assert!(outcome.is_err(), "there is no such field");
}
