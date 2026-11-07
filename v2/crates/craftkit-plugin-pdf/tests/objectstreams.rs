//! The **objects** of an **object stream**.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-objectstreams-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **compressed file**: the catalog, the pages and the page itself
/// live in an object stream, and the index of the file is an xref
/// stream.
const COMPRESSED: &str = concat!(
    "255044462d312e370a342030206f626a0a3c3c202f54797065202f4f626a53746d202f4e2033",
    "202f4669727374203134202f4c656e67746820313632203e3e0a73747265616d0a3120302032",
    "2033342033203736203c3c202f54797065202f436174616c6f67202f50616765732032203020",
    "52203e3e203c3c202f54797065202f5061676573202f4b696473205b33203020525d202f436f",
    "756e742031203e3e203c3c202f54797065202f50616765202f506172656e7420322030205220",
    "2f4d65646961426f78205b30203020353935203834325d202f436f6e74656e74732035203020",
    "52203e3e200a656e6473747265616d0a656e646f626a0a352030206f626a0a3c3c202f4c656e",
    "677468203236203e3e0a73747265616d0a4254202f4631203132205466202848656c6c6f2920",
    "546a2045540a656e6473747265616d0a656e646f626a0a362030206f626a0a3c3c202f547970",
    "65202f58526566202f57205b31203420325d202f496e646578205b3020375d202f526f6f7420",
    "3120302052202f53697a652037202f4c656e677468203439203e3e0a73747265616d0a000000",
    "0000ffff0200000004000002000000040001020000000400020100000009000001000000fb00",
    "00010000014700000a656e6473747265616d0a656e646f626a0a7374617274787265660a3332",
    "370a2525454f460a"
);

/// The **bytes** of the compressed file.
fn compressed_bytes() -> Vec<u8> {
    let digits: String = COMPRESSED.chars().filter(|character| character.is_ascii_hexdigit()).collect();
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

/// Load the compressed file into a session, and give the **page** it
/// holds.
fn load(session: &mut Session, name: &str) -> (u64, Json) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, compressed_bytes()).expect("the file is written");
    let into = format!("{directory}/streams");
    let outcome =
        session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = session
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
        .expect("the page is there");
    (page, outcome.data.clone())
}

/// The objects of an object stream are read into the document.
#[test]
fn the_objects_of_an_object_stream_are_read_into_the_document() {
    let mut first = session();
    let (page, loaded) = load(&mut first, "compressed");
    assert_eq!(loaded["objects"], json!(6), "the file holds six objects: three of them compressed");
    let text = first.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    assert_eq!(text.data["text"], json!("Hello"), "and the page that was compressed says what it says");
}

/// The references of a **compressed** object are edges of the document.
#[test]
fn the_references_of_a_compressed_object_are_edges_of_the_document() {
    let mut first = session();
    let (page, loaded) = load(&mut first, "references");
    assert_eq!(loaded["relations"], json!(5), "the file holds five references");
    let references = first.execute("pdf.refs", &json!({})).expect("the references are read");
    let listed = references.data["references"].as_array().expect("the references are a list");
    // The page names its parent and its contents.
    let from_the_page: Vec<&Json> =
        listed.iter().filter(|reference| reference["from"] == json!(page)).collect();
    assert_eq!(from_the_page.len(), 2, "the page holds two references");
    let keys: Vec<&Json> = from_the_page.iter().map(|reference| &reference["payload"]["key"]).collect();
    assert!(keys.contains(&&json!("Parent")), "one of them is the parent");
    assert!(keys.contains(&&json!("Contents")), "and one of them is the contents");
}
