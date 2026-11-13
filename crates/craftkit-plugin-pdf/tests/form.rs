//! The **form** of a file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-form-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a form in it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [5 0 R 6 0 R] >>"),
        (
            4,
            "<< /Fields [5 0 R 6 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /SigFlags 3 /CO [6 0 R] /DR << /Font << /Helv 7 0 R >> >> >>",
        ),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (a) /Rect [0 0 10 10] /P 3 0 R >>"),
        (6, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (b) /Rect [0 0 10 10] /P 3 0 R >>"),
        (7, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
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

/// A file with no form in it.
fn a_file_without_a_form() -> Vec<u8> {
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
fn load(session: &mut Session, name: &str, bytes: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **form** of the document, as the command reads it.
fn form_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.form.read", &json!({})).expect("the form is read");
    outcome.data.clone()
}

/// The **fields** of the form are the objects it lists.
#[test]
fn the_form_lists_its_fields() {
    let mut session = session();
    load(&mut session, "fields", a_file());
    let form = form_of(&mut session);
    assert_eq!(form["has_form"], json!(true), "the file has a form");
    assert_eq!(form["fields"], json!([5, 6]), "and it lists its two fields");
    assert_eq!(form["field_count"], json!(2), "and there are two of them");
}

/// The **appearances** may be left to the reader.
#[test]
fn the_form_says_whether_the_appearances_are_needed() {
    let mut session = session();
    load(&mut session, "appearances", a_file());
    let form = form_of(&mut session);
    assert_eq!(form["need_appearances"], json!(true), "the reader is to make them");
}

/// The **default appearance** of a field is read.
#[test]
fn the_default_appearance_of_the_form_is_read() {
    let mut session = session();
    load(&mut session, "appearance", a_file());
    let form = form_of(&mut session);
    assert_eq!(form["default_appearance"], json!("/Helv 12 Tf 0 g"), "the way a field looks");
}

/// The **signature flags** of the form are read.
#[test]
fn the_signature_flags_of_the_form_are_read() {
    let mut session = session();
    load(&mut session, "signatures", a_file());
    let form = form_of(&mut session);
    assert_eq!(form["signature_flags"], json!(3), "the flags are three");
}

/// The **calculation order** of the form is read.
#[test]
fn the_calculation_order_of_the_form_is_read() {
    let mut session = session();
    load(&mut session, "order", a_file());
    let form = form_of(&mut session);
    assert_eq!(form["calculation_order"], json!([6]), "the second field is calculated");
}

/// A file with no form in it has no form.
#[test]
fn a_file_without_a_form_has_no_form() {
    let mut session = session();
    load(&mut session, "without", a_file_without_a_form());
    let form = form_of(&mut session);
    assert_eq!(form["has_form"], json!(false), "there is no form");
    assert_eq!(form["fields"], Json::Null, "and no fields to list");
}

/// One entry of the form may be read by its key.
#[test]
fn one_entry_of_the_form_is_read() {
    let mut session = session();
    load(&mut session, "entry", a_file());
    let outcome = session.execute("pdf.form.read", &json!({ "key": "SigFlags" })).expect("the entry is read");
    assert_eq!(outcome.data["entries"], json!([["SigFlags", 3]]), "the flags are read");
}

/// A key the form does not have is refused.
#[test]
fn a_key_the_form_does_not_have_is_refused() {
    let mut session = session();
    load(&mut session, "refused", a_file());
    let outcome = session.execute("pdf.form.read", &json!({ "key": "Nothing" }));
    assert!(outcome.is_err(), "there is no such entry");
}
