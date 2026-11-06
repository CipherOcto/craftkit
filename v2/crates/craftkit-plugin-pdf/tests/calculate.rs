//! The **calculation** of a field.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-calculate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose fields say how their values are worked out.
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
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (total) /Rect [0 0 10 10] /P 3 0 R /AA << /C 8 0 R >> >>",
        ),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (plain) /Rect [0 0 10 10] /P 3 0 R >>"),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (written) /Rect [0 0 10 10] /P 3 0 R /AA << /C << /S /JavaScript /JS (AFSimple_Calculate(\"PRD\", new Array(\"a\", \"b\"))) >> >> >>",
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (expression) /Rect [0 0 10 10] /P 3 0 R /AA << /C 9 0 R >> >>",
        ),
        (8, "<< /S /JavaScript /JS (AFSimple_Calculate(\"SUM\", new Array(\"a\", \"b\"))) >>"),
        (9, "<< /S /JavaScript /JS (// BVCALC Price * Quantity EVCALC) >>"),
        (
            10,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (other) /Rect [0 0 10 10] /P 3 0 R /AA << /C 11 0 R >> >>",
        ),
        (11, "<< /S /JavaScript /JS (app.alert(\"hello\")) >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 12\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 12 /Root 1 0 R >>\nstartxref\n");
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

/// The **calculation** of a field, as the command reads it.
fn calculation_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.calculate", &json!({ "field": field })).expect("the calculation is read");
    outcome.data.clone()
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

/// A field that says how its value is worked out: the way and the
/// fields it is worked out from.
#[test]
fn a_field_says_how_it_is_calculated() {
    let mut session = session();
    load(&mut session, "simple");
    let field = field_of(&session, "total");
    let calculation = calculation_of(&mut session, field);
    assert_eq!(calculation["kind"], json!("simple"), "the way is the simple one");
    assert_eq!(calculation["operation"], json!("SUM"), "and it is a sum");
    assert_eq!(calculation["fields"], json!(["a", "b"]), "of the two fields");
}

/// A field that says nothing about how its value is worked out
/// calculates nothing.
#[test]
fn a_field_that_says_nothing_calculates_nothing() {
    let mut session = session();
    load(&mut session, "plain");
    let field = field_of(&session, "plain");
    let calculation = calculation_of(&mut session, field);
    assert_eq!(calculation["kind"], json!("nothing"), "there is no calculation");
    assert_eq!(calculation["operation"], Json::Null, "and no way");
}

/// The action may be written where it is used, and not named.
#[test]
fn the_action_may_be_written_in_place() {
    let mut session = session();
    load(&mut session, "written");
    let field = field_of(&session, "written");
    let calculation = calculation_of(&mut session, field);
    assert_eq!(calculation["kind"], json!("simple"), "the way is the simple one");
    assert_eq!(calculation["operation"], json!("PRD"), "and it is a product");
}

/// The **expression** a file keeps in a comment is read.
#[test]
fn the_expression_of_a_field_is_read() {
    let mut session = session();
    load(&mut session, "expression");
    let field = field_of(&session, "expression");
    let calculation = calculation_of(&mut session, field);
    assert_eq!(calculation["kind"], json!("notation"), "it is an expression");
    assert_eq!(calculation["expression"], json!("Price * Quantity"), "and it is the one the file kept");
}

/// A script this tree does not understand is said so, and is not
/// pretended to be run.
#[test]
fn a_script_that_is_not_understood_is_reported() {
    let mut session = session();
    load(&mut session, "other");
    let field = field_of(&session, "other");
    let calculation = calculation_of(&mut session, field);
    assert_eq!(calculation["kind"], json!("unsupported"), "the tree does not understand it");
    assert!(calculation["expression"].as_str().unwrap_or_default().contains("app.alert"), "and it says so");
}

/// A field that is not there is refused.
#[test]
fn a_field_that_is_not_there_is_refused() {
    let mut session = session();
    load(&mut session, "missing");
    let outcome = session.execute("pdf.field.calculate", &json!({ "field": 9999 }));
    assert!(outcome.is_err(), "there is no such field");
}
