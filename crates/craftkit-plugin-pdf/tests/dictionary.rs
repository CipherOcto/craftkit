//! The **dictionary**: a list of pairs, and not an array.
//!
//! A JSON array is one thing, and a PDF's dictionary and a PDF's array
//! are two — so the payload has to tell them apart. It does it with the
//! name the payload already uses for the dictionary of an object: a
//! dictionary is a **list of pairs under `entries`**, and an array is
//! an array.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-dictionary-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **file** whose page carries a dictionary inside its dictionary.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Group << /S /Transparency >> >>"),
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

/// The **entries** of a node's dictionary, as the pairs they are.
fn entries_of(session: &Session, id: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(id).expect("the node is there");
    node.data["entries"]
        .as_array()
        .expect("the node has a dictionary")
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            Some((key, pair.get(1)?.clone()))
        })
        .collect()
}

/// A dictionary inside a dictionary is read as a **dictionary**, and not
/// as an array.
#[test]
fn a_dictionary_is_a_list_of_pairs() {
    let directory = directory("spelling");
    let mut session = session();
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = session
        .document()
        .walk()
        .find(|node| {
            node.kind == "pdf.object" && node.data.get("object").and_then(|number| number.as_u64()) == Some(3)
        })
        .expect("the page is there");
    let entries = entries_of(&session, page.id);
    let group = entries.iter().find(|(key, _)| key == "Group").map(|(_, value)| value.clone());
    assert_eq!(
        group,
        Some(json!({ "entries": [["S", "Transparency"]] })),
        "a dictionary is a list of pairs, and not an array"
    );
    // An **array** is an array.
    let media = entries.iter().find(|(key, _)| key == "MediaBox").map(|(_, value)| value.clone());
    assert_eq!(media, Some(json!([0.0, 0.0, 595.0, 842.0])), "and an array is still an array");
}

/// The dictionary inside a dictionary **survives the round trip**: the
/// file that is written out has a dictionary in it, and not an array.
#[test]
fn a_dictionary_survives_the_round_trip() {
    let directory = directory("round");
    let mut loaded = session();
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    loaded.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let out = format!("{directory}/out.pdf");
    loaded.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    let bytes = std::fs::read(&out).expect("the file is there");
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("/Group << /S /Transparency >>"),
        "the dictionary is written as a dictionary: {text}"
    );
    // And the file that comes out is read back, and the dictionary is
    // still there.
    let mut other = session();
    let again = format!("{directory}/again");
    other.execute("pdf.file.load", &json!({ "path": out, "into": again })).expect("the file is loaded again");
    let page = other
        .document()
        .walk()
        .find(|node| {
            node.kind == "pdf.object" && node.data.get("object").and_then(|number| number.as_u64()) == Some(3)
        })
        .expect("the page is there");
    let entries = entries_of(&other, page.id);
    let group = entries.iter().find(|(key, _)| key == "Group").map(|(_, value)| value.clone());
    assert_eq!(
        group,
        Some(json!({ "entries": [["S", "Transparency"]] })),
        "the dictionary is still a dictionary"
    );
}
