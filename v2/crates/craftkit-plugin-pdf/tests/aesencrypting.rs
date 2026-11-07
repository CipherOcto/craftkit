//! The **encryption** of a PDF file, made with AES.
//!
//! A file whose streams are scrambled with **AES** and whose strings
//! are scrambled with **RC4**: the standard security handler of
//! revision four, with the crypt filters that say which is which.

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
        .join(format!("craftkit-v2-aesencrypting-{name}-{}-{number}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **AES file**: a page with a word on it and a title, where the
/// streams are scrambled with AES and the strings with RC4.
///
/// The file was made by another implementation of the standard
/// security handler, and the tree reads it.
///
/// The user's password is `open-me` and the owner's is `owner-me`.
const AES: &str = concat!(
    "255044462d312e370a312030206f626a0a3c3c202f54797065202f436174616c6f67202f",
    "5061676573203220302052203e3e0a656e646f626a0a322030206f626a0a3c3c202f5479",
    "7065202f5061676573202f4b696473205b33203020525d202f436f756e742031203e3e0a",
    "656e646f626a0a332030206f626a0a3c3c202f54797065202f50616765202f506172656e",
    "74203220302052202f4d65646961426f78205b30203020353935203834325d202f436f6e",
    "74656e7473203420302052202f5265736f7572636573203c3c202f466f6e74203c3c202f",
    "4631203520302052203e3e203e3e203e3e0a656e646f626a0a342030206f626a0a3c3c20",
    "2f4c656e677468203634203e3e0a73747265616d0a101112131415161718191a1b1c1d1e",
    "1ffade2a51bf6272cbde4c7713073a9a13cd9fb62684894629ba6fa3051a05141dfc82e1",
    "e437ce47e5afdbf60d85a99f940a656e6473747265616d0a656e646f626a0a352030206f",
    "626a0a3c3c202f54797065202f466f6e74202f53756274797065202f5479706531202f42",
    "617365466f6e74202f48656c766574696361203e3e0a656e646f626a0a362030206f626a",
    "0a3c3c202f46696c746572202f5374616e64617264202f562034202f522034202f4c656e",
    "67746820313238202f4346203c3c202f5374644346203c3c202f43464d202f4145535632",
    "202f4c656e677468203136203e3e202f5277203c3c202f43464d202f5632203e3e203e3e",
    "202f53746d46202f5374644346202f53747246202f5277202f4f20285c3331345c303331",
    "725c3030375c3031355c3232315c3234375c3233375c303237225c3237375c3231305c30",
    "30315c3234335c3334335c3333365c333030625c3331365c3231345c333431425c323335",
    "70645c3230355c3335345c3234325c3236345c3331377c5c33373629202f5520285c3030",
    "305c323630255c3235305c3231375c323433225c3030335c5c5c3335315c3232335f2b5c",
    "3232305c3336375c3333375c3030305c3030305c3030305c3030305c3030305c3030305c",
    "3030305c3030305c3030305c3030305c3030305c3030305c3030305c3030305c3030305c",
    "30303029202f50202d31203e3e0a656e646f626a0a372030206f626a0a3c3c202f546974",
    "6c6520287e7a546d5c2929202f50726f64756365722028556d5967325c3236305c303132",
    "5c30303229203e3e0a656e646f626a0a787265660a3020380a3030303030303030303020",
    "363535333520660d0a30303030303030303039203030303030206e0d0a30303030303030",
    "303538203030303030206e0d0a30303030303030313135203030303030206e0d0a303030",
    "30303030323431203030303030206e0d0a30303030303030333535203030303030206e0d",
    "0a30303030303030343235203030303030206e0d0a303030303030303831332030303030",
    "30206e0d0a747261696c65720a3c3c202f53697a652038202f526f6f7420312030205220",
    "2f496e666f203720302052202f456e6372797074203620302052202f4944205b3c303030",
    "31303230333034303530363037303830393041304230433044304530463e203c30303031",
    "303230333034303530363037303830393041304230433044304530463e5d203e3e0a7374",
    "617274787265660a3838300a2525454f460a",
);

/// The **bytes** of the AES file.
fn aes_bytes() -> Vec<u8> {
    let digits: String = AES.chars().filter(|character| character.is_ascii_hexdigit()).collect();
    let mut bytes: Vec<u8> = Vec::with_capacity(digits.len() / 2);
    let raw: Vec<u8> = digits.as_bytes().to_vec();
    let mut at = 0usize;
    while at + 1 < raw.len() {
        let pair = &raw[at..at + 2];
        let text = String::from_utf8_lossy(pair).to_string();
        bytes.push(u8::from_str_radix(&text, 16).expect("the digits are digits"));
        at += 2;
    }
    bytes
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

/// The file another implementation made is **opened** with its
/// password.
#[test]
fn a_file_another_implementation_made_is_opened() {
    let mut first = session();
    load(&mut first, "fixed", aes_bytes());
    // The file says how it is scrambled.
    let encryption = first.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["encrypted"], json!(true), "the file is encrypted");
    assert_eq!(encryption.data["revision"], json!(4), "and it is of revision four");
    assert_eq!(encryption.data["streams"], json!("aes-128"), "the streams are scrambled with AES");
    assert_eq!(encryption.data["strings"], json!("rc4"), "and the strings with RC4");
    // The file is opened with the password.
    let directory = directory("fixed-open");
    let plain = format!("{directory}/plain.pdf");
    first
        .execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": plain.clone() }))
        .expect("the password opens the file");
    // And it says what it said.
    let mut second = session();
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, std::fs::read(&plain).expect("the file is there")).expect("the file is written");
    let page = load(&mut second, "fixed-plain", std::fs::read(&path).expect("the file is there"));
    let text = second.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
    let info = second.execute("pdf.info.read", &json!({})).expect("the information is read");
    assert_eq!(info.data["Title"], json!("Hello"), "and the title of the file is there too");
}

/// A file **made** with AES is opened with its password.
#[test]
fn a_file_made_with_aes_is_opened_with_its_password() {
    let mut first = session();
    load(&mut first, "plain", a_plain_file());
    // The file is written with its encryption.
    let directory = directory("made");
    let encrypted = format!("{directory}/encrypted.pdf");
    first
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "owner": "owner-me", "path": encrypted.clone(), "cipher": "aes" }),
        )
        .expect("the file is written");
    // It says how it is scrambled.
    let mut second = session();
    load(&mut second, "made-encrypted", std::fs::read(&encrypted).expect("the file is there"));
    let encryption = second.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["revision"], json!(4), "the file is of revision four");
    assert_eq!(encryption.data["streams"], json!("aes-128"), "the streams are scrambled with AES");
    assert_eq!(encryption.data["strings"], json!("rc4"), "and the strings with RC4");
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

