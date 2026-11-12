//! The **custom stamp** on a page: the tree's own stamp of a picture.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-customstampadd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
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

/// A **picture** in the document: an image of its own.
fn a_picture(session: &mut Session, directory: &str) -> u64 {
    let bytes = format!("{directory}/picture.bin");
    std::fs::write(&bytes, [0u8; 4]).expect("the picture is written");
    let outcome = session
        .execute(
            "pdf.stream.add",
            &json!({
                "object": 20,
                "entries": [
                    ["Type", "XObject"],
                    ["Subtype", "Image"],
                    ["Width", 2],
                    ["Height", 2],
                ],
                "stream": bytes,
            }),
        )
        .expect("the picture is added");
    outcome.data["id"].as_u64().expect("the picture is there")
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

/// **A custom stamp is put on a page:** the picture of it and the name
/// of it.
#[test]
fn a_custom_stamp_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("stamp");
    let page = a_page(&mut session, &directory);
    let picture = a_picture(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.customstamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "name": "Company Seal",
                "picture": picture,
                "image": true,
            }),
        )
        .expect("the stamp is added");
    assert!(outcome.changed);
    let annot = outcome.data["id"].as_u64().expect("the stamp is there");
    let node = session.document().node(annot).cloned().expect("the stamp is there");
    let entries = entries_of(&node);
    // The **name** of the stamp is the tree's own: `PCCustom` and then
    // the letters and the digits of the name — the spaces are gone.
    assert_eq!(entry(&entries, "Name"), Some(&json!("PCCustomCompanySeal")), "the name is the tree's");
    // The **picture** is the object of the document.
    assert_eq!(entry(&entries, "PCPicture"), Some(&json!({ "ref": [20, 0] })), "the picture is there");
    // And it is an **image**.
    assert_eq!(entry(&entries, "PCPictureImage"), Some(&json!(true)), "the picture is an image");
    // An image needs **no size**: the size is the size a form is
    // mapped to.
    assert_eq!(entry(&entries, "PCPictureSize"), None, "an image has no size");
}

/// **A picture that is not an image** — a form — needs its **size**.
#[test]
fn a_form_needs_its_size() {
    let mut session = session();
    let directory = directory("form");
    let page = a_page(&mut session, &directory);
    let picture = a_picture(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.customstamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "name": "Company Seal",
                "picture": picture,
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("needs its size"), "the error says why: {error}");
    // And with the size it is made.
    let outcome = session
        .execute(
            "pdf.annot.customstamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "name": "Company Seal",
                "picture": picture,
                "size": [120.0, 40.0],
            }),
        )
        .expect("the stamp is added");
    let annot = outcome.data["id"].as_u64().expect("the stamp is there");
    let node = session.document().node(annot).cloned().expect("the stamp is there");
    let entries = entries_of(&node);
    assert_eq!(entry(&entries, "PCPictureImage"), Some(&json!(false)), "the picture is a form");
    assert_eq!(entry(&entries, "PCPictureSize"), Some(&json!([120.0, 40.0])), "the size is there");
}

/// **A name with no letters or digits in it** is refused.
#[test]
fn a_name_with_nothing_in_it_is_refused() {
    let mut session = session();
    let directory = directory("name");
    let page = a_page(&mut session, &directory);
    let picture = a_picture(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.customstamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "name": "  ",
                "picture": picture,
                "image": true,
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("letters or digits"), "the error says why: {error}");
}

/// **A size that is not greater than zero** is refused.
#[test]
fn a_size_that_is_not_greater_than_zero_is_refused() {
    let mut session = session();
    let directory = directory("size");
    let page = a_page(&mut session, &directory);
    let picture = a_picture(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.customstamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "name": "Company Seal",
                "picture": picture,
                "size": [0.0, 40.0],
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("greater than zero"), "the error says why: {error}");
}

/// **A picture that is not in the document** is refused.
#[test]
fn a_picture_that_is_not_there_is_refused() {
    let mut session = session();
    let directory = directory("nopicture");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.customstamp.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "name": "Company Seal",
                "picture": 9999,
                "image": true,
            }),
        )
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("no node"), "the error says why: {error}");
}
