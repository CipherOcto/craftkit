//! The **appearance** of a field.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-appearance-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The body of a **stream** object: its dictionary and its bytes.
fn stream(dictionary: &str, content: &str) -> String {
    format!("<< {dictionary} /Length {} >>\nstream\n{content}\nendstream", content.len())
}

/// A file whose fields have appearances.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R] >>".to_string()),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (a) /Rect [0 0 100 20] /P 3 0 R /AP << /N 7 0 R >> >>"
                .to_string(),
        ),
        (
            5,
            "<< /Type /Annot /Subtype /Widget /FT /Btn /T (b) /V /On /Rect [0 0 20 20] /P 3 0 R /AP << /N << /Off 8 0 R /On 9 0 R >> >> >>"
                .to_string(),
        ),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (plain) /Rect [0 0 10 10] /P 3 0 R >>".to_string(),
        ),
        (
            7,
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 20] /Matrix [1 0 0 1 10 20]",
                "BT /Helv 9 Tf 12 TL 1 2 Td (hello) Tj ET",
            ),
        ),
        (8, stream("/Type /XObject /Subtype /Form /BBox [0 0 20 20]", "0 0 20 20 re f")),
        (9, stream("/Type /XObject /Subtype /Form /BBox [0 0 20 20]", "0 0 10 10 re f")),
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

/// The **appearance** of a field, as the command reads it.
fn appearance_of(session: &mut Session, field: u64) -> Json {
    let outcome =
        session.execute("pdf.field.appearance", &json!({ "field": field })).expect("the appearance is read");
    outcome.data.clone()
}

/// The **box** an appearance is drawn in, and the matrix that places
/// it, are read.
#[test]
fn the_box_and_the_matrix_of_an_appearance_are_read() {
    let mut session = session();
    load(&mut session, "box");
    let field = field_of(&session, "a");
    let appearance = appearance_of(&mut session, field);
    assert_eq!(appearance["has_appearance"], json!(true), "the field has one");
    assert_eq!(appearance["box"], json!([0.0, 0.0, 100.0, 20.0]), "the box it is drawn in");
    assert_eq!(
        appearance["matrix"],
        json!([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]),
        "and the matrix that places it"
    );
}

/// The **operators** of the appearance are read.
#[test]
fn the_operators_of_an_appearance_are_read() {
    let mut session = session();
    load(&mut session, "operators");
    let field = field_of(&session, "a");
    let appearance = appearance_of(&mut session, field);
    let operations = appearance["operations"].as_array().expect("the operations are a list");
    assert_eq!(operations.first().expect("there is one")["operator"], json!("BT"), "the first is BT");
    assert!(
        operations.iter().any(|operation| operation["operator"] == json!("Tj")),
        "and the text is written"
    );
}

/// An appearance may have **states**, and the value of the field says
/// which one it is in.
#[test]
fn the_states_of_an_appearance_are_read() {
    let mut session = session();
    load(&mut session, "states");
    let field = field_of(&session, "b");
    let appearance = appearance_of(&mut session, field);
    assert_eq!(appearance["states"], json!(["Off", "On"]), "the button has two states");
    assert_eq!(appearance["state"], json!("On"), "and it is in the one it says");
}

/// A field that says nothing about its appearance has none.
#[test]
fn a_field_without_an_appearance_has_none() {
    let mut session = session();
    load(&mut session, "none");
    let field = field_of(&session, "plain");
    let appearance = appearance_of(&mut session, field);
    assert_eq!(appearance["has_appearance"], json!(false), "there is no appearance");
    assert_eq!(appearance["box"], Json::Null, "and no box");
}

/// A field that is not there is refused.
#[test]
fn a_field_that_is_not_there_is_refused() {
    let mut session = session();
    load(&mut session, "missing");
    let outcome = session.execute("pdf.field.appearance", &json!({ "field": 9999 }));
    assert!(outcome.is_err(), "there is no such field");
}