/// The **owner's password** opens a file made with AES.
#[test]
fn the_owner_password_opens_a_file_made_with_aes() {
    let mut first = session();
    load(&mut first, "owner-plain", a_plain_file());
    let directory = directory("owner-made");
    let encrypted = format!("{directory}/encrypted.pdf");
    first
        .execute(
            "pdf.file.encrypt",
            &json!({ "password": "open-me", "owner": "owner-me", "path": encrypted.clone(), "cipher": "aes" }),
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

/// A **cipher** that is not one of the two is refused, and nothing is
/// written.
#[test]
fn a_cipher_that_is_not_one_of_the_two_is_refused() {
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

/// A file made **without** a cipher is still scrambled with RC4.
#[test]
fn a_file_made_without_a_cipher_is_scrambled_with_rc4() {
    let mut first = session();
    load(&mut first, "rc4-plain", a_plain_file());
    let directory = directory("rc4-made");
    let encrypted = format!("{directory}/encrypted.pdf");
    first
        .execute("pdf.file.encrypt", &json!({ "password": "open-me", "path": encrypted.clone() }))
        .expect("the file is written");
    let mut second = session();
    load(&mut second, "rc4-encrypted", std::fs::read(&encrypted).expect("the file is there"));
    let encryption = second.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["revision"], json!(2), "the file is of revision two");
    assert_eq!(encryption.data["streams"], json!("rc4"), "and its streams are scrambled with RC4");
    let plain = format!("{directory}/plain.pdf");
    second
        .execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": plain.clone() }))
        .expect("the password opens the file");
    let mut third = session();
    let page = load(&mut third, "rc4-plain-again", std::fs::read(&plain).expect("the file is there"));
    let text = third.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
}
