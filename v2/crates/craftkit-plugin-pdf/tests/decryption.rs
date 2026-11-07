//! The **encryption** of a PDF file, undone with a password.

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
///
/// Every call makes a **directory of its own**: the tests of one file
/// run beside each other, and a directory that two of them shared
/// would be taken away by the second one under the first.
fn directory(name: &str) -> String {
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let number = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-decryption-{name}-{}-{number}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **encrypted file**: a page with a word on it and a title, both
/// scrambled with RC4 under the standard security handler of revision
/// two.
///
/// The user's password is `open-me` and the owner's is `owner-me`.
const ENCRYPTED: &str = concat!(
    "255044462d312e370a312030206f626a0a3c3c202f54797065202f436174616c6f67202f5061",
    "676573203220302052203e3e0a656e646f626a0a322030206f626a0a3c3c202f54797065202f",
    "5061676573202f4b696473205b33203020525d202f436f756e742031203e3e0a656e646f626a",
    "0a332030206f626a0a3c3c202f54797065202f50616765202f506172656e7420322030205220",
    "2f4d65646961426f78205b30203020353935203834325d202f436f6e74656e74732034203020",
    "52202f5265736f7572636573203c3c202f466f6e74203c3c202f4631203520302052203e3e20",
    "3e3e203e3e0a656e646f626a0a342030206f626a0a3c3c202f4c656e677468203236203e3e0a",
    "73747265616d0a8930c0561f134728be68790003adeb00c39b9da43b644f15f4250a656e6473",
    "747265616d0a656e646f626a0a352030206f626a0a3c3c202f54797065202f466f6e74202f53",
    "756274797065202f5479706531202f42617365466f6e74202f48656c766574696361203e3e0a",
    "656e646f626a0a362030206f626a0a3c3c202f46696c746572202f5374616e64617264202f56",
    "2031202f522032202f4f2028565c3332325c33373344675c3234305c3330352f4d5c33313642",
    "205c3234365c303234485c3235354038755c3336305c323736435c3234305c333631625c3336",
    "33205c3233315c333731305c3235375c32323629202f5520285c333731365c3231335c323536",
    "5c3330374d24605c3331355c323135435c3336335c3333355c3335335c303230735c3337375c",
    "3330375c3330354d2e575c3336346e5c3335355c3337305c3230325c3033355c3334365c3237",
    "305c3337355c30303629202f50202d31203e3e0a656e646f626a0a372030206f626a0a3c3c20",
    "2f5469746c65202838775c3231332a5929202f50726f6475636572202863726166746b697429",
    "203e3e0a656e646f626a0a787265660a3020380a303030303030303030302036353533352066",
    "0d0a30303030303030303039203030303030206e0d0a30303030303030303538203030303030",
    "206e0d0a30303030303030313135203030303030206e0d0a3030303030303032343120303030",
    "3030206e0d0a30303030303030333137203030303030206e0d0a303030303030303338372030",
    "30303030206e0d0a30303030303030363335203030303030206e0d0a747261696c65720a3c3c",
    "202f53697a652038202f526f6f74203120302052202f496e666f203720302052202f456e6372",
    "797074203620302052202f4944205b3c30303031303230333034303530363037303830393041",
    "304230433044304530463e203c30303031303230333034303530363037303830393041304230",
    "433044304530463e5d203e3e0a7374617274787265660a3639350a2525454f460a"
);

/// The **bytes** of the encrypted file.
fn encrypted_bytes() -> Vec<u8> {
    let digits: String = ENCRYPTED.chars().filter(|character| character.is_ascii_hexdigit()).collect();
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

/// A file that is **not encrypted**.
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

/// Open the encrypted file with a password, and write it out plain.
///
/// The answer is what the command said, and where it wrote the file.
fn open(session: &mut Session, name: &str, password: &str) -> Result<(Json, String), String> {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, encrypted_bytes()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let plain = format!("{directory}/plain.pdf");
    match session.execute("pdf.file.decrypt", &json!({ "password": password, "path": plain })) {
        Ok(outcome) => Ok((outcome.data.clone(), plain)),
        Err(error) => Err(error.to_string()),
    }
}

/// An encrypted file is opened with its **user's password**, and what
/// it says is said again.
#[test]
fn an_encrypted_file_is_opened_with_its_password() {
    let mut first = session();
    let (data, path) = open(&mut first, "user-file", "open-me").expect("the password opens the file");
    assert_eq!(data["objects"], json!(6), "the file holds six objects without the encryption");
    assert_eq!(data["streams"], json!(1), "and one stream");
    // The file that was written says what the encrypted one said.
    let mut plain = session();
    let page = load(&mut plain, "user-plain", std::fs::read(&path).expect("the file is there"));
    let text = plain.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
    let info = plain.execute("pdf.info.read", &json!({})).expect("the information is read");
    assert_eq!(info.data["Title"], json!("Hello"), "and the title of the file is there too");
}

/// The **owner's password** opens the file as well.
#[test]
fn the_owner_password_opens_the_file_as_well() {
    let mut first = session();
    let (data, path) =
        open(&mut first, "owner-file", "owner-me").expect("the owner's password opens the file");
    assert_eq!(data["objects"], json!(6), "the file holds six objects without the encryption");
    let mut plain = session();
    let page = load(&mut plain, "user-plain", std::fs::read(&path).expect("the file is there"));
    let text = plain.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
}

/// A password that does **not** open the file is refused.
#[test]
fn a_password_that_does_not_open_the_file_is_refused() {
    let mut first = session();
    let error = open(&mut first, "wrong-file", "not-the-password").expect_err("the password is wrong");
    assert!(error.contains("does not open"), "and the command says why: {error}");
}

/// A file that is **not encrypted** is refused.
#[test]
fn a_file_that_is_not_encrypted_is_refused() {
    let mut first = session();
    let directory = directory("plain-file");
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_plain_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    first.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let out = format!("{directory}/plain.pdf");
    let outcome = first.execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": out }));
    assert!(outcome.is_err(), "the file is not encrypted and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("not encrypted"), "and it says why: {error}");
}
