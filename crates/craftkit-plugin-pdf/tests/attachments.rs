//! The **attachments** of a file.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-attachments-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file that carries a **file** of its own.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /Names 4 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_string()),
        (4, "<< /EmbeddedFiles 5 0 R >>".to_string()),
        (5, "<< /Names [(report.txt) 6 0 R] >>".to_string()),
        (
            6,
            "<< /Type /Filespec /F (report.txt) /UF <FEFF005200650070006F00720074002E007400780074> \
             /Desc (the report) /EF << /F 7 0 R >> >>"
                .to_string(),
        ),
        (
            7,
            "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /Size 11 >> /Length 11 >>\nstream\nhello world\nendstream"
                .to_string(),
        ),
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

/// A file that carries nothing.
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

/// The **attachments** of the file, as the command reads them.
fn attachments_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.attachments.read", &json!({})).expect("the attachments are read");
    outcome.data.clone()
}

/// The files a file carries are read.
#[test]
fn the_files_a_file_carries_are_read() {
    let mut session = session();
    load(&mut session, "carried", a_file());
    let attachments = attachments_of(&mut session);
    assert_eq!(attachments["count"], json!(1), "the file carries one file");
    let one = &attachments["attachments"][0];
    assert_eq!(one["name"], json!("Report.txt"), "the name is the Unicode one");
    assert_eq!(one["description"], json!("the report"), "and it says what it is about");
    assert_eq!(one["mime"], json!("text/plain"), "and what kind of file it is");
    assert_eq!(one["size"], json!(11), "and the size the file says");
    assert_eq!(one["bytes"], json!(11), "and the bytes are there");
    assert_eq!(one["object"], json!(7), "and the object that holds them is named");
}

/// A file that carries nothing has no attachments.
#[test]
fn a_file_that_carries_nothing_has_no_attachments() {
    let mut session = session();
    load(&mut session, "plain", a_plain_file());
    let attachments = attachments_of(&mut session);
    assert_eq!(attachments["count"], json!(0), "the file carries nothing");
    assert_eq!(attachments["attachments"].as_array().map(Vec::len), Some(0), "and there are none");
}

/// An attachment may be asked for **by name**, and one that is not
/// there is refused.
#[test]
fn an_attachment_may_be_asked_for_by_name() {
    let mut session = session();
    load(&mut session, "named", a_file());
    let outcome = session
        .execute("pdf.attachments.read", &json!({ "name": "Report.txt" }))
        .expect("the attachment is read");
    assert_eq!(outcome.data["count"], json!(1), "the attachment is read by its name");
    let missing = session.execute("pdf.attachments.read", &json!({ "name": "other.txt" }));
    assert!(missing.is_err(), "an attachment that is not there is refused");
    let message = missing.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("other.txt"), "and it says which one: {message}");
}
