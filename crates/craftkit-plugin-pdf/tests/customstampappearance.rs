//! The **appearance** of a custom stamp: the picture of it, laid over
//! the rectangle.

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
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-customstampappearance-{name}-{}", std::process::id()));
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

/// A **custom stamp** on a page: the node of it.
fn a_stamp(session: &mut Session, directory: &str, image: bool, size: Option<[f64; 2]>) -> u64 {
    let page = a_page(session, directory);
    let picture = a_picture(session, directory);
    let mut params = json!({
        "page": page,
        "object": 10,
        "rect": [100.0, 700.0, 300.0, 740.0],
        "name": "Company Seal",
        "picture": picture,
    });
    if image {
        params["image"] = json!(true);
    }
    if let Some(size) = size {
        params["size"] = json!(size);
    }
    let outcome = session.execute("pdf.annot.customstamp.add", &params).expect("the stamp is added");
    outcome.data["id"].as_u64().expect("the stamp is there")
}

/// **The picture is drawn** over the rectangle of the stamp.
#[test]
fn the_picture_is_drawn() {
    let mut session = session();
    let directory = directory("drawn");
    let stamp = a_stamp(&mut session, &directory, true, None);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect("the stamp is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **picture** is mapped onto the rectangle — two hundred points
    // wide and forty high — and it is drawn under the name `Pic`.
    assert!(bytes.contains("q 200 0 0 40 100 700 cm /Pic Do Q\n"), "the picture is drawn: {bytes}");
}

/// **The picture is in the resources** of the drawing, under the name
/// the drawing uses.
#[test]
fn the_picture_is_in_the_resources() {
    let mut session = session();
    let directory = directory("resources");
    let stamp = a_stamp(&mut session, &directory, true, None);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect("the stamp is drawn");
    let appearance = outcome.data["appearance"].as_u64().expect("the drawing is there");
    let node = session.document().node(appearance).cloned().expect("the drawing is there");
    let entries = entries_of(&node);
    let resources = entry(&entries, "Resources").expect("the resources are there");
    let xobjects = dictionary_of(resources);
    let xobject =
        xobjects.iter().find(|(key, _)| key == "XObject").map(|(_, value)| value).expect("an xobject");
    let names = dictionary_of(xobject);
    let picture = names.iter().find(|(key, _)| key == "Pic").map(|(_, value)| value).expect("the picture");
    assert_eq!(picture, &json!({ "ref": [20, 0] }), "the picture is the object of the document");
}

/// **A picture that is not an image** is drawn by its **size**: the
/// tree's own rule.
#[test]
fn a_form_is_drawn_by_its_size() {
    let mut session = session();
    let directory = directory("form");
    let stamp = a_stamp(&mut session, &directory, false, Some([120.0, 40.0]));
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect("the stamp is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The rectangle is two hundred points wide and the form is a
    // hundred and twenty, so the form is drawn a little larger than
    // itself, and it is as high as the form: forty points.
    assert!(bytes.contains("q 1.6667 0 0 1 100 700 cm /Pic Do Q\n"), "the form is drawn: {bytes}");
}

/// **A stamp without its picture** cannot be drawn: the tree's own
/// name is not a stamp of the palette.
#[test]
fn a_stamp_without_its_picture_is_refused() {
    let mut session = session();
    let directory = directory("nopicture");
    let stamp = a_stamp(&mut session, &directory, true, None);
    session
        .execute("pdf.entry.remove", &json!({ "object": stamp, "key": "PCPicture" }))
        .expect("the picture is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": stamp, "to": to }))
        .expect_err("the stamp is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
