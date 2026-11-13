//! The **handler of revision six**: a file scrambled with AES-256.
//!
//! The standard security handler of PDF 2.0: the key of the file is
//! not the hash of the password, and the passwords are checked with a
//! hardened hash that takes many rounds to come out.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-revision6-{name}-{}-{number}", std::process::id()))
            .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **file**: a page with a word on it and a title, scrambled with
/// AES-256 by the handler of revision six.
///
/// The file was made by another implementation of the handler, and the
/// tree reads it.
///
/// The user's password is `open-me` and the owner's is `owner-me`.
const REVISION_SIX: &str = concat!(
    "255044462d322e300a312030206f626a0a3c3c202f54797065202f436174616c6f67202f",
    "5061676573203220302052203e3e0a656e646f626a0a322030206f626a0a3c3c202f5479",
    "7065202f5061676573202f4b696473205b33203020525d202f436f756e742031203e3e0a",
    "656e646f626a0a332030206f626a0a3c3c202f54797065202f50616765202f506172656e",
    "74203220302052202f4d65646961426f78205b30203020353935203834325d202f436f6e",
    "74656e7473203420302052202f5265736f7572636573203c3c202f466f6e74203c3c202f",
    "4631203520302052203e3e203e3e203e3e0a656e646f626a0a342030206f626a0a3c3c20",
    "2f4c656e677468203634203e3e0a73747265616d0a303132333435363738393a3b3c3d3e",
    "3f490a53f1562e78a53079be102e10f9675e4c25b353e0b720fcd659c5b761d84ff95060",
    "6494a07c81fc075c83192ea5e70a656e6473747265616d0a656e646f626a0a352030206f",
    "626a0a3c3c202f54797065202f466f6e74202f53756274797065202f5479706531202f42",
    "617365466f6e74202f48656c766574696361203e3e0a656e646f626a0a362030206f626a",
    "0a3c3c202f46696c746572202f5374616e64617264202f562035202f522036202f4c656e",
    "67746820323536202f50202d31202f456e63727970744d65746164617461207472756520",
    "2f4346203c3c202f5374644346203c3c202f54797065202f437279707446696c74657220",
    "2f43464d202f4145535633202f417574684576656e74202f446f634f70656e202f4c656e",
    "677468203332203e3e203e3e202f53746d46202f5374644346202f53747246202f537464",
    "4346202f4f20284765725c3032365c333433665c3230364e315c3237305c3237345c3137",
    "37445c3234347b5c3233336c5c333135575c3235365c3237305c3335365c3336335c3031",
    "315c3335365e435c3335305c3236365c3236365c3030325c30323720212223242526275c",
    "285c292a2b2c2d2e2f29202f5520285c3234355c3230325c3031325c3333305c3031305c",
    "33353660235c3232305c3334305c3233345c303032595c3032355c3336305c333230252f",
    "5c3237345c3335375c3032315c33333220263b69325c3033355c3232375c3030355c3334",
    "35435c3032305c3032315c3032325c3032335c3032345c3032355c3032365c3032375c30",
    "33305c3033315c3033325c3033335c3033345c3033355c3033365c30333729202f4f4520",
    "285c3335355c323534705c3032305c3031315c303333605c33303133577e73625c333236",
    "5c3230325c3234305c3330305c3230325c323530385c323330307e795c3231375c323430",
    "5c323535425c3337305c3030375c3231362d29202f554520285c3232345c323635535c32",
    "3737745c3230355c3236365c3330335c3336355c303335235c3030325c3330305c333631",
    "7d3b2f5c3335305c3230355c3236345c3336344d5c3032355c3334355c3336305c323534",
    "5c3236345c3237303f395c3030344629202f5065726d7320287d5c33353579655c333030",
    "5c333136415c3331344e3a5c3236365c3237305c333035785c3232335c33323729203e3e",
    "0a656e646f626a0a372030206f626a0a3c3c202f5469746c652028303132333435363738",
    "393a3b3c3d3e3f625c3337335c3231345c3337375c3032355c323437425c3337375c3334",
    "365c3331325c3236325c3237305c323330305c3332375729203e3e0a656e646f626a0a78",
    "7265660a3020380a3030303030303030303020363535333520660d0a3030303030303030",
    "3039203030303030206e0d0a30303030303030303538203030303030206e0d0a30303030",
    "303030313135203030303030206e0d0a30303030303030323431203030303030206e0d0a",
    "30303030303030333535203030303030206e0d0a30303030303030343235203030303030",
    "206e0d0a30303030303031313630203030303030206e0d0a747261696c65720a3c3c202f",
    "53697a652038202f526f6f74203120302052202f496e666f203720302052202f456e6372",
    "797074203620302052202f4944205b3c3030303130323033303430353036303730383039",
    "3041304230433044304530463e203c303030313032303330343035303630373038303930",
    "41304230433044304530463e5d203e3e0a7374617274787265660a313235390a2525454f",
    "460a",
);

