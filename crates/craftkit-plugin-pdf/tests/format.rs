//! The **format** of a field.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-format-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose fields say how their text is shown.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R 7 0 R 8 0 R] >>",
        ),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (amount) /Rect [0 0 10 10] /P 3 0 R /AA << /F 9 0 R >> >>",
        ),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (plain) /Rect [0 0 10 10] /P 3 0 R >>"),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (share) /Rect [0 0 10 10] /P 3 0 R /AA << /F 10 0 R >> >>",
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (postal) /Rect [0 0 10 10] /P 3 0 R /AA << /F 11 0 R >> >>",
        ),
        (
            8,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (masked) /Rect [0 0 10 10] /P 3 0 R /AA << /F 12 0 R >> >>",
        ),
        (9, "<< /S /JavaScript /JS (AFNumber_Format(2, 1, 0, 0, \"$\", true)) >>"),
        (10, "<< /S /JavaScript /JS (AFPercent_Format(1, 0)) >>"),
        (11, "<< /S /JavaScript /JS (AFSpecial_Format(0)) >>"),
        (12, "<< /S /JavaScript /JS (AFSpecial_KeystrokeEx(\"999-9999\")) >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 13\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 13 /Root 1 0 R >>\nstartxref\n");
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

/// The **format** of a field, as the command reads it.
fn format_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.format", &json!({ "field": field })).expect("the format is read");
    outcome.data.clone()
}

/// A field is shown as a **number**, with the decimals and the
/// currency the file says.
#[test]
fn a_field_is_shown_as_a_number() {
    let mut session = session();
    load(&mut session, "number");
    let field = field_of(&session, "amount");
    let format = format_of(&mut session, field);
    assert_eq!(format["kind"], json!("number"), "the field is a number");
    assert_eq!(format["decimals"], json!(2), "with two decimals");
    assert_eq!(format["currency"], json!("$"), "in dollars");
    assert_eq!(format["prepend"], json!(true), "written before the number");
}

/// A field is shown as a **percentage**.
#[test]
fn a_field_is_shown_as_a_percentage() {
    let mut session = session();
    load(&mut session, "percent");
    let field = field_of(&session, "share");
    let format = format_of(&mut session, field);
    assert_eq!(format["kind"], json!("percent"), "the field is a percentage");
    assert_eq!(format["decimals"], json!(1), "with one decimal");
}

/// A field is shown as one of the **special forms**.
#[test]
fn a_field_is_shown_as_a_special_form() {
    let mut session = session();
    load(&mut session, "special");
    let field = field_of(&session, "postal");
    let format = format_of(&mut session, field);
    assert_eq!(format["kind"], json!("special"), "the field is a special form");
    assert_eq!(format["form"], json!("postal-code"), "the postal code");
}

/// A field is typed into through a **mask**.
#[test]
fn a_field_is_typed_into_through_a_mask() {
    let mut session = session();
    load(&mut session, "mask");
    let field = field_of(&session, "masked");
    let format = format_of(&mut session, field);
    assert_eq!(format["kind"], json!("mask"), "the field has a mask");
    assert_eq!(format["mask"], json!("999-9999"), "and it is the one the file says");
}

/// A field that says nothing about how it is shown is shown as it is.
#[test]
fn a_field_that_says_nothing_is_shown_as_it_is() {
    let mut session = session();
    load(&mut session, "plain");
    let field = field_of(&session, "plain");
    let format = format_of(&mut session, field);
    assert_eq!(format["kind"], json!("nothing"), "there is no format");
}

/// A field that is not there is refused.
#[test]
fn a_field_that_is_not_there_is_refused() {
    let mut session = session();
    load(&mut session, "missing");
    let outcome = session.execute("pdf.field.format", &json!({ "field": 9999 }));
    assert!(outcome.is_err(), "there is no such field");
}
