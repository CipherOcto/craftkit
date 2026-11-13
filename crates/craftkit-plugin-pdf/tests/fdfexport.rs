//! The **FDF file** of the data of a form: the values of the fields, in
//! the syntax of PDF.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-fdf-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a form in it: a field that **holds** something, a
/// **button** that holds a state, a button that holds none, and a
/// **push button**.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [5 0 R 6 0 R 7 0 R 8 0 R] >>"),
        (4, "<< /Fields [5 0 R 6 0 R 7 0 R 8 0 R] /NeedAppearances true >>"),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (who) /V (ann) /Rect [0 0 10 10] /P 3 0 R >>"),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Btn /T (agree) /V /Yes /AP << /N << /Yes << >> /Off << >> >> >> /Rect [0 0 10 10] /P 3 0 R >>",
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Widget /FT /Btn /T (later) /AP << /N << /Off << >> >> >> /Rect [0 0 10 10] /P 3 0 R >>",
        ),
        (
            8,
            "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (send) /AP << /N << /send << >> >> >> /Rect [0 0 10 10] /P 3 0 R >>",
        ),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 9\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n");
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

/// The **FDF file** of the data, and what it says.
fn the_fdf(session: &mut Session, name: &str) -> (String, usize) {
    let directory = directory(name);
    let path = format!("{directory}/data.fdf");
    let outcome = session
        .execute("pdf.form.data.export", &json!({ "path": path, "format": "fdf", "file": "file.pdf" }))
        .expect("the file is written");
    let count = outcome.data["count"].as_u64().expect("the count is a number") as usize;
    let bytes = std::fs::read(&path).expect("the file is there");
    (String::from_utf8_lossy(&bytes).to_string(), count)
}

/// **The FDF file is written**: the header, the fields and the trailer.
#[test]
fn the_fdf_file_is_written() {
    let mut session = session();
    load(&mut session, "written");
    let (file, count) = the_fdf(&mut session, "written");
    assert!(file.starts_with("%FDF-1.2\n"), "it is an FDF file: {file}");
    assert!(file.contains("1 0 obj\n<< /FDF << /Fields ["), "the object is in it: {file}");
    assert!(file.contains("<< /T (who) /V (ann) >>"), "the field is in it: {file}");
    assert!(file.contains("/F (file.pdf)"), "and it says the file it came from: {file}");
    assert!(
        file.ends_with("endobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"),
        "and it ends as it should: {file}"
    );
    assert_eq!(count, 3, "the three fields that hold something are in it: {file}");
}

/// **A button holds a state, and a state is a name.**
#[test]
fn a_button_holds_a_state_and_a_state_is_a_name() {
    let mut session = session();
    load(&mut session, "button");
    let (file, _count) = the_fdf(&mut session, "button");
    assert!(file.contains("<< /T (agree) /V /Yes >>"), "the state is a name: {file}");
}

/// **A button that holds no state is `Off`.**
#[test]
fn a_button_that_holds_no_state_is_off() {
    let mut session = session();
    load(&mut session, "off");
    let (file, _count) = the_fdf(&mut session, "off");
    assert!(file.contains("<< /T (later) /V /Off >>"), "the state is off: {file}");
}

/// **A push button is left out.**
#[test]
fn a_push_button_is_left_out() {
    let mut session = session();
    load(&mut session, "push");
    let (file, count) = the_fdf(&mut session, "push");
    assert!(!file.contains("(send)"), "the push button is not in it: {file}");
    assert_eq!(count, 3, "and it is not counted: {file}");
}

/// **A format that is not one of the tree's own is refused.**
#[test]
fn a_format_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    load(&mut session, "refused");
    let directory = directory("refused");
    let path = format!("{directory}/data.txt");
    let refusal = session
        .execute("pdf.form.data.export", &json!({ "path": path, "format": "txt" }))
        .expect_err("the format is refused");
    let message = refusal.to_string();
    assert!(message.contains("format"), "the refusal says so: {message}");
}
