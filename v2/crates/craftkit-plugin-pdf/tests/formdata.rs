//! The **data of a form**: the values of its fields, written as a file
//! and read back from one.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-formdata-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a form in it: two fields, one of them **held** and one
/// of them empty.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [5 0 R 6 0 R] >>"),
        (4, "<< /Fields [5 0 R 6 0 R] /NeedAppearances true >>"),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (who) /V (ann) /Rect [0 0 10 10] /P 3 0 R >>"),
        (6, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (where) /Rect [0 0 10 10] /P 3 0 R >>"),
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

/// Load a file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// A file of this test's own, for the data that is written.
///
/// The directory is made and not emptied: two tests may write their
/// files into it.
fn a_data_file(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-formdata-data-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.join(format!("data-{name}")).to_string_lossy().to_string()
}

/// The **fields** of the form, as the command reads them.
fn fields_of(session: &mut Session) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session.execute("pdf.field.read", &json!({})).expect("the fields are read");
    outcome.data["fields"].as_array().expect("the fields are a list").clone()
}

/// The **XFDF file** of the data is written, and it says what it
/// carries.
#[test]
fn the_xfdf_file_of_the_data_is_written() {
    let mut session = session();
    load(&mut session, "xfdf");
    let path = a_data_file("the-data.xfdf");
    let outcome = session
        .execute("pdf.form.data.export", &json!({ "path": path, "format": "xfdf", "file": "file.pdf" }))
        .expect("the file is written");
    assert_eq!(outcome.data["count"], json!(2), "both fields are in it: {outcome:?}");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"), "it is an XML file: {text}");
    assert!(text.contains("<xfdf xmlns=\"http://ns.adobe.com/xfdf/\""), "and it is an XFDF file: {text}");
    assert!(text.contains("<field name=\"who\">"), "the field is in it: {text}");
    assert!(text.contains("<value>ann</value>"), "with the value it holds: {text}");
    assert!(text.contains("<f href=\"file.pdf\"/>"), "and it says the file it came from: {text}");
}

/// The **table** of the data is written: the names of the fields and
/// their values.
#[test]
fn the_table_of_the_data_is_written() {
    let mut session = session();
    load(&mut session, "csv");
    let path = a_data_file("the-data.csv");
    session
        .execute("pdf.form.data.export", &json!({ "path": path, "format": "csv" }))
        .expect("the file is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "there are two lines: {text}");
    assert_eq!(lines[0], "who,where", "the names of the fields: {text}");
    assert_eq!(lines[1], "ann,", "and their values: {text}");
}

/// The data of a form is **read back** from an XFDF file, and the
/// fields take their value.
#[test]
fn the_data_is_read_back_and_the_fields_take_their_value() {
    let mut session = session();
    load(&mut session, "import");
    let path = a_data_file("the-import.xfdf");
    let written = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xfdf xmlns=\"http://ns.adobe.com/xfdf/\">\n<fields>\n<field name=\"where\"><value>here</value></field>\n</fields>\n</xfdf>\n";
    std::fs::write(&path, written).expect("the file is written");
    let outcome =
        session.execute("pdf.form.data.import", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["count"], json!(1), "one field took its value: {outcome:?}");
    let fields = fields_of(&mut session);
    let where_field =
        fields.iter().find(|field| field["name"] == json!("where")).expect("the field is there");
    assert_eq!(where_field["value"], json!("here"), "and it holds it now: {fields:?}");
}

/// A field the file names and the form does **not** have is left out.
#[test]
fn a_field_the_file_names_and_the_form_does_not_have_is_left_out() {
    let mut session = session();
    load(&mut session, "left-out");
    let path = a_data_file("the-left-out.xfdf");
    let written = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xfdf xmlns=\"http://ns.adobe.com/xfdf/\">\n<fields>\n<field name=\"nowhere\"><value>x</value></field>\n</fields>\n</xfdf>\n";
    std::fs::write(&path, written).expect("the file is written");
    let outcome =
        session.execute("pdf.form.data.import", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["count"], json!(0), "no field took a value: {outcome:?}");
    let left_out = outcome.data["left_out"].as_array().expect("the names are a list");
    assert_eq!(left_out, &vec![json!("nowhere")], "and the name is said: {outcome:?}");
}

/// A file that is **not one** is refused.
#[test]
fn a_file_that_is_not_one_is_refused() {
    let mut session = session();
    load(&mut session, "refused");
    let path = a_data_file("not-xfdf.xfdf");
    std::fs::write(&path, "not an XML file at all").expect("the file is written");
    let refusal =
        session.execute("pdf.form.data.import", &json!({ "path": path })).expect_err("the file is refused");
    let message = refusal.to_string();
    assert!(!message.is_empty(), "the refusal says something: {message}");
}