/// The **bytes** of the file.
fn bytes() -> Vec<u8> {
    let digits: String = REVISION_SIX.chars().filter(|character| character.is_ascii_hexdigit()).collect();
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

/// Load the file into a session.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
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

/// The file scrambled with **AES-256** is opened with its password.
#[test]
fn a_file_scrambled_with_aes_256_is_opened() {
    let mut first = session();
    load(&mut first, "fixed");
    // The file says how it is scrambled.
    let encryption = first.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    assert_eq!(encryption.data["encrypted"], json!(true), "the file is encrypted");
    assert_eq!(encryption.data["revision"], json!(6), "and it is of revision six");
    assert_eq!(encryption.data["streams"], json!("aes-256"), "the streams are scrambled with AES-256");
    assert_eq!(encryption.data["strings"], json!("aes-256"), "and the strings too");
    // The file is opened with the password.
    let directory = directory("fixed-open");
    let plain = format!("{directory}/plain.pdf");
    first
        .execute("pdf.file.decrypt", &json!({ "password": "open-me", "path": plain.clone() }))
        .expect("the password opens the file");
    // And it says what it said.
    let mut second = session();
    let path = format!("{directory}/file.pdf");
    std::fs::copy(&plain, &path).expect("the file is copied");
    let into = format!("{directory}/streams");
    second.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = the_page(&second);
    let text = second.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
    let info = second.execute("pdf.info.read", &json!({})).expect("the information is read");
    assert_eq!(info.data["Title"], json!("Hello"), "and the title of the file is there too");
}

/// The **owner's password** opens the file too.
#[test]
fn the_owner_password_opens_the_file() {
    let mut first = session();
    load(&mut first, "owner");
    let directory = directory("owner-open");
    let plain = format!("{directory}/plain.pdf");
    first
        .execute("pdf.file.decrypt", &json!({ "password": "owner-me", "path": plain.clone() }))
        .expect("the owner's password opens the file");
    let mut second = session();
    let path = format!("{directory}/file.pdf");
    std::fs::copy(&plain, &path).expect("the file is copied");
    let into = format!("{directory}/streams");
    second.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = the_page(&second);
    let text = second.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "the page says what it said");
}

/// A password that does not open the file is **refused**.
#[test]
fn a_password_that_does_not_open_the_file_is_refused() {
    let mut session = session();
    load(&mut session, "wrong");
    let directory = directory("wrong-open");
    let plain = format!("{directory}/plain.pdf");
    let error = session
        .execute("pdf.file.decrypt", &json!({ "password": "not-me", "path": plain }))
        .expect_err("the password is refused");
    assert!(error.to_string().contains("does not open"), "the refusal says why: {error}");
}

/// A password with a **letter outside ASCII** is refused: the
/// preparation of the specification is not here.
#[test]
fn a_password_with_a_letter_outside_ascii_is_refused() {
    let mut session = session();
    load(&mut session, "letter");
    let directory = directory("letter-open");
    let plain = format!("{directory}/plain.pdf");
    let error = session
        .execute("pdf.file.decrypt", &json!({ "password": "open-mé", "path": plain }))
        .expect_err("the password is refused");
    assert!(error.to_string().contains("preparation"), "the refusal says why: {error}");
}
