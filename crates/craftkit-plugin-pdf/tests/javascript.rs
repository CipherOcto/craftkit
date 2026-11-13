//! The **JavaScript** of a file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-javascript-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file that holds a **script**.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let script = "app.alert('hello');";
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /Names 4 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_string()),
        (4, "<< /JavaScript 5 0 R >>".to_string()),
        (5, "<< /Names [(hello) 6 0 R] >>".to_string()),
        (6, format!("<< /Length {} >>\nstream\n{script}\nendstream", script.len())),
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

/// A file that holds **no script**.
fn a_plain_file() -> Vec<u8> {
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
fn load(session: &mut Session, name: &str, file: Vec<u8>) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **scripts** of the file, as the command reads them.
fn scripts_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.scripts.read", &json!({})).expect("the scripts are read");
    outcome.data.clone()
}

/// The scripts of a file are read, and they are **not run**.
#[test]
fn the_scripts_of_a_file_are_read() {
    let mut session = session();
    load(&mut session, "script", a_file());
    let scripts = scripts_of(&mut session);
    assert_eq!(scripts["count"], json!(1), "the file holds one script");
    let one = &scripts["scripts"][0];
    assert_eq!(one["name"], json!("hello"), "it is named");
    assert_eq!(one["text"], json!("app.alert('hello');"), "and its text is read");
}

/// A file that holds **no script** has none.
#[test]
fn a_file_that_holds_no_script_has_none() {
    let mut session = session();
    load(&mut session, "plain", a_plain_file());
    let scripts = scripts_of(&mut session);
    assert_eq!(scripts["count"], json!(0), "the file holds no script");
    assert_eq!(scripts["scripts"].as_array().map(Vec::len), Some(0), "and there are none");
}

/// A script may be asked for **by name**, and one that is not there is
/// refused.
#[test]
fn a_script_may_be_asked_for_by_name() {
    let mut session = session();
    load(&mut session, "named", a_file());
    let outcome =
        session.execute("pdf.scripts.read", &json!({ "name": "hello" })).expect("the script is read");
    assert_eq!(outcome.data["count"], json!(1), "the script is read by its name");
    let missing = session.execute("pdf.scripts.read", &json!({ "name": "other" }));
    assert!(missing.is_err(), "a script that is not there is refused");
    let message = missing.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("other"), "and it says which one: {message}");
}
