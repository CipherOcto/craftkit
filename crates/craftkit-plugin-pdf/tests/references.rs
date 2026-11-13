//! The **number of a reference**.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-references-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A small **file** whose objects hold references to one another.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Rotate 90 /Annots [4 0 R] /Resources << /Font << /F1 5 0 R >> >> >>",
        ),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (the child) /Parent 6 0 R /Rect [0 0 10 10] /P 3 0 R >>",
        ),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FontDescriptor 7 0 R >>"),
        (6, "<< /FT /Tx /T (the parent) >>"),
        (7, "<< /Type /FontDescriptor /FontName /Helvetica /FontFile 9 0 R >>"),
        (8, "<< /Type /Catalog >>"),
        (9, "<< /Length 3 >>\nstream\nabc\nendstream"),
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
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **page** of the document, by its number.
fn page_of(session: &Session, number: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(number))
        .map(|node| node.id)
        .expect("the page is there")
}

/// The **annotations** of a page, as the command reads them.
fn annotations_of(session: &mut Session, page: u64) -> Vec<Json> {
    let outcome =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    outcome.data["annotations"].as_array().expect("the annotations are a list").clone()
}

/// The **fields** of the document, as the command reads them.
fn fields_of(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("pdf.field.read", &json!({})).expect("the fields are read");
    outcome.data["fields"].as_array().expect("the fields are a list").clone()
}

/// The **fonts** of the document, as the command reads them.
fn fonts_of(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("pdf.fonts", &json!({})).expect("the fonts are read");
    outcome.data["fonts"].as_array().expect("the fonts are a list").clone()
}

/// The reference of a page's **annotations** is followed.
#[test]
fn the_reference_of_a_pages_annotations_is_followed() {
    let mut session = session();
    load(&mut session, "annots");
    let page = page_of(&session, 3);
    let annotations = annotations_of(&mut session, page);
    assert_eq!(annotations.len(), 1, "the annotation is there");
    assert_eq!(annotations[0]["object"], json!(4), "and it is the one the page names");
}

/// The reference of a field's **parent** is followed.
#[test]
fn the_reference_of_a_fields_parent_is_followed() {
    let mut session = session();
    load(&mut session, "parent");
    let fields = fields_of(&mut session);
    let child = fields.iter().find(|field| field["object"] == json!(4)).expect("the child is there");
    assert_eq!(child["name"], json!("the parent.the child"), "the name is qualified");
}

/// The reference of a font's **descriptor** is followed.
#[test]
fn the_reference_of_a_fonts_descriptor_is_followed() {
    let mut session = session();
    load(&mut session, "font");
    let fonts = fonts_of(&mut session);
    let font = fonts.iter().find(|font| font["base"] == json!("Helvetica")).expect("the font is there");
    assert_eq!(font["embedded"], json!(true), "the program is in the file");
}

/// The reference of the **catalog** is followed: the file has a
/// catalog.
#[test]
fn the_reference_of_the_catalog_is_followed() {
    let mut session = session();
    load(&mut session, "catalog");
    let outcome = session.execute("pdf.trailer.read", &json!({})).expect("the trailer is read");
    assert_eq!(outcome.data["encrypted"], json!(false), "the trailer is read");
    let catalog = session
        .document()
        .walk()
        .find(|node| {
            node.kind == "pdf.object"
                && crate_entries(&node.data)
                    .iter()
                    .any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
        })
        .expect("the catalog is there");
    assert!(catalog.id > 0, "the catalog is a node of the document");
}

/// The **rotation** of a page, as the command reads it.
#[test]
fn the_whole_number_of_a_page_is_read() {
    let mut session = session();
    load(&mut session, "rotation");
    let page = page_of(&session, 3);
    let outcome =
        session.execute("pdf.page.rotation", &json!({ "page": page })).expect("the rotation is read");
    assert_eq!(outcome.data["rotate"], json!(90), "the page is turned ninety degrees");
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
