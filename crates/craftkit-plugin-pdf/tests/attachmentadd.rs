//! The **file attachment** on a page: the tree's own attachment.

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

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-attachmentadd-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with a **page** on it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len())),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 5\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **page**.
fn a_page(session: &mut Session, directory: &str) -> u64 {
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
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

/// The **entries** of the dictionary of a node.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    node.data["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(
    entries: &'a [(String, craftkit_sdk::serde_json::Value)],
    key: &str,
) -> Option<&'a craftkit_sdk::serde_json::Value> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **pairs** of an inline dictionary.
fn dictionary_of(value: &craftkit_sdk::serde_json::Value) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
        })
        .collect()
}

/// A **file** to attach.
fn a_file_to_attach(directory: &str) -> String {
    let path = format!("{directory}/attached.txt");
    std::fs::write(&path, "the bytes of the attachment").expect("the file is written");
    path
}

/// **The attachment is put on a page:** the box that hangs from the
/// point, the icon and the colour.
#[test]
fn the_attachment_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("attachment");
    let page = a_page(&mut session, &directory);
    let path = a_file_to_attach(&directory);
    let outcome = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [100.0, 700.0],
                "name": "attached.txt",
                "path": path,
                "to": format!("{directory}/embedded.bin"),
            }),
        )
        .expect("the attachment is added");
    assert!(outcome.changed);
    let annot = outcome.data["id"].as_u64().expect("the attachment is there");
    let node = session.document().node(annot).cloned().expect("the attachment is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("FileAttachment")), "the attachment is an attachment");
    // The **box** of twenty points that hangs from the point.
    assert_eq!(entry(&entries, "Rect"), Some(&json!([100.0, 680.0, 120.0, 700.0])), "the box");
    // The **tree's own** blue and the tree's own pin.
    assert_eq!(entry(&entries, "C"), Some(&json!([0.0, 0.47, 0.84])), "the colour is the tree's");
    assert_eq!(entry(&entries, "Name"), Some(&json!("PushPin")), "the icon is the tree's");
}

/// **The file** is carried by the document: an embedded file of its own
/// and a file specification that names it.
#[test]
fn the_file_is_carried_by_the_document() {
    let mut session = session();
    let directory = directory("file");
    let page = a_page(&mut session, &directory);
    let path = a_file_to_attach(&directory);
    let outcome = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [100.0, 700.0],
                "name": "attached.txt",
                "path": path,
                "to": format!("{directory}/embedded.bin"),
            }),
        )
        .expect("the attachment is added");
    // The **bytes** of the file are written out, and the answer says
    // how many there were.
    assert_eq!(outcome.data["bytes"], json!(27), "the bytes are counted");
    let written = std::fs::read_to_string(format!("{directory}/embedded.bin")).expect("the bytes are there");
    assert_eq!(written, "the bytes of the attachment", "the bytes are the file's");
    // The **embedded file** is a stream of its own and says its size.
    let file = outcome.data["file"].as_u64().expect("the embedded file is there");
    let node = session.document().node(file).cloned().expect("the embedded file is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Type"), Some(&json!("EmbeddedFile")), "it is an embedded file");
    let params = entry(&entries, "Params").expect("it has parameters");
    assert!(
        dictionary_of(params).iter().any(|(key, value)| key == "Size" && value.as_u64() == Some(27)),
        "the size is the tree's"
    );
    // The **file specification** names the file and the embedded file.
    let spec = outcome.data["specification"].as_u64().expect("the specification is there");
    let node = session.document().node(spec).cloned().expect("the specification is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "Type"), Some(&json!("Filespec")), "it is a file specification");
    assert_eq!(entry(&entries, "F"), Some(&json!({ "string": "attached.txt" })), "the name is the tree's");
    let embedded = entry(&entries, "EF").expect("it names the embedded file");
    assert_eq!(
        embedded["entries"][0][1],
        json!({ "ref": [outcome.data["file_object"].as_u64().unwrap(), 0] }),
        "the embedded file is named"
    );
}

/// **The name of the attached file** must not be empty.
#[test]
fn a_file_with_no_name_is_refused() {
    let mut session = session();
    let directory = directory("noname");
    let page = a_page(&mut session, &directory);
    let path = a_file_to_attach(&directory);
    let error = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [100.0, 700.0],
                "name": "  ",
                "path": path,
                "to": format!("{directory}/embedded.bin"),
            }),
        )
        .expect_err("the attachment is refused");
    assert!(error.to_string().contains("needs a name"), "the error says why: {error}");
}

/// **A file that is not there** is refused.
#[test]
fn a_file_that_is_not_there_is_refused() {
    let mut session = session();
    let directory = directory("nofile");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [100.0, 700.0],
                "name": "attached.txt",
                "path": format!("{directory}/nowhere.txt"),
                "to": format!("{directory}/embedded.bin"),
            }),
        )
        .expect_err("the attachment is refused");
    assert!(error.to_string().contains("cannot be read"), "the error says why: {error}");
}

/// **An icon that is not one of the tree's four** is refused.
#[test]
fn an_icon_that_is_not_known_is_refused() {
    let mut session = session();
    let directory = directory("noicon");
    let page = a_page(&mut session, &directory);
    let path = a_file_to_attach(&directory);
    let error = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [100.0, 700.0],
                "name": "attached.txt",
                "path": path,
                "to": format!("{directory}/embedded.bin"),
                "icon": "Rocket",
            }),
        )
        .expect_err("the attachment is refused");
    assert!(error.to_string().contains("is not an icon"), "the error says why: {error}");
}

/// **A point that is not two numbers** is refused.
#[test]
fn a_point_that_is_not_two_numbers_is_refused() {
    let mut session = session();
    let directory = directory("nopoint");
    let page = a_page(&mut session, &directory);
    let path = a_file_to_attach(&directory);
    let error = session
        .execute(
            "pdf.annot.attachment.add",
            &json!({
                "page": page,
                "object": 10,
                "at": [100.0],
                "name": "attached.txt",
                "path": path,
                "to": format!("{directory}/embedded.bin"),
            }),
        )
        .expect_err("the attachment is refused");
    assert!(error.to_string().contains("two numbers"), "the error says why: {error}");
}
