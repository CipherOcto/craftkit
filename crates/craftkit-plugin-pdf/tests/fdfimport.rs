//! The **FDF file** read back: the values of the fields of a form, and
//! the form that takes them.

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
fn a_session() -> Session {
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-fdfin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a form in it: a field and a **button**, and each of them
/// holds what it is given — and nothing when it is given nothing.
fn a_file(who: Option<&str>, state: Option<&str>) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let value = match who {
        Some(value) => format!(" /V ({value})"),
        None => String::new(),
    };
    let held = match state {
        Some(state) => format!(" /V /{state}"),
        None => String::new(),
    };
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [5 0 R 6 0 R] >>".to_string()),
        (4, "<< /Fields [5 0 R 6 0 R] /NeedAppearances true >>".to_string()),
        (
            5,
            format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (who){value} /Rect [0 0 10 10] /P 3 0 R >>"),
        ),
        (
            6,
            format!(
                "<< /Type /Annot /Subtype /Widget /FT /Btn /T (agree){held} /AP << /N << /Yes << >> /Off << >> >> >> /Rect [0 0 10 10] /P 3 0 R >>"
            ),
        ),
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
fn load(session: &mut Session, name: &str, who: Option<&str>, state: Option<&str>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(who, state)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **values** the fields of the form hold, as the command reads them.
fn values_of(session: &mut Session) -> Vec<(String, Option<String>)> {
    let outcome = session.execute("pdf.field.read", &json!({})).expect("the fields are read");
    outcome.data["fields"]
        .as_array()
        .expect("the fields are a list")
        .iter()
        .map(|field| {
            let name = field["name"].as_str().expect("a name").to_string();
            let value = field["value"].as_str().map(|value| value.to_string());
            (name, value)
        })
        .collect()
}

/// The value of a field, as the command reads it.
fn value_of(session: &mut Session, name: &str) -> Option<String> {
    values_of(session).into_iter().find(|(held, _)| held == name).and_then(|(_, value)| value)
}

/// **The FDF file is read back**: the values of its fields are written
/// into the form.
#[test]
fn the_fdf_file_is_read_back() {
    let mut session = a_session();
    load(&mut session, "read", Some("ann"), Some("Yes"));
    let directory = directory("read");
    let path = format!("{directory}/data.fdf");
    session
        .execute("pdf.form.data.export", &json!({ "path": path, "format": "fdf", "file": "file.pdf" }))
        .expect("the file is written");
    // A **form** that holds nothing yet.
    let mut other = a_session();
    load(&mut other, "empty", None, None);
    let outcome = other.execute("pdf.form.data.import", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["count"], json!(2), "both fields took their value: {outcome:?}");
    assert_eq!(value_of(&mut other, "who").as_deref(), Some("ann"), "the field holds what the file said");
    assert_eq!(value_of(&mut other, "agree").as_deref(), Some("Yes"), "and the button holds its state");
}

/// **A button that is off holds no state.**
#[test]
fn a_button_that_is_off_holds_no_state() {
    let mut session = a_session();
    load(&mut session, "off", None, None);
    let directory = directory("off");
    let path = format!("{directory}/data.fdf");
    session
        .execute("pdf.form.data.export", &json!({ "path": path, "format": "fdf" }))
        .expect("the file is written");
    let mut other = a_session();
    load(&mut other, "offform", None, Some("Yes"));
    other.execute("pdf.form.data.import", &json!({ "path": path })).expect("the file is read");
    assert_eq!(value_of(&mut other, "agree"), None, "the button holds nothing");
}

/// **A field the form does not have is left out.**
#[test]
fn a_field_the_form_does_not_have_is_left_out() {
    let mut session = a_session();
    load(&mut session, "left", Some("ann"), Some("Yes"));
    let directory = directory("left");
    let path = format!("{directory}/data.fdf");
    // A file of the tree's own, with a field that the form does not
    // have.
    std::fs::write(
        &path,
        b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [ << /T (nowhere) /V (x) >> ] >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n",
    )
    .expect("the file is written");
    let mut other = a_session();
    load(&mut other, "leftform", None, None);
    let outcome = other.execute("pdf.form.data.import", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["count"], json!(0), "no field took a value: {outcome:?}");
    assert_eq!(outcome.data["left_out"], json!(["nowhere"]), "and the field is said: {outcome:?}");
}

/// **A file that is not an FDF file is refused.**
#[test]
fn a_file_that_is_not_an_fdf_file_is_refused() {
    let mut session = a_session();
    load(&mut session, "notfdf", Some("ann"), Some("Yes"));
    let directory = directory("notfdf");
    let path = format!("{directory}/data.fdf");
    std::fs::write(&path, b"%FDF-1.2\n1 0 obj\n<< /Nothing << >> >>\nendobj\n%%EOF\n")
        .expect("the file is written");
    let refusal =
        session.execute("pdf.form.data.import", &json!({ "path": path })).expect_err("the file is refused");
    let message = refusal.to_string();
    assert!(message.contains("FDF"), "the refusal says so: {message}");
}
