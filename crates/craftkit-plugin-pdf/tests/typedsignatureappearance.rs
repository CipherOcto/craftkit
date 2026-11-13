//! The **appearance** of a typed signature: the filled outlines of it.

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
        craftkit_scratch::dir(&format!("craftkit-v2-typedsignatureappearance-{name}-{}", std::process::id()))
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

/// A **typed signature** on a page: the node of it.
fn a_signature(session: &mut Session, directory: &str, colour: Option<[f64; 3]>) -> u64 {
    let page = a_page(session, directory);
    let mut params = json!({
        "page": page,
        "object": 10,
        "rect": [100.0, 700.0, 300.0, 740.0],
        "contours": [
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]],
        ],
    });
    if let Some(colour) = colour {
        params["colour"] = json!(colour);
    }
    let outcome = session.execute("pdf.annot.typedsignature.add", &params).expect("the signature is added");
    outcome.data["id"].as_u64().expect("the signature is there")
}

/// **The signature is drawn:** the outlines of it, laid over the
/// rectangle and filled with the even-odd rule.
#[test]
fn the_signature_is_drawn() {
    let mut session = session();
    let directory = directory("drawn");
    let signature = a_signature(&mut session, &directory, None);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": signature, "to": to }))
        .expect("the signature is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **fill** is the colour of the signature — the tree's own
    // black — and the outlines are laid over the rectangle: the first
    // contour is the whole of it and the second is the quarter of it.
    assert!(
        bytes.contains("0 0 0 rg\n100 700 m\n300 700 l\n300 740 l\n100 740 l\nh\n"),
        "the first contour: {bytes}"
    );
    assert!(bytes.contains("150 710 m\n250 710 l\n250 730 l\n150 730 l\nh\n"), "the second contour: {bytes}");
    // And the fill is with the **even-odd** rule: the tree's own.
    assert!(bytes.ends_with("f*\n"), "the outlines are filled: {bytes}");
}

/// **The colour** of the signature is the colour it was given.
#[test]
fn the_colour_of_the_signature_is_drawn() {
    let mut session = session();
    let directory = directory("colour");
    let signature = a_signature(&mut session, &directory, Some([0.8, 0.1, 0.1]));
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": signature, "to": to }))
        .expect("the signature is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("0.8 0.1 0.1 rg\n"), "the colour is drawn: {bytes}");
}

/// **A signature without its outlines** cannot be drawn: the tree's
/// own name is not a stamp of the palette.
#[test]
fn a_signature_without_its_outlines_is_refused() {
    let mut session = session();
    let directory = directory("nooutline");
    let signature = a_signature(&mut session, &directory, None);
    session
        .execute("pdf.entry.remove", &json!({ "object": signature, "key": "PCOutline" }))
        .expect("the outlines are taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": signature, "to": to }))
        .expect_err("the signature is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
