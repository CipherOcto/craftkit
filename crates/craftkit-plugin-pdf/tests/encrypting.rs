//! The **encryption** of a PDF file, made with a password.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-encrypting-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **plain** file: a page with a word on it and a title.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let content = "BT /F1 12 Tf (Hello) Tj ET";
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, "<< /Title (Hello) /Producer (craftkit) >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 6\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R /Info 5 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into a session, and give the **page** it holds.
fn load(session: &mut Session, name: &str, bytes: Vec<u8>) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    the_page(session)
}

/// The **page** of the document: the node whose type is `Page`.
fn the_page(session: &Session) -> u64 {
    session
        .document()
        .walk()
        .find(|node| {
            node.data["entries"]
                .as_array()
                .map(|entries| {
                    entries.iter().any(|entry| {
                        let Some(pair) = entry.as_array() else { return false };
                        pair.first().and_then(|key| key.as_str()) == Some("Type")
                            && pair.get(1).and_then(|value| value.as_str()) == Some("Page")
                    })
                })
                .unwrap_or(false)
        })
        .map(|node| node.id)
        .expect("the page is there")
}

/// The **text** a page shows, as the command reads it.
fn text_of(session: &mut Session, page: u64) -> Json {
    let outcome = session.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    outcome.data.clone()
}

/// Encrypt a plain file with a password, and give the file that was
/// written.
fn encrypt(name: &str, password: &str, owner: Option<&str>) -> String {
    let mut first = session();
    load(&mut first, &format!("{name}-plain"), a_file());
    let directory = directory(&format!("{name}-out"));
    let path = format!("{directory}/encrypted.pdf");
    let mut params = json!({ "password": password, "path": path.clone() });
    if let Some(owner) = owner {
        params["owner"] = json!(owner);
    }
    first.execute("pdf.file.encrypt", &params).expect("the file is encrypted");
    path
}

/// A file is encrypted with a password, and opened with it again.
#[test]
fn a_file_is_encrypted_with_a_password_and_opened_with_it_again() {
    let path = encrypt("round", "open-me", None);
    let mut second = session();
    let page = load(&mut second, "round-encrypted", std::fs::read(&path).expect("the file is there"));
    // The file says it is encrypted.
    let encryption = second.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["encrypted"], json!(true), "the file is encrypted");
    assert_eq!(encryption.data["revision"], json!(2), "and it is a revision of two");
    assert_eq!(encryption.data["strings"], json!("rc4"), "and it is scrambled with RC4");
    // The text of the page is not the text of the page: the content is
    // scrambled, and the reader refuses a stream it cannot read.
    let scrambled = second.execute("pdf.page.text", &json!({ "page": page }));
    assert!(scrambled.is_err(), "the text of an encrypted page is not read");
    // The file is opened with the password, and it says what it said.
    let directory = directory("round-open");
    let plain = format!("{directory}/plain.pdf");
    second
        .execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": plain.clone() }))
        .expect("the password opens the file");
    let mut third = session();
    let page = load(&mut third, "round-plain", std::fs::read(&plain).expect("the file is there"));
    let text = text_of(&mut third, page);
    assert_eq!(text["text"], json!("Hello"), "the page says what it said");
    let info = third.execute("pdf.info.read", &json!({})).expect("the information is read");
    assert_eq!(info.data["Title"], json!("Hello"), "and the title of the file is there too");
}

/// The **owner's password** opens the file that was encrypted.
#[test]
fn the_owner_password_opens_the_file_that_was_encrypted() {
    let path = encrypt("owned", "open-me", Some("owner-me"));
    let mut second = session();
    load(&mut second, "owned-encrypted", std::fs::read(&path).expect("the file is there"));
    let directory = directory("owned-open");
    let plain = format!("{directory}/plain.pdf");
    second
        .execute("pdf.file.decrypt", &json!({ "password": "owner-me", "path": plain.clone() }))
        .expect("the owner's password opens the file");
    let mut third = session();
    let page = load(&mut third, "owned-plain", std::fs::read(&plain).expect("the file is there"));
    let text = text_of(&mut third, page);
    assert_eq!(text["text"], json!("Hello"), "the page says what it said");
}

/// A password that does **not** open the encrypted file is refused.
#[test]
fn a_password_that_does_not_open_the_encrypted_file_is_refused() {
    let path = encrypt("refused", "open-me", None);
    let mut second = session();
    load(&mut second, "refused-encrypted", std::fs::read(&path).expect("the file is there"));
    let directory = directory("refused-open");
    let plain = format!("{directory}/plain.pdf");
    let outcome =
        second.execute("pdf.file.decrypt", &json!({ "password": "not-the-password", "path": plain }));
    assert!(outcome.is_err(), "the password is wrong and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("does not open"), "and it says why: {error}");
}

/// The **permissions** of the encrypted file are the ones the file
/// says.
#[test]
fn the_permissions_of_the_encrypted_file_are_the_ones_the_file_says() {
    let path = encrypt("permissions", "open-me", None);
    let mut second = session();
    load(&mut second, "permissions-encrypted", std::fs::read(&path).expect("the file is there"));
    let encryption = second.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["permissions"], json!(-1), "every right there is");
    assert_eq!(
        encryption.data["permission_names"],
        json!([
            "print",
            "modify",
            "copy",
            "annotate",
            "fill-forms",
            "extract-for-accessibility",
            "assemble",
            "print-high-quality"
        ]),
        "and the rights are named",
    );
}
