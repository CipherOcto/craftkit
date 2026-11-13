//! The **AES** of a file: a file scrambled with AES is opened.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-aes-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **AES file**: a page with a word on it and a title, where the
/// streams are scrambled with AES and the strings with RC4.
///
/// The user's password is `open-me` and the owner's is `owner-me`.
const AES: &str = concat!(
    "255044462d312e370a312030206f626a0a3c3c202f54797065202f436174616c6f67202f5061",
    "676573203220302052203e3e0a656e646f626a0a322030206f626a0a3c3c202f54797065202f",
    "5061676573202f4b696473205b33203020525d202f436f756e742031203e3e0a656e646f626a",
    "0a332030206f626a0a3c3c202f54797065202f50616765202f506172656e7420322030205220",
    "2f4d65646961426f78205b30203020353935203834325d202f436f6e74656e74732034203020",
    "52203e3e0a656e646f626a0a342030206f626a0a3c3c202f4c656e677468203438203e3e0a73",
    "747265616d0a101112131415161718191a1b1c1d1e1f096e5d2a7f12278d81c937ac96fd7f61",
    "4720088ee8b630519ddfe18a43c19c940a656e6473747265616d0a656e646f626a0a35203020",
    "6f626a0a3c3c202f5469746c6520285c3032315c3030355c303130785c30303429202f50726f",
    "6475636572202863726166746b697429203e3e0a656e646f626a0a362030206f626a0a3c3c20",
    "2f46696c746572202f5374616e64617264202f562034202f522034202f4c656e677468203132",
    "38202f4346203c3c202f5374644346203c3c202f43464d202f4145535632202f4c656e677468",
    "203136203e3e202f5277203c3c202f43464d202f5632203e3e203e3e202f53746d46202f5374",
    "644346202f53747246202f5277202f4f20285c3331345c303331725c3030375c3031355c3232",
    "315c3234375c3233375c303237225c3237375c3231305c3030315c3234335c3334335c333336",
    "5c333030625c3331365c3231345c333431425c32333570645c3230355c3335345c3234325c32",
    "36345c3331377c5c33373629202f5520285c3030305c323630255c3235305c3231375c323433",
    "225c3030335c5c5c3335315c3232335f2b5c3232305c3336375c3333375c3030305c3030305c",
    "3030305c3030305c3030305c3030305c3030305c3030305c3030305c3030305c3030305c3030",
    "305c3030305c3030305c3030305c30303029202f50202d31203e3e0a656e646f626a0a787265",
    "660a3020370a3030303030303030303020363535333520660d0a303030303030303030392030",
    "30303030206e0d0a30303030303030303538203030303030206e0d0a30303030303030313135",
    "203030303030206e0d0a30303030303030323032203030303030206e0d0a3030303030303033",
    "3030203030303030206e0d0a30303030303030333639203030303030206e0d0a747261696c65",
    "720a3c3c202f53697a652037202f526f6f74203120302052202f496e666f203520302052202f",
    "456e6372797074203620302052202f4944205b3c303030313032303330343035303630373038",
    "30393041304230433044304530463e203c303030313032303330343035303630373038303930",
    "41304230433044304530463e5d203e3e0a7374617274787265660a3735370a2525454f460a"
);

/// The **bytes** of the AES file.
fn aes_bytes() -> Vec<u8> {
    let digits: String = AES.chars().filter(|character| character.is_ascii_hexdigit()).collect();
    let raw: Vec<u8> = digits.as_bytes().to_vec();
    let mut bytes: Vec<u8> = Vec::with_capacity(raw.len() / 2);
    let mut at = 0usize;
    while at + 1 < raw.len() {
        let text = String::from_utf8_lossy(&raw[at..at + 2]).to_string();
        bytes.push(u8::from_str_radix(&text, 16).expect("the digits are digits"));
        at += 2;
    }
    bytes
}

/// Load the AES file into a session.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, aes_bytes()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
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

/// A file scrambled with AES is opened with its password.
#[test]
fn a_file_scrambled_with_aes_is_opened_with_its_password() {
    let mut first = session();
    load(&mut first, "aes");
    // The file says how it is scrambled.
    let encryption = first.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["encrypted"], json!(true), "the file is encrypted");
    assert_eq!(encryption.data["streams"], json!("aes-128"), "the streams are scrambled with AES");
    assert_eq!(encryption.data["strings"], json!("rc4"), "and the strings with RC4");
    // The file is opened with the password.
    let directory = directory("aes-open");
    let plain = format!("{directory}/plain.pdf");
    first
        .execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": plain.clone() }))
        .expect("the password opens the file");
    // And it says what it said.
    let mut second = session();
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, std::fs::read(&plain).expect("the file is there")).expect("the file is written");
    let into = format!("{directory}/streams");
    second.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = the_page(&second);
    let text = text_of(&mut second, page);
    assert_eq!(text["text"], json!("Hello"), "the page says what it said");
    let info = second.execute("pdf.info.read", &json!({})).expect("the information is read");
    assert_eq!(info.data["Title"], json!("Hello"), "and the title of the file is there too");
}

/// The **owner's password** opens the AES file as well.
#[test]
fn the_owner_password_opens_the_aes_file_as_well() {
    let mut first = session();
    load(&mut first, "aes-owner");
    let directory = directory("aes-owner-open");
    let plain = format!("{directory}/plain.pdf");
    first
        .execute("pdf.file.decrypt", &json!({ "password": "owner-me", "path": plain.clone() }))
        .expect("the owner's password opens the file");
    let mut second = session();
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, std::fs::read(&plain).expect("the file is there")).expect("the file is written");
    let into = format!("{directory}/streams");
    second.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = the_page(&second);
    let text = text_of(&mut second, page);
    assert_eq!(text["text"], json!("Hello"), "the page says what it said");
}

/// A password that does **not** open the AES file is refused.
#[test]
fn a_password_that_does_not_open_the_aes_file_is_refused() {
    let mut first = session();
    load(&mut first, "aes-wrong");
    let directory = directory("aes-wrong-open");
    let plain = format!("{directory}/plain.pdf");
    let outcome =
        first.execute("pdf.file.decrypt", &json!({ "password": "not-the-password", "path": plain }));
    assert!(outcome.is_err(), "the password is wrong and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("does not open"), "and it says why: {error}");
}
