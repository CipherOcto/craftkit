//! The **encryption** of a PDF file, made with AES-256.
//!
//! A file of the handler of revision six: the key of the file is
//! worked out, the salts are worked out, and the strings and the
//! streams are scrambled with AES-256.

use craftkit_sdk::serde_json::json;
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
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let number = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir()
        .join(format!("craftkit-v2-aes256writing-{name}-{}-{number}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **plain file**: a page with a word on it and a title.
fn a_plain_file() -> Vec<u8> {
    let content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET";
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        ),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
        (6, "<< /Title (Hello) /Producer (craftkit) >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let stream_at = file.len();
    file.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    file.extend_from_slice(content);
    file.extend_from_slice(b"\nendstream\nendobj\n");
    let mut rows: Vec<usize> = vec![0; 7];
    rows[1] = offsets[0];
    rows[2] = offsets[1];
    rows[3] = offsets[2];
    rows[4] = stream_at;
    rows[5] = offsets[3];
    rows[6] = offsets[4];
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 7\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in rows.iter().skip(1) {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R /Info 6 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into a session, and give back the **page** it holds.
fn load(session: &mut Session, name: &str, bytes: Vec<u8>) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    the_page(session)
}

/// The **page** of the document.
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

/// A file made with **AES-256** is opened with its password.
#[test]
fn a_file_made_with_aes_256_is_opened_with_its_password() {
    let mut first = session();
    load(&mut first, "plain", a_plain_file());
    // The file is written with its encryption.
    let directory = directory("made");
    let encrypted = format!("{directory}/encrypted.pdf");
    first
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "owner": "owner-me", "path": encrypted.clone(), "cipher": "aes-256" }),
        )
        .expect("the file is written");
    // It says how it is scrambled.
    let mut second = session();
    load(&mut second, "made-encrypted", std::fs::read(&encrypted).expect("the file is there"));
    let encryption = second.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["revision"], json!(6), "the file is of revision six");
    assert_eq!(encryption.data["streams"], json!("aes-256"), "the streams are scrambled with AES-256");
    assert_eq!(encryption.data["strings"], json!("aes-256"), "and the strings too");
    // And it is opened with its password.
    let plain = format!("{directory}/plain.pdf");
    second
        .execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": plain.clone() }))
        .expect("the password opens the file");
    let mut third = session();
    let page = load(&mut third, "made-plain", std::fs::read(&plain).expect("the file is there"));
    let text = third.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
    let info = third.execute("pdf.info.read", &json!({})).expect("the information is read");
    assert_eq!(info.data["Title"], json!("Hello"), "and the title of the file is there too");
}

/// The **owner's password** opens a file made with AES-256.
#[test]
fn the_owner_password_opens_a_file_made_with_aes_256() {
    let mut first = session();
    load(&mut first, "owner-plain", a_plain_file());
    let directory = directory("owner-made");
    let encrypted = format!("{directory}/encrypted.pdf");
    first
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "owner": "owner-me", "path": encrypted.clone(), "cipher": "aes-256" }),
        )
        .expect("the file is written");
    let mut second = session();
    load(&mut second, "owner-encrypted", std::fs::read(&encrypted).expect("the file is there"));
    let plain = format!("{directory}/plain.pdf");
    second
        .execute("pdf.file.decrypt", &json!({ "password": "owner-me", "path": plain.clone() }))
        .expect("the owner's password opens the file");
    let mut third = session();
    let page = load(&mut third, "owner-plain-again", std::fs::read(&plain).expect("the file is there"));
    let text = third.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
}

/// The same file and the same passwords always make the **same**
/// file: the salts and the key of the file are worked out and not
/// drawn at random.
#[test]
fn the_same_file_and_the_same_passwords_make_the_same_file() {
    let mut first = session();
    load(&mut first, "same-one", a_plain_file());
    let one = directory("same-one-file");
    let first_file = format!("{one}/encrypted.pdf");
    first
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "owner": "owner-me", "path": first_file.clone(), "cipher": "aes-256" }),
        )
        .expect("the file is written");
    let mut second = session();
    load(&mut second, "same-two", a_plain_file());
    let two = directory("same-two-file");
    let second_file = format!("{two}/encrypted.pdf");
    second
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "owner": "owner-me", "path": second_file.clone(), "cipher": "aes-256" }),
        )
        .expect("the file is written");
    assert_eq!(
        std::fs::read(&first_file).expect("the file is there"),
        std::fs::read(&second_file).expect("the file is there"),
        "the two files are the same file"
    );
}

/// A **cipher** that is not one of the three is refused.
#[test]
fn a_cipher_that_is_not_one_of_the_three_is_refused() {
    let mut session = session();
    load(&mut session, "refused", a_plain_file());
    let directory = directory("refused-file");
    let path = format!("{directory}/encrypted.pdf");
    let error = session
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "path": path.clone(), "cipher": "rot13" }),
        )
        .expect_err("the cipher is refused");
    assert!(error.to_string().contains("no cipher"), "the refusal says why: {error}");
    assert!(!std::path::Path::new(&path).exists(), "and no file is written");
}
